mod common;

use std::io::prelude::*;
use std::path::Path;
use std::time::SystemTime;

use sqlarfs::{Compression, Connection, ErrorKind, FileMetadata, FileMode, FileType};
use tempfile::NamedTempFile;
use xpct::{
    be_empty, be_err, be_false, be_none, be_ok, be_some, be_true, be_zero, equal, expect, fields,
    match_fields, match_pattern, pattern,
};

use common::{connection, random_bytes, truncate_mtime, WRITE_DATA_SIZE};

//
// `File::path`
//

#[test]
fn get_file_path() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let file = archive.open("path/to/file")?;

        expect!(file.path()).to(equal(Path::new("path/to/file")));

        Ok(())
    })
}

//
// `File::create_file`
//

#[test]
fn create_file_when_it_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("nonexistent-file")?;

        expect!(file.create_file()).to(be_ok());

        Ok(())
    })
}

#[test]
fn create_file_errors_when_it_already_exists() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("existing-file")?;

        file.create_file()?;

        expect!(file.create_file())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::AlreadyExists));

        Ok(())
    })
}

#[test]
fn create_file_errors_when_it_has_a_non_directory_parent() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("parent")?.create_file()?;

        let mut child = archive.open("parent/child")?;

        expect!(child.create_file())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotADirectory));

        Ok(())
    })
}

#[test]
fn create_file_errors_when_it_has_no_parent() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("parent/child")?;

        expect!(file.create_file())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn create_file_with_trailing_slash_when_it_already_exists_without_one() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        let mut file = archive.open("file/")?;

        expect!(file.create_file())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::AlreadyExists));

        Ok(())
    })
}

#[test]
fn create_file_respects_file_umask() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        file.set_umask(FileMode::GROUP_RWX | FileMode::OTHER_RWX);

        file.create_file()?;

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.mode)
            .to(be_some())
            .to(equal(FileMode::OWNER_R | FileMode::OWNER_W));

        Ok(())
    })
}

#[test]
fn create_file_errors_when_parent_has_directory_mode_but_nonzero_size() -> sqlarfs::Result<()> {
    let db_file = NamedTempFile::new()?;

    // Initialize the database with the `sqlar` table.
    Connection::open(db_file.path())?;

    let dir_mode = 0o040775;

    let conn = rusqlite::Connection::open(db_file.path())?;
    conn.execute(
        "INSERT INTO sqlar (name, mode, sz, data) VALUES (?1, ?2, 1, zeroblob(1))",
        ("dir", dir_mode),
    )?;

    let mut conn = Connection::open(db_file.path())?;

    conn.exec(|archive| {
        let mut file = archive.open("dir/file")?;

        expect!(file.create_file())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotADirectory));

        Ok(())
    })
}

//
// `File::create_dir_all`
//

#[test]
fn create_dir_all_creates_missing_parent_directories() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir_c = archive.open("a/b/c")?;

        expect!(dir_c.create_dir_all()).to(be_ok());

        expect!(dir_c.exists()).to(be_ok()).to(be_true());

        let dir_b = archive.open("a/b")?;
        expect!(dir_b.exists()).to(be_ok()).to(be_true());

        let dir_a = archive.open("a")?;
        expect!(dir_a.exists()).to(be_ok()).to(be_true());

        Ok(())
    })
}

#[test]
fn create_dir_all_does_not_error_if_directory_already_exists() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;

        dir.create_dir()?;

        expect!(dir.create_dir_all()).to(be_ok());

        Ok(())
    })
}

#[test]
fn create_dir_all_errors_if_regular_file_already_exists() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("file")?;

        dir.create_file()?;

        expect!(dir.create_dir_all())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::AlreadyExists));

        Ok(())
    })
}

//
// `File::metadata`
//

#[test]
fn file_metadata_when_creating_file_with_metadata() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        let mode = FileMode::OWNER_R | FileMode::OWNER_W | FileMode::GROUP_R | FileMode::OTHER_R;
        let precise_mtime = SystemTime::now();
        let truncated_mtime = truncate_mtime(precise_mtime);

        file.create_with(FileType::File, mode, Some(precise_mtime))?;

        expect!(file.metadata())
            .to(be_ok())
            .to(match_fields(fields!(FileMetadata {
                mtime: equal(Some(truncated_mtime)),
                mode: equal(Some(mode)),
                size: be_zero(),
                kind: equal(Some(FileType::File)),
            })));

        Ok(())
    })
}

