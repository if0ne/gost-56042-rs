use std::fmt::{self, Display};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    BufferTooSmall,
    EncodingError,
    RequiredRequisiteNotPresented,
    UnsupportedVersion,
    WrongEncodingCode(u8),
    WrongFormatId,
    WrongPair,
    WrongRequiredREquisite,
    WrongRequiredRequisiteOrder,
    WrongTechCode(String),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        todo!()
    }
}

impl std::error::Error for Error {}
