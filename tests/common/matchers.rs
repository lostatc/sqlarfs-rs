use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use sqlarfs::{FileMetadata, FileMode};
use xpct::core::{DispatchFormat, MatchOutcome, Matcher, TransformMatch};
use xpct::format::diff::DiffStyle;
use xpct::format::{DiffFormat, MessageFormat, MismatchFormat};
use xpct::matchers::diff::{DiffSegment, Diffable};
use xpct::matchers::Mismatch;
use xpct::{all, be_err, be_some, each, equal, why};

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

struct HaveSameContentsMatcher<Expected, Actual> {
    expected: Expected,
    marker: PhantomData<Actual>,
}

impl<Expected, Actual> TransformMatch for HaveSameContentsMatcher<Expected, Actual>
where
    Actual: AsRef<Path> + std::fmt::Debug,
    Expected: AsRef<Path> + std::fmt::Debug,
{
    type In = Actual;
    type PosOut = Actual;
    type NegOut = Actual;
    type PosFail = Vec<DiffSegment>;
    type NegFail = ();

    fn match_pos(
        self,
        actual: Self::In,
    ) -> xpct::Result<MatchOutcome<Self::PosOut, Self::PosFail>> {
        let actual_contents = fs::read_to_string(actual.as_ref())?;
        let expected_contents = fs::read_to_string(self.expected.as_ref())?;

        if actual_contents == expected_contents {
            Ok(MatchOutcome::Success(actual))
        } else {
            let diff = actual_contents.diff(expected_contents);
            Ok(MatchOutcome::Fail(diff))
        }
    }

    fn match_neg(
        self,
        actual: Self::In,
    ) -> xpct::Result<MatchOutcome<Self::NegOut, Self::NegFail>> {
        let actual_contents = fs::read_to_string(actual.as_ref())?;
        let expected_contents = fs::read_to_string(self.expected.as_ref())?;

        if actual_contents != expected_contents {
            Ok(MatchOutcome::Success(actual))
        } else {
            Ok(MatchOutcome::Fail(()))
        }
    }
}

pub fn have_same_contents<'a, Actual, Expected>(expected: Expected) -> Matcher<'a, Actual, Actual>
where
    Actual: std::fmt::Debug + AsRef<Path> + 'a,
    Expected: std::fmt::Debug + AsRef<Path> + 'a,
{
    let matcher = HaveSameContentsMatcher {
        expected,
        marker: PhantomData,
    };

    Matcher::transform(
        matcher,
        DispatchFormat::new(
            DiffFormat::<String, String>::new(DiffStyle::provided()),
            MessageFormat::new("", "Expected these to have different contents"),
        ),
    )
}

struct HaveSamePermissionsMatcher<Expected, Actual> {
    expected: Expected,
    marker: PhantomData<Actual>,
}

impl<Expected, Actual> TransformMatch for HaveSamePermissionsMatcher<Expected, Actual>
where
    Actual: AsRef<Path> + std::fmt::Debug,
    Expected: AsRef<Path> + std::fmt::Debug,
{
    type In = Actual;
    type PosOut = Actual;
    type NegOut = Actual;
    type PosFail = Mismatch<fs::Permissions, fs::Permissions>;
    type NegFail = ();

    fn match_pos(
        self,
        actual: Self::In,
    ) -> xpct::Result<MatchOutcome<Self::PosOut, Self::PosFail>> {
        let actual_permissions = fs::symlink_metadata(actual.as_ref())?.permissions();
        let expected_permissions = fs::symlink_metadata(self.expected.as_ref())?.permissions();

        if actual_permissions == expected_permissions {
            Ok(MatchOutcome::Success(actual))
        } else {
            Ok(MatchOutcome::Fail(Mismatch {
                actual: actual_permissions,
                expected: expected_permissions,
            }))
        }
    }

    fn match_neg(
        self,
        actual: Self::In,
    ) -> xpct::Result<MatchOutcome<Self::NegOut, Self::NegFail>> {
        let actual_permissions = fs::symlink_metadata(actual.as_ref())?.permissions();
        let expected_permissions = fs::symlink_metadata(self.expected.as_ref())?.permissions();

        if actual_permissions != expected_permissions {
            Ok(MatchOutcome::Success(actual))
        } else {
            Ok(MatchOutcome::Fail(()))
        }
    }
}

pub fn have_same_permissions<'a, Actual, Expected>(
    expected: Expected,
) -> Matcher<'a, Actual, Actual>
where
    Actual: std::fmt::Debug + AsRef<Path> + 'a,
    Expected: std::fmt::Debug + AsRef<Path> + 'a,
{
    let matcher = HaveSamePermissionsMatcher {
        expected,
        marker: PhantomData,
    };

    Matcher::transform(
        matcher,
        DispatchFormat::new(
            MismatchFormat::new("to be the same permissions as", ""),
            MessageFormat::new("", "Expected these to have different permissions"),
        ),
    )
}

