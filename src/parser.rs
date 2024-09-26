use core::marker::PhantomData;

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use encoding::Encoding;

use crate::{
    CustomRequisites, NoCustomRequisites, Payment, PaymentEncoding, PaymentHeader, Requisite,
    FORMAT_ID_BYTES, VERSION_0001_BYTES,
};

/// Интерфейс для парсеров.
pub trait ParserStrategy<T: CustomRequisites> {
    /// Преобразовать из строки.
    ///
    /// Предполагается, что тело находится в Utf-8 формате.
    fn parse_from_str(&self, val: &str) -> super::Result<Payment<T>>;

    /// Преобразование из байтов.
    fn parse_from_bytes(&self, bytes: &[u8]) -> super::Result<Payment<T>>;
}

/// Парсер из строки в структуру с информацией о платеже.
#[derive(Debug)]
pub struct PaymentParser<
    T: ParserStrategyType = StrictParser,
    RT: CustomRequisites = NoCustomRequisites,
> {
    version_id: [u8; 4],
    _req_marker: PhantomData<RT>,
    _marker: PhantomData<T>,
}

impl<T: ParserStrategyType, RT: CustomRequisites> PaymentParser<T, RT> {
    /// Установка версии.
    pub fn with_version(mut self, version_id: [u8; 4]) -> Self {
        self.version_id = version_id;
        self
    }
}

impl<RT: CustomRequisites> ParserStrategy<RT> for PaymentParser<StrictParser, RT> {
    fn parse_from_str(&self, val: &str) -> crate::Result<Payment<RT>> {
        let header = self.read_payment_header(val, true)?;

        let data = val[8..].to_string();

        let requisites = self.read_requisites(&data, header.separator as char)?;

        self.validate_required_requisites(&requisites)?;

        Ok(Payment { header, requisites })
    }

    fn parse_from_bytes(&self, bytes: &[u8]) -> crate::Result<Payment<RT>> {
        let header = self.read_payment_header_bytes(bytes)?;

        let data = self.decode_payment_body(
            header.encoding,
            &bytes[8..],
            encoding::DecoderTrap::Strict,
            |val| String::from_utf8(val.to_vec()).map_err(|_| super::Error::DecodingError),
        )?;

        let requisites = self.read_requisites(&data, header.separator as char)?;

        self.validate_required_requisites(&requisites)?;

        Ok(Payment { header, requisites })
    }
}

impl<RT: CustomRequisites> PaymentParser<StrictParser, RT> {
    fn read_requisites(&self, data: &str, separator: char) -> super::Result<Vec<Requisite<RT>>> {
        let kv = data.split(separator);

        kv.into_iter()
            .map(|kv| kv.split_once('=').ok_or(super::Error::WrongPair))
            .flat_map(|kv| kv.map(|kv| kv.try_into()))
            .collect()
    }
}

impl<RT: CustomRequisites> ParserStrategy<RT> for PaymentParser<RequisiteToleranceParser, RT> {
    fn parse_from_str(&self, val: &str) -> crate::Result<Payment<RT>> {
        let header = self.read_payment_header(val, true)?;

        let data = val[8..].to_string();

        let requisites = self.read_requisites(&data, header.separator as char);

        self.validate_required_requisites(&requisites)?;

        Ok(Payment { header, requisites })
    }

    fn parse_from_bytes(&self, bytes: &[u8]) -> crate::Result<Payment<RT>> {
        let header = self.read_payment_header_bytes(bytes)?;

        let data = self.decode_payment_body(
            header.encoding,
            &bytes[8..],
            encoding::DecoderTrap::Strict,
            |val| String::from_utf8(val.to_vec()).map_err(|_| super::Error::DecodingError),
        )?;

        let requisites = self.read_requisites(&data, header.separator as char);

        self.validate_required_requisites(&requisites)?;

        Ok(Payment { header, requisites })
    }
}

impl<RT: CustomRequisites> PaymentParser<RequisiteToleranceParser, RT> {
    fn read_requisites(&self, data: &str, separator: char) -> Vec<Requisite<RT>> {
        let kv = data.split(separator);

        kv.into_iter()
            .flat_map(|kv| kv.split_once('='))
            .flat_map(|kv| kv.try_into())
            .collect()
    }
}

impl<RT: CustomRequisites> ParserStrategy<RT> for PaymentParser<LooseParser, RT> {
    fn parse_from_str(&self, val: &str) -> crate::Result<Payment<RT>> {
        let header = self.read_payment_header(val, false)?;

        let data = val[8..].to_string();

        let requisites = self.read_requisites(&data, header.separator as char);

        Ok(Payment { header, requisites })
    }

    fn parse_from_bytes(&self, bytes: &[u8]) -> crate::Result<Payment<RT>> {
        let header = self.read_payment_header_bytes(bytes)?;

        let data = self.decode_payment_body(
            header.encoding,
            &bytes[8..],
            encoding::DecoderTrap::Replace,
            |val| Ok(String::from_utf8_lossy(val).to_string()),
        )?;

        let requisites = self.read_requisites(&data, header.separator as char);

        Ok(Payment { header, requisites })
    }
}

