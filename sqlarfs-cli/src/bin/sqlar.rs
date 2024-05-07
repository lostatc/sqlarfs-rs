//! A CLI tool for working with SQLite archives.

use std::process::ExitCode;

use clap::Parser;
use sqlarfs_cli::{Cli, UserError};

fn main() -> eyre::Result<ExitCode> {
    color_eyre::install()?;

    if let Err(err) = Cli::parse().dispatch() {
        // User-facing errors should not show a stack trace.
        if let Some(user_err) = err.downcast_ref::<UserError>() {
            eprintln!("{}", user_err);
            return Ok(ExitCode::FAILURE);
        }

        return Err(err);
    }

    Ok(ExitCode::SUCCESS)
}
