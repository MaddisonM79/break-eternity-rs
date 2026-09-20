//! Rounding to a number of decimal places or significant figures, returning a [`Decimal`].
//!
//! [`round`](Decimal::round), [`floor`](Decimal::floor), [`ceil`](Decimal::ceil) and
//! [`trunc`](Decimal::trunc) work on whole numbers. The methods here take a digit count, for
//! prices that should stay at two decimals or multipliers shown to three significant figures,
//! and unlike [`to_fixed`](Decimal::to_fixed) they give back a value you can keep computing
//! with.

use crate::constants::power_of_10;
use crate::decimal::Decimal;
#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
// shadowed by std's inherent methods whenever std is in the crate graph
use crate::math::FloatExt;

/// `2^53`: above this every `f64` is an integer, so rounding to any place is the identity.
const EXACT_INTEGER_LIMIT: f64 = 9_007_199_254_740_992.0;
/// `power_of_10` covers `10^-323..=10^308`; places past that cannot change an `f64` anyway.
const MAX_PLACES: i32 = 308;

impl Decimal {
    /// Returns the fractional part, `self - self.trunc()`, with the sign of `self`.
    ///
    /// ```
    /// use break_eternity::Decimal;
    ///
    /// assert_eq!(Decimal::from_finite(3.75).fract(), Decimal::from_finite(0.75));
    /// assert_eq!(Decimal::from_finite(-3.75).fract(), Decimal::from_finite(-0.75));
    /// assert_eq!(Decimal::try_from("1e100").unwrap().fract(), Decimal::zero());
    /// ```
    pub fn fract(&self) -> Decimal {
        if self.layer() == 0 {
            return Decimal::from_f64((self.sign() as f64 * self.mag()).fract());
        }
        if self.mag() > 0.0 {
            // Every value at or above 10^15.95 is an integer.
            Decimal::zero()
        } else {
            *self
        }
    }

    /// Returns `true` if the value is a finite whole number.
    ///
    /// Everything at layer 1 and above with a positive magnitude is an integer, since those
    /// values start at about `9e15`; infinities are not.
    ///
    /// ```
    /// use break_eternity::Decimal;
    ///
    /// assert!(Decimal::from(12).is_integer());
    /// assert!(!Decimal::from_finite(12.5).is_integer());
    /// assert!(Decimal::try_from("1e300").unwrap().is_integer());
    /// assert!(!Decimal::try_from("1e-300").unwrap().is_integer());
    /// assert!(!Decimal::inf().is_integer());
    /// ```
    pub fn is_integer(&self) -> bool {
        if self.is_infinite() || self.has_nan_mag() {
            return false;
        }
        if self.layer() == 0 {
            return self.mag().fract() == 0.0;
        }
        self.mag() > 0.0
    }

    /// Rounds to `places` digits after the decimal point, half away from zero.
    ///
    /// Negative `places` round to tens, hundreds, and so on. Values that already have no digit
    /// at that place (everything above `2^53 / 10^places`) are returned unchanged.
    ///
    /// ```
    /// use break_eternity::Decimal;
    ///
    /// let price = Decimal::from_finite(19.995);
    /// assert_eq!(price.round_to_places(2), Decimal::from_finite(20.0));
    /// assert_eq!(Decimal::from(1234).round_to_places(-2), Decimal::from(1200));
    /// assert_eq!(Decimal::from_finite(-2.5).round_to_places(0), Decimal::from(-3));
    /// assert_eq!(Decimal::try_from("1e-20").unwrap().round_to_places(5), Decimal::zero());
    /// ```
    pub fn round_to_places(&self, places: i32) -> Decimal {
        self.apply_places(places, f64::round)
    }

    /// Rounds toward negative infinity at `places` digits after the decimal point.
    ///
    /// ```
    /// use break_eternity::Decimal;
    ///
    /// assert_eq!(Decimal::from_finite(1.239).floor_to_places(2), Decimal::from_finite(1.23));
    /// assert_eq!(Decimal::from_finite(-1.231).floor_to_places(2), Decimal::from_finite(-1.24));
    /// ```
    pub fn floor_to_places(&self, places: i32) -> Decimal {
        self.apply_places(places, f64::floor)
    }

