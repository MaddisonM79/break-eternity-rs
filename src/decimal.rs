//! Core [`Decimal`] struct definition, constructors, normalization, and accessors.

use crate::constants::{power_of_10, EXPONENT_LIMIT, FIRST_NEG_LAYER, LAYER_REDUCTION_THRESHOLD};
use crate::utils::sign;

/// A Decimal number that can represent numbers as large as 10^^1e308 and as 'small' as 10^-(10^^1e308).
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(
    feature = "godot",
    derive(gdnative::prelude::FromVariant, gdnative::prelude::ToVariant)
)]
pub struct Decimal {
    /// Sign of the Decimal. 1 for positive, -1 for negative.
    pub sign: i8,
    /// Layer of magnitude.
    pub layer: i64,
    /// Magnitude of the Decimal.
    pub mag: f64,
}

impl Decimal {
    /// Creates a new Decimal.
    ///
    /// This does not normalize the Decimal, use [`Decimal::from_components`] for automatic normalization.
    pub fn new(sign: i8, layer: i64, mag: f64) -> Decimal {
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

    /// Sets the sign of the Decimal.
    pub fn set_sign(&mut self, s: i8) {
        if s == 0 {
            self.sign = 0;
            self.layer = 0;
            self.mag = 0.0;
        } else {
            self.sign = s;
        }
    }

    /// Returns the layer of the Decimal.
    pub fn layer(&self) -> i64 {
        self.layer
    }

    /// Sets the layer of the Decimal.
    pub fn set_layer(&mut self, l: i64) {
        self.layer = l;
    }

    /// Returns the magnitude of the Decimal.
    pub fn mag(&self) -> f64 {
        self.mag
    }

    /// Sets the magnitude of the Decimal.
    pub fn set_mag(&mut self, m: f64) {
        self.mag = m;
    }

    /// Creates a Decimal from a sign, a layer and a magnitude.
    ///
    /// This function normalizes the inputs.
    pub fn from_components(sign: i8, layer: i64, mag: f64) -> Decimal {
        Decimal::default().set_from_components(sign, layer, mag)
    }

    /// Creates a Decimal from a sign, a layer and a magnitude.
    ///
    /// This function does not normalize the inputs.
    pub fn from_components_no_normalize(sign: i8, layer: i64, mag: f64) -> Decimal {
        Decimal::default().set_from_components_no_normalize(sign, layer, mag)
    }

    /// Creates a Decimal from a mantissa and an exponent.
    ///
    /// This function normalizes the inputs.
    pub fn from_mantissa_exponent(m: f64, e: f64) -> Decimal {
        Decimal::default().set_from_mantissa_exponent(m, e)
    }

    /// Creates a Decimal from a mantissa and an exponent.
    ///
    /// This function does not normalize the inputs.
    pub fn from_mantissa_exponent_no_normalize(m: f64, e: f64) -> Decimal {
        Decimal::default().set_from_mantissa_exponent_no_normalize(m, e)
    }

    /// Creates a Decimal from a number (f64).
    pub fn from_number(n: f64) -> Decimal {
        if n.is_nan() {
            return Decimal::nan();
        }

        if n.is_infinite() && n.is_sign_positive() {
            return Decimal::inf();
        }

        if n.is_infinite() && n.is_sign_negative() {
            return Decimal::neg_inf();
        }

        Decimal::default().set_from_number(n)
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

    /// Sets the components of the Decimal from a sign, a layer and a magnitude.
    ///
    /// This function does not normalize the inputs.
    pub fn set_from_components_no_normalize(&mut self, sign: i8, layer: i64, mag: f64) -> Decimal {
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
    /// This function does not normalize the inputs.
    pub fn set_from_mantissa_exponent_no_normalize(&mut self, m: f64, e: f64) -> Decimal {
        self.set_from_mantissa_exponent(m, e);
        *self
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
        if self.mantissa().is_nan() {
            return f64::NAN;
        }

        if self.mantissa() == 0.0 {
            return 0.0;
        }

        crate::format::decimal_places(self.mantissa(), places)
    }

    /// Returns the magnitude with the specified amount of decimal places.
    pub fn magnitude_with_decimal_places(&self, places: i32) -> f64 {
        if self.mag.is_nan() {
            return f64::NAN;
        }

        if self.mag == 0.0 {
            return 0.0;
        }

        crate::format::decimal_places(self.mag, places)
    }

    /// Returns the absolute value of the Decimal.
    pub fn abs(&self) -> Decimal {
        Decimal::from_components_no_normalize(i8::from(self.sign != 0), self.layer, self.mag)
    }

    /// Returns a zero Decimal.
    pub fn zero() -> Decimal {
        Decimal::from_components_no_normalize(0, 0, 0.0)
    }

    /// Returns a one Decimal.
    pub fn one() -> Decimal {
        Decimal::from_components_no_normalize(1, 0, 1.0)
    }

    /// Returns a negative one Decimal.
    pub fn neg_one() -> Decimal {
        Decimal::from_components_no_normalize(-1, 0, 1.0)
    }

    /// Returns a two Decimal.
    pub fn two() -> Decimal {
        Decimal::from_components_no_normalize(1, 0, 2.0)
    }

    /// Returns a ten Decimal.
    pub fn ten() -> Decimal {
        Decimal::from_components_no_normalize(1, 0, 10.0)
    }

    /// Returns a NaN Decimal.
    pub fn nan() -> Decimal {
        Decimal::from_components_no_normalize(0, 0, f64::NAN)
    }

    /// Returns a positive infinity Decimal.
    pub fn inf() -> Decimal {
        Decimal::from_components_no_normalize(1, 0, f64::INFINITY)
    }

    /// Returns a negative infinity Decimal.
    pub fn neg_inf() -> Decimal {
        Decimal::from_components_no_normalize(-1, 0, f64::NEG_INFINITY)
    }

    /// Returns the largest safe Decimal that can be represented from an f64.
    pub fn maximum() -> Decimal {
        Decimal::from_components_no_normalize(1, 0, f64::MAX)
    }

    /// Returns the smallest safe Decimal that can be represented from an f64.
    pub fn minimum() -> Decimal {
        Decimal::from_components_no_normalize(1, 0, f64::MIN)
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

    /// Compares the absolute value of the Decimal to the absolute value of the other Decimal.
    pub fn cmpabs(&self, rhs: &Decimal) -> i8 {
        let layer_a = if self.mag > 0.0 {
            self.layer
        } else {
            -self.layer
        };
        let layer_b = if rhs.mag > 0.0 { rhs.layer } else { -rhs.layer };

        if layer_a > layer_b {
            return 1;
        }

        if layer_a < layer_b {
            return -1;
        }

        if self.mag > rhs.mag {
            return 1;
        }

        if self.mag < rhs.mag {
            return -1;
        }

        0
    }

    /// Compares the absolute value of the Decimal to the absolute value of the other Decimal
    /// and returns the bigger one.
    pub fn maxabs(&self, rhs: Decimal) -> Decimal {
        if self.cmpabs(&rhs) > 0 {
            *self
        } else {
            rhs
        }
    }

    /// Compares the absolute value of the Decimal to the absolute value of the other Decimal
    /// and returns the smaller one.
    pub fn minabs(&self, rhs: Decimal) -> Decimal {
        if self.cmpabs(&rhs) > 0 {
            rhs
        } else {
            *self
        }
    }

    /// Returns the reciprocal of the Decimal.
    pub fn recip(&self) -> Decimal {
        if self.mag == 0.0 {
            return Decimal::nan();
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

    /// Tolerance is a relative tolerance, multiplied by the greater of the magnitudes of the two arguments.
    /// For example, if you put in 1e-9, then any number closer to the
    /// larger number than (larger number)*1e-9 will count as equal.
    ///
    /// Default tolerance is 1e-7.
    pub fn eq_tolerance(&self, other: &Decimal, tolerance: f64) -> bool {
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

    /// Returns the Decimal squared.
    pub fn sqr(&self) -> Decimal {
        self.pow(Decimal::from_number(2.0))
    }

    /// Returns the square root of the Decimal.
    pub fn sqrt(&self) -> Decimal {
        if self.layer == 0 {
            return Decimal::from_number((self.sign as f64 * self.mag).sqrt());
        }

        if self.layer == 1 {
            return Decimal::from_components(1, 2, self.mag.log10() - std::f64::consts::LOG10_2);
        }

        let mut result = Decimal::from_components_no_normalize(self.sign, self.layer - 1, self.mag)
            / Decimal::from_components_no_normalize(1, 0, 2.0);
        result.layer += 1;
        result.normalize();

        result
    }

    /// Returns the Decimal cubed.
    pub fn cube(&self) -> Decimal {
        self.pow(Decimal::from_number(3.0))
    }

    /// Returns the cube root of the Decimal.
    pub fn cbrt(&self) -> Decimal {
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
            return Decimal::from_number((self.sign as f64 * self.mag).sin());
        }

        Decimal::from_components_no_normalize(0, 0, 0.0)
    }

    /// Returns the cos of the Decimal.
    pub fn cos(&self) -> Decimal {
        if self.mag < 0.0 {
            return Decimal::one();
        }

        if self.layer == 0 {
            return Decimal::from_number((self.sign as f64 * self.mag).cos());
        }

        Decimal::from_components_no_normalize(0, 0, 0.0)
    }

    /// Returns the tan of the Decimal.
    pub fn tan(&self) -> Decimal {
        if self.mag < 0.0 {
            return Decimal::from_number((self.sign as f64 * self.mag).tan());
        }

        if self.layer == 0 {
            return Decimal::from_number((self.sign as f64 * self.mag).tan());
        }

        Decimal::from_components_no_normalize(0, 0, 0.0)
    }

    /// Returns the asin of the Decimal.
    pub fn asin(&self) -> Decimal {
        if self.mag < 0.0 {
            return *self;
        }

        if self.layer == 0 {
            return Decimal::from_number((self.sign as f64 * self.mag).asin());
        }

        Decimal::nan()
    }

    /// Returns the acos of the Decimal.
    pub fn acos(&self) -> Decimal {
        if self.mag < 0.0 {
            return Decimal::from_number(self.to_number().acos());
        }

        if self.layer == 0 {
            return Decimal::from_number((self.sign as f64 * self.mag).acos());
        }

        Decimal::nan()
    }

    /// Returns the atan of the Decimal.
    pub fn atan(&self) -> Decimal {
        if self.mag < 0.0 {
            return *self;
        }

        if self.layer == 0 {
            return Decimal::from_number((self.sign as f64 * self.mag).atan());
        }

        Decimal::from_number(f64::INFINITY.atan())
    }

    /// Returns the sinh of the Decimal.
    pub fn sinh(&self) -> Decimal {
        (self.exp() - (-*self).exp()) / Decimal::from_number(2.0)
    }

    /// Returns the cosh of the Decimal.
    pub fn cosh(&self) -> Decimal {
        (self.exp() + (-*self).exp()) / Decimal::from_number(2.0)
    }

    /// Returns the tanh of the Decimal.
    pub fn tanh(&self) -> Decimal {
        self.sinh() / self.cosh()
    }

    /// Returns the asinh of the Decimal.
    pub fn asinh(&self) -> Decimal {
        (*self + (self.sqr() + Decimal::from_number(1.0)).sqrt()).ln()
    }

    /// Returns the acosh of the Decimal.
    pub fn acosh(&self) -> Decimal {
        (*self + (self.sqr() - Decimal::from_number(1.0)).sqrt()).ln()
    }

    /// Returns the atanh of the Decimal.
    pub fn atanh(&self) -> Decimal {
        if self.abs() >= Decimal::from_number(1.0) {
            return Decimal::nan();
        }

        (*self + Decimal::from_number(1.0))
            / (Decimal::from_number(1.0) - *self).ln()
            / Decimal::from_number(2.0)
    }
}

// Trait impls that need to live alongside the struct definition
impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        // Special edge cases for NaN and infinities
        if self.mag.is_nan() && other.mag.is_nan() {
            return true;
        }

        if (self.mag.is_infinite() && self.mag.is_sign_positive())
            && (other.mag.is_infinite() && other.mag.is_sign_positive())
        {
            return true;
        }

        if (self.mag.is_infinite() && self.mag.is_sign_negative())
            && (other.mag.is_infinite() && other.mag.is_sign_negative())
        {
            return true;
        }

        self.sign == other.sign
            && self.layer == other.layer
            && (self.mag - other.mag).abs() < crate::constants::COMPARE_EPSILON
    }
}

impl Eq for Decimal {}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Decimal {
    #[allow(clippy::comparison_chain)]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.sign > other.sign {
            return std::cmp::Ordering::Greater;
        }

        if self.sign < other.sign {
            return std::cmp::Ordering::Less;
        }

        let cmp_abs = self.cmpabs(other) * self.sign;
        if cmp_abs > 0 {
            std::cmp::Ordering::Greater
        } else if cmp_abs < 0 {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Equal
        }
    }
}

impl std::hash::Hash for Decimal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.sign.hash(state);
        self.layer.hash(state);
        self.mag.to_bits().hash(state);
    }
}
