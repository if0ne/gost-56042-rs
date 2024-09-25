/// Трейт необходим для расширения списка реквизитов.
///
/// Лучше всего реализовывать на `enum` типах.
pub trait CustomRequisites: for<'a> TryFrom<(&'a str, &'a str), Error = super::Error> {
    /// Ключ.
    fn key(&self) -> &str;

    /// Значение.
    fn value(&self) -> &str;
}

#[derive(Debug, PartialEq, Eq)]
pub struct NoCustomRequisites;

impl CustomRequisites for NoCustomRequisites {
    fn key(&self) -> &str {
        panic!("No key")
    }

    fn value(&self) -> &str {
        panic!("No value")
    }
}

impl TryFrom<(&str, &str)> for NoCustomRequisites {
    type Error = super::Error;

    fn try_from((key, value): (&str, &str)) -> Result<Self, Self::Error> {
        Err(super::Error::UnknownPair(
            key.to_string(),
            value.to_string(),
        ))
    }
}
