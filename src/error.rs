//
// error.rs
// Copyright (C) 2015 Adrian Perez <aperez@igalia.com>
// Distributed under terms of the MIT license.
//

use std::result;
use std::error;
use std::fmt;
use std::io;
use std::string::FromUtf8Error;


#[derive(Clone, PartialEq)]
pub enum ErrorCode {
    InvalidKey,
    UnrepresentableValue,
}


impl fmt::Debug for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::fmt::Debug;
        match *self {
            ErrorCode::InvalidKey => "Invalid key".fmt(f),
            ErrorCode::UnrepresentableValue => "Value cannot be represented".fmt(f),
        }
    }
}


#[derive(Debug)]
pub enum Error {
    SyntaxError(ErrorCode, usize, usize, usize), // Error, offset, line, column
    FromUtf8Error(FromUtf8Error),
    IoError(io::Error),
}


impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::SyntaxError(..) => "syntax error",
            Error::FromUtf8Error(ref error) => error.description(),
            Error::IoError(ref error) => error::Error::description(error),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::SyntaxError(..) => None,
            Error::FromUtf8Error(ref error) => Some(error),
            Error::IoError(ref error) => Some(error),
        }
    }
}


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::SyntaxError(ref code, _, line, column) => {
                write!(f, "{:?} at line {} column {}", code, line, column)
            },
            Error::FromUtf8Error(ref error) => fmt::Display::fmt(error, f),
            Error::IoError(ref error) => fmt::Display::fmt(error, f),
        }
    }
}


impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::IoError(error)
    }
}


impl From<FromUtf8Error> for Error {
    fn from(error: FromUtf8Error) -> Error {
        Error::FromUtf8Error(error)
    }
}


pub type Result<T> = result::Result<T, Error>;

