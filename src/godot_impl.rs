//! Godot 4 bindings via the `godot` crate (gdext). Enable with the `godot4` feature.
//!
//! Two layers:
//!
//! * [`Decimal`] implements `GodotConvert` / `FromGodot` / `ToGodot`, crossing the GDScript
//!   boundary as a `String` (through [`Display`](core::fmt::Display) and `TryFrom<&str>`),
//!   so it can be a `#[func]` parameter or return type, a `#[var]`, or a `#[export]`.
//! * [`GodotDecimal`] is a `RefCounted` class with the arithmetic, formatting and comparison
//!   surface exposed as methods, for game logic written in GDScript:
//!
//!   ```gdscript
//!   var money := GodotDecimal.from_string("1e308")
//!   money = money.mul(GodotDecimal.from_number(2.5))
//!   label.text = money.to_notation("standard", 2)  # "2.50 Ce"
//!   if money.gte(GodotDecimal.from_string("1e310")):
//!       unlock()
//!   ```
//!
//!   Fallible operations return `null` instead of raising (`div` by zero, `pow` of a negative
//!   base to a fractional exponent, `ln` of a non-positive value, ...). `str(x)` and string
//!   formatting use the same compact form as `Display`.
//!
//! # Crate version
//!
//! Compiled against `godot = "0.5"` (gdext), which needs Rust 1.94 and Godot 4.

// gdext `#[func]` parameters are taken by value (`Gd<T>` is a reference-counted handle).
#![allow(clippy::needless_pass_by_value)]

use godot::meta::conv::ByValue;
use godot::meta::error::ConvertError;
use godot::meta::shape::GodotShape;
use godot::meta::{FromGodot, GodotConvert, ToGodot};
use godot::prelude::*;

use crate::notation::Notation;
use crate::tetration::TetrationMode;
use crate::Decimal;

impl GodotConvert for Decimal {
    type Via = GString;

    fn godot_shape() -> GodotShape {
        GodotShape::of_builtin::<GString>()
    }
}

impl FromGodot for Decimal {
    fn try_from_godot(via: GString) -> Result<Self, ConvertError> {
        let s = String::from(via);
        Decimal::try_from(s.as_str()).map_err(|e| ConvertError::new(format!("Decimal parse: {e}")))
    }
}

impl ToGodot for Decimal {
    type Pass = ByValue;

    fn to_godot(&self) -> GString {
        GString::from(self.to_string().as_str())
    }
}

/// A [`Decimal`] as a Godot `RefCounted` object, for use from GDScript.
///
/// Construct with the static `from_string` / `from_number` / `from_components` functions, or
/// from Rust with [`GodotDecimal::new`]. Objects are immutable: every operation returns a new
/// one.
#[derive(GodotClass)]
#[class(init, base = RefCounted)]
pub struct GodotDecimal {
    #[init(val = Decimal::zero())]
    inner: Decimal,
}

impl GodotDecimal {
    /// Wraps a [`Decimal`] in a new Godot object.
    pub fn new(value: Decimal) -> Gd<Self> {
        Gd::from_object(Self { inner: value })
    }

    /// The wrapped value.
    pub fn value(&self) -> Decimal {
        self.inner
    }

    fn wrap(value: Decimal) -> Gd<Self> {
        Self::new(value)
    }

    fn wrap_checked<E>(value: Result<Decimal, E>) -> Option<Gd<Self>> {
        value.ok().map(Self::new)
    }

    fn mode(linear: bool) -> TetrationMode {
        if linear {
            TetrationMode::Linear
        } else {
            TetrationMode::Analytic
        }
    }
}

fn gstr(s: &str) -> GString {
    GString::from(s)
}

impl From<Decimal> for Gd<GodotDecimal> {
    fn from(value: Decimal) -> Self {
        GodotDecimal::new(value)
    }
}

#[godot_api]
impl IRefCounted for GodotDecimal {
    fn to_string(&self) -> GString {
        self.inner.to_godot()
    }
}

#[godot_api]
impl GodotDecimal {
    // -- construction -------------------------------------------------------

    /// Parses any literal `Decimal` accepts (`"1e308"`, `"ee15"`, `"10^^3"`, `"1.5M"`).
    /// Returns `null` for invalid input.
    #[func]
    fn from_string(text: GString) -> Option<Gd<Self>> {
        Self::wrap_checked(Decimal::try_from(String::from(text).as_str()))
    }

    /// Converts a float. Returns `null` for NaN or infinities.
    #[func]
    fn from_number(value: f64) -> Option<Gd<Self>> {
        Self::wrap_checked(Decimal::try_from(value))
    }

