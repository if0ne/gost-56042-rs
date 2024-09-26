#![no_std]

extern crate alloc;

mod custom;
mod error;
mod parser;
mod payment;
mod string_types;

pub use custom::*;
pub use error::{Error, Result};
pub use parser::*;
pub use payment::*;
pub use string_types::*;

#[cfg(test)]
mod tests {
    use crate::{
        string_types::StringExt, CustomRequisites, Error, ParserStrategy, Payment,
        RequiredRequisite, Requisite,
    };

    #[test]
    fn encoding_test() {
        let raw = "ST00012|Name=ООО «Три кита»|PersonalAcc=40702810138250123017|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225";

        let payment = Payment::builder(RequiredRequisite {
            name: "ООО «Три кита»".to_max_size().unwrap(),
            personal_acc: "40702810138250123017".to_exact_size().unwrap(),
            bank_name: "ОАО \"БАНК\"".to_max_size().unwrap(),
            bic: "044525225".to_exact_size().unwrap(),
            correstp_acc: "30101810400000000225".to_max_size().unwrap(),
        })
        .build();

        let payment = payment.to_utf8_lossy();
        let payment = payment.as_ref().map(|s| s.as_str());

        assert_eq!(payment, Ok(raw))
    }

    #[test]
    fn decoding_bytes_test() {
        let raw = "ST00012|Name=ООО «Три кита»|PersonalAcc=40702810138250123017|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225".as_bytes();

        let parsed_payment = Payment::parser().parse_from_bytes(raw);

        let payment = Payment::custom_builder(RequiredRequisite {
            name: "ООО «Три кита»".to_max_size().unwrap(),
            personal_acc: "40702810138250123017".to_exact_size().unwrap(),
            bank_name: "ОАО \"БАНК\"".to_max_size().unwrap(),
            bic: "044525225".to_exact_size().unwrap(),
            correstp_acc: "30101810400000000225".to_max_size().unwrap(),
        })
        .build();

        assert_eq!(parsed_payment, Ok(payment));
    }

    #[test]
    fn decoding_string_test() {
        let raw = "ST00012|Name=ООО «Три кита»|PersonalAcc=40702810138250123017|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225";

        let parsed_payment = Payment::parser().parse_from_str(raw);

        let payment = Payment::custom_builder(RequiredRequisite {
            name: "ООО «Три кита»".to_max_size().unwrap(),
            personal_acc: "40702810138250123017".to_exact_size().unwrap(),
            bank_name: "ОАО \"БАНК\"".to_max_size().unwrap(),
            bic: "044525225".to_exact_size().unwrap(),
            correstp_acc: "30101810400000000225".to_max_size().unwrap(),
        })
        .build();

        assert_eq!(parsed_payment, Ok(payment));
    }

    #[test]
    fn decoding_example_test() {
        let raw = "ST00012|Name=ООО «Три кита»|PersonalAcc=40702810138250123017|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225|PayeeINN=6200098765|LastName=Иванов|FirstName=Иван|MiddleName=Иванович|Purpose=Оплата членского взноса|PayerAddress=г.Рязань ул.Ленина д.10 кв.15|Sum=100000";

        let parsed_payment = Payment::parser().parse_from_str(raw);

        let payment = Payment::custom_builder(RequiredRequisite {
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

        assert_eq!(parsed_payment, Ok(payment));
    }

    #[test]
    fn encoding_example_test() {
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
    }

    #[test]
    fn custom_requisit_test() {
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
    }

    #[test]
    fn wrong_order_test() {
        let raw = "ST00012|PersonalAcc=40702810138250123017|Name=ООО «Три кита»|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225|PayeeINN=6200098765|LastName=Иванов|FirstName=Иван|MiddleName=Иванович|Purpose=Оплата членского взноса|PayerAddress=г.Рязань ул.Ленина д.10 кв.15|Sum=100000";
        let parsed_payment = Payment::parser().parse_from_str(raw);

        assert_eq!(
            parsed_payment,
            Err(Error::WrongRequiredRequisiteOrder {
                passed: "PersonalAcc".into(),
                expected: "Name".into()
            })
        );
    }

    #[test]
    fn strict_parser_test() {
        let raw =
            "ST00012|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225|Тест=42";

        let parsed_payment = Payment::parser().parse_from_str(raw);

        assert!(parsed_payment.is_err());
    }

    #[test]
    fn requisite_tolerance_parser_test() {
        let raw = "ST00012|Name=ООО «Три кита»|PersonalAcc=40702810138250123017|BankName=ОАО \"БАНК\"|BIC=044525225|CorrespAcc=30101810400000000225|Тест=42|fasfdsfsdfs|  |";

        let parsed_payment = Payment::requisite_tolerance_parser().parse_from_str(raw);

        let payment = Payment::builder(RequiredRequisite {
            name: "ООО «Три кита»".to_max_size().unwrap(),
            personal_acc: "40702810138250123017".to_exact_size().unwrap(),
            bank_name: "ОАО \"БАНК\"".to_max_size().unwrap(),
            bic: "044525225".to_exact_size().unwrap(),
            correstp_acc: "30101810400000000225".to_max_size().unwrap(),
        })
        .build();

        assert_eq!(parsed_payment, Ok(payment));
    }

    #[test]
    fn loose_parser_test() {
        let raw = "ST00012|Name=ООО «Три кита»||Тест=42|fasfdsfsdfs|  |";

        let parsed_payment = Payment::loose_parser().parse_from_str(raw);

        assert_eq!(parsed_payment.unwrap().get("Name"), Some("ООО «Три кита»"));
    }
}
