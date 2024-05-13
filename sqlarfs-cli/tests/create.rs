mod common;

use std::env;
use std::fs;

use clap::Parser;
use serial_test::serial;
use sqlarfs::Connection;
use sqlarfs_cli::{Cli, Commands, Create};
use xpct::be_empty;
use xpct::{be_err, be_existing_file, expect, match_pattern, pattern};

use common::{command, root_path};

#[test]
fn errors_when_source_path_is_root() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    expect!(command(&[
        "create",
        &root_path().to_string_lossy(),
        &archive_path.to_string_lossy()
    ]))
    .to(be_err());

    Ok(())
}

#[test]
fn recursive_flag_can_be_overridden() -> eyre::Result<()> {
    let cli = Cli::parse_from(["sqlar", "create", "nonexistent"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        no_recursive: false,
        ..
    }))));

    let cli = Cli::parse_from(["sqlar", "create", "--recursive", "nonexistent"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        no_recursive: false,
        ..
    }))));

    let cli = Cli::parse_from(["sqlar", "create", "--no-recursive", "nonexistent"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        no_recursive: true,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "create",
        "--recursive",
        "--no-recursive",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        no_recursive: true,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "create",
        "--no-recursive",
        "--recursive",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        no_recursive: false,
        ..
    }))));

    Ok(())
}

#[test]
fn follow_flag_can_be_overridden() -> eyre::Result<()> {
    let cli = Cli::parse_from(["sqlar", "create", "nonexistent"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        follow: false,
        ..
    }))));

    let cli = Cli::parse_from(["sqlar", "create", "--follow", "nonexistent"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        follow: true,
        ..
    }))));

    let cli = Cli::parse_from(["sqlar", "create", "--no-follow", "nonexistent"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        follow: false,
        ..
    }))));

    let cli = Cli::parse_from(["sqlar", "create", "--follow", "--no-follow", "nonexistent"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        follow: false,
        ..
    }))));

    let cli = Cli::parse_from(["sqlar", "create", "--no-follow", "--follow", "nonexistent"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        follow: true,
        ..
    }))));

    Ok(())
}

#[test]
fn preserve_flag_can_be_overridden() -> eyre::Result<()> {
    let cli = Cli::parse_from(["sqlar", "create", "nonexistent"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        no_preserve: false,
        ..
    }))));

    let cli = Cli::parse_from(["sqlar", "create", "--preserve", "nonexistent"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        no_preserve: false,
        ..
    }))));

    let cli = Cli::parse_from(["sqlar", "create", "--no-preserve", "nonexistent"]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        no_preserve: true,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "create",
        "--preserve",
        "--no-preserve",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        no_preserve: true,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "create",
        "--no-preserve",
        "--preserve",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Create(Create {
        no_preserve: false,
        ..
    }))));

    Ok(())
}

#[test]
#[serial(change_directory)]
fn creates_archive_file_in_current_directory_with_sqlar_file_extension() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let source_path = temp_dir.path().join("source");
    fs::File::create(&source_path)?;

    env::set_current_dir(temp_dir.path())?;

    command(&["create", &source_path.to_string_lossy()])?;

    expect!(temp_dir.path().join("source.sqlar")).to(be_existing_file());

    Ok(())
}

#[test]
fn creates_archive_file_at_path() -> eyre::Result<()> {
    let source_file = tempfile::NamedTempFile::new()?;
    let archive_file = tempfile::NamedTempFile::new()?;

    command(&[
        "create",
        "-f",
        &archive_file.path().to_string_lossy(),
        &source_file.path().to_string_lossy(),
    ])?;

    expect!(archive_file.path()).to(be_existing_file());

    Ok(())
}

#[test]
fn creating_db_that_already_exists_errors() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("test.sqlar");
    let source_file = tempfile::NamedTempFile::new()?;

    Connection::create_new(&db_path)?;

    expect!(command(&[
        "create",
        &source_file.path().to_string_lossy(),
        &db_path.to_string_lossy()
    ]))
    .to(be_err());

    Ok(())
}

#[test]
fn archiving_no_files_creates_an_empty_archive() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    command(&["create", "-f", &archive_path.to_string_lossy()])?;

    let mut conn = Connection::open(&archive_path)?;

    let files = conn.exec(|archive| sqlarfs::Result::Ok(archive.list()?.collect::<Vec<_>>()))?;

    expect!(files).to(be_empty());

    Ok(())
}

#[test]
fn archive_path_is_required_when_archiving_no_files() -> eyre::Result<()> {
    expect!(command(&["create"])).to(be_err());

    Ok(())
}

#[test]
fn archive_path_is_required_when_archiving_multiple_files() -> eyre::Result<()> {
    let file1 = tempfile::NamedTempFile::new()?;
    let file2 = tempfile::NamedTempFile::new()?;

    expect!(command(&[
        "create",
        &file1.path().to_string_lossy(),
        &file2.path().to_string_lossy()
    ]))
    .to(be_err());

    Ok(())
}
