mod common;

use clap::Parser;
use common::command;
use sqlarfs::Connection;
use sqlarfs_cli::{Cli, Commands, List};
use xpct::{be_ok, consist_of, equal, expect, match_pattern, pattern};

#[test]
fn tree_flag_can_be_overridden() -> eyre::Result<()> {
    let cli = Cli::parse_from(["sqlar", "list", "nonexistent.sqlar"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::List(List {
        no_tree: false,
        ..
    }))));

    let cli = Cli::parse_from(["sqlar", "list", "--tree", "nonexistent.sqlar"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::List(List {
        no_tree: false,
        ..
    }))));

    let cli = Cli::parse_from(["sqlar", "list", "--no-tree", "nonexistent.sqlar"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::List(List {
        no_tree: true,
        ..
    }))));

    let cli = Cli::parse_from(["sqlar", "list", "--tree", "--no-tree", "nonexistent.sqlar"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::List(List {
        no_tree: true,
        ..
    }))));

    let cli = Cli::parse_from(["sqlar", "list", "--no-tree", "--tree", "nonexistent.sqlar"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::List(List {
        no_tree: false,
        ..
    }))));

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
        "--tree",
        &archive_path.to_string_lossy()
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
        "--tree",
        &archive_path.to_string_lossy(),
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
        "--no-tree",
        &archive_path.to_string_lossy()
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
        "--no-tree",
        &archive_path.to_string_lossy(),
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
        "--type",
        "file",
        &archive_path.to_string_lossy(),
    ]))
    .to(be_ok())
    .map(|output| output.split('\n').map(String::from).collect::<Vec<_>>())
    .to(consist_of([
        String::from("file1"),
        String::from("dir/file2"),
    ]));

    Ok(())
}
