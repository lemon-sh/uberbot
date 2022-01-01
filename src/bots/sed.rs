use std::{fmt::Display, error::Error};

use arrayvec::{ArrayString, CapacityError};
use fancy_regex::Regex;
use lazy_static::lazy_static;
use sedregex::find_and_replace;

#[derive(Debug)]
pub enum SedErrorKind {
    Capacity(CapacityError),
    Regex(fancy_regex::Error),
    SedRegex(sedregex::ErrorKind)
}

#[derive(Debug)]
pub struct SedCapacityError(SedErrorKind);

impl Display for SedCapacityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // yeah it's ugly but there's no better way afaik
        match &self.0 {
            SedErrorKind::Capacity(e) => Display::fmt(e, f),
            SedErrorKind::Regex(e) => Display::fmt(e, f),
            SedErrorKind::SedRegex(e) => Display::fmt(e, f),
        }
    }
}

impl Error for SedCapacityError {}

impl<T> From<CapacityError<T>> for SedCapacityError {
    fn from(e: CapacityError<T>) -> Self {
        Self { 0: SedErrorKind::Capacity(e.simplify()) }
    }
}

impl From<fancy_regex::Error> for SedCapacityError {
    fn from(e: fancy_regex::Error) -> Self {
        Self { 0: SedErrorKind::Regex(e) }
    }
}

impl From<sedregex::ErrorKind> for SedCapacityError {
    fn from(e: sedregex::ErrorKind) -> Self {
        Self { 0: SedErrorKind::SedRegex(e) }
    }
}

type SedResult = Result<Option<ArrayString<512>>, SedCapacityError>;

pub fn resolve(prev_msg: &str, cmd: &str) -> SedResult {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^s/.*/.*").unwrap(); // yes this regex is valid, don't worry about it
    }

    if RE.is_match(cmd)? {
        if let Some(mat) = RE.find(cmd)? {
            let slice = &cmd[mat.start()..mat.end()];
            
            let formatted = find_and_replace(&prev_msg, [slice])?;

            return Ok(Some(ArrayString::from(&formatted)?));
        } else {
            return Ok(None);
        }
    }

    Ok(None)
}