use encoding::Encoding;

use crate::string_types::{ExactSizeString, MaxSizeString, StringExt};

const FORMAT_ID_BYTES: [u8; 2] = [b'S', b'T'];
const VERSION_0001_BYTES: [u8; 4] = [b'0', b'0', b'0', b'1'];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Payment {
    header: PaymentHeader,
    requisites: Vec<Requisite>,
}

#[derive(Debug)]
pub struct PaymentBuilder {
    payment: Payment,
}

impl PaymentBuilder {
    pub fn with_version(mut self, version: [u8; 4]) -> Self {
        self.payment.header.version = version;
        self
    }

    pub fn with_encdoing(mut self, encdoing: PaymentEncoding) -> Self {
        self.payment.header.encoding = encdoing;
        self
    }

    pub fn with_separator(mut self, separator: char) -> Self {
        assert!(separator.is_ascii());

        self.payment.header.separator = separator as u8;
        self
    }

    pub fn with_additional_requisites(
        mut self,
        requisites: impl IntoIterator<Item = Requisite>,
    ) -> Self {
        let requisites = requisites.into_iter().inspect(|requisite| {
            assert!(!matches!(requisite, Requisite::Name(_)));
            assert!(!matches!(requisite, Requisite::PersonalAcc(_)));
            assert!(!matches!(requisite, Requisite::BankName(_)));
            assert!(!matches!(requisite, Requisite::BIC(_)));
            assert!(!matches!(requisite, Requisite::CorrespAcc(_)));
        });

        self.payment.requisites.extend(requisites);
        self
    }

    pub fn build(mut self, requisites: RequiredRequisite) -> Payment {
        let required_requisites = vec![
            Requisite::Name(requisites.name),
            Requisite::PersonalAcc(requisites.personal_acc),
            Requisite::BankName(requisites.bank_name),
            Requisite::BIC(requisites.bic),
            Requisite::CorrespAcc(requisites.correstp_acc),
        ];

        let requisites = std::mem::take(&mut self.payment.requisites);
        self.payment.requisites = required_requisites.into_iter().chain(requisites).collect();

        self.payment
    }
}

impl Default for PaymentBuilder {
    fn default() -> Self {
        Self {
            payment: Payment {
                header: PaymentHeader {
                    format_id: FORMAT_ID_BYTES,
                    version: VERSION_0001_BYTES,
                    encoding: PaymentEncoding::Utf8,
                    separator: b'|',
                },
                requisites: vec![],
            },
        }
    }
}

impl Payment {
    pub fn builder() -> PaymentBuilder {
        PaymentBuilder::default()
    }

    pub fn parser() -> PaymentParser {
        PaymentParser::default()
    }

    pub fn to_gost_format(&self) -> String {
        let mut buffer = String::with_capacity(308);
        self.write_to(&mut buffer);
        buffer
    }

    pub fn write_to(&self, buffer: &mut String) {
        // Header encoding
        buffer.push(self.header.format_id[0] as char);
        buffer.push(self.header.format_id[1] as char);

        buffer.push(self.header.version[0] as char);
        buffer.push(self.header.version[1] as char);
        buffer.push(self.header.version[2] as char);
        buffer.push(self.header.version[3] as char);

        buffer.push(self.header.encoding.char());

        // Requisites encoding
        for requisite in &self.requisites {
            buffer.push(self.header.separator as char);
            buffer.push_str(requisite.key());
            buffer.push('=');
            buffer.push_str(requisite.value());
        }
    }
}

#[derive(Debug)]
pub struct PaymentParser {
    version_id: [u8; 4],
}

impl PaymentParser {
    pub fn with_version(mut self, version_id: [u8; 4]) -> Self {
        self.version_id = version_id;
        self
    }

    pub fn from_str(&self, val: &str) -> super::Result<Payment> {
        let header = self.read_payment_header(val)?;

        let data = val[8..].to_string();

        let requisites = self.read_requisites(&data, header.separator as char)?;

        self.validate_required_requisites(&requisites)?;

        Ok(Payment { header, requisites })
    }

    pub fn from_bytes(&self, bytes: &[u8]) -> super::Result<Payment> {
        let header = self.read_payment_header_bytes(bytes)?;

        let data = self.decode_payment_body(header.encoding, &bytes[8..])?;

        let requisites = self.read_requisites(&data, header.separator as char)?;

        self.validate_required_requisites(&requisites)?;

        Ok(Payment { header, requisites })
    }
}

