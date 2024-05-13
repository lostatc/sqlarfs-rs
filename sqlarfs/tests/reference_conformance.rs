#![cfg(feature = "reference-conformance-tests")]

mod common;

use std::env;
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use std::process;

use common::dump_table;
use common::have_same_contents;
use common::have_same_mtime;
use common::have_same_permissions;
use common::have_same_symlink_target;
use serial_test::serial;
use sqlarfs::Connection;
use sqlarfs::FileMode;
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

//
// These tests need to change the process's working directory to ensure the paths in the SQLite
// archive are all relative paths. To prevent race conditions, that means they must be run
// serially.
//

#[test]
#[serial(change_directory)]
fn archive_empty_regular_file() -> sqlarfs::Result<()> {
    let db_dir = tempfile::tempdir()?;
    let reference_db = db_dir.path().join("reference.sqlar");
    let crate_db = db_dir.path().join("crate.sqlar");

    let temp_dir = tempfile::tempdir()?;
    fs::File::create(temp_dir.path().join("file"))?;

    env::set_current_dir(temp_dir.path())?;

    sqlar_command(&reference_db, &["--create", "file"])?;

    Connection::create_new(&crate_db)?.exec(|archive| {
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
    let reference_db = db_dir.path().join("reference.sqlar");
    let crate_db = db_dir.path().join("crate.sqlar");

    let temp_dir = tempfile::tempdir()?;
    let mut file = fs::File::create(temp_dir.path().join("file"))?;

    write!(&mut file, "file contents")?;
    file.sync_all()?;

    env::set_current_dir(temp_dir.path())?;

    sqlar_command(&reference_db, &["--create", "file"])?;

    Connection::create_new(&crate_db)?.exec(|archive| {
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
    use std::os::unix::fs::symlink;

    let db_dir = tempfile::tempdir()?;
    let reference_db = db_dir.path().join("reference.sqlar");
    let crate_db = db_dir.path().join("crate.sqlar");

    let temp_dir = tempfile::tempdir()?;
    let symlink_target = tempfile::NamedTempFile::new()?;
    symlink(symlink_target.path(), temp_dir.path().join("symlink"))?;

    env::set_current_dir(temp_dir.path())?;

    sqlar_command(&reference_db, &["--create", "symlink"])?;

    Connection::create_new(&crate_db)?.exec(|archive| {
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
    let reference_db = db_dir.path().join("reference.sqlar");
    let crate_db = db_dir.path().join("crate.sqlar");

    let temp_dir = tempfile::tempdir()?;
    fs::create_dir_all(temp_dir.path().join("source/dir"))?;

    env::set_current_dir(temp_dir.path())?;

    sqlar_command(&reference_db, &["--create", "source"])?;

    Connection::create_new(&crate_db)?.exec(|archive| {
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
    let reference_db = db_dir.path().join("reference.sqlar");
    let crate_db = db_dir.path().join("crate.sqlar");

    let temp_dir = tempfile::tempdir()?;
    fs::create_dir_all(temp_dir.path().join("source/dir"))?;
    fs::File::create(temp_dir.path().join("source/file1"))?;
    fs::File::create(temp_dir.path().join("source/dir/file2"))?;

    env::set_current_dir(temp_dir.path())?;

    sqlar_command(&reference_db, &["--create", "source"])?;

    Connection::create_new(&crate_db)?.exec(|archive| {
        let opts = sqlarfs::ArchiveOptions::new().children(true);
        archive.archive_with(temp_dir.path(), "", &opts)
    })?;

    expect!(dump_table(&crate_db)?).to(consist_of(dump_table(&reference_db)?));

    Ok(())
}

#[test]
#[serial(change_directory)]
fn archive_regular_file_with_readonly_permissions() -> sqlarfs::Result<()> {
    let db_dir = tempfile::tempdir()?;
    let reference_db = db_dir.path().join("reference.sqlar");
    let crate_db = db_dir.path().join("crate.sqlar");

    let temp_dir = tempfile::tempdir()?;
    let file = fs::File::create(temp_dir.path().join("file"))?;

    let mut permissions = file.metadata()?.permissions();
    permissions.set_readonly(true);
    file.set_permissions(permissions)?;

    env::set_current_dir(temp_dir.path())?;

    sqlar_command(&reference_db, &["--create", "file"])?;

    Connection::create_new(&crate_db)?.exec(|archive| {
        let opts = sqlarfs::ArchiveOptions::new().children(true);
        archive.archive_with(temp_dir.path(), "", &opts)
    })?;

    expect!(dump_table(&crate_db)?).to(consist_of(dump_table(&reference_db)?));

    Ok(())
}

#[test]
#[serial(change_directory)]
fn extract_empty_regular_file() -> sqlarfs::Result<()> {
    let db_dir = tempfile::tempdir()?;
    let db = db_dir.path().join("test.sqlar");

    let crate_dest_dir = tempfile::tempdir()?;
    let reference_dest_dir = tempfile::tempdir()?;

    Connection::create_new(&db)?.exec(|archive| {
        archive.open("file")?.create_file()?;

        archive.extract("file", &crate_dest_dir.path().join("file"))
    })?;

    env::set_current_dir(reference_dest_dir.path())?;
    sqlar_command(&db, &["--extract", "file"])?;

    expect!(crate_dest_dir.path().join("file"))
        .to(have_same_contents(reference_dest_dir.path().join("file")));
    expect!(crate_dest_dir.path().join("file")).to(have_same_permissions(
        reference_dest_dir.path().join("file"),
    ));
    expect!(crate_dest_dir.path().join("file"))
        .to(have_same_mtime(reference_dest_dir.path().join("file")));

    Ok(())
}

#[test]
#[serial(change_directory)]
fn extract_regular_file_with_data() -> sqlarfs::Result<()> {
    let db_dir = tempfile::tempdir()?;
    let db = db_dir.path().join("test.sqlar");

    let crate_dest_dir = tempfile::tempdir()?;
    let reference_dest_dir = tempfile::tempdir()?;

    Connection::create_new(&db)?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create_file()?;
        file.write_str("file contents")?;

        archive.extract("file", &crate_dest_dir.path().join("file"))
    })?;

    env::set_current_dir(reference_dest_dir.path())?;
    sqlar_command(&db, &["--extract", "file"])?;

    expect!(crate_dest_dir.path().join("file"))
        .to(have_same_contents(reference_dest_dir.path().join("file")));
    expect!(crate_dest_dir.path().join("file")).to(have_same_permissions(
        reference_dest_dir.path().join("file"),
    ));
    expect!(crate_dest_dir.path().join("file"))
        .to(have_same_mtime(reference_dest_dir.path().join("file")));

    Ok(())
}

// TODO: We need to ignore this test until the following bug fixes in SQLite are released:
// - https://www.sqlite.org/src/info/4d90c3f179a3d735
// - https://www.sqlite.org/src/info/2bf8c3f99ad8b74f
#[test]
#[serial(change_directory)]
#[cfg(unix)]
#[ignore]
fn extract_symlink() -> sqlarfs::Result<()> {
    let db_dir = tempfile::tempdir()?;
    let db = db_dir.path().join("test.sqlar");

    let crate_dest_dir = tempfile::tempdir()?;
    let reference_dest_dir = tempfile::tempdir()?;

    let symlink_target = tempfile::NamedTempFile::new()?;

    Connection::create_new(&db)?.exec(|archive| {
        archive
            .open("symlink")?
            .create_symlink(symlink_target.path())?;

        archive.extract("symlink", &crate_dest_dir.path().join("symlink"))
    })?;

    env::set_current_dir(reference_dest_dir.path())?;
    sqlar_command(&db, &["--extract", "symlink"])?;

    dbg!(reference_dest_dir.path().join("symlink").exists());

    expect!(crate_dest_dir.path().join("symlink")).to(have_same_symlink_target(
        reference_dest_dir.path().join("symlink"),
    ));
    expect!(crate_dest_dir.path().join("symlink")).to(have_same_permissions(
        reference_dest_dir.path().join("symlink"),
    ));
    expect!(crate_dest_dir.path().join("symlink"))
        .to(have_same_mtime(reference_dest_dir.path().join("symlink")));

    Ok(())
}

#[test]
#[serial(change_directory)]
fn extract_empty_directory() -> sqlarfs::Result<()> {
    let db_dir = tempfile::tempdir()?;
    let db = db_dir.path().join("test.sqlar");

    let crate_dest_dir = tempfile::tempdir()?;
    let reference_dest_dir = tempfile::tempdir()?;

    Connection::create_new(&db)?.exec(|archive| {
        archive.open("dir")?.create_dir()?;

        archive.extract("dir", &crate_dest_dir.path().join("dir"))
    })?;

    env::set_current_dir(reference_dest_dir.path())?;
    sqlar_command(&db, &["--extract", "dir"])?;

    expect!(crate_dest_dir.path().join("dir"))
        .to(have_same_permissions(reference_dest_dir.path().join("dir")));
    expect!(crate_dest_dir.path().join("dir"))
        .to(have_same_mtime(reference_dest_dir.path().join("dir")));

    Ok(())
}

#[test]
#[serial(change_directory)]
fn extract_directory_with_children() -> sqlarfs::Result<()> {
    let db_dir = tempfile::tempdir()?;
    let db = db_dir.path().join("test.sqlar");

    let crate_dest_dir = tempfile::tempdir()?;
    let reference_dest_dir = tempfile::tempdir()?;

    Connection::create_new(&db)?.exec(|archive| {
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file1")?.create_file()?;
        archive.open("dir/subdir")?.create_dir()?;
        archive.open("dir/subdir/file2")?.create_file()?;

        archive.extract("dir", &crate_dest_dir.path().join("dir"))
    })?;

    env::set_current_dir(reference_dest_dir.path())?;
    sqlar_command(&db, &["--extract", "dir"])?;

    expect!(crate_dest_dir.path().join("dir"))
        .to(have_same_permissions(reference_dest_dir.path().join("dir")));
    expect!(crate_dest_dir.path().join("dir"))
        .to(have_same_mtime(reference_dest_dir.path().join("dir")));

    Ok(())
}

#[test]
#[serial(change_directory)]
fn extract_regular_file_with_readonly_permissions() -> sqlarfs::Result<()> {
    let db_dir = tempfile::tempdir()?;
    let db = db_dir.path().join("test.sqlar");

    let crate_dest_dir = tempfile::tempdir()?;
    let reference_dest_dir = tempfile::tempdir()?;

    Connection::create_new(&db)?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create_file()?;
        file.set_mode(Some(
            FileMode::OWNER_R | FileMode::GROUP_R | FileMode::OTHER_R,
        ))?;

        archive.extract("file", &crate_dest_dir.path().join("file"))
    })?;

    env::set_current_dir(reference_dest_dir.path())?;
    sqlar_command(&db, &["--extract", "file"])?;

    expect!(crate_dest_dir.path().join("file")).to(have_same_permissions(
        reference_dest_dir.path().join("file"),
    ));

    Ok(())
}