    /// Rounds toward positive infinity at `places` digits after the decimal point.
    ///
    /// ```
    /// use break_eternity::Decimal;
    ///
    /// assert_eq!(Decimal::from_finite(1.231).ceil_to_places(2), Decimal::from_finite(1.24));
    /// assert_eq!(Decimal::try_from("1e-300").unwrap().ceil_to_places(2), Decimal::from_finite(0.01));
    /// ```
    pub fn ceil_to_places(&self, places: i32) -> Decimal {
        self.apply_places(places, f64::ceil)
    }

    /// Rounds toward zero at `places` digits after the decimal point.
    ///
    /// ```
    /// use break_eternity::Decimal;
    ///
    /// assert_eq!(Decimal::from_finite(-1.239).trunc_to_places(2), Decimal::from_finite(-1.23));
    /// ```
    pub fn trunc_to_places(&self, places: i32) -> Decimal {
        self.apply_places(places, f64::trunc)
    }

    /// Rounds to `digits` significant figures (half away from zero). `digits` below 1 is
    /// treated as 1.
    ///
    /// Works at every layer: at layer 1 the mantissa is rounded and the exponent kept, so
    /// `1.23456e100` at three digits is `1.23e100`. Values whose exponent is at least `1e15`
    /// have no meaningful mantissa and are returned unchanged.
    ///
    /// ```
    /// use break_eternity::Decimal;
    ///
    /// assert_eq!(Decimal::from_finite(123_456.0).round_to_significant(3), Decimal::from(123_000));
    /// assert_eq!(Decimal::from_finite(0.0012345).round_to_significant(2), Decimal::from_finite(0.0012));
    /// assert_eq!(Decimal::from_finite(9.99).round_to_significant(2), Decimal::from(10));
    /// let big = Decimal::try_from("1.23456e100").unwrap();
    /// assert_eq!(big.round_to_significant(3), Decimal::try_from("1.23e100").unwrap());
    /// ```
    pub fn round_to_significant(&self, digits: u32) -> Decimal {
        let digits = digits.max(1) as i32;
        if self.sign() == 0 || self.is_infinite() || self.has_nan_mag() {
            return *self;
        }
        if self.layer() == 0 {
            let exponent = self.mag().log10().floor() as i32;
            return self.round_to_places(digits - 1 - exponent);
        }
        if self.layer() > 1 || self.mag().abs() >= 1e15 {
            return *self;
        }
        // Layer 1: round the mantissa in [1, 10) and rebuild.
        let exponent = self.mag().floor();
        let mut mantissa = 10f64.powf(self.mag() - exponent);
        let scale = power_of_10(digits - 1);
        mantissa = (mantissa * scale).round() / scale;
        let mut exponent = exponent;
        if mantissa >= 10.0 {
            mantissa = 1.0;
            exponent += 1.0;
        }
        Decimal::from_mantissa_exponent(self.sign() as f64 * mantissa, exponent)
    }

