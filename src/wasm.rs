//! JavaScript bindings for Decimal via wasm-bindgen.
//!
//! Exposes a JS class `Decimal` with a string-based interface. Construction
//! takes any JS string parseable by [`crate::Decimal::from_string`]; conversion
//! back to JS uses [`crate::Decimal::to_string`]. Every operation that can be
//! undefined throws a JS `Error` (via the `checked_*` forms) instead of
//! returning NaN, so callers should wrap untrusted arithmetic in `try/catch`.
//!
//! Fractional-height tetration always uses the analytic approximation
//! ([`TetrationMode::Analytic`]), matching `break_eternity.js` defaults.

use std::fmt;
use wasm_bindgen::prelude::*;

use crate::error::ArithmeticError;
use crate::notation::Notation;
use crate::tetration::TetrationMode;

/// JavaScript-facing wrapper around [`crate::Decimal`].
///
/// Constructed in JS as `new Decimal("1.5e100")` or `Decimal.fromNumber(42.0)`.
#[wasm_bindgen(js_name = Decimal)]
#[derive(Clone, Copy)]
pub struct JsDecimal(crate::Decimal);

impl fmt::Display for JsDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<crate::Decimal> for JsDecimal {
    fn from(d: crate::Decimal) -> Self {
        JsDecimal(d)
    }
}

fn js_err(e: ArithmeticError) -> JsValue {
    JsValue::from_str(&e.to_string())
}

fn wrap(r: Result<crate::Decimal, ArithmeticError>) -> Result<JsDecimal, JsValue> {
    r.map(JsDecimal).map_err(js_err)
}

#[wasm_bindgen(js_class = Decimal)]
#[allow(clippy::wrong_self_convention, clippy::should_implement_trait)] // JS-facing names; methods must take &self
impl JsDecimal {
    // -----------------------------------------------------------------------
    // Construction and conversion
    // -----------------------------------------------------------------------

    /// Construct a `Decimal` from a string.
    ///
    /// Throws if the string is not a valid `Decimal` representation.
    #[wasm_bindgen(constructor)]
    pub fn new(s: &str) -> Result<JsDecimal, JsValue> {
        crate::Decimal::from_string(s)
            .map(JsDecimal)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Construct a `Decimal` from a JS number (f64).
    ///
    /// Throws if the value is NaN. `±Infinity` produce the corresponding infinities.
    #[wasm_bindgen(js_name = fromNumber)]
    pub fn from_number(x: f64) -> Result<JsDecimal, JsValue> {
        if x.is_nan() {
            return Err(JsValue::from_str(
                "Decimal.fromNumber: NaN is not representable",
            ));
        }
        Ok(JsDecimal(crate::Decimal::from_f64(x)))
    }

    /// Construct a `Decimal` from sign, layer and magnitude components.
    #[wasm_bindgen(js_name = fromComponents)]
    pub fn from_components(sign: i8, layer: f64, mag: f64) -> Result<JsDecimal, JsValue> {
        if layer < 0.0 || layer.is_nan() {
            return Err(JsValue::from_str(
                "Decimal.fromComponents: layer must be non-negative",
            ));
        }
        Ok(JsDecimal(crate::Decimal::from_components(
            sign,
            layer as i64,
            mag,
        )))
    }

    /// Construct a `Decimal` from a mantissa and exponent.
    #[wasm_bindgen(js_name = fromMantissaExponent)]
    pub fn from_mantissa_exponent(mantissa: f64, exponent: f64) -> JsDecimal {
        JsDecimal(crate::Decimal::from_mantissa_exponent(mantissa, exponent))
    }

    /// Convert this `Decimal` to its string representation (also used by `toString()`).
    #[wasm_bindgen(js_name = toString)]
    pub fn js_to_string(&self) -> String {
        self.to_string()
    }

    /// Convert this `Decimal` to a JS number, saturating to `±Infinity`.
    #[wasm_bindgen(js_name = toNumber)]
    pub fn to_number(&self) -> f64 {
        self.0.to_number()
    }

    /// JSON serialization: the same string as `toString()`.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> String {
        self.to_string()
    }

