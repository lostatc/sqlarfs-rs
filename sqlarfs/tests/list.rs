mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlarfs::{Error, FileMode, FileType, ListOptions};
use xpct::{
    be_empty, be_err, be_gt, be_lt, be_ok, be_some, be_zero, consist_of, contain_element, equal,
    expect, fields, match_fields, match_pattern, pattern, why,
};

use common::{connection, have_file_metadata, truncate_mtime, RegularFileMetadata};

//
// `Archive::list`
//

#[test]
fn list_all_paths() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("file1")?.create_file()?;
        archive.open("file2")?.create_file()?;
        archive.open("file3")?.create_file()?;

        expect!(archive.list())
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(consist_of(&[
                PathBuf::from("file1"),
                PathBuf::from("file2"),
                PathBuf::from("file3"),
            ]));

        Ok(())
    })
}

#[test]
fn list_all_paths_with_metadata() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let file1_mtime = UNIX_EPOCH + Duration::from_secs(1);
        let file2_mtime = UNIX_EPOCH + Duration::from_secs(2);
        let file3_mtime = UNIX_EPOCH + Duration::from_secs(3);

        let mut file1 = archive.open("file1")?;
        file1.create_file()?;
        file1.set_mode(Some(FileMode::OWNER_RWX))?;
        file1.set_mtime(Some(file1_mtime))?;
        file1.write_str("123")?;

        let mut file2 = archive.open("file2")?;
        file2.create_file()?;
        file2.set_mode(Some(FileMode::GROUP_RWX))?;
        file2.set_mtime(Some(file2_mtime))?;

        let mut file3 = archive.open("file3")?;
        file3.create_file()?;
        file3.set_mode(Some(FileMode::OTHER_RWX))?;
        file3.set_mtime(Some(file3_mtime))?;

        let entries_by_path = archive
            .list()?
            .map(|entry_result| entry_result.map(|entry| (entry.path().to_path_buf(), entry)))
            .collect::<sqlarfs::Result<HashMap<_, _>>>()?;

        let file1_entry = expect!(entries_by_path.get(Path::new("file1")))
            .to(be_some())
            .into_inner();

        expect!(file1_entry.path()).to(equal(Path::new("file1")));
        expect!(file1_entry.metadata().to_owned())
            .to(have_file_metadata())
            .to(match_fields(fields!(RegularFileMetadata {
                mode: equal(Some(FileMode::OWNER_RWX)),
                mtime: equal(Some(file1_mtime)),
                size: equal(3),
            })));

        let file2_entry = expect!(entries_by_path.get(Path::new("file2")))
            .to(be_some())
            .into_inner();

        expect!(file2_entry.path()).to(equal(Path::new("file2")));
        expect!(file2_entry.metadata().to_owned())
            .to(have_file_metadata())
            .to(match_fields(fields!(RegularFileMetadata {
                mode: equal(Some(FileMode::GROUP_RWX)),
                mtime: equal(Some(file2_mtime)),
                size: be_zero(),
            })));

        let file3_entry = expect!(entries_by_path.get(Path::new("file3")))
            .to(be_some())
            .into_inner();

        expect!(file3_entry.path()).to(equal(Path::new("file3")));
        expect!(file3_entry.metadata().to_owned())
            .to(have_file_metadata())
            .to(match_fields(fields!(RegularFileMetadata {
                mode: equal(Some(FileMode::OTHER_RWX)),
                mtime: equal(Some(file3_mtime)),
                size: be_zero(),
            })));

        Ok(())
    })
}

//
// `Archive::list_with`
//

#[test]
fn specifying_mutually_exclusive_sort_options_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let opts = ListOptions::new().by_size().by_mtime();
        expect!(archive.list_with(&opts))
            .to(be_err())
            .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

        let opts = ListOptions::new().by_mtime().by_size();
        expect!(archive.list_with(&opts))
            .to(be_err())
            .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

        Ok(())
    })
}

