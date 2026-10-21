use std::error::Error;
use std::io;

#[inline]
pub(super) fn invalid_data_error(error: impl Into<Box<dyn Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

#[inline]
pub(super) fn invalid_input_error(error: impl Into<Box<dyn Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}
