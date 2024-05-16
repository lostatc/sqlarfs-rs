mod common;

use std::ffi::OsStr;
use std::fs;
use std::io::prelude::*;
use std::time::{Duration, SystemTime};

use common::{
    connection, have_file_metadata, have_symlink_metadata, into_sqlarfs_error, truncate_mtime,
    with_timeout,
};
use sqlarfs::{ArchiveOptions, Error, FileMode, FileType};
use xpct::{
    approx_eq_time, be_err, be_false, be_ok, be_some, be_true, equal, expect, match_pattern,
    pattern,
};

//
// `Archive::archive`
//

#[test]
fn archiving_when_source_path_does_not_exist_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        expect!(archive.archive("nonexistent", "dest"))
            .to(be_err())
            .to(equal(Error::FileNotFound {
                path: "nonexistent".into(),
            }));

        Ok(())
    })
}

#[test]
fn archiving_when_dest_path_has_no_parent_dir_errors() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        expect!(archive.archive(temp_file.path(), "nonexistent/file"))
            .to(be_err())
            .to(equal(Error::NoParentDirectory {
                path: "nonexistent/file".into(),
            }));

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
            .to(equal(Error::FileAlreadyExists {
                path: "file".into(),
            }));

        Ok(())
    })
}

#[test]
fn archiving_when_dest_path_is_absolute_errors() -> sqlarfs::Result<()> {
    let dest_path = if cfg!(windows) { r"C:\file" } else { "/file" };

    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        expect!(archive.archive(temp_file.path(), dest_path))
            .to(be_err())
            .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

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
            .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

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
            .into::<FileType>()
            .to(equal(FileType::File));

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
            .into::<FileType>()
            .to(equal(FileType::Dir));

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
            .to(have_file_metadata())
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

    let mut temp_file = tempfile::NamedTempFile::new()?;
    write!(temp_file.as_file_mut(), "file contents")?;

    temp_file.as_file().set_modified(expected_mtime)?;

    connection()?.exec(|archive| {
        expect!(archive.archive(temp_file.path(), "file")).to(be_ok());

        let file = archive.open("file")?;

        expect!(file.metadata())
            .to(be_ok())
            .to(have_file_metadata())
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

    mkfifo(&temp_dir.path().join("fifo"), UnixMode::S_IRWXU).map_err(into_sqlarfs_error)?;
    connection()?.exec(|archive| {
        expect!(archive.archive(temp_dir.path(), "dir")).to(be_ok());

        let special_file = archive.open("fifo")?;

        expect!(special_file.exists()).to(be_ok()).to(be_false());

        Ok(())
    })
}

#[test]
fn archive_with_trailing_slash_in_dest_path() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        expect!(archive.archive(temp_file.path(), "file/")).to(be_ok());

        let file = archive.open("file")?;

        expect!(file.metadata())
            .to(be_ok())
            .into::<FileType>()
            .to(equal(FileType::File));

        Ok(())
    })
}

//
// `ArchiveOptions::follow_symlinks`
//

