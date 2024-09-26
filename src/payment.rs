use core::{fmt::Display, marker::PhantomData};

use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};
use encoding::Encoding;

use super::{
    string_types::{ExactSizeString, MaxSizeString, StringExt},
    CustomRequisites, NoCustomRequisites,
};

const FORMAT_ID_BYTES: [u8; 2] = [b'S', b'T'];
const VERSION_0001_BYTES: [u8; 4] = [b'0', b'0', b'0', b'1'];

/// Информация о платеже.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Payment<T: CustomRequisites = NoCustomRequisites> {
    header: PaymentHeader,
    requisites: Vec<Requisite<T>>,
}

#[derive(Debug)]
pub struct PaymentBuilder<T: CustomRequisites = NoCustomRequisites> {
    payment: Payment<T>,
}

impl<T: CustomRequisites> PaymentBuilder<T> {
    /// Установка версии.
    pub fn with_version(mut self, version: [u8; 4]) -> Self {
        self.payment.header.version = version;
        self
    }

    /// Установка кодировки.
    pub fn with_encdoing(mut self, encdoing: PaymentEncoding) -> Self {
        self.payment.header.encoding = encdoing;
        self
    }

    /// Установка разделителя.
    pub fn with_separator(mut self, separator: char) -> Self {
        assert!(separator.is_ascii());

        self.payment.header.separator = separator as u8;
        self
    }

    /// Добавление дополнительных реквизитов.
    pub fn with_additional_requisites(
        mut self,
        requisites: impl IntoIterator<Item = Requisite<T>>,
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

    /// Получение структуры с информацией о платеже.
    pub fn build(self) -> Payment<T> {
        self.payment
    }
}

impl<T: CustomRequisites> Default for PaymentBuilder<T> {
    fn default() -> Self {
        Self {
            payment: Payment {
                header: PaymentHeader {
                    format_id: FORMAT_ID_BYTES,
                    version: VERSION_0001_BYTES,
                    encoding: PaymentEncoding::Utf8,
                    separator: b'|',
                },
                requisites: Vec::with_capacity(16),
            },
        }
    }
}

impl Payment {
    /// Строитель модели платежей.
    pub fn builder(requisites: RequiredRequisite) -> PaymentBuilder {
        let mut builder = PaymentBuilder::default();

        let required_requisites = alloc::vec![
            Requisite::Name(requisites.name),
            Requisite::PersonalAcc(requisites.personal_acc),
            Requisite::BankName(requisites.bank_name),
            Requisite::BIC(requisites.bic),
            Requisite::CorrespAcc(requisites.correstp_acc),
        ];

        builder.payment.requisites = required_requisites;

        builder
    }

    /// Парсер.
    pub fn parser() -> PaymentParser {
        PaymentParser::default()
    }
}

impl<T: CustomRequisites> Payment<T> {
    /// Строитель модели платежей с пользовательскими реквизитами.
    pub fn custom_builder(requisites: RequiredRequisite) -> PaymentBuilder<T> {
        let mut builder = PaymentBuilder::<T>::default();

        let required_requisites = alloc::vec![
            Requisite::Name(requisites.name),
            Requisite::PersonalAcc(requisites.personal_acc),
            Requisite::BankName(requisites.bank_name),
            Requisite::BIC(requisites.bic),
            Requisite::CorrespAcc(requisites.correstp_acc),
        ];

        builder.payment.requisites = required_requisites;

        builder
    }

    /// Парсер с пользовательскими реквизитами.
    pub fn custom_parser() -> PaymentParser<T> {
        PaymentParser::default()
    }

    /// Преобразования структуры в массив байтов согласно ГОСТ-56042.
    pub fn to_bytes(&self) -> super::Result<Vec<u8>> {
        let mut buffer = Vec::with_capacity(308);
        self.write_to(&mut buffer)?;
        Ok(buffer)
    }

    /// Заполнение буфера информацией о платеже согласно ГОСТ-56042.
    pub fn write_to(&self, buffer: &mut Vec<u8>) -> super::Result<()> {
        // Кодирование заголовка
        buffer.push(self.header.format_id[0]);
        buffer.push(self.header.format_id[1]);

        buffer.push(self.header.version[0]);
        buffer.push(self.header.version[1]);
        buffer.push(self.header.version[2]);
        buffer.push(self.header.version[3]);

        buffer.push(self.header.encoding as u8);

        // Кодирование реквизитов
        for requisite in &self.requisites {
            buffer.push(self.header.separator);
            buffer.extend(self.encode_requisite(requisite)?);
        }

        Ok(())
    }

