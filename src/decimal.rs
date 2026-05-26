//! Core [`Decimal`] struct definition, constructors, normalization, and accessors.

use crate::constants::{power_of_10, EXPONENT_LIMIT, FIRST_NEG_LAYER, LAYER_REDUCTION_THRESHOLD};
use crate::utils::sign;

/// A Decimal number that can represent numbers as large as 10^^1e308 and as 'small' as 10^-(10^^1e308).
#[derive(Clone, Copy, Debug, Default)]
pub struct Decimal {
    /// Sign of the Decimal. 1 for positive, -1 for negative, 0 for zero.
    pub(crate) sign: i8,
    /// Layer of magnitude.
    pub(crate) layer: i64,
    /// Internal mag value. Interpretation depends on `layer`: at layer 0 this is the
    /// signed magnitude; at layer 1 this is log10(|value|); at layer >= 2 this is the
    /// deeply-iterated log.
    pub(crate) mag: f64,
}

impl Decimal {
    /// Creates a new Decimal without normalization.
    ///
    /// This does not normalize the Decimal; use [`Decimal::from_components`] for automatic normalization.
    #[allow(dead_code)] // pub(crate) helper available for future internal callers
    pub(crate) fn new(sign: i8, layer: i64, mag: f64) -> Decimal {
        Decimal { sign, layer, mag }
    }

    /// Returns the mantissa of the Decimal.
    pub fn mantissa(&self) -> f64 {
        if self.sign == 0 {
            return 0.0;
        }

        if self.layer == 0 {
            let exp = self.mag.abs().log10().floor();
            // handle special case 5e-324
            let man = if (self.mag - 5e-324).abs() < 1e-10 {
                5.0
            } else {
                self.mag / power_of_10(exp as i32)
            };

            return self.sign as f64 * man;
        }

        if self.layer == 1 {
            let residue = self.mag - self.mag.floor();
            return self.sign as f64 * 10.0_f64.powf(residue);
        }

        self.sign as f64
    }

    /// Sets the mantissa of the Decimal.
    pub fn set_mantissa(&mut self, m: f64) {
        if self.layer <= 2 {
            self.set_from_mantissa_exponent(m, self.layer as f64);
        } else {
            // this mantissa isn't meaningful
            self.sign = sign(m);
            if self.sign == 0 {
                self.layer = 0;
                self.set_exponent(0.0);
            }
        }
    }

