use std::fs;
use std::time::{Duration, SystemTime};

use common::{connection, truncate_mtime};
use sqlarfs::{Error, ExtractOptions, FileMode};
use xpct::{
    be_directory, be_err, be_existing_file, be_false, be_ok, be_regular_file, be_true, equal,
    expect, match_pattern, pattern,
};

mod common;

#[test]
fn extracting_when_source_path_does_not_exist_errors() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        expect!(archive.extract("nonexistent", temp_dir.path().join("dest")))
            .to(be_err())
            .to(equal(Error::FileNotFound {
                path: "nonexistent".into(),
            }));

        Ok(())
    })
}

#[test]
fn extracting_when_source_is_a_file_and_dest_has_no_parent_dir_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        expect!(archive.extract("file", "/nonexistent/dest"))
            .to(be_err())
            .to(equal(Error::NoParentDirectory {
                path: "/nonexistent/dest".into(),
            }));

        Ok(())
    })
}

#[test]
fn extracting_when_source_is_a_dir_and_dest_has_no_parent_dir_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;

        expect!(archive.extract("dir", "/nonexistent/dest"))
            .to(be_err())
            .to(equal(Error::NoParentDirectory {
                path: "/nonexistent/dest".into(),
            }));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn extracting_when_source_is_a_symlink_and_dest_has_no_parent_dir_errors() -> sqlarfs::Result<()> {
    let symlink_target = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        archive
            .open("symlink")?
            .create_symlink(symlink_target.path())?;

        expect!(archive.extract("symlink", "/nonexistent/dest"))
            .to(be_err())
            .to(equal(Error::NoParentDirectory {
                path: "/nonexistent/dest".into(),
            }));

        Ok(())
    })
}

#[test]
fn extracting_when_source_is_a_file_and_dest_already_exists_and_is_a_file_errors(
) -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        expect!(archive.extract("file", temp_file.path()))
            .to(be_err())
            .to(equal(Error::FileAlreadyExists {
                path: temp_file.path().into(),
            }));

        Ok(())
    })
}

#[test]
fn extracting_when_source_is_a_file_and_dest_already_exists_and_is_a_dir_errors(
) -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        expect!(archive.extract("file", temp_dir.path()))
            .to(be_err())
            .to(equal(Error::FileAlreadyExists {
                path: temp_dir.path().into(),
            }));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn extracting_when_source_is_a_file_and_dest_already_exists_and_is_a_symlink_errors(
) -> sqlarfs::Result<()> {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir()?;
    let link_path = temp_dir.path().join("symlink");

    symlink("/nonexistent", &link_path)?;

    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        expect!(archive.extract("file", &link_path))
            .to(be_err())
            .to(equal(Error::FileAlreadyExists { path: link_path }));

        Ok(())
    })
}

#[test]
fn extracting_when_source_is_a_dir_and_dest_already_exists_and_is_a_file_errors(
) -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;

        expect!(archive.extract("dir", temp_file.path()))
            .to(be_err())
            .to(equal(Error::FileAlreadyExists {
                path: temp_file.path().into(),
            }));

        Ok(())
    })
}

#[test]
fn extracting_when_source_is_a_dir_and_dest_already_exists_and_is_a_dir_errors(
) -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;

        expect!(archive.extract("dir", temp_dir.path()))
            .to(be_err())
            .to(equal(Error::FileAlreadyExists {
                path: temp_dir.path().into(),
            }));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn extracting_when_source_is_a_dir_and_dest_already_exists_and_is_a_symlink_errors(
) -> sqlarfs::Result<()> {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir()?;
    let link_path = temp_dir.path().join("symlink");

    symlink("/nonexistent", &link_path)?;

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;

        expect!(archive.extract("dir", &link_path))
            .to(be_err())
            .to(equal(Error::FileAlreadyExists { path: link_path }));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn extracting_when_source_is_a_symlink_and_dest_already_exists_and_is_a_file_errors(
) -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        archive.open("symlink")?.create_symlink("/nonexistent")?;

        expect!(archive.extract("symlink", temp_file.path()))
            .to(be_err())
            .to(equal(Error::FileAlreadyExists {
                path: temp_file.path().into(),
            }));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn extracting_when_source_is_a_symlink_and_dest_already_exists_and_is_a_dir_errors(
) -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        archive.open("symlink")?.create_symlink("/nonexistent")?;

        expect!(archive.extract("symlink", temp_dir.path()))
            .to(be_err())
            .to(equal(Error::FileAlreadyExists {
                path: temp_dir.path().into(),
            }));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn extracting_when_source_is_a_symlink_and_dest_already_exists_and_is_a_symlink_errors(
) -> sqlarfs::Result<()> {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir()?;
    let link_path = temp_dir.path().join("symlink");

    symlink("/nonexistent", &link_path)?;

    connection()?.exec(|archive| {
        archive.open("symlink")?.create_symlink("/nonexistent")?;

        expect!(archive.extract("symlink", &link_path))
            .to(be_err())
            .to(equal(Error::FileAlreadyExists { path: link_path }));

        Ok(())
    })
}

#[test]
fn extracting_when_source_path_is_absolute_errors() -> sqlarfs::Result<()> {
    let src_path = if cfg!(windows) { r"C:\file" } else { "/file" };
    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        expect!(archive.extract(src_path, temp_dir.path().join("dest")))
            .to(be_err())
            .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn extracting_when_source_path_is_not_valid_unicode_errors() -> sqlarfs::Result<()> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        expect!(archive.extract(
            OsStr::from_bytes(b"invalid-unicode-\xff"),
            temp_dir.path().join("dest")
        ))
        .to(be_err())
        .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

        Ok(())
    })
}

