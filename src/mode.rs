use std::fs;
use std::path::Path;

use super::metadata::FileMode;

pub trait ReadMode {
    fn read_mode(&self, path: &Path, metadata: &fs::Metadata) -> crate::Result<FileMode>;
}

pub trait WriteMode {
    fn write_mode(&self, path: &Path, mode: FileMode) -> crate::Result<()>;
}

#[derive(Debug)]
#[cfg(unix)]
pub struct UnixModeAdapter;

#[cfg(unix)]
impl ReadMode for UnixModeAdapter {
    fn read_mode(&self, _path: &Path, metadata: &fs::Metadata) -> crate::Result<FileMode> {
        use std::os::unix::fs::MetadataExt;

        Ok(FileMode::from_mode(metadata.mode()))
    }
}

#[cfg(unix)]
impl WriteMode for UnixModeAdapter {
    fn write_mode(&self, path: &Path, mode: FileMode) -> crate::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(mode.bits());
        fs::set_permissions(path, perms)?;
        Ok(())
    }
}

#[derive(Debug)]
#[cfg(any(windows, test))]
pub struct WindowsModeAdapter;

#[cfg(any(windows, test))]
impl ReadMode for WindowsModeAdapter {
    fn read_mode(&self, _path: &Path, metadata: &fs::Metadata) -> crate::Result<FileMode> {
        use super::metadata::{mode_from_umask, FileType};

        let kind = if metadata.is_dir() {
            FileType::Dir
        } else {
            FileType::File
        };

        // The reference sqlar implementation always uses `666`/`777` permissions when archiving
        // files on Windows.
        Ok(mode_from_umask(kind, FileMode::empty()))
    }
}

#[cfg(any(windows, test))]
impl WriteMode for WindowsModeAdapter {
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
    #[cfg(unix)]
    fn unix_mode_adapter_reads_mode() -> crate::Result<()> {
        use std::os::unix::fs::PermissionsExt;

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
    #[cfg(unix)]
    fn unix_mode_adapter_writes_mode() -> crate::Result<()> {
        use std::os::unix::fs::PermissionsExt;

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

    //
    // Even though this adapter is only used on Windows, we test it on Unix because Unix-like
    // platforms allow us to set the mode of the file to test against.
    //

    #[test]
    #[cfg(unix)]
    fn windows_mode_adapter_ignores_actual_file_mode() -> crate::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let expected_mode = FileMode::OWNER_R
            | FileMode::OWNER_W
            | FileMode::GROUP_R
            | FileMode::GROUP_W
            | FileMode::OTHER_R
            | FileMode::OTHER_W;
        let adapter = WindowsModeAdapter;

        let temp_file = tempfile::NamedTempFile::new()?;
        fs::set_permissions(temp_file.path(), fs::Permissions::from_mode(0o444))?;

        expect!(adapter.read_mode(temp_file.path(), &fs::metadata(temp_file.path())?))
            .to(be_ok())
            .to(equal(expected_mode));

        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn windows_mode_adapter_does_not_set_file_mode() -> crate::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let mode_to_set = FileMode::OWNER_R | FileMode::GROUP_R | FileMode::OTHER_R;
        let adapter = WindowsModeAdapter;

        let temp_file = tempfile::NamedTempFile::new()?;

        expect!(adapter.write_mode(temp_file.path(), mode_to_set)).to(be_ok());

        let actual_mode = fs::metadata(temp_file.path())?.permissions().mode();
        let just_permissions_bits = actual_mode & 0o777;

        expect!(just_permissions_bits).to_not(equal(mode_to_set.bits()));

        Ok(())
    }
}
