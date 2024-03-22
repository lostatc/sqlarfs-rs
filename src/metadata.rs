use bitflags::bitflags;

bitflags! {
    /// A file mode.
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
