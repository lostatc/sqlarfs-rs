mod common;

use common::command;
use sqlarfs::Connection;
use xpct::{be_err, be_ok, consist_of, expect};

#[test]
fn errors_when_archive_does_not_exist() -> eyre::Result<()> {
    expect!(command(&["list", "--archive", "nonexistent.sqlar"])).to(be_err());

    Ok(())
}

#[test]
fn listing_descendants_with_no_path_specified() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    let mut conn = Connection::create_new(&archive_path)?;

    conn.exec(|archive| {
        archive.open("file1")?.create_file()?;
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file2")?.create_file()?;

        sqlarfs::Result::Ok(())
    })?;

    expect!(command(&[
        "list",
        "--archive",
        &archive_path.to_string_lossy(),
        "--tree",
    ]))
    .to(be_ok())
    .map(|output| output.split('\n').map(String::from).collect::<Vec<_>>())
    .to(consist_of([
        String::from("file1"),
        String::from("dir"),
        String::from("dir/file2"),
    ]));

    Ok(())
}

#[test]
fn listing_descendants_of_path() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    let mut conn = Connection::create_new(&archive_path)?;

    conn.exec(|archive| {
        archive.open("file1")?.create_file()?;
        archive.open("dir1")?.create_dir()?;
        archive.open("dir1/file2")?.create_file()?;
        archive.open("dir1/dir2")?.create_dir()?;
        archive.open("dir1/dir2/file3")?.create_file()?;

        sqlarfs::Result::Ok(())
    })?;

    expect!(command(&[
        "list",
        "--archive",
        &archive_path.to_string_lossy(),
        "--tree",
        "dir1",
    ]))
    .to(be_ok())
    .map(|output| output.split('\n').map(String::from).collect::<Vec<_>>())
    .to(consist_of([
        String::from("dir1/file2"),
        String::from("dir1/dir2"),
        String::from("dir1/dir2/file3"),
    ]));

    Ok(())
}

#[test]
fn listing_immediate_children_with_no_path_specified() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    let mut conn = Connection::create_new(&archive_path)?;

    conn.exec(|archive| {
        archive.open("file1")?.create_file()?;
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file2")?.create_file()?;

        sqlarfs::Result::Ok(())
    })?;

    expect!(command(&[
        "list",
        "--archive",
        &archive_path.to_string_lossy(),
        "--children",
    ]))
    .to(be_ok())
    .map(|output| output.split('\n').map(String::from).collect::<Vec<_>>())
    .to(consist_of([String::from("file1"), String::from("dir")]));

    Ok(())
}

#[test]
fn listing_immediate_children_of_path() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    let mut conn = Connection::create_new(&archive_path)?;

    conn.exec(|archive| {
        archive.open("file1")?.create_file()?;
        archive.open("dir1")?.create_dir()?;
        archive.open("dir1/file2")?.create_file()?;
        archive.open("dir1/dir2")?.create_dir()?;
        archive.open("dir1/dir2/file3")?.create_file()?;

        sqlarfs::Result::Ok(())
    })?;

    expect!(command(&[
        "list",
        "--archive",
        &archive_path.to_string_lossy(),
        "--children",
        "dir1",
    ]))
    .to(be_ok())
    .map(|output| output.split('\n').map(String::from).collect::<Vec<_>>())
    .to(consist_of([
        String::from("dir1/file2"),
        String::from("dir1/dir2"),
    ]));

    Ok(())
}

#[test]
fn listing_files_by_type() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    let mut conn = Connection::create_new(&archive_path)?;

    conn.exec(|archive| {
        archive.open("file1")?.create_file()?;
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file2")?.create_file()?;

        sqlarfs::Result::Ok(())
    })?;

    expect!(command(&[
        "list",
        "--archive",
        &archive_path.to_string_lossy(),
        "--type",
        "file",
    ]))
    .to(be_ok())
    .map(|output| output.split('\n').map(String::from).collect::<Vec<_>>())
    .to(consist_of([
        String::from("file1"),
        String::from("dir/file2"),
    ]));

    Ok(())
}
