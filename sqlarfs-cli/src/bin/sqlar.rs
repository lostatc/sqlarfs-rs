//! A CLI tool for working with SQLite archives.

use std::io;
use std::process::ExitCode;

use clap::Parser;
use sqlarfs_cli::Cli;

fn main() -> eyre::Result<ExitCode> {
    color_eyre::install()?;

    if let Err(err) = Cli::parse().dispatch(io::stdout()) {
        if let Some(user_err) = err.downcast_ref::<sqlarfs::Error>() {
            eprintln!("Error: {}", user_err);
            return Ok(ExitCode::FAILURE);
        }

        return Err(err);
    }

    Ok(ExitCode::SUCCESS)
}
