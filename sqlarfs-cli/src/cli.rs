use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args, Debug, Clone)]
pub struct Archive {
    /// The file or directory in the filesystem to archive.
    pub source: PathBuf,

    /// The destination of the file in the archive.
    pub dest: PathBuf,

    /// Follow symbolic links.
    #[arg(long, default_value = "false", overrides_with = "_no_follow")]
    pub follow: bool,

    /// Don't follow symbolic links.
    #[arg(long = "no-follow")]
    pub _no_follow: bool,

    /// Copy the given directory recursively.
    #[arg(long, default_value = "true", overrides_with = "_no_recursive")]
    pub recursive: bool,

    /// Don't copy the given directory recursively.
    #[arg(long = "no-recursive")]
    pub _no_recursive: bool,

    /// Preserve file metadata.
    #[arg(long, default_value = "true", overrides_with = "_no_preserve")]
    pub preserve: bool,

    /// Don't preserve file metadata.
    #[arg(long = "no-preserve")]
    pub _no_preserve: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Copy a file or directory into the archive.
    #[command(visible_alias = "ar")]
    Archive(Archive),
}
