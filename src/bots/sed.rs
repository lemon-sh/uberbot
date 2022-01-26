use std::{error::Error, fmt::Display};

use arrayvec::{ArrayString, CapacityError};
use fancy_regex::Regex;
use lazy_static::lazy_static;
use sedregex::find_and_replace;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum SedError {
    Capacity(CapacityError),
    Regex(fancy_regex::Error),
    SedRegex(sedregex::ErrorKind),
}

impl Display for SedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capacity(e) => e.fmt(f),
            Self::Regex(e) => e.fmt(f),
            Self::SedRegex(e) => e.fmt(f),
        }
    }
}

impl Error for SedError {}

impl<T> From<CapacityError<T>> for SedError {
    fn from(e: CapacityError<T>) -> Self {
        Self::Capacity(e.simplify())
    }
}

impl From<fancy_regex::Error> for SedError {
    fn from(e: fancy_regex::Error) -> Self {
        Self::Regex(e)
    }
}

impl From<sedregex::ErrorKind> for SedError {
    fn from(e: sedregex::ErrorKind) -> Self {
        Self::SedRegex(e)
    }
}

type SedResult = Result<Option<ArrayString<512>>, SedError>;

pub fn resolve(prev_msg: &str, cmd: &str) -> SedResult {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^s/.*/.*").unwrap();
    }

    if RE.is_match(cmd)? {
        return if let Some(mat) = RE.find(cmd)? {
            let slice = &cmd[mat.start()..mat.end()];
            let formatted = find_and_replace(prev_msg, [slice])?;
            Ok(Some(ArrayString::from(&formatted)?))
        } else {
            Ok(None)
        };
    }

    Ok(None)
}