#[test]
fn file_metadata_correctly_reports_directories() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.metadata())
            .to(be_ok())
            .to(match_fields(fields!(FileMetadata {
                kind: equal(Some(FileType::Dir)),
            })));

        Ok(())
    })
}

#[test]
fn file_size_is_zero_when_file_is_empty() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        file.create_file()?;

        let metadata = file.metadata()?;

        expect!(metadata.size).to(be_zero());

        Ok(())
    })
}

#[test]
fn file_metadata_reports_no_file_type_when_mode_indicates_it_is_a_special_file(
) -> sqlarfs::Result<()> {
    let db_file = NamedTempFile::new()?;

    // Initialize the database with the `sqlar` table.
    Connection::open(db_file.path())?;

    // Socket file.
    let special_file_mode = 0o140664;

    let conn = rusqlite::Connection::open(db_file.path())?;
    conn.execute(
        "INSERT INTO sqlar (name, mode, sz, data) VALUES (?1, ?2, 0, zeroblob(0))",
        ("file", special_file_mode),
    )?;

    let mut conn = Connection::open(db_file.path())?;

    conn.exec(|archive| {
        let file = archive.open("file")?;

        let metadata = file.metadata()?;

        expect!(metadata.kind).to(be_none());

        Ok(())
    })
}

#[test]
fn file_metadata_reports_no_file_type_when_there_is_no_mode() -> sqlarfs::Result<()> {
    let db_file = NamedTempFile::new()?;

    // Initialize the database with the `sqlar` table.
    Connection::open(db_file.path())?;

    let conn = rusqlite::Connection::open(db_file.path())?;
    conn.execute(
        "INSERT INTO sqlar (name, sz, data) VALUES (?1, 0, zeroblob(0))",
        ("file",),
    )?;

    let mut conn = Connection::open(db_file.path())?;

    conn.exec(|archive| {
        let file = archive.open("file")?;

        let metadata = file.metadata()?;

        expect!(metadata.kind).to(be_none());

        Ok(())
    })
}

//
// `File::exists
//

#[test]
fn file_correctly_reports_that_it_exists() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("existing-file")?;

        file.create_file()?;

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

//
// `File::delete`
//

#[test]
fn deleted_file_no_longer_exists() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("existing-file")?;

        file.create_file()?;

        expect!(file.delete()).to(be_ok());
        expect!(file.exists()).to(be_ok()).to(be_false());

        Ok(())
    })
}

#[test]
fn deleting_file_errors_when_it_does_not_exist() -> sqlarfs::Result<()> {
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
fn deleting_file_recursively_deletes_descendants() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        let mut file_a = archive.open("dir/file_a")?;
        file_a.create_file()?;

        let mut subdir = archive.open("dir/subdir")?;
        subdir.create_dir()?;

        let mut file_b = archive.open("dir/subdir/file_b")?;
        file_b.create_file()?;

        let mut dir = archive.open("dir")?;
        expect!(dir.delete()).to(be_ok());
        expect!(dir.exists()).to(be_ok()).to(be_false());

        let file_a = archive.open("dir/file_a")?;
        expect!(file_a.exists()).to(be_ok()).to(be_false());

        let subdir = archive.open("dir/subdir")?;
        expect!(subdir.exists()).to(be_ok()).to(be_false());

        let file_b = archive.open("dir/subdir/file_b")?;
        expect!(file_b.exists()).to(be_ok()).to(be_false());

        Ok(())
    })
}

//
// `File::compression` / `File::set_compression`
//

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

//
// `File::umask` / `File::set_umask`
//

#[test]
fn set_file_umask() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.umask()).to(equal(FileMode::OTHER_W));

        let expected_umask = FileMode::OWNER_RWX | FileMode::OTHER_RWX;

        file.set_umask(expected_umask);

        expect!(file.umask()).to(equal(expected_umask));

        Ok(())
    })
}

//
// `File::set_mode`
//

#[test]
fn set_file_mode() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        file.create_file()?;

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.mode)
            .to(be_some())
            .to(equal(FileMode::from_bits_truncate(0o664)));

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
fn set_file_mode_errors_when_file_does_not_exist() -> sqlarfs::Result<()> {
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
fn set_file_mode_preserves_file_type() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create_file()?;

        let mode = FileMode::OWNER_R | FileMode::OWNER_W | FileMode::GROUP_R | FileMode::OTHER_R;
        file.set_mode(Some(mode))?;

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.kind)
            .to(be_some())
            .to(equal(FileType::File));

        Ok(())
    })
}

