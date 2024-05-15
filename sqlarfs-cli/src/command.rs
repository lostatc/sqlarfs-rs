use std::io::Write;
use std::path::{Path, PathBuf};

use sqlarfs::{ArchiveOptions, Connection, ExtractOptions, ListOptions};

use super::cli::{Archive, Cli, Commands, Create, Extract, List, Remove};

const SQLAR_EXTENSION: &str = "sqlar";

fn file_name(path: &Path) -> Option<&Path> {
    path.file_name()
        .map(Path::new)
        .or_else(|| path.parent().and_then(|p| p.file_name().map(Path::new)))
}

impl Create {
    pub fn run(&self) -> eyre::Result<()> {
        let archive_filename = if self.source.is_empty() {
            self.archive.clone().ok_or(sqlarfs::Error::InvalidArgs {
                reason: String::from("When no files are being added to the archive, the archive path must be specified."),
            })?
        } else if self.source.len() == 1 {
            let source_filename =
                file_name(&self.source[0]).ok_or(sqlarfs::Error::InvalidArgs {
                    reason: String::from("The source path must have a filename."),
                })?;

            self.archive.to_owned().unwrap_or_else(|| {
                let mut filename = source_filename.to_owned();
                filename.set_extension(SQLAR_EXTENSION);
                filename
            })
        } else {
            self.archive.clone().ok_or(sqlarfs::Error::InvalidArgs {
                reason: String::from(
                    "When archiving multiple files, the archive path must be specified.",
                ),
            })?
        };

        let mut conn = Connection::create_new(archive_filename)?;

        let opts = ArchiveOptions::new()
            .follow_symlinks(self.follow)
            .recursive(!self.no_recursive)
            .preserve_metadata(!self.no_preserve)
            .children(false);

        conn.exec(|archive| {
            for source_path in &self.source {
                let source_filename =
                    file_name(source_path).ok_or(sqlarfs::Error::InvalidArgs {
                        reason: String::from("The source path must have a filename."),
                    })?;

                archive.archive_with(source_path, source_filename, &opts)?;
            }

            sqlarfs::Result::Ok(())
        })?;

        Ok(())
    }
}

impl Extract {
    pub fn run(&self) -> eyre::Result<()> {
        let mut conn = Connection::open(&self.archive)?;

        if let Some(source) = &self.source {
            if source.file_name().is_none() {
                return Err(sqlarfs::Error::InvalidArgs {
                    reason: String::from("The source path must have a filename."),
                }
                .into());
            }
        }

        conn.exec(|archive| match &self.source {
            Some(source) => archive.extract_with(
                source,
                &self.dest.join(source.file_name().expect("The source directory does not have a filename, but we should have already checked for this. This is a bug.")),
                &ExtractOptions::new().children(false).recursive(!self.no_recursive),
            ),
            None => archive.extract_with("", &self.dest, &ExtractOptions::new().children(true).recursive(!self.no_recursive)),
        })?;

        Ok(())
    }
}

impl Archive {
    pub fn run(&self) -> eyre::Result<()> {
        let mut conn = Connection::open(&self.archive)?;

        let opts = ArchiveOptions::new()
            .follow_symlinks(self.follow)
            .recursive(!self.no_recursive)
            .preserve_metadata(!self.no_preserve)
            .children(false);

        conn.exec(|archive| {
            let dest_path = if let Some(dest) = &self.dest {
                dest
            } else {
                file_name(&self.source).ok_or(sqlarfs::Error::InvalidArgs {
                    reason: String::from("The source path must have a filename."),
                })?
            };

            if let Some(parent) = dest_path.parent() {
                if parent != Path::new("") {
                    archive.open(parent)?.create_dir_all()?;
                }
            }

            archive.archive_with(&self.source, dest_path, &opts)?;

            sqlarfs::Result::Ok(())
        })?;

        Ok(())
    }
}

impl List {
    pub fn run(&self, mut stdout: impl Write) -> eyre::Result<()> {
        let mut conn = Connection::open(&self.archive)?;

        // We always sort by depth.
        let mut opts = ListOptions::new().by_depth();

        if self.no_tree {
            opts = opts.children_of(self.parent.as_ref().unwrap_or(&PathBuf::from("")));
        } else {
            opts = opts.descendants_of(self.parent.as_ref().unwrap_or(&PathBuf::from("")));
        }

        if let Some(kind) = self.r#type {
            opts = opts.file_type(kind.into());
        }

        conn.exec(|archive| {
            for entry in archive.list_with(&opts)? {
                writeln!(stdout, "{}", entry?.path().to_string_lossy())?;
            }

            sqlarfs::Result::Ok(())
        })?;

        Ok(())
    }
}

impl Remove {
    pub fn run(&self) -> eyre::Result<()> {
        let mut conn = Connection::open(&self.archive)?;

        conn.exec(|archive| archive.open(&self.path)?.delete())?;

        Ok(())
    }
}

impl Cli {
    pub fn dispatch(&self, stdout: impl Write) -> eyre::Result<()> {
        match &self.command {
            Commands::Create(create) => create.run(),
            Commands::Extract(extract) => extract.run(),
            Commands::Archive(archive) => archive.run(),
            Commands::List(list) => list.run(stdout),
            Commands::Remove(remove) => remove.run(),
        }
    }
}
