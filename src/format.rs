//! Display, formatting, and string-conversion impls for [`Decimal`].

use alloc::format;
use alloc::string::{String, ToString};
use core::fmt::{Display, LowerExp, UpperExp};

use crate::constants::MAX_ES_IN_A_ROW;
use crate::decimal::Decimal;
#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
// shadowed by std's inherent methods whenever std is in the crate graph
use crate::math::FloatExt;

/// JavaScript's `toFixed`/`toPrecision` refuse more than 100 digits; we clamp instead.
const MAX_PLACES: usize = 100;

// ---------------------------------------------------------------------------
// Standalone formatting helpers
// ---------------------------------------------------------------------------

/// Formats the given number to `places` decimal places.
pub fn to_fixed(num: f64, places: usize) -> String {
    let places = places.min(MAX_PLACES);
    format!("{num:.places$}")
}

/// Rounds `num` to `places` decimal places.
pub fn decimal_places(num: f64, places: i32) -> f64 {
    if !num.is_finite() {
        return num;
    }
    if places < 0 {
        return num;
    }
    let factor = 10f64.powi(places);
    (num * factor).round() / factor
}

/// Returns the fixed string for the special values, if `d` is one.
fn special(d: &Decimal) -> Option<&'static str> {
    if d.has_nan_mag() {
        return Some("NaN");
    }
    if d.is_infinite() {
        return Some(if d.sign() == 1 {
            "Infinity"
        } else {
            "-Infinity"
        });
    }
    None
}

// ---------------------------------------------------------------------------
// impl Decimal — formatting methods
// ---------------------------------------------------------------------------

impl Decimal {
    /// Formats the Decimal with `places` digits after the decimal point.
    ///
    /// Layer-0 values print as plain fixed-point numbers (`-5.00`). Larger values fall back to
    /// [`to_string_with_decimal_places`](Self::to_string_with_decimal_places), which applies the
    /// digit count to the mantissa (`1.50e100`).
    pub fn to_fixed(&self, places: usize) -> String {
        if let Some(s) = special(self) {
            return s.to_string();
        }
        if self.layer == 0 {
            return to_fixed(self.sign as f64 * self.mag, places);
        }

        self.to_string_with_decimal_places(places, None)
    }

    /// Formats the Decimal in scientific notation with `places` digits after the decimal
    /// point of the mantissa (`1.50e100`, `-1.23e-7`).
    pub fn to_exponential(&self, places: usize) -> String {
        if let Some(s) = special(self) {
            return s.to_string();
        }
        let places = places.min(MAX_PLACES);
        if self.layer == 0 {
            return format!("{:.*e}", places, self.sign as f64 * self.mag);
        }
        self.to_string_with_decimal_places(places, None)
    }

    /// Formats the Decimal with `places` significant digits.
    ///
    /// Values with an exponent below -7, or with more integer digits than `places`, use
    /// scientific notation; everything else is fixed-point. `places` is clamped to at least 1.
    pub fn to_precision(&self, places: usize) -> String {
        if let Some(s) = special(self) {
            return s.to_string();
        }
        let places = places.clamp(1, MAX_PLACES);
        let e = self.exponent();

        if !e.is_finite() || e <= -7.0 {
            return self.to_exponential(places - 1);
        }

        if places as f64 > e {
            // e may be negative, in which case we need extra decimal places.
            let fixed_places = (places as f64 - e - 1.0).max(0.0) as usize;
            return self.to_fixed(fixed_places);
        }

        self.to_exponential(places - 1)
    }

    /// Formats the Decimal with `places` digits after the decimal point of the mantissa.
    ///
    /// * Layer 0 values with `1e-7 < |x| < 1e21` (or zero) print in fixed-point.
    /// * Other layer 0 and layer 1 values print as `MeX` with the mantissa rounded to `places`
    ///   digits and an integer exponent.
    /// * Layers 2 through 5 print as `eee...X` with `X` rounded to `places` digits; higher
    ///   layers print as `(e^N)X`.
    ///
    /// `e_lower` selects a lowercase (`Some(true)`, the default) or uppercase `E`.
    pub fn to_string_with_decimal_places(&self, places: usize, e_lower: Option<bool>) -> String {
        if let Some(s) = special(self) {
            return s.to_string();
        }
        let places = places.min(MAX_PLACES);
        let e = if e_lower.unwrap_or(true) { "e" } else { "E" };
        let sign = if self.sign == -1 { "-" } else { "" };

        if self.layer == 0 {
            if (self.mag < 1e21 && self.mag > 1e-7) || self.mag == 0.0 {
                return format!("{:.*}", places, self.sign as f64 * self.mag);
            }
            return format!("{:.*}{}{}", places, self.mantissa(), e, self.exponent());
        }

        if self.layer == 1 {
            return format!("{:.*}{}{}", places, self.mantissa(), e, self.exponent());
        }

        if self.layer <= MAX_ES_IN_A_ROW as i64 {
            format!(
                "{sign}{}{:.*}",
                e.repeat(self.layer as usize),
                places,
                self.mag
            )
        } else {
            format!("{sign}({e}^{}){:.*}", self.layer, places, self.mag)
        }
    }
}

