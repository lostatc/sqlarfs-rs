use std::path::Path;

use crate::Archive;

use super::inode::InodeTable;

#[derive(Debug)]
pub struct FuseAdapter<'conn, 'ar> {
    archive: &'ar mut Archive<'conn>,

    /// A table for allocating inodes.
    inodes: InodeTable,
}

impl<'conn, 'ar> FuseAdapter<'conn, 'ar> {
    pub fn new(archive: &'ar mut Archive<'conn>, root: &Path) -> crate::Result<Self> {
        let mut inodes = InodeTable::new(root);

        Ok(Self { archive, inodes })
    }
}
