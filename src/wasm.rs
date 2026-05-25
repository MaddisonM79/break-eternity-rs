//! JavaScript bindings for Decimal via wasm-bindgen.
//!
//! Exposes a JS class `Decimal` with a string-based interface. Construction
//! takes any JS string parseable by [`crate::Decimal::try_from`]; conversion
//! back to JS uses [`crate::Decimal::to_string`]. Arithmetic and comparison
//! methods are panicking variants — JS callers should pre-validate inputs
//! or wrap calls in `try { ... } catch (e) { ... }`.

use std::fmt;
use wasm_bindgen::prelude::*;

/// JavaScript-facing wrapper around [`crate::Decimal`].
///
/// Constructed in JS as `new Decimal("1.5e100")` or `Decimal.fromNumber(42.0)`.
/// All arithmetic returns `Result` (throws a JS `Error` on undefined input);
/// comparison methods are infallible.
#[wasm_bindgen(js_name = Decimal)]
pub struct JsDecimal(crate::Decimal);

impl fmt::Display for JsDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[wasm_bindgen(js_class = Decimal)]
impl JsDecimal {
    /// Construct a `Decimal` from a string.
    ///
    /// Throws if the string is not a valid `Decimal` representation.
    #[wasm_bindgen(constructor)]
    pub fn new(s: &str) -> Result<JsDecimal, JsValue> {
        crate::Decimal::try_from(s)
            .map(JsDecimal)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Construct a `Decimal` from a JS number (f64).
    ///
    /// Throws if the value is NaN or infinite.
    #[wasm_bindgen(js_name = fromNumber)]
    pub fn from_number(x: f64) -> Result<JsDecimal, JsValue> {
        crate::Decimal::try_from(x)
            .map(JsDecimal)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    }

    /// Convert this `Decimal` to its string representation.
    ///
    /// Delegates to [`Display`](std::fmt::Display) — also available as `.toString()` in JS via
    /// the `wasm_bindgen` export of this method.
    #[wasm_bindgen(js_name = toString)]
    pub fn js_to_string(&self) -> String {
        self.to_string()
    }

    /// Add two `Decimal` values.
    ///
    /// Throws on undefined arithmetic.
    pub fn add(&self, other: &JsDecimal) -> Result<JsDecimal, JsValue> {
        self.0
            .checked_add(&other.0)
            .map(JsDecimal)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    }

    /// Subtract `other` from this `Decimal`.
    ///
    /// Throws on undefined arithmetic.
    pub fn sub(&self, other: &JsDecimal) -> Result<JsDecimal, JsValue> {
        self.0
            .checked_sub(&other.0)
            .map(JsDecimal)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    }

    /// Multiply two `Decimal` values.
    ///
    /// Throws on undefined arithmetic.
    pub fn mul(&self, other: &JsDecimal) -> Result<JsDecimal, JsValue> {
        self.0
            .checked_mul(&other.0)
            .map(JsDecimal)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    }

    /// Divide this `Decimal` by `other`.
    ///
    /// Throws on division by zero or other undefined arithmetic.
    pub fn div(&self, other: &JsDecimal) -> Result<JsDecimal, JsValue> {
        self.0
            .checked_div(&other.0)
            .map(JsDecimal)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    }

    /// Raise this `Decimal` to the power of `other`.
    ///
    /// Throws on undefined arithmetic.
    pub fn pow(&self, other: &JsDecimal) -> Result<JsDecimal, JsValue> {
        self.0
            .checked_pow(&other.0)
            .map(JsDecimal)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    }

    /// Compare two `Decimal` values.
    ///
    /// Returns `-1`, `0`, or `1` following the JS `Array.prototype.sort` convention.
    pub fn cmp(&self, other: &JsDecimal) -> i32 {
        use std::cmp::Ordering;
        match self.0.cmp(&other.0) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    /// Test exact equality between two `Decimal` values.
    pub fn eq(&self, other: &JsDecimal) -> bool {
        self.0 == other.0
    }

    /// Test approximate equality within the given tolerance.
    #[wasm_bindgen(js_name = approxEq)]
    pub fn approx_eq(&self, other: &JsDecimal, tolerance: f64) -> bool {
        self.0.approx_eq(&other.0, tolerance)
    }
}
