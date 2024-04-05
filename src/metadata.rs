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

const TYPE_MASK: u32 = 0o170000;
const FILE_MODE: u32 = 0o100000;
const DIR_MODE: u32 = 0o040000;

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

    #[test]
    fn to_file_mode() {
        expect!(test_file_mode().to_file_mode()).to(equal(0o100664));
    }

    #[test]
    fn to_dir_mode() {
        expect!(test_dir_mode().to_dir_mode()).to(equal(0o040775));
    }

    #[test]
    fn from_mode() {
        expect!(FileMode::from_mode(0o100664)).to(equal(test_file_mode()));
        expect!(FileMode::from_mode(0o040775)).to(equal(test_dir_mode()));
    }
}
