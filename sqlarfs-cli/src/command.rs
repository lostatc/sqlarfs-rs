use std::path::{Path, PathBuf};

use sqlarfs::{ArchiveOptions, Connection, OpenOptions};

use super::cli::{Cli, Commands, Create, Extract};
use super::error::user_err;

const SQLAR_EXTENSION: &str = "sqlar";

impl Create {
    pub fn run(&self) -> eyre::Result<()> {
        let source_filename = self.source.file_name().map(Path::new).map_or_else(
            || {
                self.source
                    .parent()
                    .ok_or(user_err!("The source path must have a filename."))
            },
            Ok,
        )?;

        let archive_filename = self.archive.to_owned().unwrap_or_else(|| {
            let mut filename = source_filename.to_owned();
            filename.set_extension(SQLAR_EXTENSION);
            filename
        });

        let mut conn = Connection::open(archive_filename)?;

        let opts = ArchiveOptions::new()
            .follow_symlinks(self.follow)
            .recursive(self.recursive)
            .preserve_metadata(self.preserve)
            .children(false);

        conn.exec(|archive| archive.archive_with(&self.source, source_filename, &opts))?;

        Ok(())
    }
}

impl Extract {
    pub fn run(&self) -> eyre::Result<()> {
        let mut conn = OpenOptions::new().create(false).open(&self.archive)?;

        conn.exec(|archive| {
            archive.extract(
                &self.source.to_owned().unwrap_or_else(|| PathBuf::from("")),
                &self.dest,
            )
        })?;

        Ok(())
    }
}

impl Cli {
    pub fn dispatch(&self) -> eyre::Result<()> {
        match &self.command {
            Commands::Create(create) => create.run(),
            Commands::Extract(extract) => extract.run(),
            Commands::Archive(_) => todo!(),
        }
    }
}
