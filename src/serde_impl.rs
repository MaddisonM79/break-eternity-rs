//! Optional serde support for [`Decimal`].
//!
//! Enable with the `serde` feature flag. Serializes to/from a string representation.

use std::convert::TryInto;

use crate::decimal::Decimal;
use crate::error::BreakEternityError;

impl ::serde::Serialize for Decimal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> ::serde::Deserialize<'de> for Decimal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        let d = String::deserialize(deserializer)?;
        let dec: Result<Decimal, BreakEternityError> = d.as_str().try_into();
        dec.map_err(|_| ::serde::de::Error::custom("Could not parse Decimal"))
    }
}
