mod common;

use std::io::{self, prelude::*};
use std::path::Path;
use std::time::SystemTime;

#[cfg(feature = "deflate")]
use flate2::write::ZlibEncoder;
use sqlarfs::{Compression, ErrorKind, FileMetadata, FileMode};
use xpct::{
    be_empty, be_err, be_false, be_ge, be_lt, be_none, be_ok, be_some, be_true, be_zero, eq_diff,
    equal, expect, fields, match_fields, match_pattern, pattern,
};

use common::{compressible_bytes, connection, incompressible_bytes, random_bytes, truncate_mtime};

const WRITE_DATA_SIZE: usize = 64;

// Some of our tests require inputs that we know for sure are compressible via zlib. Let's make
// absolutely sure that the test data we are using is in fact compressible.
#[test]
#[cfg(feature = "deflate")]
fn validate_compressible_bytes_are_actually_zlib_compressible() -> io::Result<()> {
    let compressible_bytes = compressible_bytes();

    let output_buf = Vec::with_capacity(compressible_bytes.len());

    let mut encoder = ZlibEncoder::new(output_buf, flate2::Compression::fast());

    encoder.write_all(&compressible_bytes)?;

    let compressed_bytes = encoder.finish()?;

    expect!(compressed_bytes.len()).to(be_lt(compressible_bytes.len()));

    Ok(())
}

// Some of our tests require inputs that we know for sure are **not** compressible via zlib. Let's
// make absolutely sure that the test data we are using is in fact not compressible.
#[test]
#[cfg(feature = "deflate")]
fn validate_incompressible_bytes_are_actually_not_zlib_compressible() -> io::Result<()> {
    let incompressible_bytes = incompressible_bytes();

    let output_buf = Vec::with_capacity(incompressible_bytes.len());

    let mut encoder = ZlibEncoder::new(output_buf, flate2::Compression::fast());

    encoder.write_all(&incompressible_bytes)?;

    let compressed_bytes = encoder.finish()?;

    expect!(compressed_bytes.len()).to(be_ge(incompressible_bytes.len()));

    Ok(())
}

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
fn write_bytes_without_compression() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create()?;

        file.set_compression(Compression::None);

        let expected = random_bytes(WRITE_DATA_SIZE);

        expect!(file.write_bytes(&expected)).to(be_ok());

        let mut reader = file.reader()?;
        let mut actual = Vec::with_capacity(expected.len());

        reader.read_to_end(&mut actual)?;

        expect!(&actual).to(eq_diff(&expected));

        drop(reader);

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.size)
            .try_into::<usize>()
            .to(equal(expected.len()));

        Ok(())
    })
}

#[test]
#[cfg(feature = "deflate")]
fn write_incompressible_bytes_with_compression() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create()?;

        file.set_compression(Compression::FAST);

        let expected = incompressible_bytes();

        expect!(file.write_bytes(&expected)).to(be_ok());

        let mut reader = file.reader()?;
        let mut actual = Vec::with_capacity(expected.len());

        reader.read_to_end(&mut actual)?;

        expect!(&actual).to(eq_diff(&expected));

        drop(reader);

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.size)
            .try_into::<usize>()
            .to(equal(expected.len()));

        Ok(())
    })
}

#[test]
#[cfg(feature = "deflate")]
fn write_compressible_bytes_with_compression() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create()?;

        file.set_compression(Compression::FAST);

        let expected = compressible_bytes();

        expect!(file.write_bytes(&expected)).to(be_ok());

        let mut reader = file.reader()?;
        let mut actual = Vec::with_capacity(expected.len());

        reader.read_to_end(&mut actual)?;

        expect!(&actual).to(eq_diff(&expected));

        drop(reader);

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.size)
            .try_into::<usize>()
            .to(equal(expected.len()));

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
fn write_string() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create()?;

        let expected = "hello world";

        expect!(file.write_str(expected)).to(be_ok());

        let mut reader = file.reader()?;
        let mut actual = String::with_capacity(expected.len());

        reader.read_to_string(&mut actual)?;

        expect!(actual.as_str()).to(eq_diff(expected));

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
fn write_from_reader_without_compression() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create()?;

        file.set_compression(Compression::None);

        let expected = random_bytes(WRITE_DATA_SIZE);

        file.write_from(&mut expected.as_slice())?;

        let mut reader = file.reader()?;
        let mut actual = Vec::with_capacity(expected.len());

        reader.read_to_end(&mut actual)?;

        expect!(&actual).to(eq_diff(&expected));

        drop(reader);

        expect!(file.is_compressed()).to(be_ok()).to(be_false());

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.size)
            .try_into::<usize>()
            .to(equal(expected.len()));

        Ok(())
    })
}

