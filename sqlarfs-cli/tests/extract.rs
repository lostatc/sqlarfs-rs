mod common;

use std::env;
use std::path::Path;

use serial_test::serial;
use sqlarfs::Connection;
use xpct::{be_directory, be_err, be_existing_file, be_regular_file, expect};

use common::command;

#[test]
#[serial(change_directory)]
fn extracts_contents_to_current_dir() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    env::set_current_dir(temp_dir.path())?;

    let mut conn = Connection::open(&archive_path)?;
    conn.exec(|archive| archive.open("file")?.create_file())?;

    command(&["extract", &archive_path.to_string_lossy()])?;

    expect!(Path::new("file")).to(be_regular_file());

    Ok(())
}

#[test]
fn extracts_contents_to_target_dir() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    let mut conn = Connection::open(&archive_path)?;
    conn.exec(|archive| archive.open("file")?.create_file())?;

    command(&[
        "extract",
        &archive_path.to_string_lossy(),
        &temp_dir.path().to_string_lossy(),
    ])?;

    expect!(temp_dir.path().join("file")).to(be_regular_file());

    Ok(())
}

#[test]
fn extracts_source_file_to_target_dir() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    let mut conn = Connection::open(&archive_path)?;
    conn.exec(|archive| {
        archive.open("file1")?.create_file()?;
        archive.open("file2")?.create_file()?;

        sqlarfs::Result::Ok(())
    })?;

    command(&[
        "extract",
        "--source",
        "file1",
        &archive_path.to_string_lossy(),
        &temp_dir.path().to_string_lossy(),
    ])?;

    expect!(temp_dir.path().join("file1")).to(be_regular_file());
    expect!(temp_dir.path().join("file2")).to_not(be_existing_file());

    Ok(())
}

#[test]
fn extracts_source_dir_to_target_dir() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    let mut conn = Connection::open(&archive_path)?;
    conn.exec(|archive| {
        archive.open("dir1/dir2")?.create_dir_all()?;
        archive.open("dir1/dir2/file1")?.create_file()?;
        archive.open("file2")?.create_file()?;

        sqlarfs::Result::Ok(())
    })?;

    command(&[
        "extract",
        "--source",
        "dir1/dir2",
        &archive_path.to_string_lossy(),
        &temp_dir.path().to_string_lossy(),
    ])?;

    expect!(temp_dir.path().join("dir2")).to(be_directory());
    expect!(temp_dir.path().join("dir2/file1")).to(be_regular_file());
    expect!(temp_dir.path().join("file2")).to_not(be_existing_file());

    Ok(())
}

#[test]
fn extract_errors_when_source_does_not_have_a_filename() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    let mut conn = Connection::open(&archive_path)?;
    conn.exec(|archive| archive.open("file")?.create_file())?;

    expect!(command(&[
        "extract",
        "--source",
        "/",
        &archive_path.to_string_lossy()
    ]))
    .to(be_err());

    Ok(())
}
