mod common;

use std::ffi::OsStr;
use std::path::Path;

use sqlarfs::{ErrorKind, FileMode};
use xpct::{be_ok, equal, expect};

use common::{connection, have_error_kind};

//
// `Archive::open`
//

#[test]
fn opening_file_with_absolute_path_errors() -> sqlarfs::Result<()> {
    let path = if cfg!(windows) {
        r"C:\path\to\file"
    } else {
        "/path/to/file"
    };

    connection()?.exec(|archive| {
        expect!(archive.open(path)).to(have_error_kind(ErrorKind::InvalidArgs));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn opening_file_with_non_utf8_path_errors() -> sqlarfs::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    connection()?.exec(|archive| {
        expect!(archive.open(OsStr::from_bytes(b"not/valid/utf8/\x80\x81")))
            .to(have_error_kind(ErrorKind::InvalidArgs));

        Ok(())
    })
}

#[test]
fn opening_file_with_empty_path_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        expect!(archive.open("")).to(have_error_kind(ErrorKind::InvalidArgs));

        Ok(())
    })
}

#[test]
fn opening_file_strips_trailing_slashes() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        expect!(archive.open("path/with/trailing/slash/"))
            .to(be_ok())
            .map(|file| file.path().to_owned())
            .to(equal(Path::new("path/with/trailing/slash")));

        Ok(())
    })
}

//
// `Archive::umask` / `Archive::set_umask`
//

#[test]
fn set_archive_umask() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        expect!(archive.umask()).to(equal(FileMode::OTHER_W));

        let expected_umask = FileMode::OWNER_RWX | FileMode::OTHER_RWX;

        archive.set_umask(expected_umask);

        expect!(archive.umask()).to(equal(expected_umask));

        Ok(())
    })
}

#[test]
fn files_inherit_archive_umask() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let expected_umask = FileMode::OWNER_RWX | FileMode::OTHER_RWX;

        archive.set_umask(expected_umask);

        let file = archive.open("file")?;

        expect!(file.umask()).to(equal(expected_umask));

        Ok(())
    })
}
