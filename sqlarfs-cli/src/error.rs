use std::fmt;

use eyre::eyre;

#[derive(Debug)]
pub struct UserError(pub String);

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<UserError> for eyre::Report {
    fn from(error: UserError) -> Self {
        eyre!("{}", error)
    }
}

macro_rules! user_err {
    ($($args:tt)*) => {
        eyre::eyre!($crate::error::UserError(format!($($args)*)))
    };
}

pub(crate) use user_err;
