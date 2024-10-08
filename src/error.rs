use core::fmt::{self, Display};

use alloc::boxed::Box;

pub type Result<T> = core::result::Result<T, Error>;

/// Ошибки при создании платежа и парсинге.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Ошибка при парсинге заголовка.
    CorruptedHeader(Box<str>),

    /// Ошибка при декодировании тела.
    DecodingError,

    /// Ошибка при кодировании тела.
    EncodingError,

    /// Обязательные реквизиты не предоставлены.
    RequiredRequisiteNotPresented,

    /// Неизвестная пара реквизитов.
    UnknownPair(Box<str>, Box<str>),

    /// Неизвестный код для кодировки.
    UnknownEncodingCode(u8),

    /// Неизвестный технический код платежа.
    UnknownTechCode(Box<str>),

    /// Неподдерживаемая версия.
    UnsupportedVersion { passed: [u8; 4], current: [u8; 4] },

    /// Неправильный Format ID.
    WrongFormatId([u8; 2]),

    /// Неправильное значение для пары-значения.
    WrongPair(Box<str>, Box<str>),

    /// Неправильный порядок обязательных реквизитов.
    WrongRequiredRequisiteOrder {
        passed: Box<str>,
        expected: Box<str>,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::CorruptedHeader(err) => write!(f, "Ошибка при парсинге заголовка: \"{}\"", err),
            Error::DecodingError => write!(f, "Ошибка при декодировании тела"),
            Error::EncodingError => write!(f, "Ошибка при кодировании тела"),
            Error::RequiredRequisiteNotPresented => {
                write!(f, "Обязательные реквизиты не предоставлены")
            }
            Error::UnknownPair(key, val) => write!(f, "Неизвестный реквизит: {}={}", key, val),
            Error::UnknownEncodingCode(code) => write!(f, "Неизвестный код кодировки {}", code),
            Error::UnknownTechCode(code) => {
                write!(f, "Неизвестный технический код платежа {}", code)
            }
            Error::UnsupportedVersion { passed, current } => write!(
                f,
                "Версия {} не поддерживается, текущая версия {}",
                core::str::from_utf8(passed).unwrap(),
                core::str::from_utf8(current).unwrap(),
            ),
            Error::WrongFormatId(format_id) => write!(
                f,
                "Неправильный Format ID {}{}",
                format_id[0] as char, format_id[1] as char
            ),
            Error::WrongPair(key, val) => write!(f, "Неправильное значение пары {}={}", key, val),
            Error::WrongRequiredRequisiteOrder { passed, expected } => write!(
                f,
                "Неправильный порядок обязательных реквизитов. Ожидалось {} встречено {}",
                expected, passed
            ),
        }
    }
}

impl core::error::Error for Error {}
