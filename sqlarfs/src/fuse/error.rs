use std::io;

// Handle a `crate::Result` in a FUSE method.
macro_rules! try_result {
    ($result:expr, $reply:expr) => {
        match $result {
            ::std::result::Result::Ok(result) => result,
            ::std::result::Result::Err(error) => {
                $reply.error($crate::Error::from(error).to_errno());
                return;
            }
        }
    };
}

// Handle an `Option` in a FUSE method.
macro_rules! try_option {
    ($result:expr, $reply:expr, $error:expr) => {
        match $result {
            ::std::option::Option::Some(result) => result,
            ::std::option::Option::None => {
                $reply.error($error);
                return;
            }
        }
    };
}

impl crate::Error {
    // Get the libc errno for this error.
    pub(super) fn to_errno(&self) -> i32 {
        use nix::libc;

        match self {
            crate::Error::InvalidArgs { .. } => libc::EINVAL,
            crate::Error::FileAlreadyExists { .. } => libc::EEXIST,
            crate::Error::FileNotFound { .. } => libc::ENOENT,
            crate::Error::NotARegularFile { .. } => libc::EISDIR,
            crate::Error::NotADirectory { .. } => libc::ENOTDIR,
            crate::Error::FilesystemLoop => libc::ELOOP,
            crate::Error::CompressionNotSupported => libc::ENOTSUP,
            crate::Error::FileTooBig => libc::EFBIG,
            crate::Error::ReadOnly => libc::EROFS,
            crate::Error::Io { kind, code } => code.unwrap_or(match kind {
                io::ErrorKind::NotFound => libc::ENOENT,
                io::ErrorKind::PermissionDenied => libc::EPERM,
                io::ErrorKind::ConnectionRefused => libc::ECONNREFUSED,
                io::ErrorKind::ConnectionReset => libc::ECONNRESET,
                io::ErrorKind::ConnectionAborted => libc::ECONNABORTED,
                io::ErrorKind::NotConnected => libc::ENOTCONN,
                io::ErrorKind::AddrInUse => libc::EADDRINUSE,
                io::ErrorKind::AddrNotAvailable => libc::EADDRNOTAVAIL,
                io::ErrorKind::BrokenPipe => libc::EPIPE,
                io::ErrorKind::AlreadyExists => libc::EEXIST,
                io::ErrorKind::WouldBlock => libc::EWOULDBLOCK,
                io::ErrorKind::InvalidInput => libc::EINVAL,
                io::ErrorKind::TimedOut => libc::ETIMEDOUT,
                io::ErrorKind::Interrupted => libc::EINTR,
                io::ErrorKind::Unsupported => libc::ENOSYS,
                _ => libc::EIO,
            }),
            _ => libc::EIO,
        }
    }
}
