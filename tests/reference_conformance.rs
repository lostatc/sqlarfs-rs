#![cfg(feature = "reference-conformance-tests")]

mod common;

use std::env;
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use std::process;

use serial_test::serial;
use sqlarfs::Connection;
use xpct::{consist_of, expect};

fn sqlar_command(db: &Path, args: &[&str]) -> sqlarfs::Result<()> {
    let output = process::Command::new("sqlite3")
        .arg("-A")
        .arg("--file")
        .arg(db.to_string_lossy().as_ref())
        .args(args)
        .output()?;

    if !output.status.success() {
        panic!(
            "Failed executing sqlite3 command:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SqlarTableRow {
    name: String,
    mode: Option<u32>,
    mtime: Option<u64>,
    sz: Option<i64>,
    data: Option<Vec<u8>>,
}

fn dump_table(db: &Path) -> sqlarfs::Result<Vec<SqlarTableRow>> {
    rusqlite::Connection::open_with_flags(
        db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sqlarfs::Error::from)?
    .prepare("SELECT name, mode, mtime, sz, data FROM sqlar;")?
    .query_map([], |row| {
        let name = row.get(0)?;
        let mode = row.get(1)?;
        let mtime = row.get(2)?;
        let sz = row.get(3)?;

        Ok(SqlarTableRow {
            name,
            mode,
            mtime,
            sz,
            data: if sz == Some(-1) {
                row.get::<_, String>(4)?.as_bytes().to_vec().into()
            } else {
                row.get(4)?
            },
        })
    })?
    .collect::<Result<Vec<_>, _>>()
    .map_err(sqlarfs::Error::from)
}

//
// These tests need to change the process's working directory to ensure the paths in the SQLite
// archive are all relative paths. To prevent race conditions, that means they must be run
// serially.
//

#[test]
#[serial(change_directory)]
fn archive_empty_regular_file() -> sqlarfs::Result<()> {
    let db_dir = tempfile::tempdir()?;
    let reference_db = db_dir.path().join("reference.db");
    let crate_db = db_dir.path().join("crate.db");

    let temp_dir = tempfile::tempdir()?;
    fs::File::create(temp_dir.path().join("file"))?;

    env::set_current_dir(temp_dir.path())?;

    sqlar_command(&reference_db, &["--create", "file"])?;

    Connection::open(&crate_db)?.exec(|archive| {
        let opts = sqlarfs::ArchiveOptions::new().children(true);
        archive.archive_with(temp_dir.path(), "", &opts)
    })?;

    expect!(dump_table(&crate_db)?).to(consist_of(dump_table(&reference_db)?));

    Ok(())
}

#[test]
#[serial(change_directory)]
fn archive_regular_file_with_data() -> sqlarfs::Result<()> {
    let db_dir = tempfile::tempdir()?;
    let reference_db = db_dir.path().join("reference.db");
    let crate_db = db_dir.path().join("crate.db");

    let temp_dir = tempfile::tempdir()?;
    let mut file = fs::File::create(temp_dir.path().join("file"))?;

    write!(&mut file, "file contents")?;
    file.sync_all()?;

    env::set_current_dir(temp_dir.path())?;

    sqlar_command(&reference_db, &["--create", "file"])?;

    Connection::open(&crate_db)?.exec(|archive| {
        let opts = sqlarfs::ArchiveOptions::new().children(true);
        archive.archive_with(temp_dir.path(), "", &opts)
    })?;

    expect!(dump_table(&crate_db)?).to(consist_of(dump_table(&reference_db)?));

    Ok(())
}

#[test]
#[serial(change_directory)]
#[cfg(unix)]
fn archive_symlink() -> sqlarfs::Result<()> {
    use nix::unistd::symlinkat;

    use crate::common::into_sqlarfs_error;

    let db_dir = tempfile::tempdir()?;
    let reference_db = db_dir.path().join("reference.db");
    let crate_db = db_dir.path().join("crate.db");

    let temp_dir = tempfile::tempdir()?;
    let symlink_target = tempfile::NamedTempFile::new()?;
    symlinkat(
        symlink_target.path(),
        None,
        &temp_dir.path().join("symlink"),
    )
    .map_err(into_sqlarfs_error)?;

    env::set_current_dir(temp_dir.path())?;

    sqlar_command(&reference_db, &["--create", "symlink"])?;

    Connection::open(&crate_db)?.exec(|archive| {
        let opts = sqlarfs::ArchiveOptions::new().children(true);
        archive.archive_with(temp_dir.path(), "", &opts)
    })?;

    expect!(dump_table(&crate_db)?).to(consist_of(dump_table(&reference_db)?));

    Ok(())
}

#[test]
#[serial(change_directory)]
fn archive_empty_directory() -> sqlarfs::Result<()> {
    let db_dir = tempfile::tempdir()?;
    let reference_db = db_dir.path().join("reference.db");
    let crate_db = db_dir.path().join("crate.db");

    let temp_dir = tempfile::tempdir()?;
    fs::create_dir_all(temp_dir.path().join("source/dir"))?;

    env::set_current_dir(temp_dir.path())?;

    sqlar_command(&reference_db, &["--create", "source"])?;

    Connection::open(&crate_db)?.exec(|archive| {
        let opts = sqlarfs::ArchiveOptions::new().children(true);
        archive.archive_with(temp_dir.path(), "", &opts)
    })?;

    expect!(dump_table(&crate_db)?).to(consist_of(dump_table(&reference_db)?));

    Ok(())
}

#[test]
#[serial(change_directory)]
fn archive_directory_with_children() -> sqlarfs::Result<()> {
    let db_dir = tempfile::tempdir()?;
    let reference_db = db_dir.path().join("reference.db");
    let crate_db = db_dir.path().join("crate.db");

    let temp_dir = tempfile::tempdir()?;
    fs::create_dir_all(temp_dir.path().join("source/dir"))?;
    fs::File::create(temp_dir.path().join("source/file1"))?;
    fs::File::create(temp_dir.path().join("source/dir/file2"))?;

    env::set_current_dir(temp_dir.path())?;

    sqlar_command(&reference_db, &["--create", "source"])?;

    Connection::open(&crate_db)?.exec(|archive| {
        let opts = sqlarfs::ArchiveOptions::new().children(true);
        archive.archive_with(temp_dir.path(), "", &opts)
    })?;

    expect!(dump_table(&crate_db)?).to(consist_of(dump_table(&reference_db)?));

    Ok(())
}
