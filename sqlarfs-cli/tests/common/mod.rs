use std::path::PathBuf;

use clap::Parser;
use sqlarfs_cli::Cli;

pub fn command(args: &[&str]) -> eyre::Result<String> {
    let mut output = Vec::new();
    let mut all_args = vec!["sqlar"];

    all_args.extend_from_slice(args);
    Cli::parse_from(all_args).dispatch(&mut output)?;

    Ok(String::from_utf8(output)?.trim().to_owned())
}

pub fn root_path() -> PathBuf {
    PathBuf::from(if cfg!(windows) { r"C:\" } else { "/" })
}