//
// `File::set_mtime`
//

#[test]
fn set_file_mtime() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        file.create_file()?;

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
fn set_file_mtime_errors_when_file_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.set_mtime(None))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

//
// `File::is_empty`
//

#[test]
fn file_correctly_reports_being_empty() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        file.create_file()?;

        expect!(file.is_empty()).to(be_ok()).to(be_true());

        Ok(())
    })
}

#[test]
fn file_correctly_reports_being_not_empty() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        file.create_file()?;
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
fn is_file_empty_errors_when_it_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.is_empty())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}

//
// `File::is_compressed`
//

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
fn is_file_compressed_errors_when_it_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.is_compressed())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}

//
// `File::reader`
//

#[test]
fn open_reader_errors_when_file_does_not_exist() -> sqlarfs::Result<()> {
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
fn open_reader_errors_when_file_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.reader())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}

#[test]
fn open_reader_errors_when_file_has_regular_file_mode_but_also_descendants() -> sqlarfs::Result<()>
{
    let db_file = NamedTempFile::new()?;

    // Initialize the database with the `sqlar` table.
    Connection::open(db_file.path())?;

    let regular_file_mode = 0o100664;

    let conn = rusqlite::Connection::open(db_file.path())?;
    conn.execute(
        "INSERT INTO sqlar (name, mode, sz, data) VALUES (?1, ?2, 0, zeroblob(0))",
        ("dir", regular_file_mode),
    )?;
    conn.execute(
        "INSERT INTO sqlar (name, mode, sz, data) VALUES (?1, ?2, 0, zeroblob(0))",
        ("dir/file", regular_file_mode),
    )?;

    let mut conn = Connection::open(db_file.path())?;

    conn.exec(|archive| {
        let mut dir = archive.open("dir")?;

        expect!(dir.reader())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}

//
// `File::truncate`
//

#[test]
fn truncated_file_returns_no_bytes() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create_file()?;

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
fn truncate_file_errors_when_it_does_not_exist() -> sqlarfs::Result<()> {
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
fn truncate_file_errors_when_it_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("dir")?;
        file.create_dir()?;

        expect!(file.truncate())
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}

//
// `File::write_bytes`
//

#[test]
fn write_bytes_errors_when_file_does_not_exist() -> sqlarfs::Result<()> {
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
fn write_bytes_errors_when_file_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.write_bytes(b"file content"))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}

#[test]
fn write_bytes_errors_when_file_has_regular_file_mode_but_also_descendants() -> sqlarfs::Result<()>
{
    let db_file = NamedTempFile::new()?;

    // Initialize the database with the `sqlar` table.
    Connection::open(db_file.path())?;

    let regular_file_mode = 0o100664;

    let conn = rusqlite::Connection::open(db_file.path())?;
    conn.execute(
        "INSERT INTO sqlar (name, mode, sz, data) VALUES (?1, ?2, 0, zeroblob(0))",
        ("dir", regular_file_mode),
    )?;
    conn.execute(
        "INSERT INTO sqlar (name, mode, sz, data) VALUES (?1, ?2, 0, zeroblob(0))",
        ("dir/file", regular_file_mode),
    )?;

    let mut conn = Connection::open(db_file.path())?;

    conn.exec(|archive| {
        let mut dir = archive.open("dir")?;

        expect!(dir.write_bytes("file contents".as_bytes()))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}

//
// `File::write_str`
//
//

#[test]
fn write_string_errors_when_file_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.write_str("file content"))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn write_string_errors_when_file_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.write_str("file content"))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}

//
// `File::write_from`
//

#[test]
fn write_from_reader_errors_when_file_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.write_from(&mut "file content".as_bytes()))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn write_from_reader_errors_when_file_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.write_from(&mut "file content".as_bytes()))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}

//
// `File::write_file`
//

#[test]
fn write_from_file_errors_when_file_does_not_exist() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::tempfile()?;

    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.write_file(&mut temp_file))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn write_from_file_errors_when_file_is_a_directory() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::tempfile()?;

    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.write_file(&mut temp_file))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::IsADirectory));

        Ok(())
    })
}
