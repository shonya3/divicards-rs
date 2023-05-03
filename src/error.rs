use std::fmt::Display;

use reqwest::Request;
use serde::Serialize;

use crate::CARDS;

#[derive(Debug)]
pub struct InvalidCardNameError(pub String);

// impl std::error::Error for InvalidCardNameError {}
impl Display for InvalidCardNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Card {} not exists. Check CARDS for full list", {
            &self.0
        })
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidCardNameError(InvalidCardNameError),
    HttpError(reqwest::Error),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidCardNameError(err) => err.fmt(f),
            Error::HttpError(err) => err.fmt(f),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Error::HttpError(value)
    }
}

impl From<InvalidCardNameError> for Error {
    fn from(value: InvalidCardNameError) -> Self {
        Error::InvalidCardNameError(value)
    }
}
