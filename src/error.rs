use std::fmt::Formatter;
use std::{fmt, io};

use openssl::error::ErrorStack;

use crate::tls::error::FnError;

#[derive(Debug, Clone)]
pub enum Error {
    /// Returned if a concrete function from the module [`tls`] fails or term evaluation fails
    Fn(FnError),
    /// OpenSSL reported an error
    OpenSSL(ErrorStack),
    /// There was an unexpected IO error. Should never happen because we are not fuzzing on a network which can fail.
    IO(String),
    /// Some error which was caused because of agents or their names. Like an agent which was not found.
    Agent(String),
    /// Error while operating on a [`Stream`]
    Stream(String),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::Fn(err) => write!(
                f,
                "error while evaluating a term or executing a function symbol: {}",
                err
            ),
            Error::OpenSSL(err) => write!(f, "error in openssl: {}", err),
            Error::IO(err) => write!(
                f,
                "error in io of openssl (this should not happen): {}",
                err
            ),
            Error::Agent(err) => write!(f, "error regarding an agent: {}", err),
            Error::Stream(err) => write!(f, "error in the stream: {}", err),
        }
    }
}

impl From<openssl::error::ErrorStack> for Error {
    fn from(err: ErrorStack) -> Self {
        Error::OpenSSL(err)
    }
}

impl From<FnError> for Error {
    fn from(err: FnError) -> Self {
        Error::Fn(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IO(err.to_string())
    }
}