# ГОСТ-56042

Библиотека для работы с ["**ГОСТ Р 56042-2014** Стандарты финансовых операций. Двумерные символы штрихового кода для осуществления платежей физических лиц"](https://www.rst.gov.ru/portal/gost/home/standarts/catalognational?portal:componentId=3503536e-2ac1-4753-8ed1-09a92fee02de&portal:isSecure=false&portal:portletMode=view&navigationalstate=JBPNS_rO0ABXdOAAplbnRpdHlOYW1lAAAAAQALRE9DVU1FTlRfMTEABmFjdGlvbgAAAAEABnNlYXJjaAAIZW50aXR5SWQAAAABAAQ3MTcxAAdfX0VPRl9f).

## Обзор

Официальный документ на русском языке можно скачать с [сайта](https://roskazna.gov.ru/dokumenty/dokumenty/vzaimodeystvie-s-bankovskoy-sistemoy/1157315/) or или [напрямую](https://roskazna.gov.ru/upload/iblock/5fa/gost_r_56042_2014.pdf).

### Кодирование

Для кодирование можно использовать методы:
* ```to_bytes(&self) -> super::Result<Vec<u8>>``` - преобразование структуры в массив байтов.
* ```write_to(&self, buffer: &mut Vec<u8>) -> super::Result<()>``` - заполнение буфера информацией о платеже.
* ```to_utf8_lossy(&self) -> super::Result<String>``` - преобразование структуры в строку. Из-за кодировок могут быть проблемы.

```rust
let raw = "ST00012|Name=ООО «Три кита»|PersonalAcc=40702810138250123017|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225|PayeeINN=6200098765|LastName=Иванов|FirstName=Иван|MiddleName=Иванович|Purpose=Оплата членского взноса|PayerAddress=г.Рязань ул.Ленина д.10 кв.15|Sum=100000";

let payment = Payment::builder(RequiredRequisite {
    name: "ООО «Три кита»".to_max_size().unwrap(),
    personal_acc: "40702810138250123017".to_exact_size().unwrap(),
    bank_name: "ОАО \"БАНК\"".to_max_size().unwrap(),
    bic: "044525225".to_exact_size().unwrap(),
    correstp_acc: "30101810400000000225".to_max_size().unwrap(),
})
.with_additional_requisites([
    Requisite::PayeeINN("6200098765".to_max_size().unwrap()),
    Requisite::LastName("Иванов".into()),
    Requisite::FirstName("Иван".into()),
    Requisite::MiddleName("Иванович".into()),
    Requisite::Purpose("Оплата членского взноса".to_max_size().unwrap()),
    Requisite::PayerAddress("г.Рязань ул.Ленина д.10 кв.15".into()),
    Requisite::Sum("100000".to_max_size().unwrap()),
])
.build();

let payment = payment.to_utf8_lossy();
let payment = payment.as_ref().map(|s| s.as_str());

assert_eq!(payment, Ok(raw));
```

### Парсинг

Для парсинга необходимо создать структуру ```PaymentParser``` с помощью ```Payment::parser()```.

```PaymentParser``` имеет следующие методы:
* ```from_str(&self, val: &str) -> super::Result<Payment<T>>``` - создание структуры из строки. Предполагается, что данные находятся в формате Utf-8.
* ```from_bytes(&self, bytes: &[u8]) -> super::Result<Payment<T>>``` - создание структуры из массива байтов.

Пример ```from_str```:

```rust
let raw = "ST00012|Name=ООО «Три кита»|PersonalAcc=40702810138250123017|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225";

let parsed_payment = Payment::parser().from_str(raw);

let payment = Payment::custom_builder(RequiredRequisite {
    name: "ООО «Три кита»".to_max_size().unwrap(),
    personal_acc: "40702810138250123017".to_exact_size().unwrap(),
    bank_name: "ОАО \"БАНК\"".to_max_size().unwrap(),
    bic: "044525225".to_exact_size().unwrap(),
    correstp_acc: "30101810400000000225".to_max_size().unwrap(),
})
.build();

assert_eq!(parsed_payment, Ok(payment));
```

Пример ```from_bytes```:

```rust
let raw = "ST00012|Name=ООО «Три кита»|PersonalAcc=40702810138250123017|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225".as_bytes();

let parsed_payment = Payment::parser().from_bytes(raw);

let payment = Payment::custom_builder(RequiredRequisite {
    name: "ООО «Три кита»".to_max_size().unwrap(),
    personal_acc: "40702810138250123017".to_exact_size().unwrap(),
    bank_name: "ОАО \"БАНК\"".to_max_size().unwrap(),
    bic: "044525225".to_exact_size().unwrap(),
    correstp_acc: "30101810400000000225".to_max_size().unwrap(),
})
.build();

assert_eq!(parsed_payment, Ok(payment));
```

### Получение реквизитов

```rust
let payment = Payment::builder(RequiredRequisite {
    name: "ООО «Три кита»".to_max_size().unwrap(),
    personal_acc: "40702810138250123017".to_exact_size().unwrap(),
    bank_name: "ОАО \"БАНК\"".to_max_size().unwrap(),
    bic: "044525225".to_exact_size().unwrap(),
    correstp_acc: "30101810400000000225".to_max_size().unwrap(),
})
.build();

assert_eq!(payment.get("Name"), Some("ООО «Три кита»"));
```

### Определение новых реквизитов

Для добавления новых реквизитов необходимо создать собственный тип и реализовать для него трейт ```CustomRequisites```.

```rust
enum MyReq {
    Foo,
    Bar,
}

impl TryFrom<(&str, &str)> for MyReq {
    type Error = Error;

    fn try_from(_: (&str, &str)) -> Result<Self, Self::Error> {
        Ok(Self::Foo)
    }
}

impl CustomRequisites for MyReq {
    fn key(&self) -> &str {
        match self {
            MyReq::Foo => "Foo",
            MyReq::Bar => "Bar",
        }
    }

    fn value(&self) -> &str {
        match self {
            MyReq::Foo => "Foo",
            MyReq::Bar => "Bar",
        }
    }
}

let raw = "ST00012|Name=ООО «Три кита»|PersonalAcc=40702810138250123017|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225|Foo=Foo|Bar=Bar";

let payment = Payment::custom_builder(RequiredRequisite {
    name: "ООО «Три кита»".to_max_size().unwrap(),
    personal_acc: "40702810138250123017".to_exact_size().unwrap(),
    bank_name: "ОАО \"БАНК\"".to_max_size().unwrap(),
    bic: "044525225".to_exact_size().unwrap(),
    correstp_acc: "30101810400000000225".to_max_size().unwrap(),
})
.with_additional_requisites([Requisite::Custom(MyReq::Foo), Requisite::Custom(MyReq::Bar)])
.build();

assert_eq!(payment.get("Foo"), Some("Foo"));
assert_eq!(payment.get("Bar"), Some("Bar"));

let payment = payment.to_utf8_lossy();
let payment = payment.as_ref().map(|s| s.as_str());

assert_eq!(payment, Ok(raw));
```
