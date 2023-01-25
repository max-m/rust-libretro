use crate::VfsSeekPosition;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StringError {
    #[error("{0} is a null pointer")]
    NullPointer(&'static str),

    #[error("invalid UTF-8 sequence")]
    NonUTF8(#[from] std::str::Utf8Error),

    #[error("string contains a null byte")]
    StringContainsNull(#[from] std::ffi::NulError),
}

#[derive(Error, Debug)]
pub enum PerformanceServiceError {
    #[error("Unknown performance counter: “{0}”")]
    UnknownPerformanceCounter(&'static str),

    #[error("Unregistered performance counter: “{0}”")]
    UnregisteredPerformanceCounter(&'static str),
}

#[derive(Error, Debug)]
pub enum LocationServiceError {
    #[error("Failed to start location service")]
    FailedToStart,

    #[error("Failed to get position")]
    FailedToGetPosition,
}

#[derive(Error, Debug)]
pub enum VfsError {
    #[error("failed to open path “{0}”")]
    FailedToOpen(String),

    #[error("failed to close file handle")]
    FailedToClose,

    #[error("failed to get file size")]
    FailedToGetFileSize,

    #[error("VFS interface version {0} < {1}")]
    VersionMismatch(u32, u32),

    #[error("failed to truncate file to {0} bytes")]
    FailedToTruncate(i64),

    #[error("failed to get cursor position")]
    FailedToTell,

    #[error("failed to seek to offset {1} ({0:?})")]
    FailedToSeek(VfsSeekPosition, i64),

    #[error("failed to read {0} bytes from file")]
    FailedToRead(usize),

    #[error("failed to write {0} bytes to file")]
    FailedToWrite(usize),

    #[error("failed to flush file to disk")]
    FailedToFlush,

    #[error("failed to remove path “{0}”")]
    FailedToRemove(String),

    #[error("failed to rename path “{0}” to {1}")]
    FailedToRename(String, String),

    #[error("failed to stat path “{0}” is invalid")]
    StatInvalidPath(String),

    #[error("failed to create path “{0}”")]
    FailedToCreateDirectory(String),

    #[error("unexpected value: “{0}”")]
    UnexpectedValue(String),
}

#[derive(Error, Debug)]
pub enum EnvironmentCallError {
    #[error("invalid string")]
    StringError(#[from] StringError),

    #[error("{0} is a null pointer")]
    NullPointer(&'static str),

    #[error("{0} is a null pointer")]
    NullPointer2(String),

    #[error("callback returned an invalid enum value: {0}")]
    InvalidEnumValue(String),

    #[error("callback returned unknown flags: {1}; Known bits: {0}")]
    UnknownBits(String, String),

    #[error("callback returned `false`")]
    Failure,

    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("failed to parse key-value pair: {0}")]
    KeyValueError(String),

    #[error("Failed to enable {0}")]
    FailedToEnable(&'static str),

    #[error("{0} interface not found, did you call `{1}`?")]
    InterfaceNotFound(&'static str, &'static str),

    #[error(transparent)]
    PerformanceServiceError(#[from] PerformanceServiceError),

    #[error(transparent)]
    LocationServiceError(#[from] LocationServiceError),

    #[error(transparent)]
    VfsError(#[from] VfsError),
}

impl<T> From<crate::sys::InvalidEnumValue<T>> for EnvironmentCallError
where
    T: std::fmt::Display,
{
    fn from(source: crate::sys::InvalidEnumValue<T>) -> Self {
        Self::InvalidEnumValue(source.to_string())
    }
}