impl<RT: CustomRequisites> PaymentParser<LooseParser, RT> {
    fn read_requisites(&self, data: &str, separator: char) -> Vec<Requisite<RT>> {
        let kv = data.split(separator);

        kv.into_iter()
            .flat_map(|kv| kv.split_once('='))
            .flat_map(|kv| kv.try_into())
            .collect()
    }
}

impl<T: ParserStrategyType, RT: CustomRequisites> PaymentParser<T, RT> {
    fn read_payment_header_bytes(&self, bytes: &[u8]) -> super::Result<PaymentHeader> {
        if bytes.len() < 8 {
            return Err(super::Error::CorruptedHeader(
                "Не возможно сформировать заголовок, так как длина меньше 8".into(),
            ));
        }

        let format_id = &bytes[0..2];

        if format_id != FORMAT_ID_BYTES {
            return Err(super::Error::WrongFormatId([format_id[0], format_id[1]]));
        }

        let version = &bytes[2..6];
        if version != self.version_id {
            return Err(super::Error::UnsupportedVersion {
                passed: [version[0], version[1], version[2], version[3]],
                current: self.version_id,
            });
        }

        let encoding: PaymentEncoding = bytes[6].try_into()?;
        let separator = bytes[7];

        Ok(PaymentHeader {
            format_id: FORMAT_ID_BYTES,
            version: self.version_id,
            encoding,
            separator,
        })
    }

    fn read_payment_header(&self, val: &str, check_encoding: bool) -> super::Result<PaymentHeader> {
        let bytes = val.chars().take(8).map(|c| c as u8).collect::<Vec<_>>();
        let header = self.read_payment_header_bytes(&bytes)?;

        if check_encoding && header.encoding != PaymentEncoding::Utf8 {
            return Err(super::Error::CorruptedHeader(
                format!(
                    "Не верная кодировка, должна быть Utf-8, установлена {}",
                    header.encoding
                )
                .into(),
            ));
        }

        Ok(header)
    }

    fn decode_payment_body(
        &self,
        encoding: PaymentEncoding,
        bytes: &[u8],
        trap: encoding::DecoderTrap,
        utf8_decode: fn(&[u8]) -> super::Result<String>,
    ) -> super::Result<String> {
        let data = match encoding {
            PaymentEncoding::Win1251 => encoding::all::WINDOWS_1251
                .decode(bytes, trap)
                .map_err(|_| super::Error::DecodingError)?,
            PaymentEncoding::Utf8 => utf8_decode(bytes)?,
            PaymentEncoding::Koi8R => encoding::all::KOI8_R
                .decode(bytes, trap)
                .map_err(|_| super::Error::DecodingError)?,
        };

        Ok(data)
    }

    fn validate_required_requisites(&self, requisites: &[Requisite<RT>]) -> super::Result<()> {
        let mut req = requisites.iter().take(5);

        let next = req.next();
        if !matches!(next, Some(Requisite::Name(_))) {
            return Err(super::Error::WrongRequiredRequisiteOrder {
                passed: next.map(|r| r.key()).unwrap_or("Пусто").into(),
                expected: "Name".into(),
            });
        }

        let next = req.next();
        if !matches!(next, Some(Requisite::PersonalAcc(_))) {
            return Err(super::Error::WrongRequiredRequisiteOrder {
                passed: next.map(|r| r.key()).unwrap_or("Пусто").into(),
                expected: "PersonalAcc".into(),
            });
        }

        let next = req.next();
        if !matches!(next, Some(Requisite::BankName(_))) {
            return Err(super::Error::WrongRequiredRequisiteOrder {
                passed: next.map(|r| r.key()).unwrap_or("Пусто").into(),
                expected: "BankName".into(),
            });
        }

        let next = req.next();
        if !matches!(next, Some(Requisite::BIC(_))) {
            return Err(super::Error::WrongRequiredRequisiteOrder {
                passed: next.map(|r| r.key()).unwrap_or("Пусто").into(),
                expected: "BIC".into(),
            });
        }

        let next = req.next();
        if !matches!(next, Some(Requisite::CorrespAcc(_))) {
            return Err(super::Error::WrongRequiredRequisiteOrder {
                passed: next.map(|r| r.key()).unwrap_or("Пусто").into(),
                expected: "CorrespAcc".into(),
            });
        }

        Ok(())
    }
}

impl<T: ParserStrategyType, RT: CustomRequisites> Default for PaymentParser<T, RT> {
    fn default() -> Self {
        Self {
            version_id: VERSION_0001_BYTES,
            _req_marker: PhantomData,
            _marker: PhantomData,
        }
    }
}

pub trait ParserStrategyType {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrictParser;
impl ParserStrategyType for StrictParser {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequisiteToleranceParser;
impl ParserStrategyType for RequisiteToleranceParser {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LooseParser;
impl ParserStrategyType for LooseParser {}
