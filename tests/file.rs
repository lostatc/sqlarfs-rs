use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlarfs::{Compression, Connection, ErrorKind, FileMetadata, FileMode};
use xpct::{
    all, approx_eq_time, be_err, be_false, be_none, be_ok, be_some, be_true, be_zero, equal,
    expect, fields, match_fields, match_pattern, pattern,
};

fn connection() -> sqlarfs::Result<Connection> {
    let mut conn = Connection::open_in_memory()?;

    conn.exec(|archive| archive.init())?;

    Ok(conn)
}

#[test]
fn get_file_path() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let file = archive.open(Path::new("path/to/file"));

        expect!(file.path()).to(equal(Path::new("path/to/file")));

        Ok(())
    })
}

#[test]
fn create_file_when_it_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open(Path::new("nonexistent-file"));

        expect!(file.create(None)).to(be_ok());

        Ok(())
    })
}

#[test]
fn create_file_when_it_already_exists() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open(Path::new("existing-file"));

        file.create(None)?;

        expect!(file.create(None))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::AlreadyExists));

        Ok(())
    })
}

#[test]
fn file_metadata_when_creating_file() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open(Path::new("file"));

        let mode = FileMode::OWNER_R | FileMode::OWNER_W | FileMode::GROUP_R | FileMode::OTHER_R;

        file.create(Some(mode))?;

        expect!(file.metadata())
            .to(be_ok())
            .to(match_fields(fields!(FileMetadata {
                mtime: all(|ctx| ctx
                    .to(be_some())?
                    .to(approx_eq_time(SystemTime::now(), Duration::from_secs(1)))),
                mode: equal(Some(mode)),
                size: be_zero(),
            })));

        Ok(())
    })
}

#[test]
fn file_correctly_reports_that_it_exists() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open(Path::new("existing-file"));

        file.create(None)?;

        expect!(file.exists()).to(be_ok()).to(be_true());

        Ok(())
    })
}

#[test]
fn file_correctly_reports_that_it_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let file = archive.open(Path::new("existing-file"));

        expect!(file.exists()).to(be_ok()).to(be_false());

        Ok(())
    })
}

#[test]
fn deleting_file_when_it_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open(Path::new("existing-file"));

        expect!(file.delete())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn deleting_file_when_it_does_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open(Path::new("existing-file"));

        file.create(None)?;

        expect!(file.delete()).to(be_ok());

        Ok(())
    })
}

#[test]
#[cfg(feature = "deflate")]
fn set_compression_method() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open(Path::new("existing-file"));

        // We specify that the default is DEFLATE compression when the feature flag is enabled, but
        // not what the default compression level is.
        expect!(file.compression()).to(match_pattern(pattern!(Compression::Deflate { .. })));

        file.set_compression(Compression::None);

        expect!(file.compression()).to(equal(Compression::None));

        Ok(())
    })
}

#[test]
fn set_file_mode() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open(Path::new("existing-file"));

        file.create(None)?;

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.mode)
            .to(be_none());

        let mode = FileMode::OWNER_R | FileMode::OWNER_W | FileMode::GROUP_R | FileMode::OTHER_R;

        file.set_mode(Some(mode))?;

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.mode)
            .to(be_some())
            .to(equal(mode));

        Ok(())
    })
}

#[test]
fn set_file_mtime() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open(Path::new("existing-file"));

        file.create(None)?;

        // Some time in the past so it's different from the default mtime for new files (now). We
        // need to round it to the nearest second, because that's what SQLite archives do.
        let unix_time_secs = (SystemTime::now() - Duration::from_secs(60))
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let rounded_mtime = UNIX_EPOCH + Duration::from_secs(unix_time_secs);

        file.set_mtime(Some(rounded_mtime))?;

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.mtime)
            .to(be_some())
            .to(equal(rounded_mtime));

        Ok(())
    })
}

#[test]
fn file_size_is_zero_when_file_is_empty() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open(Path::new("existing-file"));

        file.create(None)?;

        let metadata = file.metadata()?;

        expect!(metadata.size).to(be_zero());

        Ok(())
    })
}

#[test]
fn file_correctly_reports_being_empty() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open(Path::new("existing-file"));

        file.create(None)?;

        expect!(file.is_empty()).to(be_ok()).to(be_true());

        Ok(())
    })
}

#[test]
fn file_correctly_reports_being_not_empty() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open(Path::new("existing-file"));

        file.create(None)?;
        file.write_str("file contents")?;

        expect!(file.is_empty()).to(be_ok()).to(be_false());

        Ok(())
    })
}
