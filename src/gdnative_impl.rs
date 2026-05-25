//! Godot 3 bindings via [`gdnative`].
//!
//! `Decimal` is exposed to `GDScript` through the `Variant` string representation,
//! round-tripping through [`Display`](std::fmt::Display) and
//! [`TryFrom<&str>`](crate::Decimal::try_from).
//!
//! # Deprecation notice
//!
//! The `godot3` feature (and this module) will be removed in 0.3.0. Migrate to
//! the `godot4` feature which uses [`gdext`](https://crates.io/crates/godot).

use gdnative::prelude::*;

impl FromVariant for crate::Decimal {
    fn from_variant(variant: &Variant) -> Result<Self, FromVariantError> {
        let s = String::from_variant(variant)?;
        crate::Decimal::try_from(s.as_str())
            .map_err(|e| FromVariantError::Custom(format!("Decimal parse: {e}")))
    }
}

impl ToVariant for crate::Decimal {
    fn to_variant(&self) -> Variant {
        self.to_string().to_variant()
    }
}
