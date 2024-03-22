mod error;
mod file;
mod metadata;
mod stream;

pub use error::{Error, Result};
pub use file::{File, SeekableFile};
pub use metadata::FileMode;
