mod error;
mod file;
mod metadata;

pub use error::{Error, Result};
pub use file::{File, SeekableFile};
pub use metadata::FileMode;