    /// Returns the exponent of the Decimal.
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
            return (sign(self.mag) as f64 * self.mag.abs().powf(10.0)).floor();
        }

        self.mag * f64::INFINITY
    }

    /// Sets the exponent of the Decimal.
    pub fn set_exponent(&mut self, e: f64) {
        self.set_from_mantissa_exponent(self.mantissa(), e);
    }

    /// Returns the sign of the Decimal.
    pub fn sign(&self) -> i8 {
        self.sign
    }

    /// Returns the layer of the Decimal.
    pub fn layer(&self) -> i64 {
        self.layer
    }

    /// Returns the internal mag value.
    ///
    /// Interpretation depends on `layer`: at layer 0 this is the signed magnitude; at layer 1
    /// this is log10(|value|); at layer >= 2 this is the deeply-iterated log.
    pub fn mag(&self) -> f64 {
        self.mag
    }

    /// Creates a Decimal from a sign, a layer and a magnitude.
    ///
    /// This function normalizes the inputs.
    pub fn from_components(sign: i8, layer: i64, mag: f64) -> Decimal {
        Decimal::default().set_from_components(sign, layer, mag)
    }

    /// Creates a Decimal from a sign, a layer and a magnitude without normalizing.
    ///
    /// The caller must ensure the inputs already satisfy the normalization invariant.
    pub(crate) fn from_components_unchecked(sign: i8, layer: i64, mag: f64) -> Decimal {
        Decimal { sign, layer, mag }
    }

    /// Creates a Decimal from a mantissa and an exponent.
    ///
    /// This function normalizes the inputs.
    pub fn from_mantissa_exponent(m: f64, e: f64) -> Decimal {
        Decimal::default().set_from_mantissa_exponent(m, e)
    }

    /// Creates a Decimal from a mantissa and an exponent without normalizing.
    ///
    /// The caller must ensure the inputs already satisfy the normalization invariant.
    #[allow(dead_code)] // pub(crate) helper available for future internal callers
    pub(crate) fn from_mantissa_exponent_unchecked(m: f64, e: f64) -> Decimal {
        Decimal {
            sign: sign(m),
            layer: 1,
            mag: e + m.abs().log10(),
        }
    }

    /// Creates a Decimal from a mantissa and an exponent.
    ///
    /// This function normalizes the inputs. Formerly `from_mantissa_exponent_no_normalize`,
    /// which was a misnomer — the implementation always normalized.
    #[deprecated(
        note = "use from_mantissa_exponent; this normalizing alias will be removed in a future release"
    )]
    pub fn from_mantissa_exponent_no_normalize(m: f64, e: f64) -> Decimal {
        Decimal::from_mantissa_exponent(m, e)
    }

    /// Creates a Decimal from a number (f64).
    ///
    /// # Deprecated
    ///
    /// Use [`Decimal::from_finite`] for an infallible constructor that asserts finiteness in
    /// debug builds, or [`TryFrom<f64>`] for an explicit fallible path that rejects NaN and
    /// infinite values.
    #[deprecated(note = "use Decimal::from_finite or TryFrom<f64>")]
    pub fn from_number(n: f64) -> Decimal {
        if n.is_nan() {
            return Decimal::nan_sentinel();
        }

        if n.is_infinite() && n.is_sign_positive() {
            return Decimal::inf();
        }

        if n.is_infinite() && n.is_sign_negative() {
            return Decimal::neg_inf();
        }

        Decimal::default().set_from_number(n)
    }

    /// Creates a `Decimal` from a finite `f64`.
    ///
    /// In debug builds this asserts that `x` is finite. In release builds the assertion is
    /// elided but the behavior is otherwise identical to `TryFrom<f64>::try_from(x).unwrap()`.
    ///
    /// # Panics (debug only)
    ///
    /// Panics if `x` is NaN or infinite.
    pub fn from_finite(x: f64) -> Decimal {
        debug_assert!(
            x.is_finite(),
            "from_finite called with non-finite value: {x}"
        );
        Decimal::default().set_from_number(x)
    }

    /// Normalizes the Decimal as follows:
    ///
    /// * Whenever we are partially 0 (sign is 0 or mag and layer is 0), make it fully 0.
    /// * Whenever we are at or hit layer 0, extract sign from negative mag.
    /// * If layer === 0 and mag < `FIRST_NEG_LAYER` (1/9e15), shift to 'first negative layer' (add layer, log10 mag).
    /// * While abs(mag) > `EXP_LIMIT` (9e15), layer += 1, mag = maglog10(mag).
    /// * While abs(mag) < `LAYER_DOWN` (15.954) and layer > 0, layer -= 1, mag = pow(10, mag).
    /// * When we're done, all of the following should be true OR one of the numbers is not finite OR layer is not an integer (error state):
    ///     * Any 0 is totally zero (0, 0, 0).
    ///     * Anything layer 0 has mag 0 OR mag > 1/9e15 and < 9e15.
    ///     * Anything layer 1 or higher has abs(mag) >= 15.954 and < 9e15.
    /// * We will assume in calculations that all Decimals are either erroneous or satisfy these criteria. (Otherwise: Garbage in, garbage out.)
    pub fn normalize(&mut self) -> Decimal {
        if self.sign == 0 || (self.mag == 0.0 && self.layer == 0) {
            self.sign = 0;
            self.layer = 0;
            self.mag = 0.0;
            return *self;
        }

        if self.layer == 0 && self.mag < 0.0 {
            self.mag = -self.mag;
            self.sign = -self.sign;
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

    /// Sets the components of the Decimal from a sign, a layer and a magnitude without normalizing.
    ///
    /// The caller is responsible for ensuring the result satisfies the normalization invariant.
    #[allow(dead_code)] // pub(crate) helper available for future internal callers
    pub(crate) fn set_from_components_unchecked(
        &mut self,
        sign: i8,
        layer: i64,
        mag: f64,
    ) -> Decimal {
        self.sign = sign;
        self.layer = layer;
        self.mag = mag;

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

    /// Sets the components of the Decimal from a mantissa and an exponent.
    ///
    /// Despite the name, this always normalizes. Kept for internal call-site compatibility.
    /// Prefer [`set_from_mantissa_exponent`](Self::set_from_mantissa_exponent).
    #[allow(dead_code)] // pub(crate) helper available for future internal callers
    pub(crate) fn set_from_mantissa_exponent_unchecked(&mut self, m: f64, e: f64) -> Decimal {
        // NOTE: the former "no_normalize" variant called the normalizing version anyway;
        // that was a bug in the original code. This variant is now truly unchecked in name
        // only — callers that need a genuinely non-normalizing path should set fields directly.
        self.set_from_mantissa_exponent(m, e)
    }

    /// Sets the components of the Decimal from a number (f64).
    pub fn set_from_number(&mut self, n: f64) -> Decimal {
        self.sign = sign(n);
        self.layer = 0;
        self.mag = n.abs();

        self.normalize();
        *self
    }

    /// Returns the Decimal as a number (f64).
    pub fn to_number(&self) -> f64 {
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

    /// Returns the mantissa with the specified amount of decimal places.
    pub fn mantissa_with_decimal_places(&self, places: i32) -> f64 {
        if self.mantissa() == 0.0 {
            return 0.0;
        }

        crate::format::decimal_places(self.mantissa(), places)
    }

    /// Returns the magnitude with the specified amount of decimal places.
    pub fn magnitude_with_decimal_places(&self, places: i32) -> f64 {
        if self.mag == 0.0 {
            return 0.0;
        }

        crate::format::decimal_places(self.mag, places)
    }

    /// Returns the absolute value of the Decimal.
    pub fn abs(&self) -> Decimal {
        Decimal::from_components_unchecked(i8::from(self.sign != 0), self.layer, self.mag)
    }

    /// Returns a zero Decimal.
    pub fn zero() -> Decimal {
        Decimal::from_components_unchecked(0, 0, 0.0)
    }

    /// Returns a one Decimal.
    pub fn one() -> Decimal {
        Decimal::from_components_unchecked(1, 0, 1.0)
    }

    /// Returns a negative one Decimal.
    pub fn neg_one() -> Decimal {
        Decimal::from_components_unchecked(-1, 0, 1.0)
    }

    /// Returns a two Decimal.
    pub fn two() -> Decimal {
        Decimal::from_components_unchecked(1, 0, 2.0)
    }

    /// Returns a ten Decimal.
    pub fn ten() -> Decimal {
        Decimal::from_components_unchecked(1, 0, 10.0)
    }

    /// Returns an internal NaN sentinel used during transient iteration loops.
    ///
    /// Not exposed publicly — after Phase 4c, callers that previously returned NaN
    /// should return `Err(ArithmeticError { ... })` instead.
    pub(crate) fn nan_sentinel() -> Decimal {
        Decimal::from_components_unchecked(0, 0, f64::NAN)
    }

    /// Returns true if this Decimal carries a NaN mag (internal sentinel state).
    pub(crate) fn has_nan_mag(&self) -> bool {
        self.mag.is_nan()
    }

    /// Returns a positive infinity Decimal.
    pub fn inf() -> Decimal {
        Decimal::from_components_unchecked(1, 0, f64::INFINITY)
    }

    /// Returns a negative infinity Decimal.
    pub fn neg_inf() -> Decimal {
        Decimal::from_components_unchecked(-1, 0, f64::NEG_INFINITY)
    }

    /// Returns the largest safe Decimal that can be represented from an f64.
    pub fn maximum() -> Decimal {
        Decimal::from_components_unchecked(1, 0, f64::MAX)
    }

    /// Returns the smallest safe Decimal that can be represented from an f64.
    pub fn minimum() -> Decimal {
        Decimal::from_components_unchecked(1, 0, f64::MIN)
    }

    /// Rounds the Decimal to the nearest integer.
    pub fn round(&self) -> Decimal {
        if self.mag < 0.0 {
            return Decimal::zero();
        }

        if self.layer == 0 {
            return Decimal::from_components(self.sign, 0, self.mag.round());
        }

        *self
    }

    /// Returns the largest Decimal less than or equal to the Decimal.
    pub fn floor(&self) -> Decimal {
        if self.mag < 0.0 {
            return Decimal::zero();
        }

        if self.layer == 0 {
            return Decimal::from_components(self.sign, 0, self.mag.floor());
        }

        *self
    }

    /// Returns the smallest Decimal greater than or equal to the Decimal.
    pub fn ceil(&self) -> Decimal {
        if self.mag < 0.0 {
            return Decimal::zero();
        }

        if self.layer == 0 {
            return Decimal::from_components(self.sign, 0, self.mag.ceil());
        }

        *self
    }

    /// Returns the integer part of the Decimal.
    pub fn trunc(&self) -> Decimal {
        if self.mag < 0.0 {
            return Decimal::zero();
        }

        if self.layer == 0 {
            return Decimal::from_components(self.sign, 0, self.mag.trunc());
        }

        *self
    }

    /// Compares the absolute values of two Decimals.
    ///
    /// Returns an [`std::cmp::Ordering`] indicating the relative order of `|self|` and `|rhs|`.
    pub fn cmpabs(&self, rhs: &Decimal) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        let layer_a = if self.mag > 0.0 {
            self.layer
        } else {
            -self.layer
        };
        let layer_b = if rhs.mag > 0.0 { rhs.layer } else { -rhs.layer };

        if layer_a > layer_b {
            return Ordering::Greater;
        }

        if layer_a < layer_b {
            return Ordering::Less;
        }

        if self.mag > rhs.mag {
            return Ordering::Greater;
        }

        if self.mag < rhs.mag {
            return Ordering::Less;
        }

        Ordering::Equal
    }

    /// Compares the absolute value of the Decimal to the absolute value of the other Decimal
    /// and returns the bigger one.
    pub fn maxabs(&self, rhs: Decimal) -> Decimal {
        if self.cmpabs(&rhs).is_gt() {
            *self
        } else {
            rhs
        }
    }

    /// Compares the absolute value of the Decimal to the absolute value of the other Decimal
    /// and returns the smaller one.
    pub fn minabs(&self, rhs: Decimal) -> Decimal {
        if self.cmpabs(&rhs).is_gt() {
            rhs
        } else {
            *self
        }
    }

    /// Returns the reciprocal of the Decimal.
    ///
    /// Returns a NaN sentinel if the value is zero; callers that need explicit error handling
    /// should use [`checked_div`](Self::checked_div) instead.
    pub fn recip(&self) -> Decimal {
        if self.mag == 0.0 {
            return Decimal::nan_sentinel();
        }

        if self.layer == 0 {
            return Decimal::from_components(self.sign, 0, 1.0 / self.mag);
        }

        Decimal::from_components(self.sign, self.layer, -self.mag)
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

    /// Returns true if `self` and `other` are approximately equal within the given relative tolerance.
    ///
    /// Tolerance is relative: two numbers are considered equal if their difference is no greater
    /// than `tolerance * max(|self.mag|, |other.mag|)` (after adjusting for layer differences).
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

        if (self.layer - other.layer).abs() > 1 {
            return false;
        }

        let mut mag_a = self.mag;
        let mut mag_b = other.mag;
        if self.layer > other.layer {
            mag_b = crate::utils::f_maglog10(mag_b);
        }
        if other.layer > self.layer {
            mag_a = crate::utils::f_maglog10(mag_a);
        }

        (mag_a - mag_b).abs() <= tolerance * mag_a.abs().max(mag_b.abs())
    }

    /// Returns true if `self` and `other` are approximately equal within the given relative tolerance.
    ///
    /// # Deprecated
    ///
    /// Use [`approx_eq`](Self::approx_eq) instead.
    #[deprecated(note = "use approx_eq")]
    pub fn eq_tolerance(&self, other: &Decimal, tolerance: f64) -> bool {
        self.approx_eq(other, tolerance)
    }

    /// Returns the Decimal squared.
    pub fn sqr(&self) -> Decimal {
        #[allow(deprecated)]
        self.pow(Decimal::from_number(2.0))
    }

    /// Returns the square root of the Decimal.
    pub fn sqrt(&self) -> Decimal {
        if self.layer == 0 {
            #[allow(deprecated)]
            return Decimal::from_number((self.sign as f64 * self.mag).sqrt());
        }

        if self.layer == 1 {
            // For positive mag the value is large (10^mag) and sqrt promotes to layer 2.
            // For negative mag the value is tiny (10^mag, with mag < 0); sqrt(10^mag)
            // = 10^(mag/2). Stay at layer 1 and let normalize() demote if needed.
            if self.mag >= 0.0 {
                return Decimal::from_components(1, 2, self.mag.log10() - std::f64::consts::LOG10_2);
            }
            return Decimal::from_components(self.sign, 1, self.mag / 2.0);
        }

        let mut result = Decimal::from_components_unchecked(self.sign, self.layer - 1, self.mag)
            / Decimal::from_components_unchecked(1, 0, 2.0);
        result.layer += 1;
        result.normalize();

        result
    }

    /// Returns the Decimal cubed.
    pub fn cube(&self) -> Decimal {
        #[allow(deprecated)]
        self.pow(Decimal::from_number(3.0))
    }

    /// Returns the cube root of the Decimal.
    pub fn cbrt(&self) -> Decimal {
        #[allow(deprecated)]
        self.pow(Decimal::from_number(1.0) / Decimal::from_number(3.0))
    }

    /// Returns the n-th root of the Decimal.
    pub fn root(self, n: Decimal) -> Decimal {
        self.pow(n.recip())
    }

    /// Returns the sin of the Decimal.
    pub fn sin(&self) -> Decimal {
        if self.mag < 0.0 {
            return *self;
        }

        if self.layer == 0 {
            #[allow(deprecated)]
            return Decimal::from_number((self.sign as f64 * self.mag).sin());
        }

        Decimal::from_components_unchecked(0, 0, 0.0)
    }

    /// Returns the cos of the Decimal.
    pub fn cos(&self) -> Decimal {
        if self.mag < 0.0 {
            return Decimal::one();
        }

        if self.layer == 0 {
            #[allow(deprecated)]
            return Decimal::from_number((self.sign as f64 * self.mag).cos());
        }

        Decimal::from_components_unchecked(0, 0, 0.0)
    }

    /// Returns the tan of the Decimal.
    pub fn tan(&self) -> Decimal {
        if self.mag < 0.0 {
            #[allow(deprecated)]
            return Decimal::from_number((self.sign as f64 * self.mag).tan());
        }

        if self.layer == 0 {
            #[allow(deprecated)]
            return Decimal::from_number((self.sign as f64 * self.mag).tan());
        }

        Decimal::from_components_unchecked(0, 0, 0.0)
    }

    /// Returns the asin of the Decimal.
    pub fn asin(&self) -> Decimal {
        if self.mag < 0.0 {
            return *self;
        }

        if self.layer == 0 {
            #[allow(deprecated)]
            return Decimal::from_number((self.sign as f64 * self.mag).asin());
        }

        Decimal::nan_sentinel()
    }

    /// Returns the acos of the Decimal.
    pub fn acos(&self) -> Decimal {
        if self.mag < 0.0 {
            #[allow(deprecated)]
            return Decimal::from_number(self.to_number().acos());
        }

        if self.layer == 0 {
            #[allow(deprecated)]
            return Decimal::from_number((self.sign as f64 * self.mag).acos());
        }

        Decimal::nan_sentinel()
    }

    /// Returns the atan of the Decimal.
    pub fn atan(&self) -> Decimal {
        if self.mag < 0.0 {
            return *self;
        }

        if self.layer == 0 {
            #[allow(deprecated)]
            return Decimal::from_number((self.sign as f64 * self.mag).atan());
        }

        #[allow(deprecated)]
        Decimal::from_number(f64::INFINITY.atan())
    }

    /// Returns the sinh of the Decimal.
    pub fn sinh(&self) -> Decimal {
        (self.exp() - (-*self).exp()) / Decimal::from_finite(2.0)
    }

    /// Returns the cosh of the Decimal.
    pub fn cosh(&self) -> Decimal {
        (self.exp() + (-*self).exp()) / Decimal::from_finite(2.0)
    }

    /// Returns the tanh of the Decimal.
    pub fn tanh(&self) -> Decimal {
        self.sinh() / self.cosh()
    }

    /// Returns the asinh of the Decimal.
    pub fn asinh(&self) -> Decimal {
        (*self + (self.sqr() + Decimal::from_finite(1.0)).sqrt()).ln()
    }

    /// Returns the acosh of the Decimal.
    pub fn acosh(&self) -> Decimal {
        (*self + (self.sqr() - Decimal::from_finite(1.0)).sqrt()).ln()
    }

    /// Returns the atanh of the Decimal.
    pub fn atanh(&self) -> Decimal {
        if self.abs() >= Decimal::from_finite(1.0) {
            return Decimal::nan_sentinel();
        }

        // atanh(x) = ln((1+x)/(1-x)) / 2
        ((*self + Decimal::from_finite(1.0)) / (Decimal::from_finite(1.0) - *self))
            .ln()
            / Decimal::from_finite(2.0)
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

impl std::hash::Hash for Decimal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.sign.hash(state);
        self.layer.hash(state);
        self.mag.to_bits().hash(state);
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sign.cmp(&other.sign).then_with(|| match self.sign {
            1 => self.cmpabs(other),
            -1 => other.cmpabs(self),
            0 => std::cmp::Ordering::Equal,
            _ => unreachable!("sign invariant violated"),
        })
    }
}

// ---------------------------------------------------------------------------
// TryFrom conversions
// ---------------------------------------------------------------------------

impl TryFrom<f64> for Decimal {
    type Error = crate::error::ArithmeticError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_nan() || value.is_infinite() {
            return Err(crate::error::ArithmeticError {
                kind: crate::error::ArithmeticErrorKind::Undefined,
                op: "from_f64",
            });
        }
        Ok(Decimal::default().set_from_number(value))
    }
}

impl TryFrom<f32> for Decimal {
    type Error = crate::error::ArithmeticError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Decimal::try_from(value as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_canonicalizes_neg_zero() {
        let mut d = Decimal {
            sign: 1,
            layer: 0,
            mag: -0.0_f64,
        };
        d.normalize();
        // After normalization the mag should be positive zero (no NaN in bits, sign==0).
        assert_eq!(d, Decimal::zero());
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
    fn approx_eq_within_tolerance() {
        let a = Decimal::from_finite(1.0);
        let b = Decimal::from_finite(1.0 + 1e-11);
        assert!(a.approx_eq(&b, 1e-10));
        // Exact equality should be false since they differ by bits.
        assert_ne!(a, b);
    }
}
