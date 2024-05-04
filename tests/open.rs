mod common;

use std::fs;
use std::io::prelude::*;

use common::have_error_kind;
use sqlarfs::OpenOptions;
use xpct::{be_err, be_ok, expect};

//
// `OpenOptions::open`
//

#[test]
fn opening_creates_db_file_when_it_does_not_exist() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    let result = OpenOptions::new().create(true).open(&temp_path);

    expect!(result).to(be_ok());

    fs::remove_file(&temp_path)?;

    Ok(())
}

#[test]
fn opening_errors_when_db_does_not_exist() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    let result = OpenOptions::new().create(false).open(&temp_path);

    expect!(result).to(have_error_kind(sqlarfs::ErrorKind::CannotOpen));

    Ok(())
}

#[test]
fn any_db_operation_errors_when_file_is_not_a_db() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    temp_file.write_all(b"not a database")?;
    temp_file.flush()?;

    let result = OpenOptions::new().open(temp_file.path());

    expect!(result).to(have_error_kind(sqlarfs::ErrorKind::NotADatabase));

    Ok(())
}

#[test]
fn opening_read_only_errors_when_create_db_is_true() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    let result = OpenOptions::new()
        .read_only(true)
        .create(true)
        .open(temp_file.path());

    expect!(result).to(have_error_kind(sqlarfs::ErrorKind::InvalidArgs));

    Ok(())
}

#[test]
fn any_write_operation_errors_when_db_is_read_only() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    // Create the database and then immediately close the connection.
    OpenOptions::new().create(true).open(&temp_path)?;

    let mut conn = OpenOptions::new().read_only(true).open(&temp_path)?;

    let result = conn.exec(|archive| archive.open("file")?.create_file());

    expect!(result).to(have_error_kind(sqlarfs::ErrorKind::ReadOnly));

    fs::remove_file(&temp_path)?;

    Ok(())
}

#[test]
fn initializing_read_only_db_errors() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    let result = OpenOptions::new()
        .read_only(true)
        .init(true)
        .open(temp_file.path());

    expect!(result).to(have_error_kind(sqlarfs::ErrorKind::InvalidArgs));

    Ok(())
}

#[test]
fn writing_to_uninitialized_db_errors() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    let mut conn = OpenOptions::new().init(false).open(temp_file.path())?;

    conn.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.create_file()).to(be_err());

        Ok(())
    })
}
