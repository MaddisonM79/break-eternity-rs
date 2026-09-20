//! [`Serialize`] / [`Deserialize`] for [`Decimal`]. Enable with the `serde` Cargo feature.
//!
//! # Default representation
//!
//! * **Human-readable formats** (JSON, TOML, YAML, RON, ...) get the [`Display`] string, e.g.
//!   `"1.5e100"`. It is readable in a save file and round-trips exactly.
//! * **Binary formats** (bincode, postcard, MessagePack, ...) get the raw components as a
//!   `(sign: i8, layer: i64, mag: f64)` tuple: 17 bytes fixed-width (10 in postcard for small
//!   values) instead of a length-prefixed string, and no parsing on load.
//!
//! Deserializing from a human-readable format also accepts plain numbers, so a hand-written
//! `{"money": 100}` loads as well as `{"money": "100"}`, and a component array `[1, 1, 100.0]`.
//!
//! # Forcing one representation
//!
//! Use [`serde_components`](crate::serde_components) or [`serde_string`](crate::serde_string)
//! with `#[serde(with = "...")]` to pin a field to one form regardless of the format:
//!
//! ```
//! use break_eternity::Decimal;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Save {
//!     #[serde(with = "break_eternity::serde_components")]
//!     money: Decimal,
//! }
//!
//! let save = Save { money: Decimal::try_from("1.5e100").unwrap() };
//! let json = serde_json::to_string(&save).unwrap();
//! assert_eq!(json, r#"{"money":[1,1,100.17609125905568]}"#);
//! let back: Save = serde_json::from_str(&json).unwrap();
//! assert_eq!(back.money, save.money);
//! ```
//!
//! Infinity is written as `(±1, i64::MAX, 0.0)` in component form so that formats which cannot
//! carry a non-finite float (JSON) still round-trip it; any layer above `MAX_SAFE_LAYER`
//! normalizes back to the canonical infinity on load.
//!
//! [`Display`]: core::fmt::Display

use alloc::format;
use alloc::string::String;
use core::fmt;

use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::ser::{SerializeTuple, Serializer};
use serde::{Deserialize, Serialize};

use crate::decimal::Decimal;

impl Serialize for Decimal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serialize_string(self, serializer)
        } else {
            serialize_components(self, serializer)
        }
    }
}

impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(DecimalVisitor)
        } else {
            deserializer.deserialize_tuple(3, DecimalVisitor)
        }
    }
}

fn serialize_string<S: Serializer>(d: &Decimal, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(d)
}

fn serialize_components<S: Serializer>(d: &Decimal, serializer: S) -> Result<S::Ok, S::Error> {
    let (sign, layer, mag) = if d.is_infinite() {
        (d.sign(), i64::MAX, 0.0)
    } else {
        (d.sign(), d.layer(), d.mag())
    };
    let mut tuple = serializer.serialize_tuple(3)?;
    tuple.serialize_element(&sign)?;
    tuple.serialize_element(&layer)?;
    tuple.serialize_element(&mag)?;
    tuple.end()
}

fn parse<E: de::Error>(s: &str) -> Result<Decimal, E> {
    Decimal::try_from(s).map_err(|e| E::custom(format!("invalid Decimal {s:?}: {e}")))
}

fn from_f64<E: de::Error>(x: f64) -> Result<Decimal, E> {
    Decimal::try_from(x).map_err(|_| E::custom(format!("invalid Decimal: {x} is not finite")))
}

fn from_components<E: de::Error>(sign: i8, layer: i64, mag: f64) -> Result<Decimal, E> {
    if !matches!(sign, -1..=1) {
        return Err(E::custom(format!(
            "invalid Decimal sign {sign}, expected -1, 0 or 1"
        )));
    }
    if layer < 0 {
        return Err(E::custom(format!(
            "invalid Decimal layer {layer}, expected >= 0"
        )));
    }
    if mag.is_nan() {
        return Err(E::custom("invalid Decimal: mag is NaN"));
    }
    Ok(Decimal::from_components(sign, layer, mag))
}

struct DecimalVisitor;

impl<'de> Visitor<'de> for DecimalVisitor {
    type Value = Decimal;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "a Decimal as a string like \"1.5e100\", a number, or a [sign, layer, mag] array",
        )
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Decimal, E> {
        parse(v)
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Decimal, E> {
        from_f64(v)
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Decimal, E> {
        Ok(Decimal::from(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Decimal, E> {
        Ok(Decimal::from(v))
    }

    fn visit_i128<E: de::Error>(self, v: i128) -> Result<Decimal, E> {
        Ok(Decimal::from(v))
    }

    fn visit_u128<E: de::Error>(self, v: u128) -> Result<Decimal, E> {
        Ok(Decimal::from(v))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Decimal, A::Error> {
        let sign: i8 = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let layer: i64 = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        let mag: f64 = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(2, &self))?;
        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            return Err(de::Error::invalid_length(4, &self));
        }
        from_components(sign, layer, mag)
    }
}

/// Always serializes as a `(sign, layer, mag)` tuple. For `#[serde(with = "...")]`.
///
/// See the [module docs](crate::serde_components) for the representation and how infinity is
/// encoded. Deserialization accepts only the component form.
pub mod components {
    use super::{serialize_components, DecimalVisitor};
    use crate::decimal::Decimal;
    use serde::{Deserializer, Serializer};

    /// Serializes `d` as `(sign, layer, mag)`.
    pub fn serialize<S: Serializer>(d: &Decimal, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_components(d, serializer)
    }

    /// Deserializes a `(sign, layer, mag)` tuple.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Decimal, D::Error> {
        deserializer.deserialize_tuple(3, DecimalVisitor)
    }
}

/// Always serializes as the [`Display`](core::fmt::Display) string. For
/// `#[serde(with = "...")]`.
///
/// Deserialization accepts a string (parsed like [`FromStr`](core::str::FromStr)).
pub mod string {
    use super::{parse, serialize_string, String};
    use crate::decimal::Decimal;
    use serde::{Deserialize, Deserializer, Serializer};

    /// Serializes `d` as its `Display` string.
    pub fn serialize<S: Serializer>(d: &Decimal, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_string(d, serializer)
    }

    /// Parses a `Decimal` from a string.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Decimal, D::Error> {
        let s = String::deserialize(deserializer)?;
        parse(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn components_validation() {
        assert!(from_components::<serde_json::Error>(2, 0, 1.0).is_err());
        assert!(from_components::<serde_json::Error>(1, -1, 1.0).is_err());
        assert!(from_components::<serde_json::Error>(1, 0, f64::NAN).is_err());
        assert_eq!(
            from_components::<serde_json::Error>(1, i64::MAX, 0.0).unwrap(),
            Decimal::inf()
        );
        assert_eq!(
            from_components::<serde_json::Error>(-1, 0, 5.0).unwrap(),
            Decimal::from(-5)
        );
        assert_eq!(
            from_components::<serde_json::Error>(1, 0, 1e20)
                .unwrap()
                .layer(),
            1
        );
    }

    #[test]
    fn error_messages_name_the_input() {
        let err = serde_json::from_str::<Decimal>(r#""garbage""#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("garbage"), "{err}");
        let err = serde_json::from_str::<Decimal>("true")
            .unwrap_err()
            .to_string();
        assert!(err.contains("expected a Decimal"), "{err}");
    }
}
