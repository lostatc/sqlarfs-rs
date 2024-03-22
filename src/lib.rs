mod error;
mod file;
mod metadata;
mod seekable;
mod stream;

pub use error::{Error, Result};
pub use file::File;
pub use metadata::FileMode;
pub use seekable::SeekableFile;
pub use stream::{FileReader, FileWriter};
