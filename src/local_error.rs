use std::error;
use std::fmt;
use std::io;
use std::process;
#[derive(Debug)]
pub enum LocalError {
    IoErr(io::Error),
    ExitErr(process::ExitStatusError),
}
impl fmt::Display for LocalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LocalError::IoErr(e) => write!(f, "please use a vector with at least one element"),
            // The wrapped error contains additional information and is available
            // via the source() method.
            LocalError::ExitErr(e) => write!(f, "the provided string could not be parsed as int"),
        }
    }
}

impl error::Error for LocalError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            // match *self {
            LocalError::IoErr(e) => None,
            // The cause is the underlying implementation error type. Is implicitly
            // cast to the trait object `&error::Error`. This works because the
            // underlying type already implements the `Error` trait.
            LocalError::ExitErr(e) => None,
            // LocalError::ExitErr(e) => Some(e),
        }
    }
}

// Implement the conversion from `ExitStatusError` to `local_error`.
// This will be automatically called by `?` if a `ParseIntError`
// needs to be converted into a `local_error`.
impl From<process::ExitStatusError> for LocalError {
    fn from(err: process::ExitStatusError) -> LocalError {
        LocalError::ExitErr(err)
    }
}

// Implement the conversion from `IoError` to `local_error`.
// This will be automatically called by `?` if a `ParseIntError`
// needs to be converted into a `local_error`.
impl From<io::Error> for LocalError {
    fn from(err: io::Error) -> LocalError {
        LocalError::IoErr(err)
    }
}
