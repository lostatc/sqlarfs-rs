use std::time::SystemTime;

use bitflags::bitflags;

bitflags! {
    /// A Unix file mode.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FileMode: u32 {
        /// Read for owner (`S_IRUSR`).
        const OWNER_R = 0o0400;

        /// Write for owner (`S_IWUSR`).
        const OWNER_W = 0o0200;

        /// Execute for owner (`S_IXUSR`).
        const OWNER_X = 0o0100;

        /// Read, write, and execute for owner (`S_IRWXU`).
        const OWNER_RWX = 0o0700;

        /// Read for group (`S_IRGRP`).
        const GROUP_R = 0o0040;

        /// Write for group (`S_IWGRP`).
        const GROUP_W = 0o0020;

        /// Execute for group (`S_IXGRP`).
        const GROUP_X = 0o0010;

        /// Read, write, and execute for group (`S_IRWXG`).
        const GROUP_RWX = 0o0070;

        /// Read for others (`S_IROTH`).
        const OTHER_R = 0o0004;

        /// Write for others (`S_IWOTH`).
        const OTHER_W = 0o0002;

        /// Execute for others (`S_IXOTH`).
        const OTHER_X = 0o0001;

        /// Read, write, and execute for others (`S_IRWXO`).
        const OTHER_RWX = 0o0007;

        /// Set user ID on execution (`S_ISUID`).
        const SUID = 0o4000;

        /// Set group ID on execution (`S_ISGID`).
        const SGID = 0o2000;

        /// The sticky bit (`S_ISVTX`).
        const STICKY = 0o1000;
    }
}

/// Metadata for a [`File`].
///
/// [`File`]: crate::File
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FileMetadata {
    /// The file mode (permissions).
    pub mode: Option<FileMode>,

    /// The time the file was last modified.
    ///
    /// This value has second precision.
    pub mtime: Option<SystemTime>,

    /// The uncompressed size of the file.
    pub size: u64,

    /// Whether this is a regular file or a directory.
    ///
    /// This can be `None` if the file had no mode in the database, or if the mode indicated the
    /// file is a special file.
    pub kind: Option<FileType>,
}

impl FileMetadata {
    /// Whether this file is a regular file.
    pub fn is_file(&self) -> bool {
        matches!(self.kind, Some(FileType::File))
    }

    /// Whether this file is a directory.
    pub fn is_dir(&self) -> bool {
        matches!(self.kind, Some(FileType::Dir))
    }
}

const TYPE_MASK: u32 = 0o170000;
const FILE_MODE: u32 = 0o100000;
const DIR_MODE: u32 = 0o040000;

/// The type of a file, either a regular file or a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    /// A regular file.
    File,

    /// A directory.
    Dir,
}

impl FileType {
    pub(super) fn from_mode(mode: u32) -> Option<Self> {
        if (mode & TYPE_MASK) == FILE_MODE {
            Some(Self::File)
        } else if (mode & TYPE_MASK) == DIR_MODE {
            Some(Self::Dir)
        } else {
            None
        }
    }
}

impl FileMode {
    pub(super) fn to_file_mode(self) -> u32 {
        self.bits() | FILE_MODE
    }

    pub(super) fn to_dir_mode(self) -> u32 {
        self.bits() | DIR_MODE
    }

    pub(super) fn from_mode(mode: u32) -> Self {
        Self::from_bits_truncate(mode & !TYPE_MASK)
    }
}

#[cfg(test)]
mod tests {
    use xpct::{be_none, be_some, equal, expect};

    use super::*;

    // Typical permissions for a regular file.
    fn test_file_mode() -> FileMode {
        FileMode::OWNER_R
            | FileMode::OWNER_W
            | FileMode::GROUP_R
            | FileMode::GROUP_W
            | FileMode::OTHER_R
    }

    // Typical permissions for a directory.
    fn test_dir_mode() -> FileMode {
        FileMode::OWNER_RWX | FileMode::GROUP_RWX | FileMode::OTHER_R | FileMode::OTHER_X
    }

    #[test]
    fn get_file_mode_from_permissions() {
        expect!(test_file_mode().to_file_mode()).to(equal(0o100664));
    }

    #[test]
    fn get_dir_mode_from_permissions() {
        expect!(test_dir_mode().to_dir_mode()).to(equal(0o040775));
    }

    #[test]
    fn get_file_permissions_from_mode() {
        expect!(FileMode::from_mode(0o100664)).to(equal(test_file_mode()));
        expect!(FileMode::from_mode(0o040775)).to(equal(test_dir_mode()));
    }

    #[test]
    fn get_file_type_from_mode() {
        expect!(FileType::from_mode(0o100664))
            .to(be_some())
            .to(equal(FileType::File));

        expect!(FileType::from_mode(0o040775))
            .to(be_some())
            .to(equal(FileType::Dir));

        // This is the mode for a symlink.
        expect!(FileType::from_mode(0o120664)).to(be_none());
    }
}
