mod common;

use std::fs;
use std::io::prelude::*;

use sqlarfs::{Error, OpenOptions};
use xpct::{be_err, be_ok, equal, expect, match_pattern, pattern};

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

    // The test shouldn't fail if this cleanup fails.
    fs::remove_file(&temp_path).ok();

    Ok(())
}

#[test]
fn opening_errors_when_db_does_not_exist() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    let result = OpenOptions::new().create(false).open(&temp_path);

    expect!(result).to(be_err());

    Ok(())
}

#[test]
fn any_db_operation_errors_when_file_is_not_a_db() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    temp_file.write_all(b"not a database")?;
    temp_file.flush()?;

    let result = OpenOptions::new().open(temp_file.path());

    expect!(result).to(be_err());

    Ok(())
}

#[test]
fn creating_new_read_only_db_errors() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    let result = OpenOptions::new()
        .read_only(true)
        .create(true)
        .open(&temp_path);

    expect!(result).to(be_err());

    fs::remove_file(&temp_path).ok();

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

    expect!(result).to(be_err()).to(equal(Error::ReadOnly));

    fs::remove_file(&temp_path).ok();

    Ok(())
}

#[test]
fn initializing_read_only_db_errors() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    let result = OpenOptions::new()
        .read_only(true)
        .init(true)
        .open(&temp_path);

    expect!(result).to(be_err());

    fs::remove_file(&temp_path).ok();

    Ok(())
}

#[test]
fn writing_to_uninitialized_db_errors() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    let mut conn = OpenOptions::new().init(false).open(&temp_path)?;

    conn.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.create_file()).to(be_err());

        sqlarfs::Result::Ok(())
    })?;

    fs::remove_file(&temp_path).ok();

    Ok(())
}

#[test]
fn init_and_init_new_are_mutually_exclusive() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    let result = OpenOptions::new()
        .init_new(true)
        .init(true)
        .open(&temp_path);

    expect!(result)
        .to(be_err())
        .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

    fs::remove_file(&temp_path).ok();

    Ok(())
}

#[test]
fn init_new_errors_when_sqlar_table_already_exists() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    temp_file.close()?;

    // Create the database and then immediately close the connection.
    OpenOptions::new().init_new(true).open(&temp_path)?;

    let result = OpenOptions::new().init_new(true).open(&temp_path);

    expect!(result).to(be_err());

    fs::remove_file(&temp_path).ok();

    Ok(())
}