#[test]
#[cfg(unix)]
fn archiving_follows_symlinks() -> sqlarfs::Result<()> {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir()?;
    let symlink_target = tempfile::NamedTempFile::new()?;

    symlink(symlink_target.path(), temp_dir.path().join("symlink"))?;

    connection()?.exec(|archive| {
        let opts = ArchiveOptions::new().follow_symlinks(true);
        expect!(archive.archive_with(temp_dir.path(), "dir", &opts)).to(be_ok());

        let symlink = archive.open("dir/symlink")?;

        expect!(symlink.exists()).to(be_ok()).to(be_true());
        expect!(symlink.metadata())
            .to(be_ok())
            .into::<FileType>()
            .to(equal(FileType::File));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn archiving_follows_chained_symlinks() -> sqlarfs::Result<()> {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir()?;
    let symlink_target = tempfile::NamedTempFile::new()?;

    symlink(symlink_target.path(), temp_dir.path().join("symlink1"))?;

    symlink(
        temp_dir.path().join("symlink1"),
        temp_dir.path().join("symlink2"),
    )?;

    connection()?.exec(|archive| {
        let opts = ArchiveOptions::new().follow_symlinks(true);
        expect!(archive.archive_with(temp_dir.path(), "dir", &opts)).to(be_ok());

        let symlink = archive.open("dir/symlink2")?;

        expect!(symlink.exists()).to(be_ok()).to(be_true());
        expect!(symlink.metadata())
            .to(be_ok())
            .into::<FileType>()
            .to(equal(FileType::File));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn archiving_doest_not_follow_symlinks() -> sqlarfs::Result<()> {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir()?;
    let symlink_target = tempfile::NamedTempFile::new()?;

    symlink(symlink_target.path(), temp_dir.path().join("symlink"))?;

    connection()?.exec(|archive| {
        expect!(archive.archive(temp_dir.path().join("symlink"), "symlink")).to(be_ok());

        let symlink = archive.open("symlink")?;

        expect!(symlink.exists()).to(be_ok()).to(be_true());
        expect!(symlink.metadata())
            .to(be_ok())
            .to(have_symlink_metadata())
            .map(|metadata| metadata.target)
            .to(equal(symlink_target.path()));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn archiving_doest_not_follow_symlink_children_of_directory() -> sqlarfs::Result<()> {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir()?;
    let symlink_target = tempfile::NamedTempFile::new()?;

    symlink(symlink_target.path(), temp_dir.path().join("symlink"))?;

    connection()?.exec(|archive| {
        expect!(archive.archive(temp_dir.path(), "dir")).to(be_ok());

        let symlink = archive.open("dir/symlink")?;

        expect!(symlink.exists()).to(be_ok()).to(be_true());
        expect!(symlink.metadata())
            .to(be_ok())
            .to(have_symlink_metadata())
            .map(|metadata| metadata.target)
            .to(equal(symlink_target.path()));

        Ok(())
    })
}

//
// `ArchiveOptions::children`
//

#[test]
fn archiving_fails_when_source_is_root_and_children_is_false() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let temp_file = tempfile::NamedTempFile::new()?;

        expect!(archive.archive(temp_file.path(), ""))
            .to(be_err())
            .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

        Ok(())
    })
}

#[test]
fn archive_directory_children_to_dir() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_dir.path().join("file"))?;

    connection()?.exec(|archive| {
        let mut target = archive.open("dir")?;
        target.create_dir()?;

        let opts = ArchiveOptions::new().children(true);

        expect!(archive.archive_with(temp_dir.path(), "dir", &opts)).to(be_ok());

        let file = archive.open(if cfg!(windows) {
            r"dir\file"
        } else {
            "dir/file"
        })?;

        expect!(file.exists()).to(be_ok()).to(be_true());
        expect!(file.metadata())
            .to(be_ok())
            .into::<FileType>()
            .to(equal(FileType::File));

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
            .into::<FileType>()
            .to(equal(FileType::File));

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
            .to(equal(Error::NotADirectory {
                path: "file".into(),
            }));

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
            .to(equal(Error::FileNotFound { path: "dir".into() }));

        Ok(())
    })
}

#[test]
fn archive_directory_children_when_source_is_file_errors() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        let opts = ArchiveOptions::new().children(true);

        archive.open("dir")?.create_dir()?;

        expect!(archive.archive_with(temp_file.path(), "dir", &opts))
            .to(be_err())
            .to(equal(Error::NotADirectory {
                path: temp_file.path().into(),
            }));

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
// `ArchiveOptions::preserve_metadata`
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
            .to(have_file_metadata())
            .map(|metadata| metadata.mtime)
            .to(be_some())
            .to_not(equal(expected_mtime));

        expect!(file.metadata())
            .to(be_ok())
            .to(have_file_metadata())
            .map(|metadata| metadata.mtime)
            .to(be_some())
            .to(approx_eq_time(SystemTime::now(), Duration::from_secs(2)));

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
            .to(have_file_metadata())
            .map(|metadata| metadata.mode)
            .to(be_some())
            .to_not(equal(FileMode::from_bits_truncate(expected_mode)));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn archiving_with_filesystem_loop_in_parent_errors() -> sqlarfs::Result<()> {
    use std::os::unix::fs::symlink;

    // The currently implementation uses recursion and will stack overflow if there's a filesystem
    // loop before it times out. However, we should still set a timeout in case this implementation
    // changes to one that doesn't use recursion.
    with_timeout(Duration::from_secs(1), || {
        let parent = tempfile::tempdir()?;

        // Create a symlink that points to its parent.
        symlink(parent.path(), parent.path().join("symlink"))?;

        connection()?.exec(|archive| {
            let opts = ArchiveOptions::new().follow_symlinks(true);

            expect!(archive.archive_with(parent.path(), "dest", &opts))
                .to(be_err())
                .to(equal(Error::FilesystemLoop));

            Ok(())
        })
    })
}

#[test]
#[cfg(unix)]
fn archiving_with_filesystem_loop_in_grandparent_errors() -> sqlarfs::Result<()> {
    use std::os::unix::fs::symlink;

    with_timeout(Duration::from_secs(1), || {
        let grandparent = tempfile::tempdir()?;
        let parent = grandparent.path().join("parent");

        fs::create_dir(&parent)?;

        // Create a symlink that points to its grandparent.
        symlink(grandparent.path(), parent.join("symlink"))?;

        connection()?.exec(|archive| {
            let opts = ArchiveOptions::new().follow_symlinks(true);

            expect!(archive.archive_with(grandparent.path(), "dest", &opts))
                .to(be_err())
                .to(equal(Error::FilesystemLoop));

            Ok(())
        })
    })
}
