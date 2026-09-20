//! Core [`Decimal`] struct definition, constructors, normalization, accessors,
//! rounding, and comparison.

use core::cmp::Ordering;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::ReflectDefault;
#[cfg(all(feature = "bevy_reflect", feature = "serde"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

use crate::constants::{
    power_of_10, EXPONENT_LIMIT, FIRST_NEG_LAYER, INFINITE_LAYER, LAYER_REDUCTION_THRESHOLD,
    MAX_SAFE_LAYER,
};
use crate::error::ArithmeticError;
#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
// shadowed by std's inherent methods whenever std is in the crate graph
use crate::math::FloatExt;
use crate::utils::{f_maglog10, sign};

/// A Decimal number that can represent numbers as large as 10^^1e308 and as 'small' as 10^-(10^^1e308).
///
/// The value is `sign * 10^10^10^...(layer times) mag`. See the crate-level documentation for
/// the normalization invariant.
///
/// # Infinity
///
/// Positive and negative infinity are representable and canonical: both have
/// `mag == f64::INFINITY` and a layer above [`MAX_SAFE_LAYER`], with `sign` carrying the
/// direction. Any operation whose layer would exceed `MAX_SAFE_LAYER` saturates to infinity.
/// Infinity compares above (or below) every finite value, and `+inf - inf`, `inf * 0`, and
/// `inf / inf` are reported as [`ArithmeticErrorKind::Undefined`](crate::ArithmeticErrorKind::Undefined)
/// by the `checked_*` methods (the operator forms panic).
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(opaque, Clone, Debug, PartialEq, Hash, Default)
)]
#[cfg_attr(
    all(feature = "bevy_reflect", feature = "serde"),
    reflect(Serialize, Deserialize)
)]
pub struct Decimal {
    /// Sign of the Decimal. 1 for positive, -1 for negative, 0 for zero.
    pub(crate) sign: i8,
    /// Layer of magnitude.
    pub(crate) layer: i64,
    /// Internal mag value. Interpretation depends on `layer`: at layer 0 this is the
    /// unsigned magnitude; at layer 1 this is log10(|value|); at layer >= 2 this is the
    /// deeply-iterated log.
    pub(crate) mag: f64,
}

// ---------------------------------------------------------------------------
// Constructors and accessors
// ---------------------------------------------------------------------------

impl Decimal {
    /// Creates a new Decimal without normalization.
    pub(crate) const fn new_unchecked(sign: i8, layer: i64, mag: f64) -> Decimal {
        Decimal { sign, layer, mag }
    }

    /// Returns the mantissa of the Decimal.
    ///
    /// At layer 0 this is the signed mantissa in `[1, 10)`; at layer 1 it is `sign * 10^frac(mag)`;
    /// at higher layers the mantissa is not meaningful and the sign is returned.
    pub fn mantissa(&self) -> f64 {
        if self.sign == 0 {
            return 0.0;
        }

        if self.layer == 0 {
            let exp = self.mag.log10().floor();
            return self.sign as f64 * self.mag / power_of_10(exp as i32);
        }

        if self.layer == 1 {
            let residue = self.mag - self.mag.floor();
            return self.sign as f64 * 10.0_f64.powf(residue);
        }

        self.sign as f64
    }

    /// Sets the mantissa of the Decimal, keeping the exponent.
    ///
    /// At layers above 2 the mantissa is not meaningful, so only the sign is updated.
    pub fn set_mantissa(&mut self, m: f64) {
        if self.layer <= 2 {
            let e = self.exponent();
            self.set_from_mantissa_exponent(m, e);
        } else {
            self.sign = sign(m);
            if self.sign == 0 {
                *self = Decimal::zero();
            }
        }
    }

    /// Returns the base-10 exponent of the Decimal.
    ///
    /// At layer 0 this is `floor(log10(|x|))`; at layer 1 it is `floor(mag)`; at layer 2 it is
    /// `floor(±10^|mag|)`; beyond that it is `±infinity`.
    pub fn exponent(&self) -> f64 {
        if self.sign == 0 {
            return 0.0;
        }

        if self.layer == 0 {
            return self.mag.log10().floor();
        }

        if self.layer == 1 {
            return self.mag.floor();
        }

        if self.layer == 2 {
            return (sign(self.mag) as f64 * 10.0_f64.powf(self.mag.abs())).floor();
        }

        self.mag * f64::INFINITY
    }

    /// Sets the exponent of the Decimal, keeping the mantissa.
    pub fn set_exponent(&mut self, e: f64) {
        let m = self.mantissa();
        self.set_from_mantissa_exponent(m, e);
    }

    /// Returns the sign of the Decimal: `1`, `0`, or `-1`.
    pub fn sign(&self) -> i8 {
        self.sign
    }

    /// Returns the layer of the Decimal.
    ///
    /// For infinity this is a sentinel value above [`MAX_SAFE_LAYER`].
    pub fn layer(&self) -> i64 {
        self.layer
    }

    /// Returns the internal mag value.
    ///
    /// Interpretation depends on `layer`: at layer 0 this is the unsigned magnitude; at layer 1
    /// this is log10(|value|); at layer >= 2 this is the deeply-iterated log.
    pub fn mag(&self) -> f64 {
        self.mag
    }

    /// Creates a Decimal from a sign, a layer and a magnitude.
    ///
    /// `sign` is reduced to its signum, so any positive value means `+` and any negative value
    /// means `-`. The result is normalized.
    ///
    /// # Panics
    ///
    /// Panics if `layer` is negative.
    pub fn from_components(sign: i8, layer: i64, mag: f64) -> Decimal {
        assert!(
            layer >= 0,
            "Decimal::from_components: layer must be non-negative, got {layer}"
        );
        Decimal::default().set_from_components(sign.signum(), layer, mag)
    }

    /// Creates a Decimal from a sign, a layer and a magnitude without normalizing.
    ///
    /// The caller must ensure the inputs already satisfy the normalization invariant.
    pub(crate) const fn from_components_unchecked(sign: i8, layer: i64, mag: f64) -> Decimal {
        Decimal { sign, layer, mag }
    }

