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
    paths_by_inode: HashMap<Ino, PathBuf>,
    inodes_by_path: HashMap<PathBuf, Ino>,
}

impl InodeTable {
    pub fn new(root: &Path) -> Self {
        let fuse_root_id = Ino::from(fuser::FUSE_ROOT_ID);

        let mut table = Self {
            table: IdTable::new(vec![fuse_root_id]),
            paths_by_inode: HashMap::new(),
            inodes_by_path: HashMap::new(),
        };

        table.paths_by_inode.insert(fuse_root_id, root.to_owned());

        table
    }

    pub fn insert(&mut self, path: PathBuf) -> Ino {
        let inode = self.table.next();

        self.paths_by_inode.insert(inode, path.clone());
        self.inodes_by_path.insert(path, inode);

        inode
    }

    pub fn path(&self, inode: impl Into<Ino>) -> Option<&Path> {
        self.paths_by_inode.get(&inode.into()).map(PathBuf::as_path)
    }

    pub fn inode(&self, path: &Path) -> Option<Ino> {
        self.inodes_by_path.get(path).copied()
    }
}
