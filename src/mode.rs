use std::fs;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use crate::FileMode;

pub trait ReadMode {
    fn read_mode(&self, path: &Path, metadata: &fs::Metadata) -> crate::Result<FileMode>;
}

pub trait WriteMode {
    fn write_mode(&self, path: &Path, mode: FileMode) -> crate::Result<()>;
}

#[derive(Debug, Clone, Copy)]
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
