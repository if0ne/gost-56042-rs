use core::{fmt::Display, ops::Deref};

use alloc::boxed::Box;

/// Строка с фиксированным размером, который равен ```N```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExactSizeString<const N: usize>(Box<str>);

impl<const N: usize> ExactSizeString<N> {
    /// Проверяется размер входной строки.
    ///
    /// Если размер входной строки не равен ```N```, то вернется ```None```.
    pub fn new(val: impl Into<Box<str>>) -> Option<Self> {
        let val = val.into();

        if val.chars().count() == N {
            Some(Self(val))
        } else {
            None
        }
    }

    /// Проверяется размер входной строки.
    ///
    /// Если строка имеет размер больше ```N```, то она обрезается до размера N.
    ///
    /// Если строка меньше ```N```, то вернется ```None```.
    pub fn new_strip(val: impl Into<Box<str>>) -> Option<Self> {
        let val = val.into();

        match val.chars().count().cmp(&N) {
            core::cmp::Ordering::Less => None,
            core::cmp::Ordering::Equal => Some(Self(val)),
            core::cmp::Ordering::Greater => Some(Self(val.chars().take(N).collect())),
        }
    }

    /// Создается ```ExactSizeString<N>``` без проверки.
    ///
    /// В реализации используется ```debug_assertion``` для проверки размера входной строки в `Debug` режиме.
    pub fn new_unchecked(val: impl Into<Box<str>>) -> Self {
        let val = val.into();

        debug_assert_eq!(val.chars().count(), N);
        Self(val)
    }
}

impl<const N: usize> Display for ExactSizeString<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl<const N: usize> Deref for ExactSizeString<N> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Строка с фиксированным размером, который меньше или равен ```N```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MaxSizeString<const N: usize>(Box<str>);

impl<const N: usize> MaxSizeString<N> {
    /// Проверяется размер входной строки.
    ///
    /// Если размер входной строки больше ```N```, то вернется ```None```.
    pub fn new(val: impl Into<Box<str>>) -> Option<Self> {
        let val = val.into();
        if val.chars().count() <= N {
            Some(Self(val))
        } else {
            None
        }
    }

    /// Проверяется размер входной строки.
    ///
    /// Если размер входной строки больше ```N```, то она обрезается до ```N``` символов.
    pub fn new_strip(val: impl Into<Box<str>>) -> Self {
        let val = val.into();
        match val.chars().count().cmp(&N) {
            core::cmp::Ordering::Less | core::cmp::Ordering::Equal => Self(val),
            core::cmp::Ordering::Greater => Self(val.chars().take(N).collect()),
        }
    }

    /// Создается ```MaxSizeString<N>``` без проверки.
    ///
    /// В реализации используется ```debug_assertion``` для проверки размера входной строки в `Debug` режиме.
    pub fn new_unchecked(val: impl Into<Box<str>>) -> Self {
        let val = val.into();

        debug_assert!(val.chars().count() <= N);
        Self(val)
    }
}

impl<const N: usize> Display for MaxSizeString<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl<const N: usize> Deref for MaxSizeString<N> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait StringExt {
    fn to_exact_size<const N: usize>(self) -> Option<ExactSizeString<N>>;
    fn to_max_size<const N: usize>(self) -> Option<MaxSizeString<N>>;
}

impl StringExt for &str {
    fn to_exact_size<const N: usize>(self) -> Option<ExactSizeString<N>> {
        ExactSizeString::new(self)
    }

    fn to_max_size<const N: usize>(self) -> Option<MaxSizeString<N>> {
        MaxSizeString::new(self)
    }
}
