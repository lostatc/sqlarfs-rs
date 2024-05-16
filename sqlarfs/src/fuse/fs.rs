use std::path::Path;

use crate::Archive;

use super::handle::HandleTable;
use super::inode::InodeTable;

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
}
