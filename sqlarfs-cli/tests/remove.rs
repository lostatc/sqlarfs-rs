mod common;

use common::{command, root_path};
use sqlarfs::Connection;
use xpct::{be_err, be_false, be_ok, expect};

#[test]
fn errors_when_path_is_root() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    expect!(command(&[
        "remove",
        "-f",
        &archive_path.to_string_lossy(),
        &root_path().to_string_lossy(),
    ]))
    .to(be_err());

    Ok(())
}

#[test]
fn errors_when_archive_does_not_exist() -> eyre::Result<()> {
    expect!(command(&["remove", "-f", "nonexistent.sqlar", "path"])).to(be_err());

    Ok(())
}

#[test]
fn errors_when_path_does_not_exist() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    Connection::create_new(&archive_path)?;

    expect!(command(&[
        "remove",
        "-f",
        &archive_path.to_string_lossy(),
        "nonexistent"
    ]))
    .to(be_err());

    Ok(())
}

#[test]
fn removes_path_from_archive() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    Connection::create_new(&archive_path)?.exec(|archive| {
        archive.open("file")?.create_file()?;

        sqlarfs::Result::Ok(())
    })?;

    expect!(command(&[
        "remove",
        "-f",
        &archive_path.to_string_lossy(),
        "file"
    ]))
    .to(be_ok());

    let file_exists =
        Connection::open(&archive_path)?.exec(|archive| archive.open("file")?.exists())?;

    expect!(file_exists).to(be_false());

    Ok(())
}
