use std::borrow::Cow;

const SEP: &str = "/";

/// A path in a SQLite archive.
///
/// All file paths in a SQLite archive are encoded using the database encoding; unlike a
/// [`std::path::Path`], they must be valid Unicode.
///
/// [`Archive`]: crate::Archive
#[derive(Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Path {
    segments: Vec<String>,
}

impl Path {
    /// Construct a new empty [`Path`].
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Join this path with the given path.
    pub fn join<P: Into<Path>>(&self, other: P) -> Self {
        let mut segments = self.segments.clone();
        segments.extend(other.into().segments.iter().cloned());
        Self { segments }
    }
}

impl From<Path> for String {
    fn from(path: Path) -> String {
        path.segments.join(SEP)
    }
}

impl From<&Path> for String {
    fn from(path: &Path) -> String {
        path.segments.join(SEP)
    }
}

impl From<String> for Path {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

impl From<&str> for Path {
    fn from(value: &str) -> Self {
        Self {
            segments: value
                .split(SEP)
                .filter(|segment| !segment.is_empty())
                .map(String::from)
                .collect(),
        }
    }
}

impl<'a> From<Cow<'a, str>> for Path {
    fn from(value: Cow<str>) -> Self {
        value.as_ref().into()
    }
}

impl PartialEq<str> for Path {
    fn eq(&self, other: &str) -> bool {
        for (i, segment) in other.split(SEP).enumerate() {
            if self.segments[i] != segment {
                return false;
            }
        }

        true
    }
}

impl PartialEq<&str> for Path {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl PartialEq<String> for Path {
    fn eq(&self, other: &String) -> bool {
        self == other.as_str()
    }
}

impl<'a> PartialEq<Cow<'a, str>> for Path {
    fn eq(&self, other: &Cow<'a, str>) -> bool {
        self == other.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use xpct::{eq_diff, equal, expect};

    use super::*;

    #[test]
    fn string_converts_to_path() {
        expect!("one/two")
            .into::<Path>()
            .to(equal(Path::from("one/two")));

        expect!(String::from("one/two"))
            .into::<Path>()
            .to(equal(Path::from("one/two")));

        expect!(Cow::Borrowed("one/two"))
            .into::<Path>()
            .to(equal(Path::from("one/two")));
    }

    #[test]
    fn path_strips_leading_slash() {
        let path = Path::from("/one/two");
        expect!(path).into::<String>().to(eq_diff("one/two"));
    }

    #[test]
    fn path_strips_trailing_slash() {
        let path = Path::from("one/two/");
        expect!(path).into::<String>().to(eq_diff("one/two"));
    }

    #[test]
    fn path_coalesces_adjacent_slashes() {
        let path = Path::from("one//two");
        expect!(path).into::<String>().to(eq_diff("one/two"));
    }

    #[test]
    fn join_path_segments() {
        let path = Path::from("one").join("two").join("three");
        expect!(path).into::<String>().to(eq_diff("one/two/three"));

        let path = Path::from("one")
            .join(Path::from("two"))
            .join(Path::from("three"));
        expect!(path).into::<String>().to(eq_diff("one/two/three"));
    }

    #[test]
    fn join_multi_segment_paths() {
        let path = Path::from("one/two").join("three/four");
        expect!(String::from(path)).to(eq_diff("one/two/three/four"));

        let path = Path::from("one/two").join(Path::from("three/four"));
        expect!(path)
            .into::<String>()
            .to(eq_diff("one/two/three/four"));
    }

    #[test]
    fn path_equals_equivalent_string() {
        expect!(Path::from("one/two")).to(equal("one/two"));
        expect!(Path::from("one/two")).to(equal(String::from("one/two")));
        expect!(Path::from("one/two")).to(equal(Cow::Borrowed("one/two")));
    }
}
