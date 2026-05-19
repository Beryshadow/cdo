use std::error;
use std::fmt;
use std::io;
use std::num::ParseIntError;
use std::process;
#[derive(Debug)]
pub enum LocalError {
    IoErr(io::Error),
    ExitErr(process::ExitStatusError),
    Parse(ParseIntError),
    Custom(StringWrapper),
}

#[derive(Debug)]
pub struct StringWrapper(String);

impl fmt::Display for StringWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StringWrapper {}

impl fmt::Display for LocalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LocalError::IoErr(e) => write!(f, "{e}"),
            LocalError::ExitErr(e) => {
                write!(f, "{e}")
            }
            LocalError::Parse(e) => {
                write!(f, "{e}")
            }
            LocalError::Custom(e) => {
                write!(f, "{e}")
            }
        }
    }
}

impl error::Error for LocalError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            LocalError::IoErr(e) => Some(e),
            LocalError::ExitErr(e) => Some(e),
            LocalError::Parse(e) => Some(e),
            LocalError::Custom(e) => Some(e),
        }
    }
}

// Implement the conversion from `ExitStatusError` to `local_error`.
impl From<process::ExitStatusError> for LocalError {
    fn from(err: process::ExitStatusError) -> LocalError {
        LocalError::ExitErr(err)
    }
}

// Implement the conversion from `IoError` to `local_error`.
impl From<io::Error> for LocalError {
    fn from(err: io::Error) -> LocalError {
        LocalError::IoErr(err)
    }
}

// Implement the conversion from `ParseIntError` to `local_error`.
impl From<ParseIntError> for LocalError {
    fn from(err: ParseIntError) -> LocalError {
        LocalError::Parse(err)
    }
}

impl From<String> for LocalError {
    fn from(string: String) -> LocalError {
        LocalError::Custom(StringWrapper(string))
    }
}