    /// Creates a Decimal from a mantissa and an exponent.
    ///
    /// This function normalizes the inputs.
    pub fn from_mantissa_exponent(m: f64, e: f64) -> Decimal {
        Decimal::default().set_from_mantissa_exponent(m, e)
    }

    /// Creates a `Decimal` from a finite `f64`.
    ///
    /// In debug builds this asserts that `x` is finite. In release builds the assertion is
    /// elided; an infinite input then produces the corresponding infinity and a NaN input
    /// produces an unspecified value. Use [`TryFrom<f64>`] for an explicit fallible path.
    ///
    /// # Panics (debug only)
    ///
    /// Panics if `x` is NaN or infinite.
    pub fn from_finite(x: f64) -> Decimal {
        debug_assert!(
            x.is_finite(),
            "from_finite called with non-finite value: {x}"
        );
        Decimal::from_f64(x)
    }

    /// Creates a `Decimal` from any `f64`, mapping `±inf` to the canonical infinities and NaN
    /// to the internal NaN sentinel.
    pub(crate) fn from_f64(x: f64) -> Decimal {
        Decimal::default().set_from_number(x)
    }

    /// Normalizes the Decimal in place and returns a copy.
    ///
    /// * Any NaN component makes the value the internal NaN sentinel.
    /// * Any zero (sign 0, or `mag == 0` at layer 0, or `mag == -inf` above layer 0) becomes
    ///   `(0, 0, 0)`.
    /// * At layer 0 the sign is extracted from a negative `mag`.
    /// * An infinite `mag`, or a layer above [`MAX_SAFE_LAYER`], becomes the canonical infinity.
    /// * If `layer == 0` and `mag < FIRST_NEG_LAYER` (1/9e15), shift to the first negative layer.
    /// * If `|mag| >= EXP_LIMIT` (9e15), increment the layer and take `log10`.
    /// * While `|mag| < LAYER_DOWN` (15.954) and `layer > 0`, decrement the layer and take `10^mag`.
    /// * `-0.0` is canonicalized to `0.0` so that `Eq` and `Hash` agree.
    pub fn normalize(&mut self) -> Decimal {
        if self.mag.is_nan() {
            *self = Decimal::nan_sentinel();
            return *self;
        }

        if self.sign == 0
            || (self.mag == 0.0 && self.layer == 0)
            || (self.mag == f64::NEG_INFINITY && self.layer > 0)
        {
            *self = Decimal::zero();
            return *self;
        }

        if self.layer == 0 && self.mag < 0.0 {
            self.mag = -self.mag;
            self.sign = -self.sign;
        }

        if self.mag.is_infinite() || self.layer > MAX_SAFE_LAYER {
            self.layer = INFINITE_LAYER;
            self.mag = f64::INFINITY;
            return *self;
        }

        if self.layer == 0 && self.mag < FIRST_NEG_LAYER {
            self.layer += 1;
            self.mag = self.mag.log10();
            return *self;
        }

        let mut abs_mag = self.mag.abs();
        let mut sign_mag = sign(self.mag) as f64;

        if abs_mag >= EXPONENT_LIMIT {
            self.layer += 1;
            self.mag = sign_mag * abs_mag.log10();
            if self.layer > MAX_SAFE_LAYER {
                self.layer = INFINITE_LAYER;
                self.mag = f64::INFINITY;
            }
            return *self;
        }

        while abs_mag < LAYER_REDUCTION_THRESHOLD && self.layer > 0 {
            self.layer -= 1;
            if self.layer == 0 {
                self.mag = 10.0_f64.powf(self.mag);
            } else {
                self.mag = sign_mag * 10.0_f64.powf(abs_mag);
                abs_mag = self.mag.abs();
                sign_mag = sign(self.mag) as f64;
            }
        }

        if self.layer == 0 {
            if self.mag < 0.0 {
                self.mag = -self.mag;
                self.sign = -self.sign;
            } else if self.mag == 0.0 {
                self.sign = 0;
            }
        }

        // Canonicalize -0.0 to 0.0 so that Eq/Hash are consistent.
        if self.mag == 0.0 {
            self.mag = 0.0;
        }

        *self
    }

    /// Sets the components of the Decimal from a sign, a layer and a magnitude.
    ///
    /// This function normalizes the inputs.
    pub fn set_from_components(&mut self, sign: i8, layer: i64, mag: f64) -> Decimal {
        self.sign = sign;
        self.layer = layer;
        self.mag = mag;

        self.normalize();
        *self
    }

    /// Sets the components of the Decimal from a mantissa and an exponent.
    ///
    /// This function normalizes the inputs.
    pub fn set_from_mantissa_exponent(&mut self, m: f64, e: f64) -> Decimal {
        self.layer = 1;
        self.sign = sign(m);
        let mant = m.abs();
        self.mag = e + mant.log10();

        self.normalize();
        *self
    }

    /// Sets the components of the Decimal from a number (f64).
    ///
    /// Infinite inputs become the canonical infinities; NaN becomes an internal sentinel that
    /// every `checked_*` method reports as an error.
    pub fn set_from_number(&mut self, n: f64) -> Decimal {
        self.sign = sign(n);
        self.layer = 0;
        self.mag = n.abs();

        self.normalize();
        *self
    }

    /// Returns the Decimal as a number (f64).
    ///
    /// Values beyond the `f64` range saturate to `±inf` (or `0` for tiny values).
    pub fn to_number(&self) -> f64 {
        if self.is_infinite() {
            return self.sign as f64 * f64::INFINITY;
        }

        if self.layer == 0 {
            return self.sign as f64 * self.mag;
        }

        if self.layer == 1 {
            return self.sign as f64 * 10.0_f64.powf(self.mag);
        }

        if self.mag > 0.0 {
            if self.sign > 0 {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            }
        } else {
            0.0
        }
    }

