use std::{
    error::{self, Error},
    fmt::Display,
    io,
    ops::Deref,
};

#[derive(Debug)]
pub enum CheckError {
    IoError(io::Error),
    ParseError(binrw::Error),
    ArgumentError(Box<dyn Error>),
    ShallowError(String),
}

impl Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            CheckError::IoError(ref err) => write!(f, "IO Error: {}", err),
            CheckError::ParseError(ref err) => write!(f, "Parsing Error: {}", err),
            CheckError::ArgumentError(ref err) => write!(f, "Argument Error: {}", err),
            CheckError::ShallowError(ref err) => write!(f, "Argument Error: {}", err),
        }
    }
}

impl error::Error for CheckError {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            CheckError::IoError(ref err) => Some(err),
            CheckError::ParseError(ref err) => Some(err),
            CheckError::ArgumentError(ref err) => Some(err.deref()),
            CheckError::ShallowError(_) => None,
        }
    }
}

impl From<io::Error> for CheckError {
    fn from(value: io::Error) -> Self {
        CheckError::IoError(value)
    }
}

impl From<binrw::Error> for CheckError {
    fn from(value: binrw::Error) -> Self {
        CheckError::ParseError(value)
    }
}

impl<T: Error + 'static> From<shellexpand::LookupError<T>> for CheckError {
    fn from(value: shellexpand::LookupError<T>) -> Self {
        CheckError::ArgumentError(Box::new(value))
    }
}

impl From<glob::PatternError> for CheckError {
    fn from(value: glob::PatternError) -> Self {
        CheckError::ArgumentError(Box::new(value))
    }
}

impl From<glob::GlobError> for CheckError {
    fn from(value: glob::GlobError) -> Self {
        CheckError::ArgumentError(Box::new(value))
    }
}

impl From<zip::result::ZipError> for CheckError {
    fn from(value: zip::result::ZipError) -> Self {
        CheckError::ArgumentError(Box::new(value))
    }
}

impl From<String> for CheckError {
    fn from(value: String) -> Self {
        CheckError::ShallowError(value)
    }
}
