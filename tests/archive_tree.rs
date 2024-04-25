mod common;

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::time::{Duration, SystemTime};

use common::{connection, truncate_mtime};
use sqlarfs::{ArchiveOptions, Error, ErrorKind, FileMode};
use xpct::{approx_eq_time, be_err, be_false, be_ok, be_some, be_true, equal, expect};

//
// `Archive::archive`
//

#[test]
fn archiving_when_source_path_does_not_exist_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        expect!(archive.archive("nonexistent", ""))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn archiving_when_dest_path_already_exists_errors() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        let mut target = archive.open("file")?;
        target.create_file()?;

        expect!(archive.archive(temp_file.path(), "file"))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::AlreadyExists));

        Ok(())
    })
}

#[test]
fn archiving_when_dest_path_is_absolute_errors() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        expect!(archive.archive(temp_file.path(), "/file"))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::InvalidArgs));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn archiving_when_dest_path_is_not_valid_unicode_errors() -> sqlarfs::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        expect!(archive.archive(temp_file.path(), OsStr::from_bytes(b"invalid-unicode-\xff"),))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::InvalidArgs));

        Ok(())
    })
}

#[test]
fn archive_regular_file() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        expect!(archive.archive(temp_file.path(), "file")).to(be_ok());

        let file = archive.open("file")?;

        expect!(file.exists()).to(be_ok()).to(be_true());
        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_file())
            .to(be_true());

        Ok(())
    })
}

#[test]
fn archive_empty_directory() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        expect!(archive.archive(temp_dir.path(), "dir")).to(be_ok());

        let file = archive.open("dir")?;

        expect!(file.exists()).to(be_ok()).to(be_true());
        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_dir())
            .to(be_true());

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn archiving_preserves_unix_file_mode() -> sqlarfs::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // These are unlikely to be the default permissions of a temporary file.
    let expected_mode = 0o444;

    let temp_file = tempfile::NamedTempFile::new()?;
    fs::set_permissions(temp_file.path(), fs::Permissions::from_mode(expected_mode))?;

    // A sanity check to guard against tests passing when they shouldn't.
    expect!(fs::metadata(temp_file.path())?)
        .map(|metadata| metadata.permissions().mode())
        .to_not(equal(expected_mode));

    connection()?.exec(|archive| {
        expect!(archive.archive(temp_file.path(), "file")).to(be_ok());

        let file = archive.open("file")?;

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.mode)
            .to(be_some())
            .to(equal(FileMode::from_bits_truncate(expected_mode)));

        Ok(())
    })
}

#[test]
fn archiving_preserves_file_mtime() -> sqlarfs::Result<()> {
    // Some time in the past.
    let expected_mtime = truncate_mtime(SystemTime::now() - Duration::from_secs(60));

    let temp_file = tempfile::NamedTempFile::new()?;
    temp_file.as_file().set_modified(expected_mtime)?;

    connection()?.exec(|archive| {
        expect!(archive.archive(temp_file.path(), "file")).to(be_ok());

        let file = archive.open("file")?;

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.mtime)
            .to(be_some())
            .to(equal(expected_mtime));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn archiving_skips_special_files() -> sqlarfs::Result<()> {
    use nix::sys::stat::Mode as UnixMode;
    use nix::unistd::mkfifo;

    let temp_dir = tempfile::tempdir()?;

    mkfifo(&temp_dir.path().join("fifo"), UnixMode::S_IRWXU).map_err(|err| {
        Error::new(
            ErrorKind::Io {
                kind: io::ErrorKind::Other,
            },
            err,
        )
    })?;

    connection()?.exec(|archive| {
        expect!(archive.archive(temp_dir.path(), "dir")).to(be_ok());

        let special_file = archive.open("fifo")?;

        expect!(special_file.exists()).to(be_ok()).to(be_false());

        Ok(())
    })
}

//
// `ArchiveOptions::dereference`
//

#[test]
#[cfg(unix)]
fn archiving_follows_symlinks() -> sqlarfs::Result<()> {
    use nix::unistd::symlinkat;

    let temp_dir = tempfile::tempdir()?;
    let symlink_target = tempfile::NamedTempFile::new()?;

    symlinkat(
        symlink_target.path(),
        None,
        &temp_dir.path().join("symlink"),
    )
    .map_err(|err| {
        Error::new(
            ErrorKind::Io {
                kind: io::ErrorKind::Other,
            },
            err,
        )
    })?;

    connection()?.exec(|archive| {
        expect!(archive.archive(temp_dir.path(), "dir")).to(be_ok());

        let symlink = archive.open("dir/symlink")?;

        expect!(symlink.exists()).to(be_ok()).to(be_true());
        expect!(symlink.metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_file())
            .to(be_true());

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn archiving_does_not_follow_symlinks() -> sqlarfs::Result<()> {
    use nix::unistd::symlinkat;

    let temp_dir = tempfile::tempdir()?;
    let symlink_target = tempfile::NamedTempFile::new()?;

    symlinkat(
        symlink_target.path(),
        None,
        &temp_dir.path().join("symlink"),
    )
    .map_err(|err| {
        Error::new(
            ErrorKind::Io {
                kind: io::ErrorKind::Other,
            },
            err,
        )
    })?;

    connection()?.exec(|archive| {
        let opts = ArchiveOptions::new().follow_symlinks(false);

        expect!(archive.archive_with(temp_dir.path(), "dir", &opts)).to(be_ok());

        let symlink = archive.open("dir/symlink")?;

        expect!(symlink.exists()).to(be_ok()).to(be_false());

        Ok(())
    })
}

//
// `ArchiveOptions::children`
//

#[test]
fn archive_directory_children_to_target() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_dir.path().join("file"))?;

    connection()?.exec(|archive| {
        let mut target = archive.open("dir")?;
        target.create_dir()?;

        let opts = ArchiveOptions::new().children(true);

        expect!(archive.archive_with(temp_dir.path(), "dir", &opts,)).to(be_ok());

        let file = archive.open("dir/file")?;

        expect!(file.exists()).to(be_ok()).to(be_true());
        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_file())
            .to(be_true());

        Ok(())
    })
}

#[test]
fn archive_directory_children_to_archive_root() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_dir.path().join("file"))?;

    let opts = ArchiveOptions::new().children(true);

    connection()?.exec(|archive| {
        expect!(archive.archive_with(temp_dir.path(), "", &opts)).to(be_ok());

        let file = archive.open("file")?;

        expect!(file.exists()).to(be_ok()).to(be_true());
        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_file())
            .to(be_true());

        Ok(())
    })
}

