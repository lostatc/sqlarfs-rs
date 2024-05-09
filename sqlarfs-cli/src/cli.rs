use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args, Debug, Clone)]
pub struct Create {
    /// The directory to archive.
    pub source: PathBuf,

    /// The path of the SQLite archive to create.
    pub archive: Option<PathBuf>,

    /// Follow symbolic links.
    #[arg(long, default_value = "false", overrides_with = "_no_follow")]
    pub follow: bool,

    /// Don't follow symbolic links (default).
    #[arg(long = "no-follow", default_value = "true")]
    pub _no_follow: bool,

    /// Copy the given directory recursively (default).
    #[arg(long = "recursive", default_value = "true")]
    _recursive: bool,

    /// Don't copy the given directory recursively.
    #[arg(long, default_value = "false", overrides_with = "_recursive")]
    pub no_recursive: bool,

    /// Preserve file metadata (default).
    #[arg(long = "preserve", default_value = "true")]
    _preserve: bool,

    /// Don't preserve file metadata.
    #[arg(long, default_value = "false", overrides_with = "_preserve")]
    pub no_preserve: bool,
}

#[derive(Args, Debug, Clone)]
pub struct Extract {
    /// The path of the SQLite archive.
    pub archive: PathBuf,

    /// The path in the filesystem to extract the files to.
    #[arg(default_value = ".")]
    pub dest: PathBuf,

    /// The path of a specific file or directory in the archive to extract.
    #[arg(long)]
    pub source: Option<PathBuf>,

    /// Extract given directory recursively (default).
    #[arg(long = "recursive", default_value = "true")]
    _recursive: bool,

    /// Don't extract the given directory recursively.
    #[arg(long, default_value = "false", overrides_with = "_recursive")]
    pub no_recursive: bool,
}

#[derive(Args, Debug, Clone)]
pub struct Archive {
    /// The file or directory in the filesystem to archive.
    pub source: PathBuf,

    /// The destination of the file in the archive.
    pub dest: Option<PathBuf>,

    /// The path of the SQLite archive.
    #[arg(long, short)]
    pub archive: PathBuf,

    /// Follow symbolic links.
    #[arg(long, default_value = "false", overrides_with = "_no_follow")]
    pub follow: bool,

    /// Don't follow symbolic links (default).
    #[arg(long = "no-follow", default_value = "true")]
    pub _no_follow: bool,

    /// Copy the given directory recursively (default).
    #[arg(long = "recursive", default_value = "true")]
    _recursive: bool,

    /// Don't copy the given directory recursively.
    #[arg(long, default_value = "false", overrides_with = "_recursive")]
    pub no_recursive: bool,

    /// Preserve file metadata (default).
    #[arg(long = "preserve", default_value = "true")]
    _preserve: bool,

    /// Don't preserve file metadata.
    #[arg(long, default_value = "false", overrides_with = "_preserve")]
    pub no_preserve: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Create a new SQLite archive from the given directory.
    #[command(visible_alias = "c")]
    Create(Create),

    /// Extract a file or directory from an archive.
    #[command(visible_alias = "ex")]
    Extract(Extract),

    /// Copy a file or directory into an existing archive.
    #[command(visible_alias = "ar")]
    Archive(Archive),
}