struct HaveSameMtimeMatcher<Expected, Actual> {
    expected: Expected,
    marker: PhantomData<Actual>,
}

impl<Expected, Actual> TransformMatch for HaveSameMtimeMatcher<Expected, Actual>
where
    Actual: AsRef<Path> + std::fmt::Debug,
    Expected: AsRef<Path> + std::fmt::Debug,
{
    type In = Actual;
    type PosOut = Actual;
    type NegOut = Actual;
    type PosFail = Mismatch<SystemTime, SystemTime>;
    type NegFail = ();

    fn match_pos(
        self,
        actual: Self::In,
    ) -> xpct::Result<MatchOutcome<Self::PosOut, Self::PosFail>> {
        let actual_mtime = fs::symlink_metadata(actual.as_ref())?.modified()?;
        let expected_mtime = fs::symlink_metadata(self.expected.as_ref())?.modified()?;

        if actual_mtime == expected_mtime {
            Ok(MatchOutcome::Success(actual))
        } else {
            Ok(MatchOutcome::Fail(Mismatch {
                actual: actual_mtime,
                expected: expected_mtime,
            }))
        }
    }

    fn match_neg(
        self,
        actual: Self::In,
    ) -> xpct::Result<MatchOutcome<Self::NegOut, Self::NegFail>> {
        let actual_mtime = fs::symlink_metadata(actual.as_ref())?.modified()?;
        let expected_mtime = fs::symlink_metadata(self.expected.as_ref())?.modified()?;

        if actual_mtime != expected_mtime {
            Ok(MatchOutcome::Success(actual))
        } else {
            Ok(MatchOutcome::Fail(()))
        }
    }
}

pub fn have_same_mtime<'a, Actual, Expected>(expected: Expected) -> Matcher<'a, Actual, Actual>
where
    Actual: std::fmt::Debug + AsRef<Path> + 'a,
    Expected: std::fmt::Debug + AsRef<Path> + 'a,
{
    let matcher = HaveSamePermissionsMatcher {
        expected,
        marker: PhantomData,
    };

    Matcher::transform(
        matcher,
        DispatchFormat::new(
            MismatchFormat::new("to be the same mtime as", ""),
            MessageFormat::new("", "Expected these to have different mtimes"),
        ),
    )
}

struct HaveSameSymlinkTargetMatcher<Expected, Actual> {
    expected: Expected,
    marker: PhantomData<Actual>,
}

impl<Expected, Actual> TransformMatch for HaveSameSymlinkTargetMatcher<Expected, Actual>
where
    Actual: AsRef<Path> + std::fmt::Debug,
    Expected: AsRef<Path> + std::fmt::Debug,
{
    type In = Actual;
    type PosOut = Actual;
    type NegOut = Actual;
    type PosFail = Mismatch<PathBuf, PathBuf>;
    type NegFail = ();

    fn match_pos(
        self,
        actual: Self::In,
    ) -> xpct::Result<MatchOutcome<Self::PosOut, Self::PosFail>> {
        let actual_target = fs::read_link(actual.as_ref())?;
        let expected_target = fs::read_link(self.expected.as_ref())?;

        if actual_target == expected_target {
            Ok(MatchOutcome::Success(actual))
        } else {
            Ok(MatchOutcome::Fail(Mismatch {
                actual: actual_target,
                expected: expected_target,
            }))
        }
    }

    fn match_neg(
        self,
        actual: Self::In,
    ) -> xpct::Result<MatchOutcome<Self::NegOut, Self::NegFail>> {
        let actual_target = fs::read_link(actual.as_ref())?;
        let expected_target = fs::read_link(self.expected.as_ref())?;

        if actual_target != expected_target {
            Ok(MatchOutcome::Success(actual))
        } else {
            Ok(MatchOutcome::Fail(()))
        }
    }
}

pub fn have_same_symlink_target<'a, Actual, Expected>(
    expected: Expected,
) -> Matcher<'a, Actual, Actual>
where
    Actual: std::fmt::Debug + AsRef<Path> + 'a,
    Expected: std::fmt::Debug + AsRef<Path> + 'a,
{
    let matcher = HaveSameSymlinkTargetMatcher {
        expected,
        marker: PhantomData,
    };

    Matcher::transform(
        matcher,
        DispatchFormat::new(
            MismatchFormat::new("to be the same symlink target as", ""),
            MessageFormat::new("", "Expected these to have different symlink targets"),
        ),
    )
}
