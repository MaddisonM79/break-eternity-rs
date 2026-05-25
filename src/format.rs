//! Display, formatting, and string-conversion impls for [`Decimal`].

use std::fmt::{Display, LowerExp, UpperExp};

use crate::constants::MAX_ES_IN_A_ROW;
use crate::decimal::Decimal;

// ---------------------------------------------------------------------------
// Standalone formatting helpers (public so decimal.rs can call them)
// ---------------------------------------------------------------------------

/// Formats the given number to the given number of significant digits.
pub fn to_fixed(num: f64, places: i32) -> String {
    format!("{:.*}", places.try_into().unwrap_or(0usize), num)
}

/// Truncates the given number to the given number of significant digits and rounds, if necessary.
pub fn decimal_places(num: f64, places: i32) -> f64 {
    let len = places as f64 + 1_f64;
    let num_digits = num.abs().log10().ceil();
    let rounded = (num * 10_f64.powf(len - num_digits)).round() * 10_f64.powf(num_digits - len);
    to_fixed(rounded, (len - num_digits).max(0_f64) as i32)
        .parse()
        .unwrap()
}

// ---------------------------------------------------------------------------
// impl Decimal — formatting methods
// ---------------------------------------------------------------------------

impl Decimal {
    /// Returns the Decimal as a String with the specified amount of decimal places.
    pub fn to_fixed(&self, places: usize) -> String {
        if self.layer == 0 {
            return format!("{:.*}", places, self.mag);
        }

        self.to_string_with_decimal_places(places, None)
    }

    /// Returns the Decimal as a String with the specified amount of precision.
    ///
    /// If the amount of decimal places is larger than that the exponent, this will return a fixed representation of the number.
    ///
    /// Otherwise, this will return a scientific representation of the number.
    pub fn to_precision(&self, places: usize) -> String {
        if self.exponent() <= -7.0 {
            return format!("{:.*e}", places - 1, self);
        }

        if places as f64 > self.exponent() {
            return self.to_fixed(places - self.exponent() as usize - 1);
        }

        format!("{:.*e}", places - 1, self)
    }

    /// Returns the Decimal as a String with the specified amount of precision.
    ///
    /// This follows more rules than `to_precision`.
    ///
    /// If the layer of the Decimal is 0 and the magnitude is less than 1e21 and larger than 1e-7 (or when the magnitude is 0),
    /// this will return a fixed representation of the number.
    /// If the layer is 0, a scientific representation of the number will be returned.
    ///
    /// If the layer is 1, a scientific representation of the number will be returned.
    ///
    /// Otherwise, a scientific representation with multiple es will be returned.
    pub fn to_string_with_decimal_places(&self, places: usize, e_lower: Option<bool>) -> String {
        let e = if e_lower.unwrap_or(true) { "e" } else { "E" };

        if self.layer == 0 {
            if (self.mag < 1e21 && self.mag > 1e-7) || self.mag == 0.0 {
                return format!("{:.*}", places, self.sign as f64 * self.mag);
            }
            return format!(
                "{:.*}{}{:.*}",
                places,
                decimal_places(self.mantissa(), places as i32),
                e,
                places,
                decimal_places(self.exponent(), places as i32)
            );
        }

        if self.layer == 1 {
            return format!(
                "{:.*}{}{:.*}",
                places,
                decimal_places(self.mantissa(), places as i32),
                e,
                places,
                decimal_places(self.exponent(), places as i32)
            );
        }

        if self.layer <= MAX_ES_IN_A_ROW as i64 {
            format!(
                "{}{}{:.*}",
                if self.sign > 0 { "" } else { "-" },
                e.repeat(self.layer as usize),
                places,
                decimal_places(self.mag, places as i32)
            )
        } else {
            format!(
                "{}({}^{}){:.*}",
                if self.sign > 0 { "" } else { "-" },
                e,
                self.layer,
                places,
                decimal_places(self.mag, places as i32)
            )
        }
    }
}

// ---------------------------------------------------------------------------
// std::fmt trait impls
// ---------------------------------------------------------------------------

impl LowerExp for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Decimal::inf() {
            return write!(f, "Infinity");
        }

        if *self == Decimal::neg_inf() {
            return write!(f, "-Infinity");
        }

        if *self == Decimal::nan() {
            return write!(f, "NaN");
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Decimal::inf() {
            return write!(f, "Infinity");
        }

        if *self == Decimal::neg_inf() {
            return write!(f, "-Infinity");
        }

        if *self == Decimal::nan() {
            return write!(f, "NaN");
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Decimal::inf() {
            return write!(f, "Infinity");
        }

        if *self == Decimal::neg_inf() {
            return write!(f, "-Infinity");
        }

        if *self == Decimal::nan() {
            return write!(f, "NaN");
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