    /// Builds `sign * 10^10^...^mag` with `layer` tens; the value is normalized.
    #[func]
    fn from_components(sign: i64, layer: i64, mag: f64) -> Gd<Self> {
        Self::wrap(Decimal::from_components(
            sign.signum() as i8,
            layer.max(0),
            mag,
        ))
    }

    /// Builds `mantissa * 10^exponent`.
    #[func]
    fn from_mantissa_exponent(mantissa: f64, exponent: f64) -> Gd<Self> {
        Self::wrap(Decimal::from_mantissa_exponent(mantissa, exponent))
    }

    // -- inspection ---------------------------------------------------------

    /// The value as a float; saturates to `INF` beyond the float range.
    #[func]
    fn to_number(&self) -> f64 {
        self.inner.to_number()
    }

    /// `-1`, `0` or `1`.
    #[func]
    fn sign(&self) -> i64 {
        i64::from(self.inner.sign())
    }

    /// Number of stacked `10^`; `0` for ordinary floats.
    #[func]
    fn layer(&self) -> i64 {
        self.inner.layer()
    }

    /// The magnitude at the top of the tower.
    #[func]
    fn mag(&self) -> f64 {
        self.inner.mag()
    }

    /// The mantissa in `[1, 10)` (signed); meaningless above layer 1.
    #[func]
    fn mantissa(&self) -> f64 {
        self.inner.mantissa()
    }

    /// The base-10 exponent.
    #[func]
    fn exponent(&self) -> f64 {
        self.inner.exponent()
    }

    #[func]
    fn is_zero(&self) -> bool {
        self.inner.is_zero()
    }

    #[func]
    fn is_finite(&self) -> bool {
        self.inner.is_finite()
    }

    #[func]
    fn is_infinite(&self) -> bool {
        self.inner.is_infinite()
    }

    #[func]
    fn is_integer(&self) -> bool {
        self.inner.is_integer()
    }

    // -- formatting ---------------------------------------------------------

    /// Fixed-point with `places` decimals (mantissa decimals above layer 0).
    #[func]
    fn to_fixed(&self, places: i64) -> GString {
        gstr(&self.inner.to_fixed(places.max(0) as usize))
    }

    /// `places` significant digits.
    #[func]
    fn to_precision(&self, places: i64) -> GString {
        gstr(&self.inner.to_precision(places.max(0) as usize))
    }

    /// Scientific notation with `places` mantissa decimals.
    #[func]
    fn to_exponential(&self, places: i64) -> GString {
        gstr(&self.inner.to_exponential(places.max(0) as usize))
    }

    /// Game notation: `"scientific"`, `"engineering"`, `"standard"`, `"letters"` or
    /// `"logarithm"` (case-insensitive; anything else is scientific).
    #[func]
    fn to_notation(&self, notation: GString, places: i64) -> GString {
        let notation = match String::from(notation).to_ascii_lowercase().as_str() {
            "engineering" => Notation::Engineering,
            "standard" => Notation::Standard,
            "letters" => Notation::Letters,
            "logarithm" => Notation::Logarithm,
            _ => Notation::Scientific,
        };
        gstr(&self.inner.to_notation(notation, places.max(0) as usize))
    }

    // -- arithmetic ---------------------------------------------------------