#[test]
fn specifying_mutually_exclusive_order_options_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let opts = ListOptions::new().asc().desc();
        expect!(archive.list_with(&opts))
            .to(be_err())
            .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

        let opts = ListOptions::new().desc().asc();
        expect!(archive.list_with(&opts))
            .to(be_err())
            .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

        Ok(())
    })
}

#[test]
fn specifying_mutually_exclusive_file_type_options_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let opts = ListOptions::new().file_type(FileType::File).by_size();
        expect!(archive.list_with(&opts))
            .to(be_err())
            .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

        let opts = ListOptions::new().by_size().file_type(FileType::File);
        expect!(archive.list_with(&opts))
            .to(be_err())
            .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

        Ok(())
    })
}

#[test]
fn specifying_mutually_exclusive_descendants_options_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let opts = ListOptions::new().descendants_of("a").children_of("a");
        expect!(archive.list_with(&opts))
            .to(be_err())
            .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

        let opts = ListOptions::new().children_of("a").descendants_of("a");
        expect!(archive.list_with(&opts))
            .to(be_err())
            .to(match_pattern(pattern!(Error::InvalidArgs { .. })));

        Ok(())
    })
}

#[test]
fn list_with_default_opts() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("file1")?.create_file()?;
        archive.open("file2")?.create_file()?;
        archive.open("file3")?.create_file()?;

        expect!(archive.list_with(&ListOptions::new()))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(consist_of(&[
                PathBuf::from("file1"),
                PathBuf::from("file2"),
                PathBuf::from("file3"),
            ]));

        Ok(())
    })
}

#[test]
fn list_with_filter_immediate_children_of_dir() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("dir1")?.create_dir()?;
        archive.open("dir1/file1")?.create_file()?;
        archive.open("dir1/dir2")?.create_dir()?;
        archive.open("dir1/dir2/file2")?.create_file()?;
        archive.open("file3")?.create_file()?;

        let paths = archive
            .list_with(&ListOptions::new().children_of("dir1"))?
            .map(|entry| Ok(entry?.into_path()))
            .collect::<sqlarfs::Result<Vec<_>>>()?;

        expect!(&paths).to_not(why(
            contain_element(PathBuf::from("dir1/dir2/file2")),
            "This is a descendant of `dir1` but not an immediate child.",
        ));

        expect!(&paths).to_not(why(
            contain_element(PathBuf::from("file3")),
            "This is not a descendant of `dir1`.",
        ));

        expect!(paths).to(consist_of(&[
            PathBuf::from("dir1/file1"),
            PathBuf::from("dir1/dir2"),
        ]));

        Ok(())
    })
}

#[test]
fn list_with_filter_immediate_children_of_regular_file() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        let paths = archive
            .list_with(&ListOptions::new().children_of("file"))?
            .map(|entry| Ok(entry?.into_path()))
            .collect::<sqlarfs::Result<Vec<_>>>()?;

        expect!(paths).to(be_empty());

        Ok(())
    })
}

#[test]
fn list_with_filter_immediate_children_of_archive_root() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("file1")?.create_file()?;
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file2")?.create_file()?;

        let paths = archive
            .list_with(&ListOptions::new().children_of(""))?
            .map(|entry| Ok(entry?.into_path()))
            .collect::<sqlarfs::Result<Vec<_>>>()?;

        expect!(paths).to(consist_of(&[PathBuf::from("file1"), PathBuf::from("dir")]));

        Ok(())
    })
}

#[test]
fn list_with_filter_immediate_children_strips_trailing_slash() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file")?.create_file()?;

        let parent = if cfg!(windows) { r"dir\" } else { "dir/" };

        let paths = archive
            .list_with(&ListOptions::new().children_of(parent))?
            .map(|entry| Ok(entry?.into_path()))
            .collect::<sqlarfs::Result<Vec<_>>>()?;

        expect!(paths).to(consist_of(&[PathBuf::from("dir/file")]));

        Ok(())
    })
}

