use std::collections::HashMap;

use nix::fcntl::OFlag;

use super::inode::Ino;
use super::table::IdTable;

// A file handle number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fh(u64);

impl From<u64> for Fh {
    fn from(handle: u64) -> Self {
        Self(handle)
    }
}

impl From<Fh> for u64 {
    fn from(handle: Fh) -> Self {
        handle.0
    }
}

#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub file_name: String,
    pub file_type: fuser::FileType,
    pub inode: Ino,
}

#[derive(Debug, Clone)]
pub struct FileHandle {
    // The flags the file was openend with.
    pub flags: OFlag,

    // The current seek position of the file.
    pub pos: u64,
}

#[derive(Debug, Clone)]
pub struct DirectoryHandle {
    pub entries: Vec<DirectoryEntry>,
}

#[derive(Debug, Clone)]
pub enum HandleState {
    File(FileHandle),
    Directory(DirectoryHandle),
}

// A table for allocating file handles.
#[derive(Debug, Clone)]
pub struct HandleTable {
    id_table: IdTable<Fh>,
    state: HashMap<Fh, HandleState>,
}

impl HandleTable {
    pub fn new() -> Self {
        Self {
            id_table: IdTable::new([]),
            state: HashMap::new(),
        }
    }

    // Get a new file handle with the given `state`.
    pub fn open(&mut self, state: HandleState) -> Fh {
        let fh = self.id_table.next();
        self.state.insert(fh, state);
        fh
    }

    // Remove the given file handle from the table.
    pub fn close(&mut self, fh: Fh) {
        self.id_table.recycle(fh);
        self.state.remove(&fh);
    }

    // Get the state associated with the given file handle.
    pub fn state(&self, fh: Fh) -> Option<&HandleState> {
        self.state.get(&fh)
    }

    // Get the state associated with the given file handle.
    pub fn state_mut(&mut self, fh: Fh) -> Option<&mut HandleState> {
        self.state.get_mut(&fh)
    }
}
