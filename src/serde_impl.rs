//! Serde implementation: serializes Decimal as a string via `Display`,
//! deserializes via `TryFrom<&str>`. Human-readable and format-agnostic
//! (suitable for JSON save files). Not the most compact representation
//! for binary formats — if size matters, consider a custom Serialize.
//!
//! Enable with the `serde` Cargo feature.

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
