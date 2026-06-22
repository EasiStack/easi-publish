//! Validation types (self-contained, no external deps beyond serde).

use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedString<const MIN: usize, const MAX: usize>(String);

impl<const MIN: usize, const MAX: usize> Deref for BoundedString<MIN, MAX> {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl<const MIN: usize, const MAX: usize> fmt::Display for BoundedString<MIN, MAX> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de, const MIN: usize, const MAX: usize> Deserialize<'de> for BoundedString<MIN, MAX> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let len = s.chars().count();
        if len < MIN || len > MAX {
            return Err(de::Error::custom(format!(
                "string length {len} is not in range {MIN}..={MAX}"
            )));
        }
        Ok(Self(s))
    }
}

impl<const MIN: usize, const MAX: usize> Serialize for BoundedString<MIN, MAX> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NonNegF64(f64);

impl<'de> Deserialize<'de> for NonNegF64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = f64::deserialize(deserializer)?;
        if v.is_nan() || v < 0.0 {
            return Err(de::Error::custom("value must be non-negative"));
        }
        Ok(Self(v))
    }
}

impl Serialize for NonNegF64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundedVec<T, const MIN: usize, const MAX: usize>(Vec<T>);

impl<T, const MIN: usize, const MAX: usize> Deref for BoundedVec<T, MIN, MAX> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.0
    }
}

impl<'de, T, const MIN: usize, const MAX: usize> Deserialize<'de> for BoundedVec<T, MIN, MAX>
where
    T: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = Vec::<T>::deserialize(deserializer)?;
        let len = v.len();
        if len < MIN || len > MAX {
            return Err(de::Error::custom(format!(
                "array length {len} is not in range {MIN}..={MAX}"
            )));
        }
        Ok(Self(v))
    }
}

impl<T: Serialize, const MIN: usize, const MAX: usize> Serialize for BoundedVec<T, MIN, MAX> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}