    /// Преобразования структуры в строку согласно ГОСТ-56042.
    pub fn to_utf8_lossy(&self) -> super::Result<String> {
        let bytes = self.to_bytes()?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    /// Получить значение по ключу.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.requisites
            .iter()
            .find(|req| req.key() == key)
            .map(|req| req.value())
    }

    /// Получение заголовка
    pub fn header(&self) -> &PaymentHeader {
        &self.header
    }

    /// Получение реквизитов
    pub fn requisites(&self) -> impl Iterator<Item = &Requisite<T>> {
        self.requisites.iter()
    }
}

impl<T: CustomRequisites> Payment<T> {
    fn encode_requisite(&self, req: &Requisite<T>) -> super::Result<Vec<u8>> {
        let pair = format!("{}={}", req.key(), req.value());
        match self.header.encoding {
            PaymentEncoding::Win1251 => encoding::all::WINDOWS_1251
                .encode(&pair, encoding::EncoderTrap::Strict)
                .map_err(|_| super::Error::EncodingError),
            PaymentEncoding::Utf8 => Ok(pair.into_bytes()),
            PaymentEncoding::Koi8R => encoding::all::KOI8_R
                .encode(&pair, encoding::EncoderTrap::Strict)
                .map_err(|_| super::Error::EncodingError),
        }
    }
}

/// Парсер из строки в структуру с информацией о платеже.
#[derive(Debug)]
pub struct PaymentParser<T: CustomRequisites = NoCustomRequisites> {
    version_id: [u8; 4],
    _marker: PhantomData<T>,
}

impl<T: CustomRequisites> PaymentParser<T> {
    /// Установка версии.
    pub fn with_version(mut self, version_id: [u8; 4]) -> Self {
        self.version_id = version_id;
        self
    }

    /// Преобразовать из строки.
    ///
    /// Предполагается, что тело находится в Utf-8 формате.
    pub fn from_str(&self, val: &str) -> super::Result<Payment<T>> {
        let header = self.read_payment_header(val)?;

        let data = val[8..].to_string();

        let requisites = self.read_requisites(&data, header.separator as char)?;

        self.validate_required_requisites(&requisites)?;

        Ok(Payment { header, requisites })
    }