    #[func]
    fn add(&self, other: Gd<Self>) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_add(&other.bind().inner))
    }

    #[func]
    fn sub(&self, other: Gd<Self>) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_sub(&other.bind().inner))
    }

    #[func]
    fn mul(&self, other: Gd<Self>) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_mul(&other.bind().inner))
    }

    /// `null` on division by zero.
    #[func]
    fn div(&self, other: Gd<Self>) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_div(&other.bind().inner))
    }

    /// Remainder with the sign of `self` (like `%`). `null` for a zero divisor.
    #[func]
    fn rem(&self, other: Gd<Self>) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_rem(&other.bind().inner))
    }

    #[func]
    fn neg(&self) -> Gd<Self> {
        Self::wrap(-self.inner)
    }

    #[func]
    fn abs(&self) -> Gd<Self> {
        Self::wrap(self.inner.abs())
    }

    /// `1 / self`; `null` for zero.
    #[func]
    fn recip(&self) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_recip())
    }

    #[func]
    fn floor(&self) -> Gd<Self> {
        Self::wrap(self.inner.floor())
    }

    #[func]
    fn ceil(&self) -> Gd<Self> {
        Self::wrap(self.inner.ceil())
    }

    #[func]
    fn round(&self) -> Gd<Self> {
        Self::wrap(self.inner.round())
    }

    #[func]
    fn trunc(&self) -> Gd<Self> {
        Self::wrap(self.inner.trunc())
    }

    /// Rounds to `places` decimals (negative for tens, hundreds, ...).
    #[func]
    fn round_to_places(&self, places: i64) -> Gd<Self> {
        Self::wrap(self.inner.round_to_places(places.clamp(-400, 400) as i32))
    }

    /// Rounds to `digits` significant figures.
    #[func]
    fn round_to_significant(&self, digits: i64) -> Gd<Self> {
        Self::wrap(self.inner.round_to_significant(digits.clamp(0, 400) as u32))
    }

    // -- powers and logarithms ---------------------------------------------

    /// `self ^ exponent`. `null` for a negative base with a fractional exponent or `0 ^ negative`.
    #[func]
    fn pow(&self, exponent: Gd<Self>) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_pow(&exponent.bind().inner))
    }

    /// `self ^ exponent` with a float exponent.
    #[func]
    fn pow_number(&self, exponent: f64) -> Option<Gd<Self>> {
        Decimal::try_from(exponent)
            .ok()
            .and_then(|e| self.inner.checked_pow(&e).ok())
            .map(Self::new)
    }

    /// `10 ^ self`.
    #[func]
    fn pow10(&self) -> Gd<Self> {
        Self::wrap(self.inner.pow10())
    }

    /// `e ^ self`.
    #[func]
    fn exp(&self) -> Gd<Self> {
        Self::wrap(self.inner.exp())
    }

    /// `null` for a negative value.
    #[func]
    fn sqrt(&self) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_sqrt())
    }

    #[func]
    fn cbrt(&self) -> Gd<Self> {
        Self::wrap(self.inner.cbrt())
    }

    /// `degree`-th root; real for odd integer degrees of negative values.
    #[func]
    fn root(&self, degree: Gd<Self>) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_root(&degree.bind().inner))
    }

    /// Natural logarithm; `null` for non-positive values.
    #[func]
    fn ln(&self) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_ln())
    }

    /// `null` for non-positive values.
    #[func]
    fn log10(&self) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_log10())
    }

    /// `null` for non-positive values.
    #[func]
    fn log2(&self) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_log2())
    }

    /// Logarithm in an arbitrary base.
    #[func]
    fn log(&self, base: Gd<Self>) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_log(&base.bind().inner))
    }

    /// Gamma function; `null` at the poles.
    #[func]
    fn gamma(&self) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_gamma())
    }

    /// `self!` (exact up to `171!`); `null` at the poles.
    #[func]
    fn factorial(&self) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_factorial())
    }

    // -- hyperoperations ----------------------------------------------------

    /// `self ^^ height` (a power tower `height` tall). `null` when undefined.
    #[func]
    fn tetrate(&self, height: f64, linear: bool) -> Option<Gd<Self>> {
        Self::wrap_checked(
            self.inner
                .checked_tetrate(height, Decimal::one(), Self::mode(linear)),
        )
    }

    /// Super-logarithm in `base`: the height of the tower that equals `self`.
    #[func]
    fn slog(&self, base: Gd<Self>, linear: bool) -> Option<Gd<Self>> {
        Self::wrap_checked(
            self.inner
                .checked_slog(base.bind().inner, Self::mode(linear)),
        )
    }

    /// `log_base` applied `times` times (fractional `times` allowed).
    #[func]
    fn iteratedlog(&self, base: Gd<Self>, times: f64, linear: bool) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_iteratedlog(
            base.bind().inner,
            times,
            Self::mode(linear),
        ))
    }

    /// Adds `diff` to the value's height in base 10.
    #[func]
    fn layer_add_10(&self, diff: f64, linear: bool) -> Option<Gd<Self>> {
        Self::wrap_checked(
            self.inner
                .checked_layer_add_10(Decimal::from_finite(diff), Self::mode(linear)),
        )
    }

    /// Super square root: the `x` with `x ^ x = self`.
    #[func]
    fn ssqrt(&self) -> Option<Gd<Self>> {
        Self::wrap_checked(self.inner.checked_ssqrt())
    }

    /// `self ^^^ height`.
    #[func]
    fn pentate(&self, height: f64, linear: bool) -> Option<Gd<Self>> {
        Self::wrap_checked(
            self.inner
                .checked_pentate(height, Decimal::one(), Self::mode(linear)),
        )
    }

    // -- comparison ---------------------------------------------------------

    /// `-1`, `0` or `1`.
    #[func]
    fn cmp(&self, other: Gd<Self>) -> i64 {
        self.inner.cmp(&other.bind().inner) as i64
    }

    #[func]
    fn eq(&self, other: Gd<Self>) -> bool {
        self.inner == other.bind().inner
    }

    #[func]
    fn lt(&self, other: Gd<Self>) -> bool {
        self.inner < other.bind().inner
    }

    #[func]
    fn lte(&self, other: Gd<Self>) -> bool {
        self.inner <= other.bind().inner
    }

    #[func]
    fn gt(&self, other: Gd<Self>) -> bool {
        self.inner > other.bind().inner
    }

    #[func]
    fn gte(&self, other: Gd<Self>) -> bool {
        self.inner >= other.bind().inner
    }

    /// Equal within a relative `tolerance`.
    #[func]
    fn approx_eq(&self, other: Gd<Self>, tolerance: f64) -> bool {
        self.inner.approx_eq(&other.bind().inner, tolerance)
    }

    /// The smallest representable value greater than this one.
    #[func]
    fn next_up(&self) -> Gd<Self> {
        Self::wrap(self.inner.next_up())
    }

    /// The largest representable value less than this one.
    #[func]
    fn next_down(&self) -> Gd<Self> {
        Self::wrap(self.inner.next_down())
    }

    /// The spacing of representable values at this magnitude.
    #[func]
    fn ulp(&self) -> Gd<Self> {
        Self::wrap(self.inner.ulp())
    }

    /// Whether a representable value lies strictly between the two; equal or adjacent values
    /// cannot be told apart, and arithmetic between them is noise.
    #[func]
    fn distinguishable(&self, other: Gd<Self>) -> bool {
        self.inner.distinguishable(&other.bind().inner)
    }

    #[func]
    fn max(&self, other: Gd<Self>) -> Gd<Self> {
        Self::wrap(self.inner.max(other.bind().inner))
    }

    #[func]
    fn min(&self, other: Gd<Self>) -> Gd<Self> {
        Self::wrap(self.inner.min(other.bind().inner))
    }

    #[func]
    fn clamp(&self, low: Gd<Self>, high: Gd<Self>) -> Gd<Self> {
        Self::wrap(self.inner.clamp(low.bind().inner, high.bind().inner))
    }

    // -- game helpers -------------------------------------------------------

    /// How many purchases `resources` afford when each costs `ratio` times the previous,
    /// starting at `price_start`, with `owned` already bought.
    #[func]
    fn afford_geometric_series(
        resources: Gd<Self>,
        price_start: Gd<Self>,
        ratio: Gd<Self>,
        owned: Gd<Self>,
    ) -> Option<Gd<Self>> {
        Self::wrap_checked(Decimal::checked_afford_geometric_series(
            resources.bind().inner,
            price_start.bind().inner,
            ratio.bind().inner,
            owned.bind().inner,
        ))
    }

    /// Total cost of `count` purchases under a geometric price curve.
    #[func]
    fn sum_geometric_series(
        count: Gd<Self>,
        price_start: Gd<Self>,
        ratio: Gd<Self>,
        owned: Gd<Self>,
    ) -> Option<Gd<Self>> {
        Self::wrap_checked(Decimal::checked_sum_geometric_series(
            count.bind().inner,
            price_start.bind().inner,
            ratio.bind().inner,
            owned.bind().inner,
        ))
    }

    /// How many purchases `resources` afford when each costs `price_add` more than the
    /// previous.
    #[func]
    fn afford_arithmetic_series(
        resources: Gd<Self>,
        price_start: Gd<Self>,
        price_add: Gd<Self>,
        owned: Gd<Self>,
    ) -> Option<Gd<Self>> {
        Self::wrap_checked(Decimal::checked_afford_arithmetic_series(
            resources.bind().inner,
            price_start.bind().inner,
            price_add.bind().inner,
            owned.bind().inner,
        ))
    }

    /// Total cost of `count` purchases under an arithmetic price curve.
    #[func]
    fn sum_arithmetic_series(
        count: Gd<Self>,
        price_start: Gd<Self>,
        price_add: Gd<Self>,
        owned: Gd<Self>,
    ) -> Option<Gd<Self>> {
        Self::wrap_checked(Decimal::checked_sum_arithmetic_series(
            count.bind().inner,
            price_start.bind().inner,
            price_add.bind().inner,
            owned.bind().inner,
        ))
    }
}
