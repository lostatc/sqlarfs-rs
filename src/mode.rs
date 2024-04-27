use std::fs;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use super::metadata::FileMode;
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
    fn write_mode(&self, path: &Path, mode: FileMode) -> crate::Result<()> {
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(mode.bits());
        fs::set_permissions(path, perms)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct UmaskModeAdapter {
    umask: FileMode,
}

impl UmaskModeAdapter {
    #[cfg(any(test, not(unix)))]
    pub fn new(umask: FileMode) -> Self {
        Self { umask }
    }
}

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

impl WriteMode for UmaskModeAdapter {
    fn write_mode(&self, _path: &Path, _mode: FileMode) -> crate::Result<()> {
        // Do nothing; use the default permissions set by the OS.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use xpct::{be_ok, equal, expect};

    #[test]
    fn unix_mode_adapter_reads_mode() -> crate::Result<()> {
        let expected_mode = FileMode::OWNER_R | FileMode::GROUP_R | FileMode::OTHER_R;
        let adapter = UnixModeAdapter;

        let temp_file = tempfile::NamedTempFile::new()?;
        fs::set_permissions(
            temp_file.path(),
            fs::Permissions::from_mode(expected_mode.bits()),
        )?;

        expect!(adapter.read_mode(temp_file.path(), &fs::metadata(temp_file.path())?))
            .to(be_ok())
            .to(equal(expected_mode));

        Ok(())
    }

    #[test]
    fn unix_mode_adapter_writes_mode() -> crate::Result<()> {
        let expected_mode = FileMode::OWNER_R | FileMode::GROUP_R | FileMode::OTHER_R;
        let adapter = UnixModeAdapter;

        let temp_file = tempfile::NamedTempFile::new()?;

        let actual_mode = fs::metadata(temp_file.path())?.permissions().mode();
        let just_permissions_bits = actual_mode & 0o777;

        expect!(just_permissions_bits).to_not(equal(expected_mode.bits()));

        expect!(adapter.write_mode(temp_file.path(), expected_mode)).to(be_ok());

        let actual_mode = fs::metadata(temp_file.path())?.permissions().mode();
        let just_permissions_bits = actual_mode & 0o777;

        expect!(just_permissions_bits).to(equal(expected_mode.bits()));

        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn umask_mode_adapter_ignores_actual_file_mode() -> crate::Result<()> {
        let umask = FileMode::OTHER_W;
        let expected_mode = FileMode::OWNER_R
            | FileMode::OWNER_W
            | FileMode::GROUP_R
            | FileMode::GROUP_W
            | FileMode::OTHER_R;
        let adapter = UmaskModeAdapter::new(umask);

        let temp_file = tempfile::NamedTempFile::new()?;
        fs::set_permissions(temp_file.path(), fs::Permissions::from_mode(0o444))?;

        expect!(adapter.read_mode(temp_file.path(), &fs::metadata(temp_file.path())?))
            .to(be_ok())
            .to(equal(expected_mode));

        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn umask_mode_adapter_does_not_set_file_mode() -> crate::Result<()> {
        let umask = FileMode::OTHER_W;
        let mode_to_set = FileMode::OWNER_R | FileMode::GROUP_R | FileMode::OTHER_R;
        let adapter = UmaskModeAdapter::new(umask);

        let temp_file = tempfile::NamedTempFile::new()?;

        expect!(adapter.write_mode(temp_file.path(), mode_to_set)).to(be_ok());

        let actual_mode = fs::metadata(temp_file.path())?.permissions().mode();
        let just_permissions_bits = actual_mode & 0o777;

        expect!(just_permissions_bits).to_not(equal(mode_to_set.bits()));

        Ok(())
    }
}