#[test]
#[cfg(feature = "deflate")]
fn write_incompressible_data_from_reader_with_compression() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create()?;

        file.set_compression(Compression::FAST);

        let expected = incompressible_bytes();

        file.write_from(&mut expected.as_slice())?;

        let mut reader = file.reader()?;
        let mut actual = Vec::with_capacity(expected.len());

        reader.read_to_end(&mut actual)?;

        expect!(&actual).to(eq_diff(&expected));

        drop(reader);

        expect!(file.is_compressed()).to(be_ok()).to(be_false());

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.size)
            .try_into::<usize>()
            .to(equal(expected.len()));

        Ok(())
    })
}

#[test]
#[cfg(feature = "deflate")]
fn write_compressible_data_from_reader_with_compression() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create()?;

        file.set_compression(Compression::FAST);

        let expected = compressible_bytes();

        file.write_from(&mut expected.as_slice())?;

        let mut reader = file.reader()?;
        let mut actual = Vec::with_capacity(expected.len());

        reader.read_to_end(&mut actual)?;

        expect!(&actual).to(eq_diff(&expected));

        drop(reader);

        expect!(file.is_compressed()).to(be_ok()).to(be_true());

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.size)
            .try_into::<usize>()
            .to(equal(expected.len()));

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
fn write_from_file_without_compression() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::tempfile()?;

    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create()?;

        file.set_compression(Compression::None);

        let expected = random_bytes(WRITE_DATA_SIZE);

        temp_file.write_all(&expected)?;

        file.write_file(&mut temp_file)?;

        let mut reader = file.reader()?;
        let mut actual = Vec::with_capacity(expected.len());

        reader.read_to_end(&mut actual)?;

        expect!(&actual).to(eq_diff(&expected));

        drop(reader);

        expect!(file.is_compressed()).to(be_ok()).to(be_false());

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.size)
            .try_into::<usize>()
            .to(equal(expected.len()));

        Ok(())
    })
}

#[test]
#[cfg(feature = "deflate")]
fn write_incompressible_data_from_file_with_compression() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::tempfile()?;

    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create()?;

        file.set_compression(Compression::FAST);

        let expected = incompressible_bytes();

        temp_file.write_all(&expected)?;
        temp_file.seek(io::SeekFrom::Start(0))?;

        file.write_file(&mut temp_file)?;

        let mut reader = file.reader()?;
        let mut actual = Vec::with_capacity(expected.len());

        reader.read_to_end(&mut actual)?;

        expect!(&actual).to(eq_diff(&expected));

        drop(reader);

        expect!(file.is_compressed()).to(be_ok()).to(be_false());

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.size)
            .try_into::<usize>()
            .to(equal(expected.len()));

        Ok(())
    })
}

#[test]
#[cfg(feature = "deflate")]
fn write_compressible_data_from_file_with_compression() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::tempfile()?;

    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create()?;

        file.set_compression(Compression::FAST);

        let expected = compressible_bytes();

        temp_file.write_all(&expected)?;
        temp_file.seek(io::SeekFrom::Start(0))?;

        file.write_file(&mut temp_file)?;

        let mut reader = file.reader()?;
        let mut actual = Vec::with_capacity(expected.len());

        reader.read_to_end(&mut actual)?;

        expect!(&actual).to(eq_diff(&expected));

        drop(reader);

        expect!(file.is_compressed()).to(be_ok()).to(be_true());

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.size)
            .try_into::<usize>()
            .to(equal(expected.len()));

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
