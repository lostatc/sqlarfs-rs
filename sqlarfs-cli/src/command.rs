use std::path::Path;

use sqlarfs::{ArchiveOptions, Connection};

use super::cli::{Cli, Commands, Create};
use super::error::user_err;

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

        let mut conn = Connection::open(&self.archive)?;

        let opts = ArchiveOptions::new()
            .follow_symlinks(self.follow)
            .recursive(self.recursive)
            .preserve_metadata(self.preserve)
            .children(false);

        conn.exec(|archive| archive.archive_with(&self.source, source_filename, &opts))?;

        Ok(())
    }
}

impl Cli {
    pub fn dispatch(&self) -> eyre::Result<()> {
        match &self.command {
            Commands::Create(create) => create.run(),
            Commands::Archive(_) => todo!(),
            Commands::Extract(_) => todo!(),
        }
    }
}