    /// Fixed-point formatting with `places` digits after the decimal point.
    #[wasm_bindgen(js_name = toFixed)]
    pub fn to_fixed(&self, places: u32) -> String {
        self.0.to_fixed(places as usize)
    }

    /// Formatting with `places` significant digits.
    #[wasm_bindgen(js_name = toPrecision)]
    pub fn to_precision(&self, places: u32) -> String {
        self.0.to_precision(places as usize)
    }

    /// Formats in a game notation: `"scientific"`, `"engineering"`, `"standard"`,
    /// `"letters"`, or `"logarithm"` (case-insensitive). Unknown names are an error.
    #[wasm_bindgen(js_name = toNotation)]
    pub fn to_notation(&self, notation: &str, places: u32) -> Result<String, JsError> {
        let notation = match notation.to_ascii_lowercase().as_str() {
            "scientific" => Notation::Scientific,
            "engineering" => Notation::Engineering,
            "standard" => Notation::Standard,
            "letters" => Notation::Letters,
            "logarithm" => Notation::Logarithm,
            other => return Err(JsError::new(&format!("unknown notation {other:?}"))),
        };
        Ok(self.0.to_notation(notation, places as usize))
    }

    /// Scientific-notation formatting with `places` digits after the decimal point.
    #[wasm_bindgen(js_name = toExponential)]
    pub fn to_exponential(&self, places: u32) -> String {
        self.0.to_exponential(places as usize)
    }

    /// The sign component: 1, 0, or -1.
    #[wasm_bindgen(getter)]
    pub fn sign(&self) -> i8 {
        self.0.sign()
    }

    /// The layer component.
    #[wasm_bindgen(getter)]
    pub fn layer(&self) -> f64 {
        self.0.layer() as f64
    }

    /// The magnitude component.
    #[wasm_bindgen(getter)]
    pub fn mag(&self) -> f64 {
        self.0.mag()
    }

    /// The mantissa (`m` in break_eternity.js).
    #[wasm_bindgen(getter)]
    pub fn mantissa(&self) -> f64 {
        self.0.mantissa()
    }

    /// The exponent (`e` in break_eternity.js).
    #[wasm_bindgen(getter)]
    pub fn exponent(&self) -> f64 {
        self.0.exponent()
    }

    /// True if the value is finite.
    #[wasm_bindgen(js_name = isFinite)]
    pub fn is_finite(&self) -> bool {
        self.0.is_finite()
    }

    // -----------------------------------------------------------------------
    // Arithmetic
    // -----------------------------------------------------------------------

