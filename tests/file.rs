mod common;

use std::ffi::OsStr;
use std::io::prelude::*;
use std::path::Path;
use std::time::{Duration, SystemTime};

use sqlarfs::{Compression, Connection, ErrorKind, FileMode, FileType};
use tempfile::NamedTempFile;
use xpct::{
    be_empty, be_false, be_ok, be_some, be_true, be_zero, equal, expect, fields, match_fields,
    match_pattern, pattern, why,
};

use common::{
    connection, have_error_kind, have_file_metadata, have_symlink_metadata, random_bytes,
    truncate_mtime, RegularFileMetadata, WRITE_DATA_SIZE,
};

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

        expect!(file.create_file()).to(have_error_kind(ErrorKind::AlreadyExists));

        Ok(())
    })
}

#[test]
fn create_file_errors_when_it_has_a_non_directory_parent() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("parent")?.create_file()?;

        let mut child = archive.open("parent/child")?;

        expect!(child.create_file()).to(have_error_kind(ErrorKind::NotADirectory));

        Ok(())
    })
}

#[test]
fn create_file_errors_when_it_has_no_parent() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("parent/child")?;

        expect!(file.create_file()).to(have_error_kind(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn create_file_with_trailing_slash_when_it_already_exists_without_one() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        let mut file = archive.open(if cfg!(windows) { r"file\" } else { "file/" })?;

        expect!(file.create_file()).to(have_error_kind(ErrorKind::AlreadyExists));

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
            .to(have_file_metadata())
            .map(|metadata| metadata.mode)
            .to(why(be_some(), "the file mode is not set"))
            .to(equal(FileMode::OWNER_R | FileMode::OWNER_W));

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

        expect!(dir.create_dir_all()).to(have_error_kind(ErrorKind::AlreadyExists));

        Ok(())
    })
}

//
// `File::create_symlink`
//

#[test]
fn create_symlink_with_empty_target_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut link = archive.open("link")?;

        expect!(link.create_symlink("")).to(have_error_kind(ErrorKind::InvalidArgs));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn create_symlink_with_non_utf8_target_errors() -> sqlarfs::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    connection()?.exec(|archive| {
        let mut link = archive.open("link")?;

        expect!(link.create_symlink(OsStr::from_bytes(b"not/valid/utf8/\x80\x81")))
            .to(have_error_kind(ErrorKind::InvalidArgs));

        Ok(())
    })
}

//
// `File::metadata`
//

#[test]
fn file_metadata_when_creating_file_with_metadata() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mode = FileMode::OWNER_R | FileMode::OWNER_W | FileMode::GROUP_R | FileMode::OTHER_R;
        let precise_mtime = SystemTime::now();
        let truncated_mtime = truncate_mtime(precise_mtime);

        let mut file = archive.open("file")?;
        file.create_file()?;
        file.set_mode(Some(mode))?;
        file.set_mtime(Some(precise_mtime))?;

        let metadata = expect!(file.metadata()).to(be_ok()).into_inner();

        expect!(metadata.clone())
            .to(have_file_metadata())
            .to(match_fields(fields!(RegularFileMetadata {
                mode: equal(Some(mode)),
                mtime: equal(Some(truncated_mtime)),
                size: be_zero(),
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
            .map(|metadata| metadata.kind())
            .to(equal(FileType::Dir));

        Ok(())
    })
}

#[test]
fn file_metadata_correctly_reports_symlinks() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut link = archive.open("link")?;
        link.create_symlink("target")?;

        expect!(link.metadata())
            .to(be_ok())
            .map(|metadata| metadata.kind())
            .to(equal(FileType::Symlink));

        Ok(())
    })
}

#[test]
fn file_metadata_contains_symlink_target() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut link = archive.open("link")?;
        link.create_symlink("target")?;

        expect!(link.metadata())
            .to(be_ok())
            .to(have_symlink_metadata())
            .map(|metadata| metadata.target)
            .to(equal(Path::new("target")));

        Ok(())
    })
}

#[test]
fn file_size_is_zero_when_file_is_empty() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        file.create_file()?;

        expect!(file.metadata())
            .to(be_ok())
            .to(have_file_metadata())
            .map(|metadata| metadata.size)
            .to(be_zero());

        Ok(())
    })
}

#[test]
fn file_has_regular_file_metadata_even_when_mode_indicates_it_is_a_special_file(
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

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.kind())
            .to(equal(FileType::File));

        Ok(())
    })
}

#[test]
fn file_has_regular_file_metadata_even_when_there_is_no_mode() -> sqlarfs::Result<()> {
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

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.kind())
            .to(equal(FileType::File));

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

        expect!(file.delete()).to(have_error_kind(ErrorKind::NotFound));

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
            .map(|metadata| metadata.mode())
            .to(be_some())
            .to(equal(FileMode::from_bits_truncate(0o664)));

        let mode = FileMode::OWNER_R | FileMode::OWNER_W | FileMode::GROUP_R | FileMode::OTHER_R;

        file.set_mode(Some(mode))?;

        expect!(file.metadata())
            .to(be_ok())
            .map(|metadata| metadata.mode())
            .to(be_some())
            .to(equal(mode));

        Ok(())
    })
}

#[test]
fn set_file_mode_errors_when_file_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.set_mode(None)).to(have_error_kind(ErrorKind::NotFound));
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
            .map(|metadata| metadata.kind())
            .to(equal(FileType::File));

        Ok(())
    })
}