#[test]
fn extract_regular_file() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let dest_path = temp_dir.path().join("dest");

    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        expect!(archive.extract("file", &dest_path)).to(be_ok());

        expect!(dest_path).to(be_regular_file());

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn extract_symlink() -> sqlarfs::Result<()> {
    use xpct::be_symlink;

    let temp_dir = tempfile::tempdir()?;
    let symlink_target = tempfile::NamedTempFile::new()?;
    let dest_path = temp_dir.path().join("dest");

    connection()?.exec(|archive| {
        archive
            .open("symlink")?
            .create_symlink(symlink_target.path())?;

        expect!(archive.extract("symlink", &dest_path)).to(be_ok());

        expect!(&dest_path).to(be_symlink());
        expect!(fs::read_link(dest_path))
            .to(be_ok())
            .to(equal(symlink_target.path()));

        Ok(())
    })
}

#[test]
#[cfg(windows)]
fn extracting_symlinks_is_a_noop_on_windows() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let symlink_target = tempfile::NamedTempFile::new()?;
    let dest_path = temp_dir.path().join("dest");

    connection()?.exec(|archive| {
        archive
            .open("symlink")?
            .create_symlink(symlink_target.path())?;

        expect!(archive.extract("symlink", &dest_path)).to(be_ok());

        expect!(dest_path.try_exists()).to(be_ok()).to(be_false());

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn extract_symlink_when_dest_already_exists() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let symlink_target = tempfile::NamedTempFile::new()?;
    let dest_path = temp_dir.path().join("dest");

    fs::File::create(&dest_path)?;

    connection()?.exec(|archive| {
        archive
            .open("symlink")?
            .create_symlink(symlink_target.path())?;

        expect!(archive.extract("symlink", &dest_path))
            .to(be_err())
            .to(equal(Error::FileAlreadyExists { path: dest_path }));

        Ok(())
    })
}

#[test]
fn extract_empty_directory() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let dest_path = temp_dir.path().join("dest");

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;

        expect!(archive.extract("dir", &dest_path)).to(be_ok());

        expect!(dest_path.exists()).to(be_true());
        expect!(dest_path).to(be_directory());

        Ok(())
    })
}

#[test]
fn extract_directory_with_children() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let dest_dir = temp_dir.path().join("dest");

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;
        archive.open("dir/child-file")?.create_file()?;
        archive.open("dir/child-dir")?.create_dir()?;

        expect!(archive.extract("dir", &dest_dir)).to(be_ok());

        expect!(&dest_dir).to(be_directory());
        expect!(dest_dir.join("child-file")).to(be_regular_file());
        expect!(dest_dir.join("child-dir")).to(be_directory());

        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn extracting_preserves_unix_file_mode() -> sqlarfs::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempfile::tempdir()?;
    let dest_path = temp_dir.path().join("dest");
    let expected_mode = FileMode::OWNER_R | FileMode::GROUP_R | FileMode::OTHER_R;

    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create_file()?;
        file.set_mode(Some(expected_mode))?;

        expect!(archive.extract("file", &dest_path)).to(be_ok());

        let actual_mode = dest_path.metadata()?.permissions().mode();
        let just_permissions_bits = actual_mode & 0o777;

        expect!(just_permissions_bits).to(equal(expected_mode.bits()));

        Ok(())
    })
}

#[test]
fn extracting_preserves_file_mtime() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let dest_path = temp_dir.path().join("dest");

    // Some time in the past that a newly-created file could not have by default.
    let expected_mtime = SystemTime::now() - Duration::from_secs(60);

    connection()?.exec(|archive| {
        let mut file = archive.open("file")?;
        file.create_file()?;
        file.set_mtime(Some(expected_mtime))?;

        expect!(archive.extract("file", &dest_path)).to(be_ok());

        let actual_mtime = dest_path.metadata()?.modified()?;
        expect!(actual_mtime).to(equal(truncate_mtime(expected_mtime)));

        Ok(())
    })
}

#[test]
fn extracting_with_trailing_slash_in_source_path() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let dest_path = temp_dir.path().join("dest");

    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        expect!(archive.extract("file/", &dest_path)).to(be_ok());

        expect!(dest_path).to(be_regular_file());

        Ok(())
    })
}

//
// `ExtractOptions::children`
//

#[test]
fn extracting_fails_when_source_is_root_and_children_is_false_errors() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        expect!(archive.extract_with(
            "",
            temp_dir.path().join("dest"),
            &ExtractOptions::new().children(false)
        ))
        .to(be_err())
        .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

        Ok(())
    })
}

