mod common;

use clap::Parser;
use common::{command, root_path};
use sqlarfs_cli::{Archive, Cli, Commands};
use xpct::{be_err, be_ok, be_true, expect, match_pattern, pattern};

#[test]
fn errors_when_source_path_is_root() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    expect!(command(&[
        "archive",
        "--archive",
        &archive_path.to_string_lossy(),
        &root_path().to_string_lossy(),
    ]))
    .to(be_err());

    Ok(())
}

#[test]
fn recursive_flag_can_be_overridden() -> eyre::Result<()> {
    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        no_recursive: false,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "--recursive",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        no_recursive: false,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "--no-recursive",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        no_recursive: true,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "--recursive",
        "--no-recursive",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        no_recursive: true,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "--no-recursive",
        "--recursive",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        no_recursive: false,
        ..
    }))));

    Ok(())
}

#[test]
fn follow_flag_can_be_overridden() -> eyre::Result<()> {
    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        follow: false,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "--follow",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        follow: true,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "--no-follow",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        follow: false,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "--follow",
        "--no-follow",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        follow: false,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "--no-follow",
        "--follow",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        follow: true,
        ..
    }))));

    Ok(())
}

#[test]
fn preserve_flag_can_be_overridden() -> eyre::Result<()> {
    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        no_preserve: false,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "--preserve",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        no_preserve: false,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "--no-preserve",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        no_preserve: true,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "--preserve",
        "--no-preserve",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        no_preserve: true,
        ..
    }))));

    let cli = Cli::parse_from([
        "sqlar",
        "archive",
        "--archive",
        "nonexistent.sqlar",
        "--no-preserve",
        "--preserve",
        "nonexistent",
    ]);
    expect!(cli.command).to(match_pattern(pattern!(Commands::Archive(Archive {
        no_preserve: false,
        ..
    }))));

    Ok(())
}

#[test]
fn archiving_errors_if_archive_does_not_exist() -> eyre::Result<()> {
    let source_file = tempfile::NamedTempFile::new()?;

    expect!(command(&[
        "archive",
        "--archive",
        "nonexistent.sqlar",
        &source_file.path().to_string_lossy(),
    ]))
    .to(be_err());

    Ok(())
}

#[test]
fn archiving_errors_if_source_does_not_exist() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");

    sqlarfs::Connection::create_new(&archive_path)?;

    expect!(command(&[
        "archive",
        "--archive",
        &archive_path.to_string_lossy(),
        "nonexistent",
    ]))
    .to(be_err());

    Ok(())
}

#[test]
fn archive_file_to_archive_root() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");
    let source_file = tempfile::NamedTempFile::new()?;

    sqlarfs::Connection::create_new(&archive_path)?;

    expect!(command(&[
        "archive",
        "--archive",
        &archive_path.to_string_lossy(),
        &source_file.path().to_string_lossy(),
    ]))
    .to(be_ok());

    let mut conn = sqlarfs::Connection::open(&archive_path)?;

    let file_exists = conn.exec(|archive| {
        sqlarfs::Result::Ok(
            archive
                .open(source_file.path().file_name().unwrap())?
                .metadata()?
                .is_file(),
        )
    })?;

    expect!(file_exists).to(be_true());

    Ok(())
}

#[test]
fn archive_file_to_dest_in_archive_root() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");
    let source_file = tempfile::NamedTempFile::new()?;

    sqlarfs::Connection::create_new(&archive_path)?;

    expect!(command(&[
        "archive",
        "--archive",
        &archive_path.to_string_lossy(),
        &source_file.path().to_string_lossy(),
        "dest",
    ]))
    .to(be_ok());

    let mut conn = sqlarfs::Connection::open(&archive_path)?;

    let file_exists =
        conn.exec(|archive| sqlarfs::Result::Ok(archive.open("dest")?.metadata()?.is_file()))?;

    expect!(file_exists).to(be_true());

    Ok(())
}

#[test]
fn archiving_to_dest_creates_parent_directories() -> eyre::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("test.sqlar");
    let source_file = tempfile::NamedTempFile::new()?;

    sqlarfs::Connection::create_new(&archive_path)?;

    expect!(command(&[
        "archive",
        "--archive",
        &archive_path.to_string_lossy(),
        &source_file.path().to_string_lossy(),
        "path/to/dest",
    ]))
    .to(be_ok());

    let mut conn = sqlarfs::Connection::open(&archive_path)?;

    let file_exists = conn
        .exec(|archive| sqlarfs::Result::Ok(archive.open("path/to/dest")?.metadata()?.is_file()))?;

    expect!(file_exists).to(be_true());

    Ok(())
}