    /// Returns the mantissa rounded to the specified number of decimal places.
    pub fn mantissa_with_decimal_places(&self, places: i32) -> f64 {
        let m = self.mantissa();
        if m == 0.0 {
            return 0.0;
        }

        crate::format::decimal_places(m, places)
    }

    /// Returns the magnitude rounded to the specified number of decimal places.
    pub fn magnitude_with_decimal_places(&self, places: i32) -> f64 {
        if self.mag == 0.0 {
            return 0.0;
        }

        crate::format::decimal_places(self.mag, places)
    }

    // -----------------------------------------------------------------------
    // Predicates
    // -----------------------------------------------------------------------

    /// Returns `true` if the value is neither infinite nor the internal NaN sentinel.
    pub fn is_finite(&self) -> bool {
        self.mag.is_finite()
    }

    /// Returns `true` if the value is positive or negative infinity.
    pub fn is_infinite(&self) -> bool {
        self.mag.is_infinite()
    }

    /// Returns `true` if the value is exactly zero.
    pub fn is_zero(&self) -> bool {
        self.sign == 0
    }

    /// Returns `true` if the value is greater than zero.
    pub fn is_positive(&self) -> bool {
        self.sign > 0
    }

    /// Returns `true` if the value is less than zero.
    pub fn is_negative(&self) -> bool {
        self.sign < 0
    }

    /// Returns true if this Decimal carries a NaN mag (internal sentinel state).
    pub(crate) fn has_nan_mag(&self) -> bool {
        self.mag.is_nan()
    }

    /// Converts an internal NaN sentinel into an `Undefined` error for operation `op`.
    pub(crate) fn nan_to_err(self, op: &'static str) -> Result<Decimal, ArithmeticError> {
        if self.has_nan_mag() {
            Err(ArithmeticError::undefined(op))
        } else {
            Ok(self)
        }
    }

    // -----------------------------------------------------------------------
    // Constants
    // -----------------------------------------------------------------------

    /// Returns the absolute value of the Decimal.
    pub fn abs(&self) -> Decimal {
        Decimal::from_components_unchecked(i8::from(self.sign != 0), self.layer, self.mag)
    }

    /// Returns a zero Decimal.
    pub const fn zero() -> Decimal {
        Decimal::new_unchecked(0, 0, 0.0)
    }

    /// Returns a one Decimal.
    pub const fn one() -> Decimal {
        Decimal::new_unchecked(1, 0, 1.0)
    }

    /// Returns a negative one Decimal.
    pub const fn neg_one() -> Decimal {
        Decimal::new_unchecked(-1, 0, 1.0)
    }

    /// Returns a two Decimal.
    pub const fn two() -> Decimal {
        Decimal::new_unchecked(1, 0, 2.0)
    }

    /// Returns a ten Decimal.
    pub const fn ten() -> Decimal {
        Decimal::new_unchecked(1, 0, 10.0)
    }

    /// Returns an internal NaN sentinel used during transient iteration loops.
    ///
    /// Not exposed publicly — every `checked_*` method converts it into an error before
    /// returning, and the panicking forms panic instead.
    pub(crate) const fn nan_sentinel() -> Decimal {
        Decimal::new_unchecked(0, 0, f64::NAN)
    }

    /// Returns positive infinity.
    pub const fn inf() -> Decimal {
        Decimal::new_unchecked(1, INFINITE_LAYER, f64::INFINITY)
    }

    /// Returns negative infinity.
    pub const fn neg_inf() -> Decimal {
        Decimal::new_unchecked(-1, INFINITE_LAYER, f64::INFINITY)
    }

    /// Returns the largest finite `f64` (`f64::MAX`) lifted into a Decimal.
    pub fn maximum() -> Decimal {
        Decimal::from_finite(f64::MAX)
    }

    /// Returns the smallest positive normal `f64` (`f64::MIN_POSITIVE`) lifted into a Decimal.
    pub fn minimum() -> Decimal {
        Decimal::from_finite(f64::MIN_POSITIVE)
    }

    /// Returns the largest Decimal at which adding one to the layer is still a safe operation,
    /// approximately `10^^9e15`. Larger values saturate to [`inf`](Self::inf).
    pub fn layer_safe_max() -> Decimal {
        Decimal::from_components_unchecked(1, MAX_SAFE_LAYER, EXPONENT_LIMIT - 1.0)
    }

    /// Returns the smallest positive Decimal at which subtracting one from the layer is still a
    /// safe operation, approximately `1 / 10^^9e15`.
    pub fn layer_safe_min() -> Decimal {
        Decimal::from_components_unchecked(1, MAX_SAFE_LAYER, -(EXPONENT_LIMIT - 1.0))
    }

    // -----------------------------------------------------------------------
    // Rounding
    // -----------------------------------------------------------------------

    /// Rounds the Decimal to the nearest integer (half away from zero).
    pub fn round(&self) -> Decimal {
        if self.mag < 0.0 {
            return Decimal::zero();
        }

        if self.layer == 0 {
            return Decimal::from_f64((self.sign as f64 * self.mag).round());
        }

        *self
    }

    /// Returns the largest integer less than or equal to the Decimal.
    pub fn floor(&self) -> Decimal {
        if self.mag < 0.0 {
            return if self.sign == -1 {
                Decimal::neg_one()
            } else {
                Decimal::zero()
            };
        }

        if self.layer == 0 {
            return Decimal::from_f64((self.sign as f64 * self.mag).floor());
        }

        *self
    }

    /// Returns the smallest integer greater than or equal to the Decimal.
    pub fn ceil(&self) -> Decimal {
        if self.mag < 0.0 {
            return if self.sign == 1 {
                Decimal::one()
            } else {
                Decimal::zero()
            };
        }

        if self.layer == 0 {
            return Decimal::from_f64((self.sign as f64 * self.mag).ceil());
        }

        *self
    }

    /// Returns the integer part of the Decimal (rounding toward zero).
    pub fn trunc(&self) -> Decimal {
        if self.mag < 0.0 {
            return Decimal::zero();
        }

        if self.layer == 0 {
            return Decimal::from_f64((self.sign as f64 * self.mag).trunc());
        }

        *self
    }

