use std::{fmt::Display, ops::Deref};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExactSizeString<const N: usize>(String);

impl<const N: usize> ExactSizeString<N> {
    pub fn new(val: String) -> Option<Self> {
        if val.chars().count() == N {
            Some(Self(val))
        } else {
            None
        }
    }

    pub fn new_strip(val: String) -> Option<Self> {
        match val.chars().count().cmp(&N) {
            std::cmp::Ordering::Less => None,
            std::cmp::Ordering::Equal => Some(Self(val)),
            std::cmp::Ordering::Greater => Some(Self(val.chars().take(N).collect())),
        }
    }

    pub fn new_unchecked(val: String) -> Self {
        debug_assert_eq!(val.chars().count(), N);
        Self(val)
    }
}

impl<const N: usize> Display for ExactSizeString<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<const N: usize> Deref for ExactSizeString<N> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MaxSizeString<const N: usize>(String);

impl<const N: usize> MaxSizeString<N> {
    pub fn new(val: String) -> Option<Self> {
        if val.chars().count() <= N {
            Some(Self(val))
        } else {
            None
        }
    }

    pub fn new_strip(val: String) -> Self {
        match val.chars().count().cmp(&N) {
            std::cmp::Ordering::Less | std::cmp::Ordering::Equal => Self(val),
            std::cmp::Ordering::Greater => Self(val.chars().take(N).collect()),
        }
    }

    pub fn new_unchecked(val: String) -> Self {
        debug_assert!(val.chars().count() <= N);
        Self(val)
    }
}

impl<const N: usize> Display for MaxSizeString<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    fn to_exact_size<const N: usize>(&self) -> Option<ExactSizeString<N>>;
    fn to_max_size<const N: usize>(&self) -> Option<MaxSizeString<N>>;
}

impl StringExt for &str {
    fn to_exact_size<const N: usize>(&self) -> Option<ExactSizeString<N>> {
        ExactSizeString::new(self.to_string())
    }

    fn to_max_size<const N: usize>(&self) -> Option<MaxSizeString<N>> {
        MaxSizeString::new(self.to_string())
    }
}
