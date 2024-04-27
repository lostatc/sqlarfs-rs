use common::connection;
use sqlarfs::ErrorKind;
use xpct::{be_err, be_ok, be_true, equal, expect};

mod common;

#[test]
fn extracting_when_source_path_does_not_exist_errors() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        expect!(archive.extract("nonexistent", temp_dir.path().join("dest")))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn extracting_when_dest_path_has_no_parent_dir_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        expect!(archive.extract("file", "/nonexistent/dest"))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn archiving_when_dest_path_already_exists_and_is_a_file_errors() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        expect!(archive.extract("file", temp_file.path()))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::AlreadyExists));

        Ok(())
    })
}

#[test]
fn archiving_when_dest_path_already_exists_and_is_a_dir_errors() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        expect!(archive.extract("file", temp_dir.path()))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::AlreadyExists));

        Ok(())
    })
}

#[test]
fn archiving_when_source_path_is_absolute_errors() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        expect!(archive.extract("/file", temp_dir.path().join("dest")))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::InvalidArgs));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn archiving_when_source_path_is_not_valid_unicode_errors() -> sqlarfs::Result<()> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        expect!(archive.extract(
            OsStr::from_bytes(b"invalid-unicode-\xff"),
            temp_dir.path().join("dest")
        ))
        .to(be_err())
        .map(|err| err.into_kind())
        .to(equal(ErrorKind::InvalidArgs));

        Ok(())
    })
}

#[test]
fn extract_regular_file() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        expect!(archive.extract("file", temp_dir.path().join("dest"))).to(be_ok());

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
fn extract_empty_directory() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;

        expect!(archive.extract("dir", temp_dir.path().join("dest"))).to(be_ok());

        let dir = archive.open("dir")?;

        expect!(dir.exists()).to(be_ok()).to(be_true());
        expect!(dir.metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_dir())
            .to(be_true());

        Ok(())
    })
}

#[test]
fn extract_directory_with_children() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;
        archive.open("dir/child-file")?.create_file()?;
        archive.open("dir/child-dir")?.create_dir()?;

        expect!(archive.extract("dir", temp_dir.path().join("dest"))).to(be_ok());

        let dir = archive.open("dir")?;

        expect!(dir.exists()).to(be_ok()).to(be_true());
        expect!(dir.metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_dir())
            .to(be_true());

        let child_file = archive.open("dir/child-file")?;

        expect!(child_file.exists()).to(be_ok()).to(be_true());
        expect!(child_file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_file())
            .to(be_true());

        let child_dir = archive.open("dir/child-dir")?;

        expect!(child_dir.exists()).to(be_ok()).to(be_true());
        expect!(child_dir.metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_dir())
            .to(be_true());

        Ok(())
    })
}
