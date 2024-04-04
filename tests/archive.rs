mod common;

use std::ffi::OsStr;
use std::path::Path;

use sqlarfs::ErrorKind;
use xpct::{be_err, be_ok, equal, expect};

use common::connection;

#[test]
fn opening_file_with_absolute_path_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        expect!(archive.open("/path/to/file"))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::InvalidArgs));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn opening_file_with_non_utf8_path_errors() -> sqlarfs::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    connection()?.exec(|archive| {
        expect!(archive.open(OsStr::from_bytes(b"not/valid/utf8/\x80\x81")))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::InvalidArgs));

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
