use std::io;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    DatabaseNotFound,
    DatabaseInvalid,
    DatabaseVersionMismatch,
    ChecksumError,
    // A wrapper around the IO error
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::DatabaseNotFound => write!(f, "No valid database found at the given path."),
            Error::DatabaseInvalid => {
                write!(f,
                       "Invalid database. Invalid is returned when /
                    both meta pages on a database are invalid. This typically occurs when a /
                    file is not a oxygendb database.")
            }
            Error::DatabaseVersionMismatch => {
                write!(f,
                       "Version mismatch. VersionMismatch is /
                    returned when the data file was created with a different version of Oxygen.")
            }
            Error::ChecksumError => {
                write!(f,
                       "Checksum error. Checksum is returned when either /
                    meta page checksum does not match.")
            }
            Error::Io(ref err) => write!(f, "{}", err),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}
