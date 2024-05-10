//! A CLI tool for working with SQLite archives.

use std::process::ExitCode;

use clap::Parser;
use sqlarfs_cli::Cli;

fn main() -> eyre::Result<ExitCode> {
    color_eyre::install()?;

    Cli::parse().dispatch()?;

    Ok(ExitCode::SUCCESS)
}
