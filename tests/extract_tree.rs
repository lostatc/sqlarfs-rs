use std::time::{Duration, SystemTime};

use common::{connection, truncate_mtime};
use sqlarfs::{ErrorKind, FileMode};
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
fn extracting_when_source_is_a_file_and_dest_has_no_parent_dir_errors() -> sqlarfs::Result<()> {
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
fn extracting_when_source_is_a_dir_and_dest_has_no_parent_dir_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;

        expect!(archive.extract("dir", "/nonexistent/dest"))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn archiving_when_source_is_a_file_and_dest_already_exists_and_is_a_file_errors(
) -> sqlarfs::Result<()> {
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
fn archiving_when_source_is_a_file_and_dest_already_exists_and_is_a_dir_errors(
) -> sqlarfs::Result<()> {
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
fn archiving_when_source_is_a_dir_and_dest_already_exists_and_is_a_file_errors(
) -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;

        expect!(archive.extract("dir", temp_file.path()))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::AlreadyExists));

        Ok(())
    })
}

#[test]
fn archiving_when_source_is_a_dir_and_dest_already_exists_and_is_a_dir_errors(
) -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;

        expect!(archive.extract("dir", temp_dir.path()))
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
    let dest_path = temp_dir.path().join("dest");

    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        expect!(archive.extract("file", &dest_path)).to(be_ok());

        expect!(dest_path.exists()).to(be_true());
        expect!(dest_path.metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_file())
            .to(be_true());

        Ok(())
    })
}

#[test]
fn extract_empty_directory() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let dest_path = temp_dir.path().join("dest");

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;

        expect!(archive.extract("dir", &dest_path)).to(be_ok());

        expect!(dest_path.exists()).to(be_true());
        expect!(dest_path.metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_dir())
            .to(be_true());

        Ok(())
    })
}

#[test]
fn extract_directory_with_children() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let dest_dir = temp_dir.path().join("dest");

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;
        archive.open("dir/child-file")?.create_file()?;
        archive.open("dir/child-dir")?.create_dir()?;

        expect!(archive.extract("dir", &dest_dir)).to(be_ok());

        expect!(dest_dir.exists()).to(be_true());
        expect!(dest_dir.metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_dir())
            .to(be_true());

        expect!(dest_dir.join("child-file").exists()).to(be_true());
        expect!(dest_dir.join("child-file").metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_file())
            .to(be_true());

        expect!(dest_dir.join("child-dir").exists()).to(be_true());
        expect!(dest_dir.join("child-dir").metadata())
            .to(be_ok())
            .map(|metadata| metadata.is_dir())
            .to(be_true());

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn extracting_preserves_unix_file_mode() -> sqlarfs::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempfile::tempdir()?;
    let dest_path = temp_dir.path().join("dest");
    let expected_mode = FileMode::OWNER_R | FileMode::GROUP_R | FileMode::OTHER_R;

    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create_file()?;
        file.set_mode(Some(expected_mode))?;

        expect!(archive.extract("file", &dest_path)).to(be_ok());

        let actual_mode = dest_path.metadata()?.permissions().mode();
        let just_permissions_bits = actual_mode & 0o777;

        expect!(just_permissions_bits).to(equal(expected_mode.bits()));

        Ok(())
    })
}

#[test]
fn extracting_preserves_file_mtime() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let dest_path = temp_dir.path().join("dest");

    // Some time in the past that a newly-created file could not have by default.
    let expected_mtime = SystemTime::now() - Duration::from_secs(60);

    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create_file()?;
        file.set_mtime(Some(expected_mtime))?;

        expect!(archive.extract("file", &dest_path)).to(be_ok());

        let actual_mtime = dest_path.metadata()?.modified()?;
        expect!(actual_mtime).to(equal(truncate_mtime(expected_mtime)));

        Ok(())
    })
}
