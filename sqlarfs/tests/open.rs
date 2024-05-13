mod common;

use std::fs;
use std::io::prelude::*;

use sqlarfs::{Connection, Error};
use xpct::{be_err, be_ok, equal, expect};

//
// `Connection::open`
//

#[test]
fn open_archive_errors_when_db_does_not_exist() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    let result = Connection::open(temp_path);

    expect!(result).to(be_err()).to(equal(Error::CannotOpen));

    Ok(())
}

#[test]
fn open_archive_errors_when_file_is_not_a_db() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    temp_file.write_all(b"not a database")?;
    temp_file.flush()?;

    let result = Connection::open(temp_file.path());

    expect!(result).to(be_err()).to(equal(Error::NotADatabase));

    Ok(())
}

//
// `Connection::create`
//

#[test]
fn create_archive_creates_db_file_when_it_does_not_exist() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    let result = Connection::create(&temp_path);

    expect!(result).to(be_ok());

    let result = Connection::open(&temp_path);

    expect!(result).to(be_ok());

    // The test shouldn't fail if this cleanup fails.
    fs::remove_file(&temp_path).ok();

    Ok(())
}

#[test]
fn create_archive_errors_when_file_is_not_a_db() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    temp_file.write_all(b"not a database")?;
    temp_file.flush()?;

    let result = Connection::create(temp_file.path());

    expect!(result).to(be_err()).to(equal(Error::NotADatabase));

    Ok(())
}

//
// `Connection::create_new`
//

#[test]
fn create_new_archive_errors_when_sqlar_table_already_exists() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    // Create the database and then immediately close the connection.
    Connection::create_new(&temp_path)?;

    let result = Connection::create_new(&temp_path);

    expect!(result)
        .to(be_err())
        .to(equal(Error::SqlarAlreadyExists));

    fs::remove_file(&temp_path).ok();

    Ok(())
}

//
// `Connection::open_readonly`
//

#[test]
fn any_write_operation_when_db_is_read_only_errors() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    // Create the database and then immediately close the connection.
    Connection::create_new(&temp_path)?;

    let mut conn = Connection::open_readonly(&temp_path)?;

    let result = conn.exec(|archive| archive.open("file")?.create_file());

    expect!(result).to(be_err()).to(equal(Error::ReadOnly));

    fs::remove_file(&temp_path).ok();

    Ok(())
}

#[test]
fn open_archive_readonly_errors_when_db_does_not_exist() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    let result = Connection::open_readonly(temp_path);

    expect!(result).to(be_err()).to(equal(Error::CannotOpen));

    Ok(())
}

#[test]
fn open_archive_readonly_errors_when_file_is_not_a_db() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    temp_file.write_all(b"not a database")?;
    temp_file.flush()?;

    let result = Connection::open_readonly(temp_file.path());

    expect!(result).to(be_err()).to(equal(Error::NotADatabase));

    Ok(())
}