#[test]
fn set_file_mode_is_a_noop_for_symlinks() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut link = archive.open("link")?;
        link.create_symlink("target")?;

        let mode = FileMode::OWNER_R | FileMode::OWNER_W | FileMode::GROUP_R | FileMode::OTHER_R;
        expect!(link.set_mode(Some(mode))).to(be_ok());

        expect!(link.metadata())
            .to(be_ok())
            .map(|metadata| metadata.mode())
            .to(be_some())
            .to(equal(
                FileMode::OWNER_RWX | FileMode::GROUP_RWX | FileMode::OTHER_RWX,
            ));

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
            .map(|metadata| metadata.mtime())
            .to(be_some())
            .to(equal(truncated_mtime));

        Ok(())
    })
}

#[test]
fn set_file_mtime_errors_when_file_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.set_mtime(None)).to(have_error_kind(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn set_file_mtime_with_pre_epoch_mtime_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create_file()?;

        let pre_epoch_mtime = SystemTime::UNIX_EPOCH - Duration::from_secs(1);

        expect!(file.set_mtime(Some(pre_epoch_mtime))).to(have_error_kind(ErrorKind::InvalidArgs));

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

        expect!(file.is_empty()).to(have_error_kind(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn is_file_empty_errors_when_it_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.is_empty()).to(have_error_kind(ErrorKind::NotARegularFile));

        Ok(())
    })
}

#[test]
fn is_file_empty_errors_when_it_is_a_symlink() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut link = archive.open("link")?;
        link.create_symlink("target")?;

        expect!(link.is_empty()).to(have_error_kind(ErrorKind::NotARegularFile));

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

        expect!(file.is_compressed()).to(have_error_kind(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn is_file_compressed_errors_when_it_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.is_compressed()).to(have_error_kind(ErrorKind::NotARegularFile));

        Ok(())
    })
}

#[test]
fn is_file_compressed_errors_when_it_is_a_symlink() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut link = archive.open("link")?;
        link.create_symlink("target")?;

        expect!(link.is_compressed()).to(have_error_kind(ErrorKind::NotARegularFile));

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

        expect!(file.reader()).to(have_error_kind(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn open_reader_errors_when_file_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.reader()).to(have_error_kind(ErrorKind::NotARegularFile));

        Ok(())
    })
}

#[test]
fn open_reader_errors_when_file_is_a_symlink() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut link = archive.open("link")?;
        link.create_symlink("target")?;

        expect!(link.reader()).to(have_error_kind(ErrorKind::NotARegularFile));

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
            .to(have_file_metadata())
            .map(|metadata| metadata.size)
            .to(be_zero());

        Ok(())
    })
}

#[test]
fn truncate_file_errors_when_it_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.truncate()).to(have_error_kind(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn truncate_file_errors_when_it_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("dir")?;
        file.create_dir()?;

        expect!(file.truncate()).to(have_error_kind(ErrorKind::NotARegularFile));

        Ok(())
    })
}

#[test]
fn truncate_file_errors_when_it_is_a_symlink() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("link")?;
        file.create_symlink("target")?;

        expect!(file.truncate()).to(have_error_kind(ErrorKind::NotARegularFile));

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

        expect!(file.write_bytes(&expected)).to(have_error_kind(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn write_bytes_errors_when_file_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.write_bytes(b"file content")).to(have_error_kind(ErrorKind::NotARegularFile));

        Ok(())
    })
}

#[test]
fn write_bytes_errors_when_file_is_a_symlink() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut link = archive.open("link")?;
        link.create_symlink("target")?;

        expect!(link.write_bytes(b"file content")).to(have_error_kind(ErrorKind::NotARegularFile));

        Ok(())
    })
}

//
// `File::write_str`
//

#[test]
fn write_string_errors_when_file_does_not_exist() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;

        expect!(file.write_str("file content")).to(have_error_kind(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn write_string_errors_when_file_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.write_str("file content")).to(have_error_kind(ErrorKind::NotARegularFile));

        Ok(())
    })
}

#[test]
fn write_string_errors_when_file_is_a_symlink() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut link = archive.open("link")?;
        link.create_symlink("target")?;

        expect!(link.write_str("file content")).to(have_error_kind(ErrorKind::NotARegularFile));

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
            .to(have_error_kind(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn write_from_reader_errors_when_file_is_a_directory() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.write_from(&mut "file content".as_bytes()))
            .to(have_error_kind(ErrorKind::NotARegularFile));

        Ok(())
    })
}

#[test]
fn write_from_reader_errors_when_file_is_a_symlink() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut link = archive.open("link")?;
        link.create_symlink("target")?;

        expect!(link.write_from(&mut "file content".as_bytes()))
            .to(have_error_kind(ErrorKind::NotARegularFile));

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

        expect!(file.write_file(&mut temp_file)).to(have_error_kind(ErrorKind::NotFound));

        Ok(())
    })
}

#[test]
fn write_from_file_errors_when_file_is_a_directory() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::tempfile()?;

    connection()?.exec(|archive| {
        let mut dir = archive.open("dir")?;
        dir.create_dir()?;

        expect!(dir.write_file(&mut temp_file)).to(have_error_kind(ErrorKind::NotARegularFile));

        Ok(())
    })
}

#[test]
fn write_from_file_errors_when_file_is_a_symlink() -> sqlarfs::Result<()> {
    let mut temp_file = tempfile::tempfile()?;

    connection()?.exec(|archive| {
        let mut link = archive.open("link")?;
        link.create_symlink("target")?;

        expect!(link.write_file(&mut temp_file)).to(have_error_kind(ErrorKind::NotARegularFile));

        Ok(())
    })
}
