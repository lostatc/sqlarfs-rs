use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use fuser::{FileAttr, ReplyAttr, ReplyDirectory, ReplyEmpty, ReplyOpen, Request};
use nix::libc;

use super::error::{try_option, try_result};
use super::handle::{DirectoryEntry, DirectoryHandle, HandleState, HandleTable};
use super::inode::{Ino, InodeTable};
use crate::{Archive, FileMetadata, FileType, ListOptions};

// Generations are a concept in libfuse in which an additional integer ID is associated with each
// inode to ensure they're unique even when the inode numbers are reused.
//
// However, because this is a read-only filesystem, we never reuse inode numbers. Even if a file is
// removed from the archive by another process, we still keep the inode number allocated.
const DEFAULT_GENERATION: u64 = 0;

// The block size used to calculate `st_blocks`.
const BLOCK_SIZE: u32 = 512;

// The value of `st_rdev` value to use if the file is not a character or block device (which will
// always be the case for SQLite archives).
const NON_SPECIAL_RDEV: u32 = 0;

// TODO: What TTL should we use? Can the contents of the archive be modified out from under FUSE?
const DEFAULT_TTL: Duration = Duration::ZERO;

impl FileMetadata {
    fn mode_or_default(&self) -> u16 {
        self.mode()
            .map(|mode| {
                mode.bits()
                    .try_into()
                    .expect("Expected file mode to fit into a u16. This is a bug.")
            })
            .unwrap_or_else(|| match self.kind() {
                FileType::File => 0o644,
                FileType::Dir => 0o755,
                FileType::Symlink => 0o777,
            })
    }
}

#[derive(Debug)]
pub struct FuseAdapter<'conn, 'ar> {
    archive: &'ar mut Archive<'conn>,
    inodes: InodeTable,
    handles: HandleTable,
}

impl<'conn, 'ar> FuseAdapter<'conn, 'ar> {
    fn get_attrs(&mut self, req: &Request, inode: Ino) -> crate::Result<FileAttr> {
        let file_path = self.inodes.path(inode).ok_or(crate::Error::FileNotFound {
            // We don't have a path to include in this error message, so we just use the inode.
            // Users should never be seeing this error message regardless.
            path: PathBuf::from(Into::<u64>::into(inode).to_string()),
        })?;

        let metadata = self.archive.open(file_path)?.metadata()?;

        let size = match &metadata {
            FileMetadata::File { size, .. } => *size,
            FileMetadata::Dir { .. } => 0,
            // The `st_size` of a symlink should be the length of the pathname it contains.
            FileMetadata::Symlink { target, .. } => target.as_os_str().len() as u64,
        };

        let now = SystemTime::now();

        Ok(FileAttr {
            ino: inode.into(),
            size,
            blocks: size / u64::from(BLOCK_SIZE),
            atime: now,
            mtime: metadata.mtime().unwrap_or(now),
            ctime: now,
            crtime: now,
            kind: match &metadata.kind() {
                crate::FileType::File => fuser::FileType::RegularFile,
                crate::FileType::Dir => fuser::FileType::Directory,
                crate::FileType::Symlink => fuser::FileType::Symlink,
            },
            perm: metadata.mode_or_default(),
            // SQLite archives don't support hard links.
            nlink: 0,
            uid: req.uid(),
            gid: req.gid(),
            rdev: NON_SPECIAL_RDEV,
            blksize: BLOCK_SIZE,
            flags: 0,
        })
    }
}

impl<'conn, 'ar> FuseAdapter<'conn, 'ar> {
    pub fn new(archive: &'ar mut Archive<'conn>, root: &Path) -> crate::Result<Self> {
        Ok(Self {
            archive,
            inodes: InodeTable::new(root),
            handles: HandleTable::new(),
        })
    }

    fn getattr(&mut self, req: &Request, ino: u64, reply: ReplyAttr) {
        let attr = try_result!(self.get_attrs(req, ino.into()), reply);
        reply.attr(&DEFAULT_TTL, &attr);
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

    fn readdir(
        &mut self,
        _req: &Request,
        _ino: u64,
        fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let entries = match self.handles.state(fh.into()) {
            None => {
                reply.error(libc::EBADF);
                return;
            }
            Some(HandleState::File(_)) => {
                reply.error(libc::ENOTDIR);
                return;
            }
            Some(HandleState::Directory(DirectoryHandle { entries })) => entries,
        };

        for (i, dir_entry) in entries[offset as usize..].iter().enumerate() {
            if reply.add(
                dir_entry.inode.into(),
                (i + 1) as i64,
                dir_entry.file_type,
                &dir_entry.file_name,
            ) {
                break;
            }
        }

        reply.ok();
    }

    fn releasedir(&mut self, _req: &Request, _ino: u64, fh: u64, _flags: i32, reply: ReplyEmpty) {
        self.handles.close(fh.into());
        reply.ok()
    }
}
