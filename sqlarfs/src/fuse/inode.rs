use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::table::IdTable;

// An inode number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ino(u64);

impl From<u64> for Ino {
    fn from(inode: u64) -> Self {
        Self(inode)
    }
}

impl From<Ino> for u64 {
    fn from(inode: Ino) -> Self {
        inode.0
    }
}

// A table for allocating inodes.
#[derive(Debug)]
pub struct InodeTable {
    table: IdTable<Ino>,
    paths: HashMap<Ino, PathBuf>,
}

impl InodeTable {
    pub fn new(root: &Path) -> Self {
        let fuse_root_id = Ino::from(fuser::FUSE_ROOT_ID);

        let mut table = Self {
            table: IdTable::new(vec![fuse_root_id]),
            paths: HashMap::new(),
        };

        table.paths.insert(fuse_root_id, root.to_owned());

        table
    }

    pub fn insert(&mut self, path: PathBuf) -> Ino {
        let inode = self.table.next();

        self.paths.insert(inode, path);

        inode
    }

    pub fn path(&self, inode: Ino) -> Option<&Path> {
        self.paths.get(&inode).map(PathBuf::as_path)
    }
}