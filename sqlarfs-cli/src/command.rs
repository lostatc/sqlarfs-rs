use std::path::Path;

use sqlarfs::{ArchiveOptions, Connection, ExtractOptions, OpenOptions};

use super::cli::{Cli, Commands, Create, Extract};

const SQLAR_EXTENSION: &str = "sqlar";

fn file_name(path: &Path) -> Option<&Path> {
    path.file_name().map(Path::new).or_else(|| path.parent())
}

impl Create {
    pub fn run(&self) -> eyre::Result<()> {
        let source_filename = file_name(&self.source).ok_or(sqlarfs::Error::msg(
            sqlarfs::ErrorKind::InvalidArgs,
            "The source path must have a filename.",
        ))?;

        let archive_filename = self.archive.to_owned().unwrap_or_else(|| {
            let mut filename = source_filename.to_owned();
            filename.set_extension(SQLAR_EXTENSION);
            filename
        });

        let mut conn = Connection::open(archive_filename)?;

        let opts = ArchiveOptions::new()
            .follow_symlinks(self.follow)
            .recursive(!self.no_recursive)
            .preserve_metadata(!self.no_preserve)
            .children(false);

        conn.exec(|archive| archive.archive_with(&self.source, source_filename, &opts))?;

        Ok(())
    }
}

impl Extract {
    pub fn run(&self) -> eyre::Result<()> {
        let mut conn = OpenOptions::new().create(false).open(&self.archive)?;

        if let Some(source) = &self.source {
            if source.file_name().is_none() {
                return Err(sqlarfs::Error::msg(
                    sqlarfs::ErrorKind::InvalidArgs,
                    "The source path must have a filename.",
                )
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

impl Cli {
    pub fn dispatch(&self) -> eyre::Result<()> {
        match &self.command {
            Commands::Create(create) => create.run(),
            Commands::Extract(extract) => extract.run(),
            Commands::Archive(_) => todo!(),
        }
    }
}
