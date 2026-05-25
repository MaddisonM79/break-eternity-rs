//! Godot 4 bindings via the `godot` crate (gdext).
//!
//! `Decimal` is exposed to `GDScript` as a `String` (round-tripping through
//! [`Display`](std::fmt::Display) + [`TryFrom<&str>`](crate::Decimal)). For
//! tighter integration, downstream authors can wrap `Decimal` in a
//! `GodotClass`.
//!
//! # Crate version
//!
//! Compiled against `godot = "0.5"` (gdext). Requires Godot 4.

use godot::meta::conv::ByValue;
use godot::meta::error::ConvertError;
use godot::meta::shape::GodotShape;
use godot::meta::{FromGodot, GodotConvert, ToGodot};
use godot::prelude::GString;

impl GodotConvert for crate::Decimal {
    type Via = GString;

    fn godot_shape() -> GodotShape {
        GodotShape::of_builtin::<GString>()
    }
}

impl FromGodot for crate::Decimal {
    fn try_from_godot(via: GString) -> Result<Self, ConvertError> {
        let s = String::from(via);
        crate::Decimal::try_from(s.as_str())
            .map_err(|e| ConvertError::new(format!("Decimal parse: {e}")))
    }
}

impl ToGodot for crate::Decimal {
    type Pass = ByValue;

    fn to_godot(&self) -> GString {
        GString::from(self.to_string().as_str())
    }
}