    /// Преобразование из байтов.
    pub fn from_bytes(&self, bytes: &[u8]) -> super::Result<Payment<T>> {
        let header = self.read_payment_header_bytes(bytes)?;

        let data = self.decode_payment_body(header.encoding, &bytes[8..])?;

        let requisites = self.read_requisites(&data, header.separator as char)?;

        self.validate_required_requisites(&requisites)?;

        Ok(Payment { header, requisites })
    }
}

impl<T: CustomRequisites> PaymentParser<T> {
    fn read_payment_header(&self, val: &str) -> super::Result<PaymentHeader> {
        let bytes = val.chars().take(8).map(|c| c as u8).collect::<Vec<_>>();
        let header = self.read_payment_header_bytes(&bytes)?;

        if header.encoding != PaymentEncoding::Utf8 {
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

    fn decode_payment_body(
        &self,
        encoding: PaymentEncoding,
        bytes: &[u8],
    ) -> super::Result<String> {
        let data = match encoding {
            PaymentEncoding::Win1251 => encoding::all::WINDOWS_1251
                .decode(bytes, encoding::DecoderTrap::Strict)
                .map_err(|_| super::Error::DecodingError)?,
            PaymentEncoding::Utf8 => {
                String::from_utf8(bytes.to_vec()).map_err(|_| super::Error::DecodingError)?
            }
            PaymentEncoding::Koi8R => encoding::all::KOI8_R
                .decode(bytes, encoding::DecoderTrap::Strict)
                .map_err(|_| super::Error::DecodingError)?,
        };

        Ok(data)
    }

    fn read_requisites(&self, data: &str, separator: char) -> super::Result<Vec<Requisite<T>>> {
        let kv = data.split(separator);

        kv.into_iter()
            .map(|kv| kv.split_once('=').ok_or(super::Error::WrongPair))
            .flat_map(|kv| kv.map(|kv| kv.try_into()))
            .collect()
    }

    fn validate_required_requisites(&self, requisites: &[Requisite<T>]) -> super::Result<()> {
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

impl<T: CustomRequisites> Default for PaymentParser<T> {
    fn default() -> Self {
        Self {
            version_id: VERSION_0001_BYTES,
            _marker: PhantomData,
        }
    }
}

/// Заголовок платежа.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentHeader {
    /// Идентификатор формата
    format_id: [u8; 2],

    /// Версия стандарта
    version: [u8; 4],

    /// Признак набора кодированных знаков
    encoding: PaymentEncoding,

    /// Разделитель
    separator: u8,
}

/// Требуемые реквизиты.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiredRequisite {
    pub name: MaxSizeString<160>,
    pub personal_acc: ExactSizeString<20>,
    pub bank_name: MaxSizeString<45>,
    pub bic: ExactSizeString<9>,
    pub correstp_acc: MaxSizeString<20>,
}

/// Варианты реквизитов.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Requisite<T: CustomRequisites> {
    // Обязательные
    /// Наименование получателя платежа
    Name(MaxSizeString<160>),

    /// Номер счета получателя платежа
    PersonalAcc(ExactSizeString<20>),

    /// Наименование банка получателя платежа
    BankName(MaxSizeString<45>),

    /// БИК
    BIC(ExactSizeString<9>),

    /// Номер кор./сч. банка получателя платежа
    CorrespAcc(MaxSizeString<20>),

    // Дополнительные
    /// Сумма платежа, в копейках
    Sum(MaxSizeString<18>),

    /// Наименование платежа (назначение)
    Purpose(MaxSizeString<210>),

    /// ИНН получателя платежа
    PayeeINN(MaxSizeString<12>),

    /// ИНН плательщика
    PayerINN(MaxSizeString<12>),

    /// Статус составителя платежного документа
    DrawerStatus(MaxSizeString<2>),

    /// КПП получателя платежа
    KPP(MaxSizeString<9>),

    /// КБК
    CBC(MaxSizeString<20>),

    /// Общероссийский классификатор территорий муниципальных образований (ОКТМО)
    OKTMO(MaxSizeString<11>),

    /// Основание налогового платежа
    PaytReason(MaxSizeString<2>),

    /// Налоговый период
    TaxPeriod(MaxSizeString<10>),

    /// Номер документа
    DocNo(MaxSizeString<15>),

    ///  Дата документа
    DocDate(MaxSizeString<10>),

    ///  Тип платежа
    TaxPayKind(MaxSizeString<2>),

    // Другие
    /// Фамилия плательщика
    LastName(Box<str>),

    /// Имя плательщика
    FirstName(Box<str>),

    /// Отчество плательщика
    MiddleName(Box<str>),

    /// Адрес плательщика
    PayerAddress(Box<str>),

    /// Лицевой счет бюджетного получателя
    PersonalAccount(Box<str>),

    /// Индекс платежного документа
    DocIdx(Box<str>),

    /// № лицевого счета в системе персонифицированного учета в ПФР - СНИЛС
    PensAcc(Box<str>),

    /// Номер договора
    Contract(Box<str>),

    /// Номер лицевого счета плательщика в организации (в системе учета ПУ)
    PersAcc(Box<str>),

    /// Номер квартиры
    Flat(Box<str>),

    /// Номер телефона
    Phone(Box<str>),

    /// Вид ДУЛ плательщика
    PayerIdType(Box<str>),

    /// Номер ДУЛ плательщика
    PayerIdNum(Box<str>),

    /// Ф.И.О. ребенка/учащегося
    ChildFio(Box<str>),

    /// Дата рождения
    BirthDate(Box<str>),

    /// Срок платежа/дата выставления счета
    PaymTerm(Box<str>),

    /// Период оплаты
    PaymPeriod(Box<str>),

    /// Вид платежа
    Category(Box<str>),

    /// Код услуги/название прибора учета
    ServiceName(Box<str>),

    /// Номер прибора учета
    CounterId(Box<str>),

    /// Показание прибора учета
    CounterVal(Box<str>),

    /// Номер извещения, начисления, счета
    QuittId(Box<str>),

    /// Дата извещения/начисления/счета/постановления (для ГИБДД)
    QuittDate(Box<str>),

    /// Номер учреждения (образовательного, медицинского)
    InstNum(Box<str>),

    /// Номер группы детсада/класса школы
    ClassNum(Box<str>),

    /// ФИО преподавателя, специалиста, оказывающего услугу
    SpecFio(Box<str>),

    /// Сумма страховки/дополнительной услуги/Сумма пени (в копейках)
    AddAmount(Box<str>),

    /// Номер постановления (для ГИБДД)
    RuleId(Box<str>),

    /// Номер исполнительного производства
    ExecId(Box<str>),

    /// Код вида платежа (например, для платежей в адрес Росреестра)
    RegType(Box<str>),

    /// Уникальный идентификатор начисления
    UIN(Box<str>),

    /// Технический код, рекомендуемый для заполнения поставщиком услуг. Может использоваться принимающей организацией для вызова соответствующей обрабатывающей ИТ-системы.
    TechCode(TechCode),

    /// Собственный вариант реквизита
    Custom(T),
}

impl<T: CustomRequisites> Requisite<T> {
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
            Requisite::Contract(_) => "Contract",
            Requisite::PersAcc(_) => "PersAcc",
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
            Requisite::Custom(v) => v.key(),
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
            Requisite::Contract(v) => v,
            Requisite::PersAcc(v) => v,
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
            Requisite::Custom(v) => v.value(),
        }
    }
}