    /// Shared implementation: applies `f` to `self * 10^places` and scales back.
    fn apply_places(&self, places: i32, f: fn(f64) -> f64) -> Decimal {
        if self.sign() == 0 || self.is_infinite() || self.has_nan_mag() {
            return *self;
        }
        let places = places.clamp(-MAX_PLACES, MAX_PLACES);

        if self.layer() == 0 {
            let x = self.sign() as f64 * self.mag();
            // Multiply or divide by an exact integer power of ten; never by 10^-n.
            let scaled = if places >= 0 {
                x * power_of_10(places)
            } else {
                x / power_of_10(-places)
            };
            if scaled.abs() >= EXACT_INTEGER_LIMIT || scaled.is_infinite() {
                return *self;
            }
            let rounded = if places >= 0 {
                f(scaled) / power_of_10(places)
            } else {
                f(scaled) * power_of_10(-places)
            };
            return Decimal::from_f64(rounded);
        }

        if self.mag() > 0.0 {
            // At or above 10^15.95: already an integer.
            return *self;
        }

        // Tiny: 0 < |x| < 1e-16. Scaling by 10^places may or may not reach f64 range; when it
        // does not, the direction of `f` still decides between 0 and one unit in the last place.
        let scale = Decimal::ten().pow(Decimal::from(places));
        let scaled = *self * scale;
        let unit = if scaled.layer() == 0 {
            f(scaled.to_number())
        } else {
            f(self.sign() as f64 * f64::MIN_POSITIVE)
        };
        if unit == 0.0 {
            return Decimal::zero();
        }
        Decimal::from_f64(unit) / scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    #[test]
    fn places_layer0() {
        assert_eq!(d("123.456").round_to_places(2), d("123.46"));
        assert_eq!(d("123.456").round_to_places(0), d("123"));
        assert_eq!(d("123.456").round_to_places(-1), d("120"));
        assert_eq!(d("125").round_to_places(-1), d("130"));
        assert_eq!(d("-125").round_to_places(-1), d("-130"));
        assert_eq!(d("0.5").round_to_places(0), d("1"));
        assert_eq!(d("-0.5").round_to_places(0), d("-1"));
        assert_eq!(d("1.005").round_to_places(2), d("1.0"));
        assert_eq!(d("2.675").floor_to_places(2), d("2.67"));
        assert_eq!(d("2.671").ceil_to_places(2), d("2.68"));
        assert_eq!(d("-2.679").trunc_to_places(2), d("-2.67"));
        assert_eq!(d("-2.671").floor_to_places(2), d("-2.68"));
        assert_eq!(
            d("9007199254740993").round_to_places(2),
            d("9007199254740993")
        );
        assert_eq!(d("1e15").round_to_places(400), d("1e15"));
        assert_eq!(d("0").round_to_places(3), Decimal::zero());
        assert_eq!(Decimal::inf().round_to_places(3), Decimal::inf());
    }

    #[test]
    fn places_large_and_tiny() {
        assert_eq!(d("1e100").round_to_places(2), d("1e100"));
        assert_eq!(d("1.5e100").floor_to_places(-50), d("1.5e100"));
        assert_eq!(d("1e-20").round_to_places(2), Decimal::zero());
        assert_eq!(d("1e-20").ceil_to_places(2), d("0.01"));
        assert_eq!(d("-1e-20").floor_to_places(2), d("-0.01"));
        assert_eq!(d("-1e-20").ceil_to_places(2), Decimal::zero());
        assert_eq!(d("1e-300").ceil_to_places(2), d("0.01"));
        assert_eq!(d("1e-20").round_to_places(20), d("1e-20"));
        assert_eq!(d("1.4e-20").round_to_places(20), d("1e-20"));
        assert_eq!(d("1.6e-20").round_to_places(20), d("2e-20"));
        let deep = Decimal::from_components(1, 3, 20.0).recip();
        assert_eq!(deep.ceil_to_places(1), d("0.1"));
        assert_eq!(deep.round_to_places(1), Decimal::zero());
    }

    #[test]
    fn significant() {
        assert_eq!(d("123456").round_to_significant(3), d("123000"));
        assert_eq!(d("123456").round_to_significant(1), d("100000"));
        assert_eq!(d("0.0012345").round_to_significant(2), d("0.0012"));
        assert_eq!(d("9.99").round_to_significant(2), d("10"));
        assert_eq!(d("-9.99").round_to_significant(2), d("-10"));
        assert_eq!(d("0.5").round_to_significant(0), d("0.5"));
        // Layer-1 values carry ~15 digits, so compare against the same literal, not Display.
        assert_eq!(d("1.23456e100").round_to_significant(3), d("1.23e100"));
        assert_eq!(d("9.999e100").round_to_significant(2), d("1e101"));
        assert_eq!(d("1.23456e-100").round_to_significant(3), d("1.23e-100"));
        assert_eq!(d("1e1e15").round_to_significant(3), d("1e1e15"));
        assert_eq!(d("10^^3").round_to_significant(3), d("10^^3"));
        assert_eq!(d("1e100").round_to_significant(1), d("1e100"));
        assert_eq!(Decimal::zero().round_to_significant(3), Decimal::zero());
    }

    #[test]
    fn fract_and_is_integer() {
        assert_eq!(d("3.75").fract(), d("0.75"));
        assert_eq!(d("-3.75").fract(), d("-0.75"));
        assert_eq!(d("1e100").fract(), Decimal::zero());
        assert_eq!(d("1e-100").fract(), d("1e-100"));
        assert!(d("1e100").is_integer());
        assert!(d("3").is_integer());
        assert!(Decimal::zero().is_integer());
        assert!(!d("3.5").is_integer());
        assert!(!d("1e-100").is_integer());
        assert!(!Decimal::neg_inf().is_integer());
    }
}