#[test]
fn list_with_filter_descendants_of_dir() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("a")?.create_dir()?;
        archive.open("one")?.create_dir()?;
        archive.open("one/b")?.create_file()?;
        archive.open("onetwo")?.create_dir()?;
        archive.open("ONE")?.create_dir()?;
        archive.open("ONE/c")?.create_file()?;
        archive.open("one/two")?.create_dir()?;
        archive.open("one/two/d")?.create_file()?;

        let paths = archive
            .list_with(&ListOptions::new().descendants_of("one"))?
            .map(|entry| Ok(entry?.into_path()))
            .collect::<sqlarfs::Result<Vec<_>>>()?;

        expect!(&paths).to_not(why(
            contain_element(PathBuf::from("a")),
            "This is the parent directory.",
        ));

        expect!(&paths).to_not(why(
            contain_element(PathBuf::from("onetwo")),
            "Matching on string prefixes isn't the same thing as matching on descendants.",
        ));

        expect!(&paths).to_not(why(
            contain_element(PathBuf::from("ONE/c")),
            "File path matching must be case-sensitive.",
        ));

        expect!(paths).to(consist_of(&[
            PathBuf::from("one/b"),
            PathBuf::from("one/two"),
            PathBuf::from("one/two/d"),
        ]));

        Ok(())
    })
}

#[test]
fn list_with_filter_descendants_of_regular_file() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("file")?.create_file()?;

        let paths = archive
            .list_with(&ListOptions::new().descendants_of("file"))?
            .map(|entry| Ok(entry?.into_path()))
            .collect::<sqlarfs::Result<Vec<_>>>()?;

        expect!(paths).to(be_empty());

        Ok(())
    })
}

#[test]
fn list_with_filter_descendants_of_archive_root() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("file1")?.create_file()?;
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file2")?.create_file()?;

        let paths = archive
            .list_with(&ListOptions::new().descendants_of(""))?
            .map(|entry| Ok(entry?.into_path()))
            .collect::<sqlarfs::Result<Vec<_>>>()?;

        expect!(paths).to(consist_of(&[
            PathBuf::from("file1"),
            PathBuf::from("dir"),
            PathBuf::from("dir/file2"),
        ]));

        Ok(())
    })
}

#[test]
fn list_with_filter_descendants_strips_trailing_slash() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file")?.create_file()?;

        let ancestor = if cfg!(windows) { r"dir\" } else { "dir/" };

        let paths = archive
            .list_with(&ListOptions::new().descendants_of(ancestor))?
            .map(|entry| Ok(entry?.into_path()))
            .collect::<sqlarfs::Result<Vec<_>>>()?;

        expect!(paths).to(consist_of(&[PathBuf::from("dir/file")]));

        Ok(())
    })
}

#[test]
fn list_with_sort_by_mtime() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let base_time = SystemTime::now();

        let mut file1 = archive.open("now")?;
        file1.create_file()?;
        file1.set_mtime(Some(base_time))?;

        // This will be truncated to one full second behind `base_time`.
        let mut file2 = archive.open("100_millis_behind")?;
        file2.create_file()?;
        file2.set_mtime(Some(truncate_mtime(base_time) - Duration::from_millis(100)))?;

        let mut file3 = archive.open("two_secs_behind")?;
        file3.create_file()?;
        file3.set_mtime(Some(base_time - Duration::from_secs(2)))?;

        let mut file4 = archive.open("three_secs_behind")?;
        file4.create_file()?;
        file4.set_mtime(Some(base_time - Duration::from_secs(3)))?;

        expect!(archive.list_with(&ListOptions::new().by_mtime().asc()))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(equal(&[
                PathBuf::from("three_secs_behind"),
                PathBuf::from("two_secs_behind"),
                PathBuf::from("100_millis_behind"),
                PathBuf::from("now"),
            ]));

        expect!(archive.list_with(&ListOptions::new().by_mtime().desc()))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(equal(&[
                PathBuf::from("now"),
                PathBuf::from("100_millis_behind"),
                PathBuf::from("two_secs_behind"),
                PathBuf::from("three_secs_behind"),
            ]));

        Ok(())
    })
}