impl<T: CustomRequisites> TryFrom<(&str, &str)> for Requisite<T> {
    type Error = super::Error;

    fn try_from((key, val): (&str, &str)) -> super::Result<Self> {
        let requisite = match key {
            "Name" => Requisite::Name(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "PersonalAcc" => Requisite::PersonalAcc(
                val.to_exact_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "BankName" => Requisite::BankName(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "BIC" => Requisite::BIC(
                val.to_exact_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "CorrespAcc" => Requisite::CorrespAcc(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "Sum" => Requisite::Sum(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "Purpose" => Requisite::Purpose(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "PayeeINN" => Requisite::PayeeINN(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "PayerINN" => Requisite::PayerINN(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "DrawerStatus" => Requisite::DrawerStatus(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "KPP" => Requisite::KPP(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "CBC" => Requisite::CBC(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "OKTMO" => Requisite::OKTMO(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "PaytReason" => Requisite::PaytReason(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "TaxPeriod" => Requisite::TaxPeriod(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "DocNo" => Requisite::DocNo(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "DocDate" => Requisite::DocDate(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "TaxPayKind" => Requisite::TaxPayKind(
                val.to_max_size()
                    .ok_or(super::Error::WrongPair(key.into(), val.into()))?,
            ),
            "LastName" => Requisite::LastName(val.into()),
            "FirstName" => Requisite::FirstName(val.into()),
            "MiddleName" => Requisite::MiddleName(val.into()),
            "PayerAddress" => Requisite::PayerAddress(val.into()),
            "PersonalAccount" => Requisite::PersonalAccount(val.into()),
            "DocIdx" => Requisite::DocIdx(val.into()),
            "PensAcc" => Requisite::PensAcc(val.into()),
            "Flat" => Requisite::Flat(val.into()),
            "Phone" => Requisite::Phone(val.into()),
            "PayerIdType" => Requisite::PayerIdType(val.into()),
            "PayerIdNum" => Requisite::PayerIdNum(val.into()),
            "ChildFio" => Requisite::ChildFio(val.into()),
            "BirthDate" => Requisite::BirthDate(val.into()),
            "PaymTerm" => Requisite::PaymTerm(val.into()),
            "PaymPeriod" => Requisite::PaymPeriod(val.into()),
            "Category" => Requisite::Category(val.into()),
            "ServiceName" => Requisite::ServiceName(val.into()),
            "CounterId" => Requisite::CounterId(val.into()),
            "CounterVal" => Requisite::CounterVal(val.into()),
            "QuittId" => Requisite::QuittId(val.into()),
            "QuittDate" => Requisite::QuittDate(val.into()),
            "InstNum" => Requisite::InstNum(val.into()),
            "ClassNum" => Requisite::ClassNum(val.into()),
            "SpecFio" => Requisite::SpecFio(val.into()),
            "AddAmount" => Requisite::AddAmount(val.into()),
            "RuleId" => Requisite::RuleId(val.into()),
            "ExecId" => Requisite::ExecId(val.into()),
            "RegType" => Requisite::RegType(val.into()),
            "UIN" => Requisite::UIN(val.into()),
            "TechCode" => Requisite::TechCode(TechCode::from_str(val)?),
            _ => Requisite::Custom((key, val).try_into()?),
        };

        Ok(requisite)
    }
}

/// Значения технического кода платежа
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TechCode {
    /// Мобильная связь, стационарный телефон
    Mobile,

    /// Коммунальные услуги, ЖКХ
    HousingAndUtilites,

    /// ГИБДД, налоги, пошлины, бюджетные платежи
    Taxes,

    /// Охранные услуги
    SecurityServices,

    /// Услуги, оказываемые УФМС
    FMS,

    // ПФР
    PFR,

    /// Погашение кредитов
    LoanRepayments,

    /// Образовательные учреждения
    EducationalInstitutions,

    /// Интернет и ТВ
    InternetTV,

    /// Электронные деньги
    Emoney,

    /// Отдых и путешествия
    Vacation,

    /// Инвестиции и страхование
    InvestmentInsurance,

    /// Спорт и здоровье
    SportHealth,

    /// Благотворительные и общественные организации
    Charity,

    ///  Прочие услуги
    Other,
}

impl TechCode {
    fn as_str(&self) -> &str {
        match self {
            TechCode::Mobile => "01",
            TechCode::HousingAndUtilites => "02",
            TechCode::Taxes => "03",
            TechCode::SecurityServices => "04",
            TechCode::FMS => "05",
            TechCode::PFR => "06",
            TechCode::LoanRepayments => "07",
            TechCode::EducationalInstitutions => "08",
            TechCode::InternetTV => "09",
            TechCode::Emoney => "10",
            TechCode::Vacation => "11",
            TechCode::InvestmentInsurance => "12",
            TechCode::SportHealth => "13",
            TechCode::Charity => "14",
            TechCode::Other => "15",
        }
    }

    fn from_str(val: &str) -> super::Result<TechCode> {
        match val {
            "01" => Ok(TechCode::Mobile),
            "02" => Ok(TechCode::HousingAndUtilites),
            "03" => Ok(TechCode::Taxes),
            "04" => Ok(TechCode::SecurityServices),
            "05" => Ok(TechCode::FMS),
            "06" => Ok(TechCode::PFR),
            "07" => Ok(TechCode::LoanRepayments),
            "08" => Ok(TechCode::EducationalInstitutions),
            "09" => Ok(TechCode::InternetTV),
            "10" => Ok(TechCode::Emoney),
            "11" => Ok(TechCode::Vacation),
            "12" => Ok(TechCode::InvestmentInsurance),
            "13" => Ok(TechCode::SportHealth),
            "14" => Ok(TechCode::Charity),
            "15" => Ok(TechCode::Other),
            _ => Err(super::Error::UnknownTechCode(val.into())),
        }
    }
}

/// Признак набора кодированных знаков.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PaymentEncoding {
    /// Windows-1251
    Win1251 = b'1',

    /// Utf-8
    Utf8 = b'2',

    /// КОИ8-R
    Koi8R = b'3',
}

impl TryFrom<u8> for PaymentEncoding {
    type Error = super::Error;

    fn try_from(value: u8) -> super::Result<PaymentEncoding> {
        match value {
            b'1' => Ok(Self::Win1251),
            b'2' => Ok(Self::Utf8),
            b'3' => Ok(Self::Koi8R),
            code => Err(super::Error::UnknownEncodingCode(code)),
        }
    }
}

impl Display for PaymentEncoding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PaymentEncoding::Win1251 => write!(f, "Windows-1251"),
            PaymentEncoding::Utf8 => write!(f, "Utf-8"),
            PaymentEncoding::Koi8R => write!(f, "KOI8-R"),
        }
    }
}
