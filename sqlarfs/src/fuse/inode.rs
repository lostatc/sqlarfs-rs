use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::table::IdTable;

type InodeGeneration = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InodeNumber(u64);

impl From<u64> for InodeNumber {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<InodeNumber> for u64 {
    fn from(inode: InodeNumber) -> Self {
        inode.0
    }
}

#[derive(Debug)]
pub struct InodeTable {
    // The table which uniquely allocates integers to act as inode numbers.
    id_table: IdTable<InodeNumber>,

    // A map of inode numbers to the set of paths which refer to the inode.
    paths: HashMap<InodeNumber, HashSet<PathBuf>>,

    // A map of inode numbers to their generations.
    //
    // Generations are a concept in libfuse in which an additional integer ID is associated with
    // each inode to ensure they're unique even when the inode values are reused.
    //
    // If an inode is not in this map, its generation is `0`.
    generations: HashMap<InodeNumber, InodeGeneration>,
}

impl InodeTable {
    pub fn new(root: &Path) -> Self {
        let fuse_root_id = InodeNumber::from(fuser::FUSE_ROOT_ID);

        let mut table = Self {
            id_table: IdTable::new(vec![fuse_root_id]),
            paths: HashMap::new(),
            generations: HashMap::new(),
        };

        // Add the root entry to the table.
        let mut root_paths = HashSet::new();
        root_paths.insert(root.to_owned());
        table.paths.insert(fuse_root_id, root_paths);

        table
    }
}
