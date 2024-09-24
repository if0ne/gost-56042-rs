use encoding::Encoding;

const FORMAT_ID_BYTES: [u8; 2] = [b'S', b'T'];
const VERSION_0001_BYTES: [u8; 4] = [b'0', b'0', b'0', b'1'];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Payment {
    header: PaymentHeader,
    required: RequiredRequisite,
    additional: Vec<AdditionalRequisite>,
}

impl Payment {
    pub fn new(requisites: RequiredRequisite) -> Self {
        Self::with_splitter('|', requisites)
    }

    pub fn with_splitter(splitter: char, requisite: RequiredRequisite) -> Self {
        assert!(splitter.is_ascii());

        Self {
            header: PaymentHeader {
                format_id: FORMAT_ID_BYTES,
                version: VERSION_0001_BYTES,
                encoding: PaymentEncoding::Utf8,
                separator: splitter as u8,
            },
            required: requisite,
            additional: vec![],
        }
    }

    pub fn add_additional_requisite(&mut self, requisite: AdditionalRequisite) {
        self.additional.push(requisite);
    }

    pub fn extend_additional_requisites(
        &mut self,
        requisites: impl IntoIterator<Item = AdditionalRequisite>,
    ) {
        self.additional.extend(requisites);
    }

    pub fn to_string(&self) -> String {
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

        // Required requisites encoding
        buffer.push(self.header.separator as char);
        buffer.push_str("Name=");
        buffer.push_str(&self.required.name);

        buffer.push(self.header.separator as char);
        buffer.push_str("PersonalAcc=");
        buffer.push_str(&self.required.personal_acc);

        buffer.push(self.header.separator as char);
        buffer.push_str("BankName=");
        buffer.push_str(&self.required.bank_name);

        buffer.push(self.header.separator as char);
        buffer.push_str("BIC=");
        buffer.push_str(&self.required.bic);

        buffer.push(self.header.separator as char);
        buffer.push_str("CorrespAcc=");
        buffer.push_str(&self.required.correstp_acc);

        for additional in &self.additional {
            buffer.push(self.header.separator as char);
            buffer.push_str(additional.key());
            buffer.push('=');
            buffer.push_str(additional.value());
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> super::Result<Self> {
        if bytes.len() < 8 {
            return Err(super::Error::BufferTooSmall);
        }

        let format_id = &bytes[0..2];

        if format_id != FORMAT_ID_BYTES {
            return Err(super::Error::WrongFormatId);
        }

        let version = &bytes[2..6];
        if version != VERSION_0001_BYTES {
            return Err(super::Error::UnsupportedVersion);
        }

        let encoding: PaymentEncoding = bytes[6].try_into()?;
        let separator = bytes[7];

        let bytes = &bytes[8..];

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

        let mut kv = data.split(separator as char);

        let keys = ["Name", "PersonalAcc", "BankName", "BIC", "CorrespAcc"];

        let mut name = String::new();
        let mut personal_acc = String::new();
        let mut bank_name = String::new();
        let mut bic = String::new();
        let mut correstp_acc = String::new();

        for cur_key in keys {
            let Some(kv) = kv.next() else {
                return Err(super::Error::RequiredRequisiteNotPresented);
            };

            let Some((key, val)) = kv.split_once('=') else {
                return Err(super::Error::WrongPair);
            };

            if cur_key == key {
                match cur_key {
                    "Name" => name = val.to_string(),
                    "PersonalAcc" => personal_acc = val.to_string(),
                    "BankName" => bank_name = val.to_string(),
                    "BIC" => bic = val.to_string(),
                    "CorrespAcc" => correstp_acc = val.to_string(),
                    _ => return Err(super::Error::WrongRequiredREquisite),
                }
            } else {
                return Err(super::Error::WrongRequiredRequisiteOrder);
            }
        }

        let mut additional = vec![];

        for kv in kv {
            let Some(additional_pair) = kv.split_once('=') else {
                continue;
            };

            additional.push(additional_pair.into());
        }

        Ok(Payment {
            header: PaymentHeader {
                format_id: FORMAT_ID_BYTES,
                version: VERSION_0001_BYTES,
                encoding,
                separator,
            },
            required: RequiredRequisite {
                name,
                personal_acc,
                bank_name,
                bic,
                correstp_acc,
            },
            additional,
        })
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
    name: String,
    personal_acc: String,
    bank_name: String,
    bic: String,
    correstp_acc: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdditionalRequisite {
    Some,
}

impl AdditionalRequisite {
    pub fn key(&self) -> &str {
        "todo"
    }

    pub fn value(&self) -> &str {
        "todo"
    }
}

impl From<(&str, &str)> for AdditionalRequisite {
    fn from(value: (&str, &str)) -> Self {
        AdditionalRequisite::Some
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
    use super::{Payment, RequiredRequisite};

    #[test]
    fn encoding_test() {
        let payment = Payment::new(RequiredRequisite {
            name: "ООО «Три кита»".to_string(),
            personal_acc: "40702810138250123017".to_string(),
            bank_name: "ОАО \"БАНК\"".to_string(),
            bic: "044525225".to_string(),
            correstp_acc: "30101810400000000225".to_string(),
        });

        let payment = payment.to_string();

        assert_eq!(payment, "ST00012|Name=ООО «Три кита»|PersonalAcc=40702810138250123017|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225")
    }

    #[test]
    fn decoding_test() {
        let raw = "ST00012|Name=ООО «Три кита»|PersonalAcc=40702810138250123017|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225".as_bytes();

        let parsed_payment = Payment::from_bytes(raw);

        let payment = Payment::new(RequiredRequisite {
            name: "ООО «Три кита»".to_string(),
            personal_acc: "40702810138250123017".to_string(),
            bank_name: "ОАО \"БАНК\"".to_string(),
            bic: "044525225".to_string(),
            correstp_acc: "30101810400000000225".to_string(),
        });

        assert_eq!(parsed_payment, Ok(payment));
    }
}
