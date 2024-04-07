//! A file archive format and virtual filesystem backed by a SQLite database.
//!
//! This library is a Rust implementation of the [sqlar](https://sqlite.org/sqlar.html) format for
//! SQLite archive files.
//!
//! ```
//! use std::io::prelude::*;
//!
//! use sqlarfs::Connection;
//!
//! fn main() -> sqlarfs::Result<()> {
//!     let mut conn = Connection::open_in_memory()?;
//!
//!     let expected = "hello world";
//!
//!     let actual = conn.exec(|archive| {
//!         let mut dir = archive.open("path/to")?;
//!         dir.create_dir_all()?;
//!
//!         let mut file = archive.open("path/to/file")?;
//!         file.create_file()?;
//!
//!         file.write_str(expected)?;
//!
//!         let mut actual = String::new();
//!         let mut reader = file.reader()?;
//!
//!         reader.read_to_string(&mut actual)?;
//!
//!         sqlarfs::Result::Ok(actual)
//!     })?;
//!
//!     assert_eq!(actual, expected);
//!
//!     Ok(())
//! }
//! ```

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
pub use file::File;
pub use list::{ListEntries, ListEntry, ListOptions};
pub use metadata::{FileMetadata, FileMode, FileType};
pub use open::OpenOptions;
pub use stream::{Compression, FileReader};
pub use transaction::{Connection, Transaction, TransactionBehavior};
