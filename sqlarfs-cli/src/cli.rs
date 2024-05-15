use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args, Debug, Clone)]
pub struct Create {
    /// The files to add to the archive.
    ///
    /// These files are put in the root of the archive.
    pub source: Vec<PathBuf>,

    /// The path of the SQLite archive to create.
    ///
    /// This is required when archiving multiple files or when creating an empty archive.
    #[arg(long, short)]
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
    /// The directory in the filesystem to extract the files into.
    #[arg(default_value = ".")]
    pub dest: PathBuf,

    /// The path of the SQLite archive.
    #[arg(long, short)]
    pub archive: PathBuf,

    /// The path of a specific file or directory in the archive to extract.
    ///
    /// This can be passed multiple times.
    #[arg(short, long)]
    pub source: Vec<PathBuf>,

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FileType {
    /// Regular files.
    File,

    /// Directories.
    Dir,

    /// Symbolic links.
    Symlink,
}

impl From<FileType> for sqlarfs::FileType {
    fn from(kind: FileType) -> Self {
        match kind {
            FileType::File => sqlarfs::FileType::File,
            FileType::Dir => sqlarfs::FileType::Dir,
            FileType::Symlink => sqlarfs::FileType::Symlink,
        }
    }
}

#[derive(Args, Debug, Clone)]
pub struct List {
    /// Only return descendants of this directory.
    pub parent: Option<PathBuf>,

    /// The path of the SQLite archive.
    #[arg(long, short)]
    pub archive: PathBuf,

    /// Return all descendants (children, grandchildren, etc.) (default).
    #[arg(long, default_value = "true", conflicts_with = "children")]
    pub tree: bool,

    /// Only return immediate children.
    #[arg(long, short, default_value = "false", conflicts_with = "tree")]
    pub children: bool,

    /// Only return files of this type.
    #[arg(long, short, value_enum)]
    pub r#type: Option<FileType>,
}

#[derive(Args, Debug, Clone)]
pub struct Remove {
    /// The path of the file or directory to remove.
    pub path: PathBuf,

    /// The path of the SQLite archive.
    #[arg(long, short)]
    pub archive: PathBuf,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Create a new SQLite archive from the given files.
    #[command(visible_alias = "c")]
    Create(Create),

    /// Extract file or directories from an archive.
    ///
    /// Unless --source passed, this extracts the entire archive.
    #[command(visible_alias = "ex")]
    Extract(Extract),

    /// Copy a file or directory into an existing archive.
    #[command(visible_alias = "ar")]
    Archive(Archive),

    /// List files in an archive.
    #[command(visible_alias = "ls")]
    List(List),

    /// Remove a file or directory from an archive.
    #[command(visible_alias = "rm")]
    Remove(Remove),
}
