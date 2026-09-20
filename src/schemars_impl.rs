//! [`JsonSchema`] for [`Decimal`]. Enable with the `schemars` Cargo feature (implies `serde`).
//!
//! The schema describes what the `serde` impl reads and writes in human-readable formats: a
//! string such as `"1.5e100"` (the [`Display`] form, and what serialization always emits) or,
//! because deserialization is lenient, a plain JSON number or a `[sign, layer, mag]` component
//! array. An editor with JSON Schema support therefore completes and validates a `Decimal`
//! field without flagging a hand-written `100`.
//!
//! The string branch carries a `pattern` that admits every notation the parser accepts (see
//! [`Decimal::from_string`]). It cannot reproduce the parser exactly, so it errs on the side of
//! accepting: what it rejects is text outside that grammar's alphabet, so `"1e1O0"` or
//! `"lots"` is caught in the editor rather than at load time, while a malformed `"1e"` is left
//! for the parser.
//!
//! ```
//! use break_eternity::Decimal;
//! use schemars::{schema_for, JsonSchema};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(JsonSchema, Serialize, Deserialize)]
//! struct Upgrade {
//!     cost: Decimal,
//!     multiplier: Decimal,
//! }
//!
//! let schema = schema_for!(Upgrade);
//! let json = serde_json::to_value(&schema).unwrap();
//! assert_eq!(json["properties"]["cost"]["$ref"], "#/$defs/Decimal");
//! assert_eq!(json["$defs"]["Decimal"]["anyOf"][0]["type"], "string");
//! ```
//!
//! [`Display`]: core::fmt::Display

use alloc::borrow::Cow;

use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};

use crate::decimal::Decimal;

/// The JSON Schema `pattern` for the string form. Anchored; whitespace-tolerant at the ends
/// like the parser; case-insensitive by spelling out both cases (ECMA-262 patterns carry no
/// flags).
///
/// The alternatives are, in order: scientific and stacked exponents (`1.5e100`, `1e-45`,
/// `ee10`, `5e3e2`); a plain number; the `(e^N)M` form for high layers; `X^Y`, `X^^N[;P]` and
/// `X^^^N[;P]`; the `N PT M` / `NpM` tetrate shorthands; the `MfN` / `fN` shorthands; and
/// `Infinity` / `inf`.
pub const DECIMAL_PATTERN: &str = concat_pattern();

const fn concat_pattern() -> &'static str {
    // `concat!` needs literals, so the number grammar, `[0-9][0-9,]*(\.[0-9]*)?|\.[0-9]+`
    // (a digit run with the thousands separators the parser strips, and an optional
    // fraction), is spelled out in every alternative rather than named once.
    concat!(
        r"^\s*[+-]?(?:",
        // stacked exponents: an optional mantissa, then `e[±][number]` groups ending in a number
        r"(?:[0-9][0-9,]*(?:\.[0-9]*)?|\.[0-9]+)?(?:[eE][+-]?(?:[0-9][0-9,]*(?:\.[0-9]*)?|\.[0-9]+)?)*[eE][+-]?(?:[0-9][0-9,]*(?:\.[0-9]*)?|\.[0-9]+)",
        r"|",
        // a plain number
        r"(?:[0-9][0-9,]*(?:\.[0-9]*)?|\.[0-9]+)",
        r"|",
        // (e^N)M
        r"\([eE]\^[+-]?(?:[0-9][0-9,]*(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?\)[+-]?(?:[0-9][0-9,]*(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?",
        r"|",
        // X^Y, X^^N, X^^N;P, X^^^N, X^^^N;P
        r"(?:[0-9][0-9,]*(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?\^{1,3}[+-]?(?:[0-9][0-9,]*(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?(?:;[+-]?(?:[0-9][0-9,]*(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)?",
        r"|",
        // N PT M, N PT (M), NpM
        r"(?:(?:[0-9][0-9,]*(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)?\s*[pP][tT]?\s*\(?\s*[+-]?(?:[0-9][0-9,]*(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?\s*\)?",
        r"|",
        // MfN, fN, f(N)
        r"(?:(?:[0-9][0-9,]*(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)?[fF]\(?[+-]?(?:[0-9][0-9,]*(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?\)?",
        r"|",
        // Infinity, inf
        r"[iI][nN][fF](?:[iI][nN][iI][tT][yY])?",
        r")\s*$"
    )
}

impl JsonSchema for Decimal {
    fn schema_name() -> Cow<'static, str> {
        "Decimal".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "break_eternity::Decimal".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "title": "Decimal",
            "description": "A break_eternity number, sign * 10^10^...^mag, from 10^^9e15 down to 10^-(10^^9e15). Written as a string: \"100\", \"1.5e100\", \"ee10\", \"(e^7)10\", \"Infinity\". A plain JSON number or a [sign, layer, mag] component array is also accepted when loading.",
            "anyOf": [
                {
                    "type": "string",
                    "pattern": DECIMAL_PATTERN,
                    "examples": ["100", "1.5e100", "1e-20", "ee10", "(e^7)10", "Infinity"]
                },
                { "type": "number" },
                {
                    "type": "array",
                    "prefixItems": [
                        { "type": "integer", "enum": [-1, 0, 1], "description": "sign" },
                        { "type": "integer", "minimum": 0, "description": "layer" },
                        { "type": "number", "description": "mag" }
                    ],
                    "minItems": 3,
                    "maxItems": 3
                }
            ]
        })
    }
}