#[test]
fn extract_directory_children_to_dir() -> sqlarfs::Result<()> {
    let dest_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file1")?.create_file()?;
        archive.open("dir/file2")?.create_file()?;

        let opts = ExtractOptions::new().children(true);
        expect!(archive.extract_with("dir", &dest_dir, &opts)).to(be_ok());

        expect!(dest_dir.path().join("file1")).to(be_regular_file());
        expect!(dest_dir.path().join("file2")).to(be_regular_file());

        Ok(())
    })
}

#[test]
fn extract_files_from_archive_root() -> sqlarfs::Result<()> {
    let dest_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        archive.open("file1")?.create_file()?;
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file2")?.create_file()?;

        let opts = ExtractOptions::new().children(true);
        expect!(archive.extract_with("", &dest_dir, &opts)).to(be_ok());

        expect!(dest_dir.path().join("file1")).to(be_regular_file());
        expect!(dest_dir.path().join("dir")).to(be_directory());
        expect!(dest_dir.path().join("dir/file2")).to(be_regular_file());

        Ok(())
    })
}

#[test]
fn extracting_directory_children_when_target_doest_not_exist_errors() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let dest_dir = temp_dir.path().join("dest");

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file")?.create_file()?;

        let opts = ExtractOptions::new().children(true);
        expect!(archive.extract_with("dir", &dest_dir, &opts))
            .to(be_err())
            .to(equal(Error::FileNotFound { path: dest_dir }));

        Ok(())
    })
}

#[test]
fn extracting_directory_children_when_target_is_file_errors() -> sqlarfs::Result<()> {
    let temp_file = tempfile::NamedTempFile::new()?;

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file")?.create_file()?;

        let opts = ExtractOptions::new().children(true);
        expect!(archive.extract_with("dir", temp_file.path(), &opts))
            .to(be_err())
            .to(equal(Error::NotADirectory {
                path: temp_file.path().into(),
            }));

        Ok(())
    })
}

#[test]
fn extract_directory_children_when_source_does_not_exist_errors() -> sqlarfs::Result<()> {
    let dest_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        let opts = ExtractOptions::new().children(true);
        expect!(archive.extract_with("nonexistent", &dest_dir, &opts))
            .to(be_err())
            .to(equal(Error::FileNotFound {
                path: "nonexistent".into(),
            }));

        Ok(())
    })
}

#[test]
fn extract_directory_children_when_source_is_file_errors() -> sqlarfs::Result<()> {
    let dest_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        let opts = ExtractOptions::new().children(true);
        expect!(archive.extract_with("file", &dest_dir, &opts))
            .to(be_err())
            .to(equal(Error::NotADirectory {
                path: "file".into(),
            }));

        Ok(())
    })
}

//
// `ExtractOptions::recursive`
//

#[test]
fn extract_directory_with_children_non_recursively() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let dest_path = temp_dir.path().join("dest");

    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file1")?.create_file()?;

        let opts = ExtractOptions::new().recursive(false);
        expect!(archive.extract_with("dir", &dest_path, &opts)).to(be_ok());

        expect!(&dest_path).to(be_directory());
        expect!(dest_path.join("file1")).to_not(be_existing_file());

        Ok(())
    })
}

#[test]
fn extract_regualar_file_non_recursively() -> sqlarfs::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let dest_path = temp_dir.path().join("dest");

    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        let opts = ExtractOptions::new().recursive(false);
        expect!(archive.extract_with("file", &dest_path, &opts)).to(be_ok());

        expect!(dest_path).to(be_regular_file());

        Ok(())
    })
}

#[test]
fn extract_directory_children_to_dir_non_recursively() -> sqlarfs::Result<()> {
    let dest_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        archive.open("file1")?.create_file()?;
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file2")?.create_file()?;
        archive.open("dir/dir2")?.create_dir()?;
        archive.open("dir/dir2/file3")?.create_file()?;

        let opts = ExtractOptions::new().children(true).recursive(false);
        expect!(archive.extract_with("dir", &dest_dir, &opts)).to(be_ok());

        expect!(dest_dir.path().join("file2")).to(be_regular_file());
        expect!(dest_dir.path().join("dir2")).to(be_directory());
        expect!(dest_dir.path().join("dir2/file3")).to_not(be_existing_file());

        Ok(())
    })
}

#[test]
fn extract_files_from_archive_root_non_recursively() -> sqlarfs::Result<()> {
    let dest_dir = tempfile::tempdir()?;

    connection()?.exec(|archive| {
        archive.open("file1")?.create_file()?;
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file2")?.create_file()?;

        let opts = ExtractOptions::new().children(true).recursive(false);
        expect!(archive.extract_with("", &dest_dir, &opts)).to(be_ok());

        expect!(dest_dir.path().join("file1")).to(be_regular_file());
        expect!(dest_dir.path().join("dir")).to(be_directory());
        expect!(dest_dir.path().join("dir/file2")).to_not(be_existing_file());

        Ok(())
    })
}