    // -----------------------------------------------------------------------
    // Comparison, min/max, clamp
    // -----------------------------------------------------------------------

    /// Compares the absolute values of two Decimals.
    ///
    /// Returns an [`Ordering`] indicating the relative order of `|self|` and `|rhs|`.
    pub fn cmpabs(&self, rhs: &Decimal) -> Ordering {
        let layer_a = if self.mag > 0.0 {
            self.layer
        } else {
            -self.layer
        };
        let layer_b = if rhs.mag > 0.0 { rhs.layer } else { -rhs.layer };

        layer_a
            .cmp(&layer_b)
            .then_with(|| self.mag.partial_cmp(&rhs.mag).unwrap_or(Ordering::Equal))
    }

    /// Returns whichever of the two Decimals has the larger absolute value.
    pub fn maxabs(&self, rhs: Decimal) -> Decimal {
        if self.cmpabs(&rhs).is_gt() {
            *self
        } else {
            rhs
        }
    }

    /// Returns whichever of the two Decimals has the smaller absolute value.
    pub fn minabs(&self, rhs: Decimal) -> Decimal {
        if self.cmpabs(&rhs).is_gt() {
            rhs
        } else {
            *self
        }
    }

    /// Returns the bigger of the two Decimals.
    pub fn max(&self, other: Decimal) -> Decimal {
        if self > &other {
            *self
        } else {
            other
        }
    }

    /// Returns the smaller of the two Decimals.
    pub fn min(&self, other: Decimal) -> Decimal {
        if self < &other {
            *self
        } else {
            other
        }
    }

    /// Clamps the Decimal to the given range.
    pub fn clamp(&self, min: Decimal, max: Decimal) -> Decimal {
        self.max(min).min(max)
    }

    /// Clamps the Decimal to a minimum value.
    pub fn clamp_min(&self, min: Decimal) -> Decimal {
        self.max(min)
    }

    /// Clamps the Decimal to a maximum value.
    pub fn clamp_max(&self, max: Decimal) -> Decimal {
        self.min(max)
    }

    // -----------------------------------------------------------------------
    // Tolerance comparisons
    // -----------------------------------------------------------------------

    /// Returns true if `self` and `other` are approximately equal within the given relative tolerance.
    ///
    /// Tolerance is relative: two numbers are considered equal if their difference is no greater
    /// than `tolerance * max(|self.mag|, |other.mag|)` (after adjusting for layer differences).
    /// Infinities are approximately equal only to themselves.
    ///
    /// # Example
    ///
    /// ```
    /// use break_eternity::Decimal;
    ///
    /// let a = Decimal::from_finite(1.0);
    /// let b = Decimal::from_finite(1.0 + 1e-11);
    /// assert!(a.approx_eq(&b, 1e-10));
    /// assert!(a != b);
    /// ```
    pub fn approx_eq(&self, other: &Decimal, tolerance: f64) -> bool {
        if self.sign != other.sign {
            return false;
        }

        if self.is_infinite() || other.is_infinite() {
            return self == other;
        }

        if (self.layer - other.layer).abs() > 1 {
            return false;
        }

        let mut mag_a = self.mag;
        let mut mag_b = other.mag;
        if self.layer > other.layer {
            mag_b = f_maglog10(mag_b);
        }
        if other.layer > self.layer {
            mag_a = f_maglog10(mag_a);
        }

        (mag_a - mag_b).abs() <= tolerance * mag_a.abs().max(mag_b.abs())
    }

    /// Returns true if `self` and `other` are *not* approximately equal within `tolerance`.
    pub fn approx_ne(&self, other: &Decimal, tolerance: f64) -> bool {
        !self.approx_eq(other, tolerance)
    }

    /// Compares two Decimals, treating values within `tolerance` of each other as equal.
    pub fn cmp_tolerance(&self, other: &Decimal, tolerance: f64) -> Ordering {
        if self.approx_eq(other, tolerance) {
            Ordering::Equal
        } else {
            self.cmp(other)
        }
    }

    /// Returns true if `self < other` and the two are not within `tolerance` of each other.
    pub fn approx_lt(&self, other: &Decimal, tolerance: f64) -> bool {
        !self.approx_eq(other, tolerance) && self < other
    }

    /// Returns true if `self < other` or the two are within `tolerance` of each other.
    pub fn approx_le(&self, other: &Decimal, tolerance: f64) -> bool {
        self.approx_eq(other, tolerance) || self < other
    }

    /// Returns true if `self > other` and the two are not within `tolerance` of each other.
    pub fn approx_gt(&self, other: &Decimal, tolerance: f64) -> bool {
        !self.approx_eq(other, tolerance) && self > other
    }

    /// Returns true if `self > other` or the two are within `tolerance` of each other.
    pub fn approx_ge(&self, other: &Decimal, tolerance: f64) -> bool {
        self.approx_eq(other, tolerance) || self > other
    }

    // -----------------------------------------------------------------------
    // Resolution
    // -----------------------------------------------------------------------

    /// Returns the smallest representable Decimal greater than `self`: [`f64::next_up`] over
    /// the whole `(sign, layer, mag)` lattice.
    ///
    /// Within a layer this steps `mag` by one `f64` ulp; at the edges of a layer it crosses to
    /// the neighbouring layer exactly where normalization would, so the result is always the
    /// immediate successor in the crate's total order. Zero's successor is
    /// [`layer_safe_min`](Self::layer_safe_min), the smallest positive value;
    /// [`layer_safe_max`](Self::layer_safe_max) steps to infinity; infinity is its own successor.
    ///
    /// # Example
    ///
    /// ```
    /// use break_eternity::Decimal;
    ///
    /// let one = Decimal::one();
    /// assert_eq!(one.next_up(), Decimal::from_finite(1.0 + f64::EPSILON));
    /// assert_eq!(one.next_up().next_down(), one);
    ///
    /// // 9e15 is where layer 0 hands over to layer 1.
    /// let last_layer0 = Decimal::from_finite(8_999_999_999_999_999.0);
    /// assert_eq!(last_layer0.next_up(), Decimal::from_finite(9e15));
    /// assert_eq!(last_layer0.next_up().layer(), 1);
    /// ```
    pub fn next_up(&self) -> Decimal {
        match self.sign {
            0 => Decimal::layer_safe_min(),
            1 => self.step_away_from_zero(),
            _ => -self.abs().step_toward_zero(),
        }
    }