#[test]
fn list_with_sort_by_size() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file_a = archive.open("size 1")?;
        file_a.create_file()?;
        file_a.write_str("a")?;

        let mut file_b = archive.open("size 2")?;
        file_b.create_file()?;
        file_b.write_str("bb")?;

        let mut file_c = archive.open("size 3")?;
        file_c.create_file()?;
        file_c.write_str("ccc")?;

        let mut file_d = archive.open("size 4")?;
        file_d.create_file()?;
        file_d.write_str("dddd")?;

        expect!(archive.list_with(&ListOptions::new().by_size().asc()))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(equal(&[
                PathBuf::from("size 1"),
                PathBuf::from("size 2"),
                PathBuf::from("size 3"),
                PathBuf::from("size 4"),
            ]));

        expect!(archive.list_with(&ListOptions::new().by_size().desc()))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(equal(&[
                PathBuf::from("size 4"),
                PathBuf::from("size 3"),
                PathBuf::from("size 2"),
                PathBuf::from("size 1"),
            ]));

        Ok(())
    })
}

#[test]
fn list_with_sort_by_depth() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("a")?.create_dir()?;
        archive.open("a/b")?.create_dir()?;
        archive.open("c")?.create_dir()?;
        archive.open("a/b/d")?.create_file()?;

        // Technically, the default sort order is unspecified. As currently implemented, it's based
        // on the SQLite rowid, which represents the order in which paths were added to the
        // archive. We need to ensure the order is *not* sorted by depth by default so that this
        // test doesn't pass when it should fail.
        expect!(archive.list_with(&ListOptions::new()))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(equal(vec![
                PathBuf::from("a"),
                PathBuf::from("a/b"),
                PathBuf::from("c"),
                PathBuf::from("a/b/d"),
            ]));

        let entries = expect!(archive.list_with(&ListOptions::new().by_depth().asc()))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .into_inner();

        let path_a = entries
            .iter()
            .position(|entry| entry == Path::new("a"))
            .unwrap();
        let path_b = entries
            .iter()
            .position(|entry| entry == Path::new("a/b"))
            .unwrap();
        let path_c = entries
            .iter()
            .position(|entry| entry == Path::new("c"))
            .unwrap();
        let path_d = entries
            .iter()
            .position(|entry| entry == Path::new("a/b/d"))
            .unwrap();

        expect!(path_a).to(be_lt(path_b));
        expect!(path_a).to(be_lt(path_d));
        expect!(path_c).to(be_lt(path_b));
        expect!(path_c).to(be_lt(path_d));
        expect!(path_b).to(be_lt(path_d));

        let entries = expect!(archive.list_with(&ListOptions::new().by_depth().desc()))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .into_inner();

        let path_a = entries
            .iter()
            .position(|entry| entry == Path::new("a"))
            .unwrap();
        let path_b = entries
            .iter()
            .position(|entry| entry == Path::new("a/b"))
            .unwrap();
        let path_c = entries
            .iter()
            .position(|entry| entry == Path::new("c"))
            .unwrap();
        let path_d = entries
            .iter()
            .position(|entry| entry == Path::new("a/b/d"))
            .unwrap();

        expect!(path_a).to(be_gt(path_b));
        expect!(path_a).to(be_gt(path_d));
        expect!(path_c).to(be_gt(path_b));
        expect!(path_c).to(be_gt(path_d));
        expect!(path_b).to(be_gt(path_d));

        Ok(())
    })
}

