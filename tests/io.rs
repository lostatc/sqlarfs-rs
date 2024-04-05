mod common;

use std::io::{self, prelude::*};

#[cfg(feature = "deflate")]
use flate2::write::ZlibEncoder;
use sqlarfs::Compression;
use xpct::{be_false, be_ge, be_lt, be_ok, be_true, eq_diff, equal, expect};

use common::{compressible_bytes, connection, incompressible_bytes, random_bytes, WRITE_DATA_SIZE};

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
fn write_bytes_without_compression() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create_file()?;

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
        file.create_file()?;

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
        file.create_file()?;

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
fn write_from_reader_without_compression() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create_file()?;

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
        file.create_file()?;

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
        file.create_file()?;

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
fn write_from_file_without_compression() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::tempfile()?;

    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create_file()?;

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
        file.create_file()?;

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
        file.create_file()?;

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
fn write_string() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create_file()?;

        let expected = "hello world";

        expect!(file.write_str(expected)).to(be_ok());

        let mut reader = file.reader()?;
        let mut actual = String::with_capacity(expected.len());

        reader.read_to_string(&mut actual)?;

        expect!(actual.as_str()).to(eq_diff(expected));

        Ok(())
    })
}
