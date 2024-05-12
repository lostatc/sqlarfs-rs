#![allow(dead_code)]

mod matchers;

use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rand::prelude::*;
use sqlarfs::Connection;

#[allow(unused_imports)]
pub use matchers::*;

pub const WRITE_DATA_SIZE: usize = 64;

pub fn connection() -> sqlarfs::Result<Connection> {
    Connection::open_in_memory()
}

// This is for rounding a file mtime down to the nearest whole second, because that's what this
// implementation of SQLite archives does.
pub fn truncate_mtime(time: SystemTime) -> SystemTime {
    let unix_time_secs = time.duration_since(UNIX_EPOCH).unwrap().as_secs();
    UNIX_EPOCH + Duration::from_secs(unix_time_secs)
}

pub fn random_bytes(len: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(len);
    let mut rng = SmallRng::from_entropy();
    rng.fill_bytes(&mut buf);
    buf
}

// A buffer of bytes that can be compressed to a smaller size.
pub fn compressible_bytes() -> Vec<u8> {
    vec![0u8; 64]
}

// A buffer of bytes that cannot be compressed to a smaller size.
pub fn incompressible_bytes() -> Vec<u8> {
    // Make it deterministic to ensure tests don't flake, just in case zlib *can* manage to
    // compress these random non-repeating bytes.
    let mut rng = SmallRng::seed_from_u64(0);

    // No repeating bytes.
    let pool = (0..255).collect::<Vec<u8>>();

    pool.choose_multiple(&mut rng, 64).copied().collect()
}

// Run the given function in a separate thread with a timeout. This is for tests where the failure
// case could result in them hanging.
pub fn with_timeout<F>(timeout: Duration, f: F) -> sqlarfs::Result<()>
where
    F: FnOnce() -> sqlarfs::Result<()> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        let result = f();
        tx.send(()).unwrap();
        result
    });

    rx.recv_timeout(timeout)
        .unwrap_or_else(|_| panic!("test timed out after {} seconds", timeout.as_secs()));

    handle.join().unwrap()
}

// TODO: Use `eyre::Result` instead of `sqlarfs::Result` for all tests, making this unnecessary.
pub fn into_sqlarfs_error<E>(_: E) -> sqlarfs::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    sqlarfs::Error::Io {
        kind: io::ErrorKind::Other,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlarTableRow {
    pub name: String,
    pub mode: Option<u32>,
    pub mtime: Option<u64>,
    pub sz: Option<i64>,
    pub data: Option<Vec<u8>>,
}

pub fn dump_table(db: &Path) -> sqlarfs::Result<Vec<SqlarTableRow>> {
    rusqlite::Connection::open_with_flags(
        db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sqlarfs::Error::from)?
    .prepare("SELECT name, mode, mtime, sz, data FROM sqlar;")?
    .query_map([], |row| {
        let name = row.get(0)?;
        let mode = row.get(1)?;
        let mtime = row.get(2)?;
        let sz = row.get(3)?;

        Ok(SqlarTableRow {
            name,
            mode,
            mtime,
            sz,
            data: if sz == Some(-1) {
                row.get::<_, String>(4)?.as_bytes().to_vec().into()
            } else {
                row.get(4)?
            },
        })
    })?
    .collect::<Result<Vec<_>, _>>()
    .map_err(sqlarfs::Error::from)
}
