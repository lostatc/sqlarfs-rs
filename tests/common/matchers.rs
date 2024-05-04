use std::path::PathBuf;
use std::time::SystemTime;

use sqlarfs::{FileMetadata, FileMode};
use xpct::core::Matcher;
use xpct::{all, be_err, be_some, equal, why};

#[derive(Debug)]
pub struct RegularFileMetadata {
    pub mode: Option<FileMode>,
    pub mtime: Option<SystemTime>,
    pub size: u64,
}

#[derive(Debug)]
pub struct DirMetadata {
    pub mode: Option<FileMode>,
    pub mtime: Option<SystemTime>,
}

#[derive(Debug)]
pub struct SymlinkMetadata {
    pub mtime: Option<SystemTime>,
    pub target: PathBuf,
}

pub fn have_file_metadata<'a>() -> Matcher<'a, FileMetadata, RegularFileMetadata, ()> {
    all(|ctx| {
        ctx.map(|metadata| match metadata {
            FileMetadata::File { mode, mtime, size } => {
                Some(RegularFileMetadata { mode, mtime, size })
            }
            _ => None,
        })
        .to(why(
            be_some(),
            "this is not the metadata for a regular file",
        ))
    })
}

pub fn have_dir_metadata<'a>() -> Matcher<'a, FileMetadata, DirMetadata, ()> {
    all(|ctx| {
        ctx.map(|metadata| match metadata {
            FileMetadata::Dir { mode, mtime } => Some(DirMetadata { mode, mtime }),
            _ => None,
        })
        .to(why(be_some(), "this is not the metadata for a directory"))
    })
}

pub fn have_symlink_metadata<'a>() -> Matcher<'a, FileMetadata, SymlinkMetadata, ()> {
    all(|ctx| {
        ctx.map(|metadata| match metadata {
            FileMetadata::Symlink { mtime, target } => Some(SymlinkMetadata { mtime, target }),
            _ => None,
        })
        .to(why(
            be_some(),
            "this is not the metadata for a symbolic link",
        ))
    })
}

pub fn have_error_kind<'a, T>(
    kind: sqlarfs::ErrorKind,
) -> Matcher<'a, sqlarfs::Result<T>, sqlarfs::ErrorKind, ()>
where
    T: std::fmt::Debug + 'a,
{
    all(|ctx| {
        ctx.to(be_err())?
            .map(|err: sqlarfs::Error| err.kind().clone())
            .to(equal(kind))
    })
}
