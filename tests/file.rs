mod common;

use std::io::prelude::*;
use std::path::Path;
use std::time::SystemTime;

use sqlarfs::{Compression, ErrorKind, FileMetadata, FileMode};
use xpct::{
    be_empty, be_err, be_false, be_none, be_ok, be_some, be_true, be_zero, equal, expect, fields,
    match_fields, match_pattern, pattern,
};

use common::{connection, random_bytes, truncate_mtime, WRITE_DATA_SIZE};

#[test]
fn get_file_path() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let file = archive.open("path/to/file")?;

        expect!(file.path()).to(equal(Path::new("path/to/file")));

        Ok(())
    })
}

#[test]
fn create_file_when_it_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("nonexistent-file")?;

        expect!(file.create()).to(be_ok());

        Ok(())
    })
}

#[test]
fn create_file_when_it_already_exists() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("existing-file")?;

        file.create()?;

        expect!(file.create())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::AlreadyExists));

        Ok(())
    })
}

#[test]
fn create_file_when_it_has_a_non_directory_parent() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut parent = archive.open("parent")?;
        parent.create()?;
        parent.write_str("this file is not a directory because it has contents")?;

        let mut child = archive.open("parent/child")?;

        expect!(child.create())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotADirectory));

        Ok(())
    })
}

#[test]
fn create_file_with_metadata_when_it_has_a_non_directory_parent() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut parent = archive.open("parent")?;
        parent.create()?;
        parent.write_str("this file is not a directory because it has contents")?;

        let mut child = archive.open("parent/child")?;

        expect!(child.create_with(FileMode::empty(), SystemTime::now()))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotADirectory));

        Ok(())
    })
}

#[test]
fn file_metadata_when_creating_file_with_metadata() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        let mode = FileMode::OWNER_R | FileMode::OWNER_W | FileMode::GROUP_R | FileMode::OTHER_R;
        let precise_mtime = SystemTime::now();
        let truncated_mtime = truncate_mtime(precise_mtime);

        file.create_with(mode, precise_mtime)?;

        expect!(file.metadata())
            .to(be_ok())
            .to(match_fields(fields!(FileMetadata {
                mtime: equal(Some(truncated_mtime)),
                mode: equal(Some(mode)),
                size: be_zero(),
            })));

        Ok(())
    })
}

#[test]
fn file_correctly_reports_that_it_exists() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("existing-file")?;

        file.create()?;

        expect!(file.exists()).to(be_ok()).to(be_true());

        Ok(())
    })
}

#[test]
fn file_correctly_reports_that_it_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let file = archive.open("nonexistent-file")?;

        expect!(file.exists()).to(be_ok()).to(be_false());

        Ok(())
    })
}

#[test]
fn deleting_file_when_it_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("nonexistent-file")?;

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
        let mut file = archive.open("existing-file")?;

        file.create()?;

        expect!(file.delete()).to(be_ok());

        Ok(())
    })
}

#[test]
#[cfg(feature = "deflate")]
fn set_compression_method() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

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
        let mut file = archive.open("file")?;

        file.create()?;

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
fn set_file_mode_when_file_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.set_mode(None))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn set_file_mtime() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        file.create()?;

        let precise_mtime = SystemTime::now();
        let truncated_mtime = truncate_mtime(precise_mtime);

        file.set_mtime(Some(precise_mtime))?;

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.mtime)
            .to(be_some())
            .to(equal(truncated_mtime));

        Ok(())
    })
}

#[test]
fn set_file_mtime_when_file_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.set_mtime(None))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn file_size_is_zero_when_file_is_empty() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        file.create()?;

        let metadata = file.metadata()?;

        expect!(metadata.size).to(be_zero());

        Ok(())
    })
}

#[test]
fn file_correctly_reports_being_empty() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        file.create()?;

        expect!(file.is_empty()).to(be_ok()).to(be_true());

        Ok(())
    })
}

#[test]
fn file_correctly_reports_being_not_empty() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        file.create()?;
        file.write_str("file contents")?;

        expect!(file.is_empty()).to(be_ok()).to(be_false());

        Ok(())
    })
}

#[test]
fn is_file_empty_errors_when_it_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let file = archive.open("file")?;

        expect!(file.is_empty())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn is_file_compressed_errors_when_it_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let file = archive.open("file")?;

        expect!(file.is_compressed())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn write_bytes_when_file_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        let expected = random_bytes(WRITE_DATA_SIZE);

        expect!(file.write_bytes(&expected))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn open_reader_when_file_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.reader())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn open_reader_when_file_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create()?;

        let mut file = archive.open("dir/file")?;
        file.create()?;

        let mut dir = archive.open("dir")?;

        expect!(dir.reader())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}

#[test]
fn truncated_file_returns_no_bytes() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create()?;

        let expected = random_bytes(WRITE_DATA_SIZE);

        file.write_bytes(&expected)?;

        expect!(file.truncate()).to(be_ok());

        let mut reader = file.reader()?;
        let mut actual = Vec::new();

        expect!(reader.read_to_end(&mut actual))
            .to(be_ok())
            .to(be_zero());

        expect!(&actual).to(be_empty());

        drop(reader);

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.size)
            .to(be_zero());

        Ok(())
    })
}

#[test]
fn truncate_file_when_it_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.truncate())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn write_bytes_when_file_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create()?;

        let mut file = archive.open("dir/file")?;
        file.create()?;

        let mut dir = archive.open("dir")?;

        expect!(dir.write_bytes(b"file content"))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}

#[test]
fn write_string_when_file_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create()?;

        let mut file = archive.open("dir/file")?;
        file.create()?;

        let mut dir = archive.open("dir")?;

        expect!(dir.write_str("file content"))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}

#[test]
fn write_from_reader_when_file_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create()?;

        let mut file = archive.open("dir/file")?;
        file.create()?;

        let mut dir = archive.open("dir")?;

        expect!(dir.write_from(&mut "file content".as_bytes()))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}

#[test]
fn write_from_file_when_file_is_a_directory() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::tempfile()?;

    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create()?;

        let mut file = archive.open("dir/file")?;
        file.create()?;

        let mut dir = archive.open("dir")?;

        expect!(dir.write_file(&mut temp_file))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}
