//! A CLI tool for working with SQLite archives.

use std::process::ExitCode;

use clap::Parser;
use sqlarfs::Error;
use sqlarfs_cli::Cli;

fn run() -> eyre::Result<()> {
    let _cli = Cli::parse();

    // TODO: Run the command.

    Ok(())
}

fn main() -> eyre::Result<ExitCode> {
    color_eyre::install()?;

    if let Err(err) = run() {
        // User-facing errors should not show a stack trace.
        if let Some(user_err) = err.downcast_ref::<Error>() {
            eprintln!("{}", user_err);
            return Ok(ExitCode::FAILURE);
        }

        return Err(err);
    }

    Ok(ExitCode::SUCCESS)
}