    /// Returns the largest representable Decimal less than `self`: the mirror of
    /// [`next_up`](Self::next_up). Zero's predecessor is `-layer_safe_min()`, infinity's is
    /// [`layer_safe_max`](Self::layer_safe_max), and negative infinity is its own predecessor.
    pub fn next_down(&self) -> Decimal {
        match self.sign {
            0 => -Decimal::layer_safe_min(),
            1 => self.step_toward_zero(),
            _ => -self.abs().step_away_from_zero(),
        }
    }

    /// Returns the spacing of representable values at `self`: the distance from `|self|` to the
    /// next representable value of greater magnitude.
    ///
    /// * At layer 0 (`|self| < 9e15`) this is one `f64` ulp of the value: `1.ulp()` is
    ///   `f64::EPSILON` and `2^52.ulp()` is `1`.
    /// * At layer 1 (`10^mag`) it is `10^mag · (10^ulp(mag) − 1)`. The relative resolution
    ///   grows with the exponent: about `1.5e-13` of the value at `1e300`, so the spacing there
    ///   is `1.5e287`, and about `2e-6` of the value at `1e10000000`.
    /// * From layer 2 up the spacing exceeds the value itself, so this returns the successor.
    ///
    /// Zero returns [`layer_safe_min`](Self::layer_safe_min) and infinity returns infinity. The
    /// result is the same for `self` and `-self`.
    ///
    /// # Example
    ///
    /// ```
    /// use break_eternity::Decimal;
    ///
    /// assert_eq!(Decimal::one().ulp(), Decimal::from_finite(f64::EPSILON));
    /// assert_eq!(Decimal::from_finite(4503599627370496.0).ulp(), Decimal::one());
    ///
    /// let balance: Decimal = "1e300".parse().unwrap();
    /// let spacing = balance.ulp();
    /// assert!(spacing > "1e287".parse::<Decimal>().unwrap());
    /// assert!(spacing < "2e287".parse::<Decimal>().unwrap());
    /// // A cost below the spacing cannot register against the balance.
    /// let cost: Decimal = "1e286".parse().unwrap();
    /// assert!(cost < spacing);
    /// assert_eq!(balance - cost, balance);
    /// ```
    pub fn ulp(&self) -> Decimal {
        if self.sign == 0 {
            return Decimal::layer_safe_min();
        }
        if self.is_infinite() {
            return Decimal::inf();
        }
        match self.layer {
            0 => Decimal::from_finite(self.mag.next_up() - self.mag),
            1 => {
                // 10^mag · (10^step − 1) with step one ulp of mag (an exact difference); in
                // log form that is mag + log10(expm1(step · ln 10)), and expm1 keeps the full
                // precision that 10^step − 1 would lose to cancellation.
                let step = self.mag.next_up() - self.mag;
                Decimal::from_components(
                    1,
                    1,
                    self.mag + (step * core::f64::consts::LN_10).exp_m1().log10(),
                )
            }
            _ => self.abs().next_up(),
        }
    }

    /// Returns true if the representation can tell `self` and `other` apart: they differ, and
    /// at least one representable value lies strictly between them.
    ///
    /// Equal values are not distinguishable, and neither are adjacent ones (`x` and
    /// `x.next_up()`): they differ by a single rounding step, so the true quantities behind
    /// them could sit on either side of each other and any arithmetic between them is noise.
    /// This is the check for the relative-precision hazard of a layered representation. At
    /// `1e300` the spacing is about `1e287`, so `1e300 + 1e286` *is* `1e300`, and
    /// `1e300 + 1e287` is one step away from it. An economy that must not act on a
    /// difference the numbers cannot carry can treat a cost that is indistinguishable from the
    /// balance as unaffordable.
    ///
    /// Unlike [`approx_eq`](Self::approx_eq) there is no tolerance to choose: the threshold is
    /// the representation's own resolution, which changes with layer and magnitude. For a wider
    /// margin, compare the difference against a multiple of [`ulp`](Self::ulp).
    ///
    /// # Example
    ///
    /// ```
    /// use break_eternity::Decimal;
    ///
    /// let balance: Decimal = "1e300".parse().unwrap();
    /// let close: Decimal = "1.0000000000001e300".parse().unwrap();
    /// let clear: Decimal = "1.000000001e300".parse().unwrap();
    ///
    /// assert!(balance != close);
    /// assert!(!balance.distinguishable(&close));
    /// assert!(balance.distinguishable(&clear));
    /// assert!(Decimal::one().distinguishable(&Decimal::two()));
    /// assert!(!balance.distinguishable(&balance));
    /// ```
    pub fn distinguishable(&self, other: &Decimal) -> bool {
        match self.cmp(other) {
            Ordering::Equal => false,
            Ordering::Less => self.next_up() != *other,
            Ordering::Greater => other.next_up() != *self,
        }
    }

