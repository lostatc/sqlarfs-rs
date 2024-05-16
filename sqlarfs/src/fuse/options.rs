/// The default mount options which are always passed to libfuse.
pub fn default_mount_opts() -> Vec<MountOption> {
    vec![
        // This means we don't have to implement permissions checking ourselves.
        MountOption::Custom(String::from("default_permissions")),
        // Let FUSE know the filesystem is read-only.
        MountOption::Custom(String::from("ro")),
        // SQLite archives don't support device files.
        MountOption::Custom(String::from("nodev")),
        // SQLite archives don't store the file atime, so there's no sense trying to update it.
        MountOption::Custom(String::from("noatime")),
    ]
}

/// A mount option accepted when mounting a FUSE file system.
///
/// See `man mount.fuse` for details.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MountOption {
    /// Set the name of the source in mtab.
    FsName(String),

    /// Set the filesystem subtype in mtab.
    Subtype(String),

    /// Allow all users to access files on this filesystem. By default access is restricted to the
    /// user who mounted it.
    AllowOther,

    /// Allow the root user to access this filesystem in addition to the user who mounted it.
    AllowRoot,

    /// Automatically unmount when the mounting process exits.
    ///
    /// `AutoUnmount` requires `AllowOther` or `AllowRoot`. If `AutoUnmount` is set and neither of
    /// those is set, the FUSE configuration must permit `allow_other`, otherwise mounting will
    /// fail.
    AutoUnmount,

    /// Honor set-user-id and set-groupd-id bits on files.
    Suid,

    /// Don't honor set-user-id and set-groupd-id bits on files.
    NoSuid,

    /// Allow execution of binaries.
    Exec,

    /// Don't allow execution of binaries.
    NoExec,

    /// All modifications to directories will be done synchronously.
    DirSync,

    /// All I/O will be done synchronously.
    Sync,

    /// All I/O will be done asynchronously.
    Async,

    /// Pass an option which is not otherwise supported in this enum.
    Custom(String),
}

impl MountOption {
    pub(crate) fn into_fuser(self) -> fuser::MountOption {
        use fuser::MountOption::*;

        match self {
            Self::FsName(name) => FSName(name),
            Self::Subtype(name) => Subtype(name),
            Self::AllowOther => AllowOther,
            Self::AllowRoot => AllowRoot,
            Self::AutoUnmount => AutoUnmount,
            Self::Suid => Suid,
            Self::NoSuid => NoSuid,
            Self::Exec => Exec,
            Self::NoExec => NoExec,
            Self::DirSync => DirSync,
            Self::Sync => Sync,
            Self::Async => Async,
            Self::Custom(value) => CUSTOM(value),
        }
    }
}
