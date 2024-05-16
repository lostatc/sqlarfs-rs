use std::path::Path;

use fuser::{ReplyOpen, Request};
use nix::libc;

use super::error::{try_option, try_result};
use super::handle::{DirectoryEntry, DirectoryHandle, HandleState, HandleTable};
use super::inode::InodeTable;
use crate::{Archive, ListOptions};

#[derive(Debug)]
pub struct FuseAdapter<'conn, 'ar> {
    archive: &'ar mut Archive<'conn>,
    inodes: InodeTable,
    handles: HandleTable,
}

impl<'conn, 'ar> FuseAdapter<'conn, 'ar> {
    pub fn new(archive: &'ar mut Archive<'conn>, root: &Path) -> crate::Result<Self> {
        Ok(Self {
            archive,
            inodes: InodeTable::new(root),
            handles: HandleTable::new(),
        })
    }

    fn opendir(&mut self, _req: &Request, ino: u64, _flags: i32, reply: ReplyOpen) {
        let dir_path = try_option!(self.inodes.path(ino.into()), reply, libc::ENOENT);

        let dir = try_result!(self.archive.open(dir_path), reply);
        let meatadata = try_result!(dir.metadata(), reply);

        // Listing the children of a regular file does not normally return an error, so we need to
        // handle this error case specially.
        if !meatadata.is_dir() {
            reply.error(libc::ENOTDIR);
            return;
        }

        let list_opts = ListOptions::new().children_of(dir_path);
        let archive_entries = try_result!(self.archive.list_with(&list_opts), reply);

        let fuse_entries = archive_entries.map(|entry| {
            let entry = entry?;

            let file_name = entry
                .path()
                .file_name()
                .expect("Expected directory entry to have a file name. This is a bug.")
                .to_string_lossy()
                .into_owned();

            let file_type = match entry.metadata.kind() {
                crate::FileType::File => fuser::FileType::RegularFile,
                crate::FileType::Dir => fuser::FileType::Directory,
                crate::FileType::Symlink => fuser::FileType::Symlink,
            };

            let inode = self.inodes.insert(entry.path().to_owned());

            crate::Result::Ok(DirectoryEntry {
                file_name,
                file_type,
                inode,
            })
        });

        let entries = try_result!(fuse_entries.collect::<crate::Result<Vec<_>>>(), reply);

        let state = HandleState::Directory(DirectoryHandle { entries });
        let fh = self.handles.open(state);

        reply.opened(fh.into(), 0);

        todo!();
    }
}