    /// Successor of a positive value in the lattice.
    ///
    /// The lattice, in increasing order, is `(L, -9e15+1) … (L, -T)` for `L` from
    /// `MAX_SAFE_LAYER` down to 1, then `(0, 1/9e15) … (0, 9e15-1)`, then `(L, T) … (L, 9e15-1)`
    /// for `L` from 1 up, where `T` is [`LAYER_REDUCTION_THRESHOLD`] (`log10(9e15)`). Stepping
    /// off the end of a block lands on the first entry of the next one, which is the value
    /// normalization would produce for `mag == ±9e15`.
    fn step_away_from_zero(self) -> Decimal {
        if self.is_infinite() {
            return self;
        }
        let Decimal { layer, mag, .. } = self;
        if layer == 0 {
            let up = mag.next_up();
            return if up >= EXPONENT_LIMIT {
                Decimal::from_components_unchecked(1, 1, LAYER_REDUCTION_THRESHOLD)
            } else {
                Decimal::from_components_unchecked(1, 0, up)
            };
        }
        if mag > 0.0 {
            let up = mag.next_up();
            return if up < EXPONENT_LIMIT {
                Decimal::from_components_unchecked(1, layer, up)
            } else if layer >= MAX_SAFE_LAYER {
                Decimal::inf()
            } else {
                Decimal::from_components_unchecked(1, layer + 1, LAYER_REDUCTION_THRESHOLD)
            };
        }
        // Negative mag is a value below 1/9e15; a less negative mag is a larger value.
        if mag == -LAYER_REDUCTION_THRESHOLD {
            return if layer == 1 {
                Decimal::from_components_unchecked(1, 0, FIRST_NEG_LAYER)
            } else {
                Decimal::from_components_unchecked(1, layer - 1, -EXPONENT_LIMIT.next_down())
            };
        }
        Decimal::from_components_unchecked(1, layer, mag.next_up())
    }

    /// Predecessor of a positive value in the lattice; see
    /// [`step_away_from_zero`](Self::step_away_from_zero).
    fn step_toward_zero(self) -> Decimal {
        if self.is_infinite() {
            return Decimal::layer_safe_max();
        }
        let Decimal { layer, mag, .. } = self;
        if layer == 0 {
            return if mag == FIRST_NEG_LAYER {
                Decimal::from_components_unchecked(1, 1, -LAYER_REDUCTION_THRESHOLD)
            } else {
                Decimal::from_components_unchecked(1, 0, mag.next_down())
            };
        }
        if mag > 0.0 {
            return if mag == LAYER_REDUCTION_THRESHOLD {
                Decimal::from_components_unchecked(1, layer - 1, EXPONENT_LIMIT.next_down())
            } else {
                Decimal::from_components_unchecked(1, layer, mag.next_down())
            };
        }
        let down = mag.next_down();
        if down > -EXPONENT_LIMIT {
            Decimal::from_components_unchecked(1, layer, down)
        } else if layer >= MAX_SAFE_LAYER {
            Decimal::zero()
        } else {
            Decimal::from_components_unchecked(1, layer + 1, -LAYER_REDUCTION_THRESHOLD)
        }
    }
}

// ---------------------------------------------------------------------------
// Trait impls that need to live alongside the struct definition
// ---------------------------------------------------------------------------

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.sign == other.sign
            && self.layer == other.layer
            && self.mag.to_bits() == other.mag.to_bits()
    }
}

impl Eq for Decimal {}

impl core::hash::Hash for Decimal {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.sign.hash(state);
        self.layer.hash(state);
        self.mag.to_bits().hash(state);
    }
}

// Ord::cmp and partial_cmp agree on every value a public API can return; they differ only on
// the crate-internal NaN sentinel, where partial_cmp is None so that the ported JS search
// loops see the same (all-false) comparisons as upstream.
#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for Decimal {
    /// Total order on every public value. The crate-internal NaN sentinel (never returned by a
    /// public API) compares as unordered here, so the `<`/`>` operators are false against it,
    /// mirroring JavaScript and keeping the ported search loops on the same control-flow path.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.has_nan_mag() || other.has_nan_mag() {
            return None;
        }
        Some(self.cmp(other))
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sign.cmp(&other.sign).then_with(|| match self.sign {
            1 => self.cmpabs(other),
            -1 => other.cmpabs(self),
            _ => Ordering::Equal,
        })
    }
}

// ---------------------------------------------------------------------------
// TryFrom conversions
// ---------------------------------------------------------------------------

impl TryFrom<f64> for Decimal {
    type Error = ArithmeticError;

    /// Converts a finite `f64`. NaN and `±inf` are rejected; use [`Decimal::inf`] /
    /// [`Decimal::neg_inf`] for explicit infinities.
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if !value.is_finite() {
            return Err(ArithmeticError::undefined("from_f64"));
        }
        Ok(Decimal::from_f64(value))
    }
}

