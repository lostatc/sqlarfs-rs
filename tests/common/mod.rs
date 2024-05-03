#![allow(dead_code)]

mod matchers;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rand::prelude::*;
use sqlarfs::Connection;

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