// ---------------------------------------------------------------------------
// core::fmt trait impls
// ---------------------------------------------------------------------------

impl LowerExp for Decimal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(s) = special(self) {
            return write!(f, "{s}");
        }

        let precision = f.precision().unwrap_or(2);

        if self.layer == 0 {
            return write!(f, "{:.*e}", precision, self.sign as f64 * self.mag);
        }
        write!(
            f,
            "{}",
            self.to_string_with_decimal_places(precision, Some(true))
        )
    }
}

impl UpperExp for Decimal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(s) = special(self) {
            return write!(f, "{s}");
        }

        let precision = f.precision().unwrap_or(2);

        if self.layer == 0 {
            return write!(f, "{:.*E}", precision, self.sign as f64 * self.mag);
        }
        write!(
            f,
            "{}",
            self.to_string_with_decimal_places(precision, Some(false))
        )
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(s) = special(self) {
            return write!(f, "{s}");
        }

        if self.layer == 0 {
            if (self.mag < 1e21 && self.mag > 1e-7) || self.mag == 0.0 {
                return write!(f, "{}", self.sign as f64 * self.mag);
            }
            return write!(f, "{}e{}", self.mantissa(), self.exponent());
        }
        if self.layer == 1 {
            return write!(f, "{}e{}", self.mantissa(), self.exponent());
        }
        if self.layer <= MAX_ES_IN_A_ROW as i64 {
            return write!(
                f,
                "{}{}{}",
                if self.sign == -1 { "-" } else { "" },
                "e".repeat(self.layer as usize),
                self.mag
            );
        }
        write!(
            f,
            "{}(e^{}){}",
            if self.sign == -1 { "-" } else { "" },
            self.layer,
            self.mag
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    #[test]
    fn to_fixed_keeps_sign_and_integer_exponent() {
        assert_eq!(Decimal::from(-5).to_fixed(2), "-5.00");
        assert_eq!(d("-1.5e100").to_fixed(2), "-1.50e100");
        assert_eq!(d("1.5e100").to_fixed(0), "2e100");
        assert_eq!(d("ee100.5").to_fixed(2), "ee100.50");
        assert_eq!(d("(e^10)33.3").to_fixed(1), "(e^10)33.3");
        assert_eq!(Decimal::from_finite(0.00123).to_fixed(2), "0.00");
        assert_eq!(Decimal::inf().to_fixed(2), "Infinity");
        assert_eq!(Decimal::neg_inf().to_fixed(2), "-Infinity");
    }

    #[test]
    fn to_precision_handles_small_and_negative_values() {
        assert_eq!(Decimal::from(-5).to_precision(3), "-5.00");
        assert_eq!(Decimal::from_finite(0.00123).to_precision(2), "0.0012");
        assert_eq!(Decimal::from_finite(12345.0).to_precision(2), "1.2e4");
        assert_eq!(Decimal::from_finite(12345.0).to_precision(6), "12345.0");
        assert_eq!(Decimal::from_finite(1e-8).to_precision(3), "1.00e-8");
        assert_eq!(d("1.5e100").to_precision(3), "1.50e100");
        // places == 0 is clamped to 1 instead of panicking.
        assert_eq!(Decimal::from_finite(0.00123).to_precision(0), "0.001");
        assert_eq!(d("ee-100").to_precision(3), "ee-100.00");
    }

    #[test]
    fn to_exponential() {
        assert_eq!(Decimal::from(1234).to_exponential(2), "1.23e3");
        assert_eq!(Decimal::from(-1234).to_exponential(1), "-1.2e3");
        assert_eq!(Decimal::from_finite(1e-7).to_exponential(1), "1.0e-7");
        assert_eq!(d("1e1000").to_exponential(3), "1.000e1000");
        assert_eq!(
            format!("{:.3e}", Decimal::from_finite(1234.5678)),
            "1.235e3"
        );
        assert_eq!(format!("{:E}", d("1.5e100")), "1.50E100");
    }

    #[test]
    fn display_round_trips_and_specials() {
        for s in [
            "0",
            "1",
            "-1",
            "0.1",
            "1e-7",
            "1e21",
            "1e308",
            "1e1000",
            "-ee100",
            "ee-100",
            "(e^10)100",
            "-(e^7)15.5",
        ] {
            let v = d(s);
            let back = d(&v.to_string());
            assert_eq!(v, back, "{s}");
        }
        assert_eq!(Decimal::inf().to_string(), "Infinity");
        assert_eq!(Decimal::neg_inf().to_string(), "-Infinity");
        assert_eq!((Decimal::inf() * Decimal::two()).to_string(), "Infinity");
        assert_eq!(Decimal::neg_inf().abs().to_string(), "Infinity");
        assert_eq!(Decimal::inf().floor().to_string(), "Infinity");
        assert_eq!(Decimal::inf().ln().to_string(), "Infinity");
    }
}