impl TryFrom<f32> for Decimal {
    type Error = ArithmeticError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Decimal::try_from(value as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn normalize_canonicalizes_neg_zero() {
        let mut d = Decimal {
            sign: 1,
            layer: 0,
            mag: -0.0_f64,
        };
        d.normalize();
        assert_eq!(d, Decimal::zero());
    }

    #[test]
    fn normalize_canonicalizes_infinities() {
        assert_eq!(Decimal::from_f64(f64::INFINITY), Decimal::inf());
        assert_eq!(Decimal::from_f64(f64::NEG_INFINITY), Decimal::neg_inf());
        assert_eq!(
            Decimal::from_components(1, 0, f64::INFINITY),
            Decimal::inf()
        );
        assert_eq!(
            Decimal::from_components(-1, 5, f64::INFINITY),
            Decimal::neg_inf()
        );
        assert_eq!(
            Decimal::from_components(1, MAX_SAFE_LAYER + 1, 100.0),
            Decimal::inf()
        );
        // 10^(-inf) at any positive layer is zero.
        assert_eq!(
            Decimal::from_components(1, 1, f64::NEG_INFINITY),
            Decimal::zero()
        );
        assert_eq!(-Decimal::inf(), Decimal::neg_inf());
        assert_eq!(Decimal::neg_inf().abs(), Decimal::inf());
    }

    #[test]
    fn infinity_orders_above_everything() {
        let huge: Decimal = "(e^1000)10".parse().unwrap();
        assert!(Decimal::inf() > huge);
        assert!(Decimal::neg_inf() < -huge);
        assert!(Decimal::inf() > Decimal::layer_safe_max());
        assert_eq!(huge.max(Decimal::inf()), Decimal::inf());
        assert_eq!(Decimal::inf().clamp_max(Decimal::ten()), Decimal::ten());
        assert_eq!(Decimal::inf().cmp(&Decimal::inf()), Ordering::Equal);
        assert_eq!(Decimal::inf().to_number(), f64::INFINITY);
        assert_eq!(Decimal::neg_inf().to_number(), f64::NEG_INFINITY);
    }

    #[test]
    fn try_from_nan_errors() {
        assert!(Decimal::try_from(f64::NAN).is_err());
    }

    #[test]
    fn try_from_infinity_errors() {
        assert!(Decimal::try_from(f64::INFINITY).is_err());
        assert!(Decimal::try_from(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn try_from_finite_ok() {
        let d = Decimal::try_from(1.23456789_f64).unwrap();
        assert!((d.to_number() - 1.23456789).abs() < 1e-12);
    }

    #[test]
    fn from_finite_roundtrip() {
        let d = Decimal::from_finite(42.0);
        assert_eq!(d.to_number(), 42.0);
    }

    #[test]
    fn from_components_normalizes_sign() {
        assert_eq!(Decimal::from_components(5, 0, 5.0).sign(), 1);
        assert_eq!(Decimal::from_components(-7, 0, 5.0).sign(), -1);
    }

    #[test]
    #[should_panic(expected = "layer must be non-negative")]
    fn from_components_rejects_negative_layer() {
        let _ = Decimal::from_components(1, -1, 5.0);
    }

    #[test]
    fn eq_and_hash_consistent() {
        use std::collections::HashMap;
        let a = Decimal::from_finite(1.0);
        let b = Decimal::from_finite(1.0);
        assert_eq!(a, b);

        let mut map = HashMap::new();
        map.insert(a, "found");
        assert_eq!(map.get(&b), Some(&"found"));
    }

    #[test]
    fn cmpabs_returns_ordering() {
        let small = Decimal::from_finite(1.0);
        let large = Decimal::from_finite(2.0);
        assert!(small.cmpabs(&large).is_lt());
        assert!(large.cmpabs(&small).is_gt());
        assert!(small.cmpabs(&small).is_eq());
    }

    #[test]
    fn ord_negative_less_than_positive() {
        let neg = Decimal::from_finite(-5.0);
        let pos = Decimal::from_finite(3.0);
        assert!(neg < pos);
    }

    #[test]
    fn next_up_and_next_down_walk_the_lattice() {
        let t = LAYER_REDUCTION_THRESHOLD;
        let top = EXPONENT_LIMIT.next_down(); // 9e15 - 1
        let c = Decimal::from_components_unchecked;
        // Every lattice edge, including each layer boundary in both directions.
        let edges: [(Decimal, Decimal); 12] = [
            (Decimal::zero(), Decimal::layer_safe_min()),
            (
                Decimal::layer_safe_min(),
                Decimal::layer_safe_min().next_up(),
            ),
            (c(1, 2, -t), c(1, 1, -top)),
            (c(1, 1, -t), c(1, 0, FIRST_NEG_LAYER)),
            (c(1, 0, FIRST_NEG_LAYER), c(1, 0, FIRST_NEG_LAYER.next_up())),
            (Decimal::one(), Decimal::from_finite(1.0 + f64::EPSILON)),
            (c(1, 0, top), c(1, 1, t)),
            (c(1, 1, top), c(1, 2, t)),
            (c(1, 5, t), c(1, 5, t.next_up())),
            (c(1, 7, -20.0), c(1, 7, (-20.0f64).next_up())),
            (Decimal::layer_safe_max(), Decimal::inf()),
            (Decimal::inf(), Decimal::inf()),
        ];
        for (lo, hi) in edges {
            assert_eq!(lo.next_up(), hi, "next_up of {lo:?}");
            if hi.is_finite() {
                assert_eq!(hi.next_down(), lo, "next_down of {hi:?}");
            }
            if lo != hi {
                assert!(lo < hi, "{lo:?} < {hi:?}");
            }
            // Mirror image on the negative side.
            if lo != hi {
                assert_eq!((-hi).next_up(), -lo, "next_up of {:?}", -hi);
            }
            if hi.is_finite() {
                assert_eq!((-lo).next_down(), -hi, "next_down of {:?}", -lo);
            }
        }
        // The layer-0 / layer-1 handover is exactly where normalization puts it.
        assert_eq!(c(1, 0, top).next_up(), Decimal::from_finite(9e15));
        assert_eq!(c(1, 0, top).next_up().layer(), 1);
        assert_eq!(Decimal::neg_inf().next_down(), Decimal::neg_inf());
        assert_eq!(Decimal::neg_inf().next_up(), -Decimal::layer_safe_max());
        assert_eq!(Decimal::inf().next_down(), Decimal::layer_safe_max());
    }

    #[test]
    fn ulp_is_the_spacing_at_each_layer() {
        assert_eq!(Decimal::one().ulp(), Decimal::from_finite(f64::EPSILON));
        assert_eq!(Decimal::from(-1).ulp(), Decimal::from_finite(f64::EPSILON));
        assert_eq!(
            Decimal::from_finite(4503599627370496.0).ulp(),
            Decimal::one()
        );
        assert_eq!(
            Decimal::from_finite(8999999999999999.0).ulp(),
            Decimal::one()
        );
        assert_eq!(Decimal::zero().ulp(), Decimal::layer_safe_min());
        assert_eq!(Decimal::inf().ulp(), Decimal::inf());
        assert_eq!(Decimal::neg_inf().ulp(), Decimal::inf());

        // Layer 1: 10^300 · ln(10) · ulp(300), and it matches the successor's distance.
        let big: Decimal = "1e300".parse().unwrap();
        let expected = 1e300 * core::f64::consts::LN_10 * (300.0f64.next_up() - 300.0);
        let expected = Decimal::from_finite(expected);
        assert!(
            big.ulp().approx_eq(&expected, 1e-9),
            "{} vs {expected}",
            big.ulp()
        );
        // The subtraction loses a few digits to cancellation in 10^step - 1; the analytic
        // form is the accurate one, so only agreement in the leading digits is expected.
        assert!(
            big.ulp().approx_eq(&(big.next_up() - big), 1e-2),
            "{} vs {}",
            big.ulp(),
            big.next_up() - big
        );
        assert_eq!((-big).ulp(), big.ulp());
        // Tiny values live at layer 1 with a negative mag.
        let tiny: Decimal = "1e-20".parse().unwrap();
        let expected = 1e-20 * core::f64::consts::LN_10 * ((-20.0f64).next_up() + 20.0);
        assert!(tiny.ulp().approx_eq(&Decimal::from_finite(expected), 1e-9));
        // Layer 2 and up: the spacing is the successor itself.
        let tower: Decimal = "ee400".parse().unwrap();
        assert_eq!(tower.ulp(), tower.next_up());
    }

    #[test]
    fn distinguishable_needs_a_value_in_between() {
        let one = Decimal::one();
        assert!(one.distinguishable(&Decimal::two()));
        assert!(!one.distinguishable(&one));
        assert!(!one.distinguishable(&one.next_up()));
        assert!(!one.next_up().distinguishable(&one));
        assert!(one.distinguishable(&one.next_up().next_up()));
        assert!(one.distinguishable(&-one));

        // 2^52 + 0.5 is not representable, so 2^52 and 2^52 + 1 straddle a hole.
        let p52 = Decimal::from_finite(4503599627370496.0);
        assert!(!p52.distinguishable(&(p52 + Decimal::one())));
        assert!(p52.distinguishable(&(p52 + Decimal::two())));

        // The relative-precision hazard at layer 1.
        let balance: Decimal = "1e300".parse().unwrap();
        let below: Decimal = "1e286".parse().unwrap();
        let at: Decimal = "1e287".parse().unwrap();
        assert_eq!(balance + below, balance);
        assert!(!balance.distinguishable(&(balance + below)));
        assert_ne!(balance + at, balance);
        assert!(!balance.distinguishable(&(balance + at)));
        let clear: Decimal = "1.000000001e300".parse().unwrap();
        assert!(balance.distinguishable(&clear));

        // Boundaries and specials.
        assert!(!Decimal::zero().distinguishable(&Decimal::layer_safe_min()));
        assert!(Decimal::zero().distinguishable(&"1e-300".parse().unwrap()));
        assert!(!Decimal::inf().distinguishable(&Decimal::layer_safe_max()));
        assert!(Decimal::inf().distinguishable(&balance));
        assert!(Decimal::inf().distinguishable(&Decimal::neg_inf()));
        assert!(
            !Decimal::from_finite(8999999999999999.0).distinguishable(&Decimal::from_finite(9e15))
        );
    }

    #[test]
    fn approx_eq_within_tolerance() {
        let a = Decimal::from_finite(1.0);
        let b = Decimal::from_finite(1.0 + 1e-11);
        assert!(a.approx_eq(&b, 1e-10));
        assert_ne!(a, b);
        assert!(Decimal::inf().approx_eq(&Decimal::inf(), 1e-10));
        assert!(!Decimal::inf().approx_eq(&Decimal::maximum(), 1e-10));
    }

    #[test]
    fn tolerance_comparisons() {
        let a = Decimal::from_finite(100.0);
        let b = Decimal::from_finite(100.0 + 1e-9);
        let c = Decimal::from_finite(101.0);
        assert_eq!(a.cmp_tolerance(&b, 1e-8), Ordering::Equal);
        assert_eq!(a.cmp_tolerance(&c, 1e-8), Ordering::Less);
        assert!(a.approx_le(&b, 1e-8));
        assert!(a.approx_ge(&b, 1e-8));
        assert!(!a.approx_lt(&b, 1e-8));
        assert!(a.approx_lt(&c, 1e-8));
        assert!(c.approx_gt(&a, 1e-8));
        assert!(a.approx_ne(&c, 1e-8));
    }

    #[test]
    fn rounding_of_tiny_and_negative_values() {
        assert_eq!(Decimal::from_finite(1e-20).ceil(), Decimal::one());
        assert_eq!(Decimal::from_finite(1e-20).floor(), Decimal::zero());
        assert_eq!(Decimal::from_finite(-1e-20).floor(), Decimal::neg_one());
        assert_eq!(Decimal::from_finite(-1e-20).ceil(), Decimal::zero());
        assert_eq!(Decimal::from_finite(-1e-20).round(), Decimal::zero());
        assert_eq!(
            Decimal::from_finite(-2.5).floor(),
            Decimal::from_finite(-3.0)
        );
        assert_eq!(
            Decimal::from_finite(-2.5).ceil(),
            Decimal::from_finite(-2.0)
        );
        assert_eq!(
            Decimal::from_finite(-2.5).trunc(),
            Decimal::from_finite(-2.0)
        );
        assert_eq!(
            Decimal::from_finite(-2.5).round(),
            Decimal::from_finite(-3.0)
        );
        assert_eq!(Decimal::inf().floor(), Decimal::inf());
    }

    #[test]
    fn exponent_at_layer_two() {
        let d: Decimal = "ee2.5".parse().unwrap();
        // 10^(10^2.5) = 10^316.2, so the exponent is floor(10^2.5) = 316.
        assert_eq!(d.exponent(), 316.0);
    }

    #[test]
    fn set_mantissa_keeps_exponent() {
        let mut d: Decimal = "1.5e100".parse().unwrap();
        d.set_mantissa(2.5);
        assert!(d.approx_eq(&"2.5e100".parse().unwrap(), 1e-12));
    }

    #[test]
    fn minimum_is_positive_and_tiny() {
        assert!(Decimal::minimum() > Decimal::zero());
        assert!(Decimal::minimum() < Decimal::from_finite(1e-300));
        assert_eq!(
            Decimal::minimum().to_string().parse::<Decimal>().unwrap(),
            Decimal::minimum()
        );
    }
}