#[test]
fn archiving_directory_children_when_target_is_file_errors() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_dir.path().join("file"))?;

    connection()?.exec(|archive| {
        let mut target = archive.open("file")?;
        target.create_file()?;

        let opts = ArchiveOptions::new().children(true);

        expect!(archive.archive_with(temp_dir.path(), "file", &opts))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotADirectory));

        Ok(())
    })
}

#[test]
fn archiving_directory_children_when_target_doest_not_exist_errors() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_dir.path().join("file"))?;

    connection()?.exec(|archive| {
        let opts = ArchiveOptions::new().children(true);

        expect!(archive.archive_with(temp_dir.path(), "dir", &opts))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn archive_directory_children_when_source_is_file() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        let opts = ArchiveOptions::new().children(true);

        expect!(archive.archive_with(temp_file.path(), "file", &opts)).to(be_ok());

        let file = archive.open("file")?;

        expect!(file.exists()).to(be_ok()).to(be_true());
        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_file())
            .to(be_true());

        Ok(())
    })
}

//
// `ArchiveOptions::recursive`
//

#[test]
fn archive_non_recursively() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_dir.path().join("file"))?;

    connection()?.exec(|archive| {
        let opts = ArchiveOptions::new().recursive(false);

        expect!(archive.archive_with(temp_dir.path(), "dir", &opts)).to(be_ok());

        let dir = archive.open("dir")?;
        expect!(dir.exists()).to(be_ok()).to(be_true());

        let file = archive.open("dir/file")?;
        expect!(file.exists()).to(be_ok()).to(be_false());

        Ok(())
    })
}

//
// `ArchiveOptions::preserve`
//

#[test]
fn archiving_does_not_preserve_file_mtime() -> sqlarfs::Result<()> {
    // Some time in the past.
    let expected_mtime = truncate_mtime(SystemTime::now() - Duration::from_secs(60));

    let temp_file = tempfile::NamedTempFile::new()?;
    temp_file.as_file().set_modified(expected_mtime)?;

    connection()?.exec(|archive| {
        let opts = ArchiveOptions::new().preserve_metadata(false);

        expect!(archive.archive_with(temp_file.path(), "file", &opts)).to(be_ok());

        let file = archive.open("file")?;

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.mtime)
            .to(be_some())
            .to_not(equal(expected_mtime));

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.mtime)
            .to(be_some())
            .to(approx_eq_time(SystemTime::now(), Duration::from_secs(1)));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn archiving_does_not_preserve_unix_file_mode() -> sqlarfs::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // These are unlikely to be the default permissions of a temporary file.
    let expected_mode = 0o444;

    let temp_file = tempfile::NamedTempFile::new()?;
    fs::set_permissions(temp_file.path(), fs::Permissions::from_mode(expected_mode))?;

    // A sanity check to guard against tests passing when they shouldn't.
    expect!(fs::metadata(temp_file.path())?)
        .map(|metadata| metadata.permissions().mode())
        .to_not(equal(expected_mode));

    connection()?.exec(|archive| {
        let opts = ArchiveOptions::new().preserve_metadata(false);

        expect!(archive.archive_with(temp_file.path(), "file", &opts)).to(be_ok());

        let file = archive.open("file")?;

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.mode)
            .to(be_some())
            .to_not(equal(FileMode::from_bits_truncate(expected_mode)));

        Ok(())
    })
}