    /// Add two `Decimal` values. Throws on `Infinity + -Infinity`.
    pub fn add(&self, other: &JsDecimal) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_add(&other.0))
    }

    /// Subtract `other` from this `Decimal`. Throws on `Infinity - Infinity`.
    pub fn sub(&self, other: &JsDecimal) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_sub(&other.0))
    }

    /// Multiply two `Decimal` values. Throws on `Infinity * 0`.
    pub fn mul(&self, other: &JsDecimal) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_mul(&other.0))
    }

    /// Divide this `Decimal` by `other`. Throws on division by zero.
    pub fn div(&self, other: &JsDecimal) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_div(&other.0))
    }

    /// Truncated remainder. Throws on a zero divisor.
    #[wasm_bindgen(js_name = mod)]
    pub fn rem(&self, other: &JsDecimal) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_rem(&other.0))
    }

    /// Negation.
    pub fn neg(&self) -> JsDecimal {
        JsDecimal(-self.0)
    }

    /// Absolute value.
    pub fn abs(&self) -> JsDecimal {
        JsDecimal(self.0.abs())
    }

    /// Reciprocal. Throws for zero.
    pub fn recip(&self) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_recip())
    }

    /// Round to the nearest integer.
    pub fn round(&self) -> JsDecimal {
        JsDecimal(self.0.round())
    }

    /// Round down.
    pub fn floor(&self) -> JsDecimal {
        JsDecimal(self.0.floor())
    }

    /// Rounds to `places` digits after the decimal point (negative for tens, hundreds, ...).
    #[wasm_bindgen(js_name = roundToPlaces)]
    pub fn round_to_places(&self, places: i32) -> JsDecimal {
        JsDecimal(self.0.round_to_places(places))
    }

    /// Rounds to `digits` significant figures.
    #[wasm_bindgen(js_name = roundToSignificant)]
    pub fn round_to_significant(&self, digits: u32) -> JsDecimal {
        JsDecimal(self.0.round_to_significant(digits))
    }

    /// Whether the value is a finite whole number.
    #[wasm_bindgen(js_name = isInteger)]
    pub fn is_integer(&self) -> bool {
        self.0.is_integer()
    }

    /// Round up.
    pub fn ceil(&self) -> JsDecimal {
        JsDecimal(self.0.ceil())
    }

    /// Truncate toward zero.
    pub fn trunc(&self) -> JsDecimal {
        JsDecimal(self.0.trunc())
    }

    // -----------------------------------------------------------------------
    // Powers, roots, logs
    // -----------------------------------------------------------------------

    /// Raise this `Decimal` to the power of `other`. Throws on undefined results.
    pub fn pow(&self, other: &JsDecimal) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_pow(&other.0))
    }

    /// `10 ^ this`.
    pub fn pow10(&self) -> JsDecimal {
        JsDecimal(self.0.pow10())
    }

    /// `e ^ this`.
    pub fn exp(&self) -> JsDecimal {
        JsDecimal(self.0.exp())
    }

    /// Square root. Throws for negative input.
    pub fn sqrt(&self) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_sqrt())
    }

    /// Cube root.
    pub fn cbrt(&self) -> JsDecimal {
        JsDecimal(self.0.cbrt())
    }

    /// `n`-th root. Throws when undefined.
    pub fn root(&self, n: &JsDecimal) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_root(&n.0))
    }

    /// Natural logarithm. Throws for non-positive input.
    pub fn ln(&self) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_ln())
    }

    /// Base-10 logarithm. Throws for non-positive input.
    pub fn log10(&self) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_log10())
    }

    /// Base-2 logarithm. Throws for non-positive input.
    pub fn log2(&self) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_log2())
    }

    /// Logarithm to an arbitrary base. Throws when undefined.
    pub fn log(&self, base: &JsDecimal) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_log(&base.0))
    }

    /// Gamma function. Throws at the poles.
    pub fn gamma(&self) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_gamma())
    }

    /// Factorial via the gamma function. Throws for negative integers.
    pub fn factorial(&self) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_factorial())
    }

    /// Principal branch of the Lambert W function. Throws outside its domain.
    pub fn lambertw(&self) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_lambertw())
    }

    // -----------------------------------------------------------------------
    // Tetration family
    // -----------------------------------------------------------------------

    /// Tetration `this ^^ height` with an optional payload (defaults to 1).
    pub fn tetrate(&self, height: f64, payload: Option<JsDecimal>) -> Result<JsDecimal, JsValue> {
        let payload = payload.map_or_else(crate::Decimal::one, |p| p.0);
        wrap(
            self.0
                .checked_tetrate(height, payload, TetrationMode::Analytic),
        )
    }

    /// Iterated logarithm.
    pub fn iteratedlog(&self, base: &JsDecimal, times: f64) -> Result<JsDecimal, JsValue> {
        wrap(
            self.0
                .checked_iteratedlog(base.0, times, TetrationMode::Analytic),
        )
    }

    /// Super-logarithm with the given base (defaults to 10).
    pub fn slog(&self, base: Option<JsDecimal>) -> Result<JsDecimal, JsValue> {
        let base = base.map_or_else(crate::Decimal::ten, |b| b.0);
        wrap(self.0.checked_slog(base, TetrationMode::Analytic))
    }

    /// Super square root.
    pub fn ssqrt(&self) -> Result<JsDecimal, JsValue> {
        wrap(self.0.checked_ssqrt())
    }

    /// Pentation `this ^^^ height`.
    pub fn pentate(&self, height: f64) -> Result<JsDecimal, JsValue> {
        wrap(
            self.0
                .checked_pentate(height, crate::Decimal::one(), TetrationMode::Analytic),
        )
    }

    // -----------------------------------------------------------------------
    // Comparison
    // -----------------------------------------------------------------------

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

    /// Exact equality.
    pub fn eq(&self, other: &JsDecimal) -> bool {
        self.0 == other.0
    }

    /// Exact inequality.
    pub fn neq(&self, other: &JsDecimal) -> bool {
        self.0 != other.0
    }

    /// `this < other`.
    pub fn lt(&self, other: &JsDecimal) -> bool {
        self.0 < other.0
    }

    /// `this <= other`.
    pub fn lte(&self, other: &JsDecimal) -> bool {
        self.0 <= other.0
    }

    /// `this > other`.
    pub fn gt(&self, other: &JsDecimal) -> bool {
        self.0 > other.0
    }

    /// `this >= other`.
    pub fn gte(&self, other: &JsDecimal) -> bool {
        self.0 >= other.0
    }

    /// The larger of the two values.
    pub fn max(&self, other: &JsDecimal) -> JsDecimal {
        JsDecimal(self.0.max(other.0))
    }

    /// The smaller of the two values.
    pub fn min(&self, other: &JsDecimal) -> JsDecimal {
        JsDecimal(self.0.min(other.0))
    }

    /// Clamp into `[min, max]`.
    pub fn clamp(&self, min: &JsDecimal, max: &JsDecimal) -> JsDecimal {
        JsDecimal(self.0.clamp(min.0, max.0))
    }

    /// Test approximate equality within the given relative tolerance.
    #[wasm_bindgen(js_name = approxEq)]
    pub fn approx_eq(&self, other: &JsDecimal, tolerance: f64) -> bool {
        self.0.approx_eq(&other.0, tolerance)
    }

    // -----------------------------------------------------------------------
    // Game helpers
    // -----------------------------------------------------------------------

    /// How many items can be afforded in a geometric cost series.
    #[wasm_bindgen(js_name = affordGeometricSeries)]
    pub fn afford_geometric_series(
        resources_available: &JsDecimal,
        price_start: &JsDecimal,
        price_ratio: &JsDecimal,
        current_owned: &JsDecimal,
    ) -> Result<JsDecimal, JsValue> {
        wrap(crate::Decimal::checked_afford_geometric_series(
            resources_available.0,
            price_start.0,
            price_ratio.0,
            current_owned.0,
        ))
    }

    /// Total cost of `num_items` in a geometric cost series.
    #[wasm_bindgen(js_name = sumGeometricSeries)]
    pub fn sum_geometric_series(
        num_items: &JsDecimal,
        price_start: &JsDecimal,
        price_ratio: &JsDecimal,
        current_owned: &JsDecimal,
    ) -> Result<JsDecimal, JsValue> {
        wrap(crate::Decimal::checked_sum_geometric_series(
            num_items.0,
            price_start.0,
            price_ratio.0,
            current_owned.0,
        ))
    }

    /// How many items can be afforded in an arithmetic cost series.
    #[wasm_bindgen(js_name = affordArithmeticSeries)]
    pub fn afford_arithmetic_series(
        resources_available: &JsDecimal,
        price_start: &JsDecimal,
        price_add: &JsDecimal,
        current_owned: &JsDecimal,
    ) -> Result<JsDecimal, JsValue> {
        wrap(crate::Decimal::checked_afford_arithmetic_series(
            resources_available.0,
            price_start.0,
            price_add.0,
            current_owned.0,
        ))
    }

    /// Total cost of `num_items` in an arithmetic cost series.
    #[wasm_bindgen(js_name = sumArithmeticSeries)]
    pub fn sum_arithmetic_series(
        num_items: &JsDecimal,
        price_start: &JsDecimal,
        price_add: &JsDecimal,
        current_owned: &JsDecimal,
    ) -> Result<JsDecimal, JsValue> {
        wrap(crate::Decimal::checked_sum_arithmetic_series(
            num_items.0,
            price_start.0,
            price_add.0,
            current_owned.0,
        ))
    }
}
