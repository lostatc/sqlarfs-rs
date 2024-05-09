use clap::Parser;
use sqlarfs_cli::Cli;

pub fn command(args: &[&str]) -> eyre::Result<()> {
    let mut all_args = vec!["sqlar"];
    all_args.extend_from_slice(args);
    Cli::parse_from(all_args).dispatch()
}
