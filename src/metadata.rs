use std::{path::PathBuf, time::SystemTime};

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

/// The metadata of a file in a SQLite archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileMetadata {
    /// A regular file.
    File {
        /// The file mode (permissions).
        mode: Option<FileMode>,

        /// The time the file was last modified.
        ///
        /// This has a precision of 1 second.
        mtime: Option<SystemTime>,

        /// The uncompressed size of the file in bytes.
        size: u64,
    },

    /// A directory.
    Dir {
        /// The file mode (permissions).
        mode: Option<FileMode>,

        /// The time the file was last modified.
        ///
        /// This has a precision of 1 second.
        mtime: Option<SystemTime>,
    },

    /// A symbolic link.
    Symlink {
        /// The time the file was last modified.
        ///
        /// This has a precision of 1 second.
        mtime: Option<SystemTime>,

        /// The path of the file the symbolic link points to.
        target: PathBuf,
    },
}

impl FileMetadata {
    /// The [`FileType`] of this file.
    pub fn kind(&self) -> FileType {
        match self {
            Self::File { .. } => FileType::File,
            Self::Dir { .. } => FileType::Dir,
            Self::Symlink { .. } => FileType::Symlink,
        }
    }

    /// The time the file was last modified.
    ///
    /// This has a precision of 1 second.
    pub fn mtime(&self) -> Option<SystemTime> {
        match self {
            Self::File { mtime, .. } | Self::Dir { mtime, .. } | Self::Symlink { mtime, .. } => {
                *mtime
            }
        }
    }
}

pub const TYPE_MASK: u32 = 0o170000;
pub const FILE_MODE: u32 = 0o100000;
pub const DIR_MODE: u32 = 0o040000;
pub const SYMLINK_MODE: u32 = 0o120000;

/// The type of a file, either a regular file or a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    /// A regular file.
    File,

    /// A directory.
    Dir,

    /// A symbolic link.
    Symlink,
}

impl FileMode {
    pub(super) fn to_file_mode(self) -> u32 {
        self.bits() | FILE_MODE
    }

    pub(super) fn to_dir_mode(self) -> u32 {
        self.bits() | DIR_MODE
    }

    pub(super) fn to_symlink_mode(self) -> u32 {
        self.bits() | SYMLINK_MODE
    }

    pub(super) fn from_mode(mode: u32) -> Self {
        Self::from_bits_truncate(mode & !TYPE_MASK)
    }
}

pub fn mode_from_umask(kind: FileType, umask: FileMode) -> FileMode {
    match kind {
        FileType::File => {
            !umask & FileMode::OWNER_R
                | FileMode::OWNER_W
                | FileMode::GROUP_R
                | FileMode::GROUP_W
                | FileMode::OTHER_R
                | FileMode::OTHER_W
        }
        FileType::Dir => !umask & FileMode::OWNER_RWX | FileMode::GROUP_RWX | FileMode::OTHER_RWX,
        // The permissions for a symlink are always 0o777, so we don't apply the umask.
        FileType::Symlink => FileMode::OWNER_RWX | FileMode::GROUP_RWX | FileMode::OTHER_RWX,
    }
}

#[cfg(test)]
mod tests {
    use xpct::{equal, expect};

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

    fn test_symlink_mode() -> FileMode {
        FileMode::OWNER_RWX | FileMode::GROUP_RWX | FileMode::OTHER_RWX
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
    fn get_symlink_mode_from_permissions() {
        expect!(test_symlink_mode().to_symlink_mode()).to(equal(0o120777));
    }

    #[test]
    fn get_file_permissions_from_mode() {
        expect!(FileMode::from_mode(0o100664)).to(equal(test_file_mode()));
        expect!(FileMode::from_mode(0o040775)).to(equal(test_dir_mode()));
    }
}
