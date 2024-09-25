use std::fmt::{self, Display};

pub type Result<T> = std::result::Result<T, Error>;

/// Ошибки при создании платежа и парсинге.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Ошибка при парсинге заголовка.
    CorruptedHeader,

    /// Ошибка при декодировании тела.
    DecodingError,

    /// Обязательные реквизиты не предоставлены.
    RequiredRequisiteNotPresented,

    /// Неизвестная пара реквизитов.
    UnknownPair(String, String),

    /// Неизвестный код для кодировки.
    UnknownEncodingCode(u8),

    /// Неизвестный технический код платежа.
    UnknownTechCode(String),

    /// Неподдерживаемая версия.
    UnsupportedVersion,

    /// Неправильный Format ID.
    WrongFormatId,

    /// Неправильное значение для пары-значения.
    WrongPair(String, String),

    /// Неправильный порядок обязательных реквизитов.
    WrongRequiredRequisiteOrder,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        todo!()
    }
}

impl std::error::Error for Error {}