impl PaymentParser {
    fn read_payment_header(&self, val: &str) -> super::Result<PaymentHeader> {
        let bytes = val.chars().take(8).map(|c| c as u8).collect::<Vec<_>>();
        let header = self.read_payment_header_bytes(&bytes)?;

        if header.encoding != PaymentEncoding::Utf8 {
            return Err(super::Error::CorruptedHeader);
        }

        Ok(header)
    }

    fn read_payment_header_bytes(&self, bytes: &[u8]) -> super::Result<PaymentHeader> {
        if bytes.len() < 8 {
            return Err(super::Error::CorruptedHeader);
        }

        let format_id = &bytes[0..2];

        if format_id != FORMAT_ID_BYTES {
            return Err(super::Error::WrongFormatId);
        }

        let version = &bytes[2..6];
        if version != self.version_id {
            return Err(super::Error::UnsupportedVersion);
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

    fn decode_payment_body(
        &self,
        encoding: PaymentEncoding,
        bytes: &[u8],
    ) -> super::Result<String> {
        let data = match encoding {
            PaymentEncoding::Win1251 => encoding::all::WINDOWS_1251
                .decode(bytes, encoding::DecoderTrap::Strict)
                .map_err(|_| super::Error::EncodingError)?,
            PaymentEncoding::Utf8 => {
                String::from_utf8(bytes.to_vec()).map_err(|_| super::Error::EncodingError)?
            }
            PaymentEncoding::Koi8R => encoding::all::KOI8_R
                .decode(bytes, encoding::DecoderTrap::Strict)
                .map_err(|_| super::Error::EncodingError)?,
        };

        Ok(data)
    }

    fn read_requisites(&self, data: &str, separator: char) -> super::Result<Vec<Requisite>> {
        let kv = data.split(separator);

        kv.into_iter()
            .map(|kv| kv.split_once('=').ok_or(super::Error::WrongPair))
            .flat_map(|kv| kv.map(|kv| kv.try_into()))
            .collect()
    }

    fn validate_required_requisites(&self, requisites: &[Requisite]) -> super::Result<()> {
        let mut req = requisites.iter().take(5);

        if !matches!(req.next(), Some(Requisite::Name(_))) {
            return Err(super::Error::WrongRequiredRequisiteOrder);
        }

        if !matches!(req.next(), Some(Requisite::PersonalAcc(_))) {
            return Err(super::Error::WrongRequiredRequisiteOrder);
        }

        if !matches!(req.next(), Some(Requisite::BankName(_))) {
            return Err(super::Error::WrongRequiredRequisiteOrder);
        }

        if !matches!(req.next(), Some(Requisite::BIC(_))) {
            return Err(super::Error::WrongRequiredRequisiteOrder);
        }

        if !matches!(req.next(), Some(Requisite::CorrespAcc(_))) {
            return Err(super::Error::WrongRequiredRequisiteOrder);
        }

        Ok(())
    }
}

impl Default for PaymentParser {
    fn default() -> Self {
        Self {
            version_id: VERSION_0001_BYTES,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentHeader {
    format_id: [u8; 2],
    version: [u8; 4],
    encoding: PaymentEncoding,
    separator: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiredRequisite {
    name: MaxSizeString<160>,
    personal_acc: ExactSizeString<20>,
    bank_name: MaxSizeString<45>,
    bic: ExactSizeString<9>,
    correstp_acc: MaxSizeString<20>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Requisite {
    // Required
    Name(MaxSizeString<160>),
    PersonalAcc(ExactSizeString<20>),
    BankName(MaxSizeString<45>),
    BIC(ExactSizeString<9>),
    CorrespAcc(MaxSizeString<20>),

    // Additional
    Sum(MaxSizeString<18>),
    Purpose(MaxSizeString<210>),
    PayeeINN(MaxSizeString<12>),
    PayerINN(MaxSizeString<12>),
    DrawerStatus(MaxSizeString<2>),
    KPP(MaxSizeString<9>),
    CBC(MaxSizeString<20>),
    OKTMO(MaxSizeString<11>),
    PaytReason(MaxSizeString<2>),
    TaxPeriod(MaxSizeString<10>),
    DocNo(MaxSizeString<15>),
    DocDate(MaxSizeString<10>),
    TaxPayKind(MaxSizeString<2>),

    // Other
    LastName(String),
    FirstName(String),
    MiddleName(String),
    PayerAddress(String),
    PersonalAccount(String),
    DocIdx(String),
    PensAcc(String),
    Flat(String),
    Phone(String),
    PayerIdType(String),
    PayerIdNum(String),
    ChildFio(String),
    BirthDate(String),
    PaymTerm(String),
    PaymPeriod(String),
    Category(String),
    ServiceName(String),
    CounterId(String),
    CounterVal(String),
    QuittId(String),
    QuittDate(String),
    InstNum(String),
    ClassNum(String),
    SpecFio(String),
    AddAmount(String),
    RuleId(String),
    ExecId(String),
    RegType(String),
    UIN(String),
    TechCode(TechCode),

    Custom(String, String),
}

impl Requisite {
    pub fn key(&self) -> &str {
        match self {
            Requisite::Name(_) => "Name",
            Requisite::PersonalAcc(_) => "PersonalAcc",
            Requisite::BankName(_) => "BankName",
            Requisite::BIC(_) => "BIC",
            Requisite::CorrespAcc(_) => "CorrespAcc",
            Requisite::Sum(_) => "Sum",
            Requisite::Purpose(_) => "Purpose",
            Requisite::PayeeINN(_) => "PayeeINN",
            Requisite::PayerINN(_) => "PayerINN",
            Requisite::DrawerStatus(_) => "DrawerStatus",
            Requisite::KPP(_) => "KPP",
            Requisite::CBC(_) => "CBC",
            Requisite::OKTMO(_) => "OKTMO",
            Requisite::PaytReason(_) => "PaytReason",
            Requisite::TaxPeriod(_) => "TaxPeriod",
            Requisite::DocNo(_) => "DocNo",
            Requisite::DocDate(_) => "DocDate",
            Requisite::TaxPayKind(_) => "TaxPayKind",
            Requisite::LastName(_) => "LastName",
            Requisite::FirstName(_) => "FirstName",
            Requisite::MiddleName(_) => "MiddleName",
            Requisite::PayerAddress(_) => "PayerAddress",
            Requisite::PersonalAccount(_) => "PersonalAccount",
            Requisite::DocIdx(_) => "DocIdx",
            Requisite::PensAcc(_) => "PensAcc",
            Requisite::Flat(_) => "Flat",
            Requisite::Phone(_) => "Phone",
            Requisite::PayerIdType(_) => "PayerIdType",
            Requisite::PayerIdNum(_) => "PayerIdNum",
            Requisite::ChildFio(_) => "ChildFio",
            Requisite::BirthDate(_) => "BirthDate",
            Requisite::PaymTerm(_) => "PaymTerm",
            Requisite::PaymPeriod(_) => "PaymPeriod",
            Requisite::Category(_) => "Category",
            Requisite::ServiceName(_) => "ServiceName",
            Requisite::CounterId(_) => "CounterId",
            Requisite::CounterVal(_) => "CounterVal",
            Requisite::QuittId(_) => "QuittId",
            Requisite::QuittDate(_) => "QuittDate",
            Requisite::InstNum(_) => "InstNum",
            Requisite::ClassNum(_) => "ClassNum",
            Requisite::SpecFio(_) => "SpecFio",
            Requisite::AddAmount(_) => "AddAmount",
            Requisite::RuleId(_) => "RuleId",
            Requisite::ExecId(_) => "ExecId",
            Requisite::RegType(_) => "RegType",
            Requisite::UIN(_) => "UIN",
            Requisite::TechCode(_) => "TechCode",
            Requisite::Custom(k, _) => k,
        }
    }

    pub fn value(&self) -> &str {
        match self {
            Requisite::Name(v) => v,
            Requisite::PersonalAcc(v) => v,
            Requisite::BankName(v) => v,
            Requisite::BIC(v) => v,
            Requisite::CorrespAcc(v) => v,
            Requisite::Sum(v) => v,
            Requisite::Purpose(v) => v,
            Requisite::PayeeINN(v) => v,
            Requisite::PayerINN(v) => v,
            Requisite::DrawerStatus(v) => v,
            Requisite::KPP(v) => v,
            Requisite::CBC(v) => v,
            Requisite::OKTMO(v) => v,
            Requisite::PaytReason(v) => v,
            Requisite::TaxPeriod(v) => v,
            Requisite::DocNo(v) => v,
            Requisite::DocDate(v) => v,
            Requisite::TaxPayKind(v) => v,
            Requisite::LastName(v) => v,
            Requisite::FirstName(v) => v,
            Requisite::MiddleName(v) => v,
            Requisite::PayerAddress(v) => v,
            Requisite::PersonalAccount(v) => v,
            Requisite::DocIdx(v) => v,
            Requisite::PensAcc(v) => v,
            Requisite::Flat(v) => v,
            Requisite::Phone(v) => v,
            Requisite::PayerIdType(v) => v,
            Requisite::PayerIdNum(v) => v,
            Requisite::ChildFio(v) => v,
            Requisite::BirthDate(v) => v,
            Requisite::PaymTerm(v) => v,
            Requisite::PaymPeriod(v) => v,
            Requisite::Category(v) => v,
            Requisite::ServiceName(v) => v,
            Requisite::CounterId(v) => v,
            Requisite::CounterVal(v) => v,
            Requisite::QuittId(v) => v,
            Requisite::QuittDate(v) => v,
            Requisite::InstNum(v) => v,
            Requisite::ClassNum(v) => v,
            Requisite::SpecFio(v) => v,
            Requisite::AddAmount(v) => v,
            Requisite::RuleId(v) => v,
            Requisite::ExecId(v) => v,
            Requisite::RegType(v) => v,
            Requisite::UIN(v) => v,
            Requisite::TechCode(tech_code) => tech_code.as_str(),
            Requisite::Custom(_, v) => v,
        }
    }
}

impl TryFrom<(&str, &str)> for Requisite {
    type Error = super::Error;

    fn try_from((key, val): (&str, &str)) -> super::Result<Self> {
        let requisite = match key {
            "Name" => Requisite::Name(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "PersonalAcc" => Requisite::PersonalAcc(
                val.to_exact_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "BankName" => Requisite::BankName(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "BIC" => Requisite::BIC(
                val.to_exact_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "CorrespAcc" => Requisite::CorrespAcc(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "Sum" => Requisite::Sum(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "Purpose" => Requisite::Purpose(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "PayeeINN" => Requisite::PayeeINN(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "PayerINN" => Requisite::PayerINN(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "DrawerStatus" => Requisite::DrawerStatus(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "KPP" => Requisite::KPP(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "CBC" => Requisite::CBC(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "OKTMO" => Requisite::OKTMO(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "PaytReason" => Requisite::PaytReason(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "TaxPeriod" => Requisite::TaxPeriod(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "DocNo" => Requisite::DocNo(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "DocDate" => Requisite::DocDate(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "TaxPayKind" => Requisite::TaxPayKind(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.to_string(), val.to_string()))?,
            ),
            "LastName" => Requisite::LastName(val.to_string()),
            "FirstName" => Requisite::FirstName(val.to_string()),
            "MiddleName" => Requisite::MiddleName(val.to_string()),
            "PayerAddress" => Requisite::PayerAddress(val.to_string()),
            "PersonalAccount" => Requisite::PersonalAccount(val.to_string()),
            "DocIdx" => Requisite::DocIdx(val.to_string()),
            "PensAcc" => Requisite::PensAcc(val.to_string()),
            "Flat" => Requisite::Flat(val.to_string()),
            "Phone" => Requisite::Phone(val.to_string()),
            "PayerIdType" => Requisite::PayerIdType(val.to_string()),
            "PayerIdNum" => Requisite::PayerIdNum(val.to_string()),
            "ChildFio" => Requisite::ChildFio(val.to_string()),
            "BirthDate" => Requisite::BirthDate(val.to_string()),
            "PaymTerm" => Requisite::PaymTerm(val.to_string()),
            "PaymPeriod" => Requisite::PaymPeriod(val.to_string()),
            "Category" => Requisite::Category(val.to_string()),
            "ServiceName" => Requisite::ServiceName(val.to_string()),
            "CounterId" => Requisite::CounterId(val.to_string()),
            "CounterVal" => Requisite::CounterVal(val.to_string()),
            "QuittId" => Requisite::QuittId(val.to_string()),
            "QuittDate" => Requisite::QuittDate(val.to_string()),
            "InstNum" => Requisite::InstNum(val.to_string()),
            "ClassNum" => Requisite::ClassNum(val.to_string()),
            "SpecFio" => Requisite::SpecFio(val.to_string()),
            "AddAmount" => Requisite::AddAmount(val.to_string()),
            "RuleId" => Requisite::RuleId(val.to_string()),
            "ExecId" => Requisite::ExecId(val.to_string()),
            "RegType" => Requisite::RegType(val.to_string()),
            "UIN" => Requisite::UIN(val.to_string()),
            "TechCode" => Requisite::TechCode(TechCode::from_str(val)?),
            _ => Requisite::Custom(key.to_string(), val.to_string()),
        };

        Ok(requisite)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TechCode {
    _01,
    _02,
    _03,
    _04,
    _05,
    _06,
    _07,
    _08,
    _09,
    _10,
    _11,
    _12,
    _13,
    _14,
    _15,
}

impl TechCode {
    fn as_str(&self) -> &str {
        match self {
            TechCode::_01 => "01",
            TechCode::_02 => "02",
            TechCode::_03 => "03",
            TechCode::_04 => "04",
            TechCode::_05 => "05",
            TechCode::_06 => "06",
            TechCode::_07 => "07",
            TechCode::_08 => "08",
            TechCode::_09 => "09",
            TechCode::_10 => "10",
            TechCode::_11 => "11",
            TechCode::_12 => "12",
            TechCode::_13 => "13",
            TechCode::_14 => "14",
            TechCode::_15 => "15",
        }
    }

    fn from_str(val: &str) -> super::Result<TechCode> {
        match val {
            "01" => Ok(TechCode::_01),
            "02" => Ok(TechCode::_02),
            "03" => Ok(TechCode::_03),
            "04" => Ok(TechCode::_04),
            "05" => Ok(TechCode::_05),
            "06" => Ok(TechCode::_06),
            "07" => Ok(TechCode::_07),
            "08" => Ok(TechCode::_08),
            "09" => Ok(TechCode::_09),
            "10" => Ok(TechCode::_10),
            "11" => Ok(TechCode::_11),
            "12" => Ok(TechCode::_12),
            "13" => Ok(TechCode::_13),
            "14" => Ok(TechCode::_14),
            "15" => Ok(TechCode::_15),
            _ => Err(super::Error::WrongTechCode(val.to_string())),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PaymentEncoding {
    Win1251 = b'1',
    Utf8 = b'2',
    Koi8R = b'3',
}

impl PaymentEncoding {
    fn char(&self) -> char {
        match self {
            PaymentEncoding::Win1251 => '1',
            PaymentEncoding::Utf8 => '2',
            PaymentEncoding::Koi8R => '3',
        }
    }
}

impl TryFrom<u8> for PaymentEncoding {
    type Error = super::Error;

    fn try_from(value: u8) -> super::Result<PaymentEncoding> {
        match value {
            b'1' => Ok(Self::Win1251),
            b'2' => Ok(Self::Utf8),
            b'3' => Ok(Self::Koi8R),
            code => Err(super::Error::WrongEncodingCode(code)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::string_types::StringExt;

    use super::{Payment, RequiredRequisite};

    #[test]
    fn encoding_test() {
        let payment = Payment::builder().build(RequiredRequisite {
            name: "ООО «Три кита»".to_max_size().unwrap(),
            personal_acc: "40702810138250123017".to_exact_size().unwrap(),
            bank_name: "ОАО \"БАНК\"".to_max_size().unwrap(),
            bic: "044525225".to_exact_size().unwrap(),
            correstp_acc: "30101810400000000225".to_max_size().unwrap(),
        });

        let payment = payment.to_gost_format();

        assert_eq!(payment, "ST00012|Name=ООО «Три кита»|PersonalAcc=40702810138250123017|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225")
    }

    #[test]
    fn decoding_bytes_test() {
        let raw = "ST00012|Name=ООО «Три кита»|PersonalAcc=40702810138250123017|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225".as_bytes();

        let parsed_payment = Payment::parser().from_bytes(raw);

        let payment = Payment::builder().build(RequiredRequisite {
            name: "ООО «Три кита»".to_max_size().unwrap(),
            personal_acc: "40702810138250123017".to_exact_size().unwrap(),
            bank_name: "ОАО \"БАНК\"".to_max_size().unwrap(),
            bic: "044525225".to_exact_size().unwrap(),
            correstp_acc: "30101810400000000225".to_max_size().unwrap(),
        });

        assert_eq!(parsed_payment, Ok(payment));
    }

    #[test]
    fn decoding_string_test() {
        let raw = "ST00012|Name=ООО «Три кита»|PersonalAcc=40702810138250123017|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225";

        let parsed_payment = Payment::parser().from_str(raw);

        let payment = Payment::builder().build(RequiredRequisite {
            name: "ООО «Три кита»".to_max_size().unwrap(),
            personal_acc: "40702810138250123017".to_exact_size().unwrap(),
            bank_name: "ОАО \"БАНК\"".to_max_size().unwrap(),
            bic: "044525225".to_exact_size().unwrap(),
            correstp_acc: "30101810400000000225".to_max_size().unwrap(),
        });

        assert_eq!(parsed_payment, Ok(payment));
    }
}
