use std::{error, fmt::Display, io, string::FromUtf8Error};

pub type Result<T> = std::result::Result<T, Error>;

/// 系统错误。一般只包含可能引起系统宕机的错误。
#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    BrokenProtocol(String),
    Unsupported(String),
    Encode(String),

    EOF,
    NotReady,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IO(e)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(_: FromUtf8Error) -> Self {
        Error::Encode("invalid utf8 format".into())
    }
}