#[test]
fn list_with_sort_by_mtime_while_filtering_descendants() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let base_time = SystemTime::now();

        let mut file_a = archive.open("a")?;
        file_a.create_file()?;

        let mut dir_one = archive.open("one")?;
        dir_one.create_dir()?;

        let mut file_b = archive.open("one/b")?;
        file_b.create_file()?;
        file_b.set_mtime(Some(base_time - Duration::from_secs(1)))?;

        let mut dir_two = archive.open("one/two")?;
        dir_two.create_dir()?;
        dir_two.set_mtime(Some(base_time - Duration::from_secs(2)))?;

        let mut file_d = archive.open("one/two/d")?;
        file_d.create_file()?;
        file_d.set_mtime(Some(base_time - Duration::from_secs(3)))?;

        let mut file_c = archive.open("one/two/c")?;
        file_c.create_file()?;
        file_c.set_mtime(Some(base_time - Duration::from_secs(4)))?;

        let mut dir_three = archive.open("one/two/three")?;
        dir_three.create_dir()?;
        dir_three.set_mtime(Some(base_time - Duration::from_secs(5)))?;

        let mut file_e = archive.open("one/two/three/e")?;
        file_e.create_file()?;
        file_e.set_mtime(Some(base_time - Duration::from_secs(6)))?;

        let opts = ListOptions::new()
            .descendants_of(Path::new("one"))
            .by_mtime()
            .asc();

        expect!(archive.list_with(&opts))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(equal(&[
                PathBuf::from("one/two/three/e"),
                PathBuf::from("one/two/three"),
                PathBuf::from("one/two/c"),
                PathBuf::from("one/two/d"),
                PathBuf::from("one/two"),
                PathBuf::from("one/b"),
            ]));

        let opts = ListOptions::new()
            .descendants_of(Path::new("one"))
            .by_mtime()
            .desc();

        expect!(archive.list_with(&opts))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(equal(&[
                PathBuf::from("one/b"),
                PathBuf::from("one/two"),
                PathBuf::from("one/two/d"),
                PathBuf::from("one/two/c"),
                PathBuf::from("one/two/three"),
                PathBuf::from("one/two/three/e"),
            ]));

        Ok(())
    })
}

#[test]
fn list_with_sort_by_size_while_filtering_descendants() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file_a = archive.open("a")?;
        file_a.create_file()?;

        archive.open("one")?.create_dir()?;

        let mut file_b = archive.open("one/b")?;
        file_b.create_file()?;
        file_b.write_str("b")?;

        archive.open("one/two")?.create_dir()?;

        let mut file_d = archive.open("one/two/d")?;
        file_d.create_file()?;
        file_d.write_str("dd")?;

        let mut file_c = archive.open("one/two/c")?;
        file_c.create_file()?;
        file_c.write_str("ccc")?;

        archive.open("one/two/three")?.create_dir()?;

        let mut file_e = archive.open("one/two/three/e")?;
        file_e.create_file()?;
        file_e.write_str("eeee")?;

        let opts = ListOptions::new()
            .descendants_of(Path::new("one"))
            .by_size()
            .asc();

        expect!(archive.list_with(&opts))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(equal(&[
                PathBuf::from("one/b"),
                PathBuf::from("one/two/d"),
                PathBuf::from("one/two/c"),
                PathBuf::from("one/two/three/e"),
            ]));

        let opts = ListOptions::new()
            .descendants_of(Path::new("one"))
            .by_size()
            .desc();

        expect!(archive.list_with(&opts))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(equal(&[
                PathBuf::from("one/two/three/e"),
                PathBuf::from("one/two/c"),
                PathBuf::from("one/two/d"),
                PathBuf::from("one/b"),
            ]));

        Ok(())
    })
}

#[test]
fn list_with_filter_only_files() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file")?.create_file()?;

        let opts = ListOptions::new().file_type(FileType::File);

        expect!(archive.list_with(&opts))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(consist_of(&[PathBuf::from("dir/file")]));

        Ok(())
    })
}

#[test]
fn list_with_filter_only_dirs() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file")?.create_file()?;

        let opts = ListOptions::new().file_type(FileType::Dir);

        expect!(archive.list_with(&opts))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(consist_of(&[PathBuf::from("dir")]));

        Ok(())
    })
}

#[test]
fn list_with_filter_only_symlinks() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("dir")?.create_dir()?;
        archive.open("dir/file")?.create_file()?;
        archive.open("dir/symlink")?.create_symlink("target")?;

        let opts = ListOptions::new().file_type(FileType::Symlink);

        expect!(archive.list_with(&opts))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(consist_of(&[PathBuf::from("dir/symlink")]));

        Ok(())
    })
}
