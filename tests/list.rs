mod common;

use std::time::SystemTime;
use std::{path::PathBuf, time::Duration};

use sqlarfs::{Connection, FileMode, ListOptions};
use xpct::{be_ok, consist_of, contain_element, equal, expect, why};

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
            .open("two_sec_behind")?
            .create_with(FileMode::empty(), base_time - Duration::from_secs(2))?;

        archive
            .open("three_sec_behind")?
            .create_with(FileMode::empty(), base_time - Duration::from_secs(3))?;

        expect!(archive.list_with(&ListOptions::new().by_mtime().asc()))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(equal(&[
                PathBuf::from("three_sec_behind"),
                PathBuf::from("two_sec_behind"),
                PathBuf::from("100_millis_behind"),
                PathBuf::from("now"),
            ]));

        expect!(archive.list_with(&ListOptions::new().by_mtime().desc()))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(equal(&[
                PathBuf::from("now"),
                PathBuf::from("100_millis_behind"),
                PathBuf::from("two_sec_behind"),
                PathBuf::from("three_sec_behind"),
            ]));

        Ok(())
    })
}

#[test]
fn list_with_sort_by_size() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        let mut file_a = archive.open("a")?;
        file_a.create()?;
        file_a.write_str("a")?;

        let mut file_b = archive.open("b")?;
        file_b.create()?;
        file_b.write_str("bb")?;

        let mut file_c = archive.open("c")?;
        file_c.create()?;
        file_c.write_str("ccc")?;

        let mut file_d = archive.open("d")?;
        file_d.create()?;
        file_d.write_str("dddd")?;

        expect!(archive.list_with(&ListOptions::new().by_size().asc()))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(equal(&[
                PathBuf::from("a"),
                PathBuf::from("b"),
                PathBuf::from("c"),
                PathBuf::from("d"),
            ]));

        expect!(archive.list_with(&ListOptions::new().by_size().desc()))
            .to(be_ok())
            .iter_try_map(|entry| Ok(entry?.into_path()))
            .to(equal(&[
                PathBuf::from("d"),
                PathBuf::from("c"),
                PathBuf::from("b"),
                PathBuf::from("a"),
            ]));

        Ok(())
    })
}
