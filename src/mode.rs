use std::fs;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use super::metadata::FileMode;
#[cfg(not(unix))]
use super::metadata::{mode_from_umask, FileType};

pub trait ReadMode {
    fn read_mode(&self, path: &Path, metadata: &fs::Metadata) -> crate::Result<FileMode>;
}

pub trait WriteMode {
    fn write_mode(&self, path: &Path, mode: FileMode) -> crate::Result<()>;
}

#[derive(Debug)]
#[cfg(unix)]
pub struct UnixModeAdapter;

impl ReadMode for UnixModeAdapter {
    fn read_mode(&self, _path: &Path, metadata: &fs::Metadata) -> crate::Result<FileMode> {
        Ok(FileMode::from_mode(metadata.mode()))
    }
}

impl WriteMode for UnixModeAdapter {
    fn write_mode(&self, _path: &Path, _mode: FileMode) -> crate::Result<()> {
        todo!()
    }
}

#[derive(Debug)]
#[cfg(not(unix))]
pub struct UmaskModeAdapter {
    umask: FileMode,
}

#[cfg(not(unix))]
impl UmaskModeAdapter {
    pub fn new(umask: FileMode) -> Self {
        Self { umask }
    }
}

#[cfg(not(unix))]
impl ReadMode for UmaskModeAdapter {
    fn read_mode(&self, _path: &Path, metadata: &fs::Metadata) -> crate::Result<FileMode> {
        let kind = if metadata.is_dir() {
            FileType::Dir
        } else {
            FileType::File
        };

        Ok(mode_from_umask(kind, self.umask))
    }
}

#[cfg(not(unix))]
impl WriteMode for UmaskModeAdapter {
    fn write_mode(&self, _path: &Path, _mode: FileMode) -> crate::Result<()> {
        // Do nothing; use the default permissions set by the OS.
        Ok(())
    }
}
