mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlarfs::{Connection, FileMode, ListOptions};
use xpct::{be_ok, be_some, be_zero, consist_of, contain_element, equal, expect, why};

use common::truncate_mtime;

fn connection() -> sqlarfs::Result<Connection> {
    Connection::open_in_memory()
}

#[test]
fn list_all_paths() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("file1")?.create()?;
        archive.open("file2")?.create()?;
        archive.open("file3")?.create()?;

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
        file1.create_with(FileMode::OWNER_RWX, file1_mtime)?;
        file1.write_str("123")?;

        archive
            .open("file2")?
            .create_with(FileMode::GROUP_RWX, file2_mtime)?;

        archive
            .open("file3")?
            .create_with(FileMode::OTHER_RWX, file3_mtime)?;

        let entries_by_path = archive
            .list()?
            .map(|entry_result| entry_result.map(|entry| (entry.path().to_path_buf(), entry)))
            .collect::<sqlarfs::Result<HashMap<_, _>>>()?;

        let file1_entry = expect!(entries_by_path.get(Path::new("file1")))
            .to(be_some())
            .into_inner();

        expect!(file1_entry.path()).to(equal(Path::new("file1")));
        expect!(file1_entry.mode())
            .to(be_some())
            .to(equal(FileMode::OWNER_RWX));
        expect!(file1_entry.mtime())
            .to(be_some())
            .to(equal(file1_mtime));
        expect!(file1_entry.size()).to(equal(3));

        let file2_entry = expect!(entries_by_path.get(Path::new("file2")))
            .to(be_some())
            .into_inner();

        expect!(file2_entry.path()).to(equal(Path::new("file2")));
        expect!(file2_entry.mode())
            .to(be_some())
            .to(equal(FileMode::GROUP_RWX));
        expect!(file2_entry.mtime())
            .to(be_some())
            .to(equal(file2_mtime));
        expect!(file2_entry.size()).to(be_zero());

        let file3_entry = expect!(entries_by_path.get(Path::new("file3")))
            .to(be_some())
            .into_inner();

        expect!(file3_entry.path()).to(equal(Path::new("file3")));
        expect!(file3_entry.mode())
            .to(be_some())
            .to(equal(FileMode::OTHER_RWX));
        expect!(file3_entry.mtime())
            .to(be_some())
            .to(equal(file3_mtime));
        expect!(file3_entry.size()).to(be_zero());

        Ok(())
    })
}

#[test]
fn list_with_default_opts() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("file1")?.create()?;
        archive.open("file2")?.create()?;
        archive.open("file3")?.create()?;

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
fn list_with_filter_descendants() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        archive.open("a")?.create()?;
        archive.open("one/b")?.create()?;
        archive.open("one/")?.create()?;
        archive.open("onetwo")?.create()?;
        archive.open("ONE/c")?.create()?;
        archive.open("one/two/d")?.create()?;

        let paths = archive
            .list_with(&ListOptions::new().descendants_of("one"))?
            .map(|entry| Ok(entry?.into_path()))
            .collect::<sqlarfs::Result<Vec<_>>>()?;

        expect!(&paths).to_not(why(
            contain_element(PathBuf::from("a")),
            "This is the parent directory.",
        ));

        expect!(&paths).to_not(why(
            contain_element(PathBuf::from("one/")),
            "The same path with a trailing slash is not a descendant.",
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
            PathBuf::from("one/two/d"),
        ]));

        Ok(())
    })
}

#[test]
fn list_with_sort_by_mtime() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let base_time = SystemTime::now();

        archive
            .open("now")?
            .create_with(FileMode::empty(), base_time)?;

        // This will be truncated to one full second behind `base_time`.
        archive.open("100_millis_behind")?.create_with(
            FileMode::empty(),
            truncate_mtime(base_time) - Duration::from_millis(100),
        )?;

        archive
            .open("two_secs_behind")?
            .create_with(FileMode::empty(), base_time - Duration::from_secs(2))?;

        archive
            .open("three_secs_behind")?
            .create_with(FileMode::empty(), base_time - Duration::from_secs(3))?;

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
        file_a.create()?;
        file_a.write_str("a")?;

        let mut file_b = archive.open("size 2")?;
        file_b.create()?;
        file_b.write_str("bb")?;

        let mut file_c = archive.open("size 3")?;
        file_c.create()?;
        file_c.write_str("ccc")?;

        let mut file_d = archive.open("size 4")?;
        file_d.create()?;
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
