use std::fs;
use std::io::prelude::*;

use xpct::{be_err, be_ok, equal, expect};

use sqlarfs::OpenOptions;

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
fn opening_fails_when_db_does_not_exist() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    let result = OpenOptions::new().create(false).open(&temp_path);

    expect!(result)
        .to(be_err())
        .map(|err| err.into_kind())
        .to(equal(sqlarfs::ErrorKind::CannotOpen));

    Ok(())
}

#[test]
fn any_db_operation_fails_when_file_is_not_a_db() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    temp_file.write_all(b"not a database")?;
    temp_file.flush()?;

    let mut conn = OpenOptions::new().open(temp_file.path())?;
    let result = conn.exec(|archive| archive.init());

    expect!(result)
        .to(be_err())
        .map(|err| err.into_kind())
        .to(equal(sqlarfs::ErrorKind::NotADatabase));

    Ok(())
}

#[test]
fn opening_read_only_fails_when_create_db_is_true() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    let result = OpenOptions::new()
        .create(true)
        .read_only(true)
        .open(temp_file.path());

    expect!(result)
        .to(be_err())
        .map(|err| err.into_kind())
        .to(equal(sqlarfs::ErrorKind::InvalidArgs));

    Ok(())
}

#[test]
fn any_write_operation_fails_when_db_is_read_only() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    // Create the database and then immediately close the connection.
    OpenOptions::new().create(true).open(&temp_path)?;

    let mut conn = OpenOptions::new()
        .read_only(true)
        .create(false)
        .open(&temp_path)?;
    let result = conn.exec(|archive| archive.init());

    expect!(result)
        .to(be_err())
        .map(|err| err.into_kind())
        .to(equal(sqlarfs::ErrorKind::ReadOnly));

    fs::remove_file(&temp_path)?;

    Ok(())
}

#[test]
fn opening_in_memory_succeeds() {
    expect!(OpenOptions::new().open_in_memory()).to(be_ok());
}

#[test]
fn any_write_operation_fails_when_in_memory_db_is_read_only() -> sqlarfs::Result<()> {
    let mut conn = OpenOptions::new()
        .create(false)
        .read_only(true)
        .open_in_memory()?;

    let result = conn.exec(|archive| archive.init());

    expect!(result)
        .to(be_err())
        .map(|err| err.into_kind())
        .to(equal(sqlarfs::ErrorKind::ReadOnly));

    Ok(())
}

#[test]
fn create_false_is_ignored_for_open_in_memory() -> sqlarfs::Result<()> {
    expect!(OpenOptions::new().create(false).open_in_memory()).to(be_ok());

    Ok(())
}
