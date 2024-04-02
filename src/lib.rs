//! A file archive format and virtual filesystem backed by a SQLite database.
//!
//! This library is a Rust implementation of the [sqlar](https://sqlite.org/sqlar.html) format for
//! SQLite archive files.

// This requires the nightly toolchain.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

mod archive;
mod error;
mod file;
mod list;
mod metadata;
mod open;
mod store;
mod stream;
mod transaction;
mod util;

pub use archive::Archive;
pub use error::{Error, ErrorKind, Result, SqliteErrorCode};
pub use file::{File, FileMetadata};
pub use list::{ListEntries, ListEntry, ListOptions};
pub use metadata::FileMode;
pub use open::OpenOptions;
pub use stream::{Compression, FileReader};
pub use transaction::{Connection, Transaction, TransactionBehavior};
