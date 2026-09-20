//! Game-style number notations: scientific, engineering, standard (illion names), letters,
//! and logarithm.
//!
//! [`Decimal::to_notation`] returns a `String`; [`Decimal::display`] returns a
//! [`NotationDisplay`] adapter that implements [`Display`] so the value can
//! be written straight into a formatter. Both take a [`Notation`] and the number of digits to
//! show after the decimal point of the mantissa.
//!
//! ```
//! use break_eternity::{Decimal, Notation};
//!
//! let money = Decimal::from_finite(1_234_567.0);
//! assert_eq!(money.to_notation(Notation::Scientific, 2), "1.23e6");
//! assert_eq!(money.to_notation(Notation::Engineering, 2), "1.23e6");
//! assert_eq!(money.to_notation(Notation::Standard, 2), "1.23 M");
//! assert_eq!(money.to_notation(Notation::Letters, 2), "1.23 b");
//! assert_eq!(money.to_notation(Notation::Logarithm, 2), "e6.09");
//! assert_eq!(format!("{}", Decimal::from_finite(42.0).display(Notation::Standard, 1)), "42.0");
//! ```
//!
//! # Common rules
//!
//! * Values whose magnitude rounds to less than `1000` (and is at least `0.001`) print as a
//!   plain fixed-point number with `places` digits, in every notation.
//! * Magnitudes below `0.001` use scientific notation (`5.00e-4`) in every notation except
//!   [`Engineering`](Notation::Engineering) (`500.00e-6`) and
//!   [`Logarithm`](Notation::Logarithm) (`e-3.30`).
//! * Exponents are printed as integers while they are below `1e9`. Above that the mantissa of
//!   an `f64` exponent carries no information, so the value is written as `e` followed by its
//!   base-10 logarithm in the same notation: `10^(1.5e12)` is `e1.50e12` in scientific and
//!   `ee12.18` in logarithm notation. This repeats for each layer up to
//!   [`MAX_ES_IN_A_ROW`]; taller towers print as `(e^N)X` like
//!   [`Display`].
//! * Rounding carries: `999_999.5` at two places is `1.00e6`, not `10.00e5`.
//!
//! The illion abbreviations follow the scheme used by *Antimatter Dimensions*
//! (`K`, `M`, `B`, `T`, `Qa`, `Qt`, `Sx`, `Sp`, `Oc`, `No`, `Dc`, `UDc`, ... `Ce`, ... `MI`).
//! [`standard_abbreviation`] and [`letters_abbreviation`] expose the suffix tables for
//! building custom formatters.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::{self, Display, Write};

use crate::constants::{power_of_10, MAX_ES_IN_A_ROW};
use crate::decimal::Decimal;
#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
// shadowed by std's inherent methods whenever std is in the crate graph
use crate::math::FloatExt;

/// Largest exponent that is printed as an integer; see the [module docs](self).
const EXPONENT_SHOWN_BELOW: f64 = 1e9;
/// Magnitudes below this use scientific notation instead of fixed point.
const FIXED_MIN: f64 = 1e-3;
/// Magnitudes that round to at least this use a mantissa/exponent form.
const FIXED_MAX: f64 = 1e3;
/// `to_fixed` clamps at this many places; so do we.
const MAX_PLACES: usize = 100;

/// A way of writing a [`Decimal`] for display in a game UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Notation {
    /// `1.23e45`. The default.
    #[default]
    Scientific,
    /// `123.45e42`: the exponent is a multiple of three and the mantissa is in `[1, 1000)`.
    Engineering,
    /// `1.23 QaDc`: illion abbreviations, see [`standard_abbreviation`].
    Standard,
    /// `1.23 ab`: bijective base-26 letters, one step per factor of `1000` (`a` = `1e3`,
    /// `z` = `1e78`, `aa` = `1e81`).
    Letters,
    /// `e45.09`: the base-10 logarithm, prefixed with `e`.
    Logarithm,
}

impl Notation {
    /// Formats `value` in this notation with `places` digits after the decimal point.
    ///
    /// Same as [`Decimal::to_notation`].
    pub fn format(self, value: &Decimal, places: usize) -> String {
        value.display(self, places).to_string()
    }
}

/// [`Display`] adapter returned by [`Decimal::display`].
#[derive(Debug, Clone, Copy)]
pub struct NotationDisplay<'a> {
    value: &'a Decimal,
    notation: Notation,
    places: usize,
}

impl Display for NotationDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d = self.value;
        if d.has_nan_mag() {
            return f.write_str("NaN");
        }
        if d.is_infinite() {
            return f.write_str(if d.sign() == 1 {
                "Infinity"
            } else {
                "-Infinity"
            });
        }
        if d.sign() == -1 {
            f.write_char('-')?;
        }
        write_magnitude(&d.abs(), self.notation, self.places.min(MAX_PLACES), f)
    }
}

impl Decimal {
    /// Formats the value in `notation` with `places` digits after the decimal point.
    ///
    /// See the [`notation`](crate::notation) module docs for the rules shared by all notations.
    ///
    /// ```
    /// use break_eternity::{Decimal, Notation};
    ///
    /// let d: Decimal = "1.5e100".parse().unwrap();
    /// assert_eq!(d.to_notation(Notation::Scientific, 1), "1.5e100");
    /// assert_eq!(d.to_notation(Notation::Engineering, 1), "15.0e99");
    /// assert_eq!(d.to_notation(Notation::Standard, 1), "15.0 DTg");
    /// assert_eq!(d.to_notation(Notation::Letters, 1), "15.0 ag");
    /// assert_eq!(d.to_notation(Notation::Logarithm, 1), "e100.2");
    /// ```
    pub fn to_notation(&self, notation: Notation, places: usize) -> String {
        self.display(notation, places).to_string()
    }

    /// Returns a [`Display`] adapter that writes the value in `notation` with `places` digits
    /// after the decimal point, without allocating a `String` first.
    ///
    /// ```
    /// use break_eternity::{Decimal, Notation};
    ///
    /// let d = Decimal::from_finite(2.5e9);
    /// let label = format!("Gold: {}", d.display(Notation::Standard, 2));
    /// assert_eq!(label, "Gold: 2.50 B");
    /// ```
    pub fn display(&self, notation: Notation, places: usize) -> NotationDisplay<'_> {
        NotationDisplay {
            value: self,
            notation,
            places,
        }
    }
}

/// Rounds `x` to `places` decimal places.
fn round_places(x: f64, places: usize) -> f64 {
    let scale = 10f64.powi(places as i32);
    (x * scale).round() / scale
}

/// Splits a positive layer-0 magnitude into `(mantissa in [1, 10), integer exponent)`.
fn split_layer0(x: f64) -> (f64, i64) {
    let mut e = x.log10().floor() as i64;
    let mut m = x / power_of_10(e as i32);
    // log10 can be off by one ulp right at a power of ten.
    if m >= 10.0 {
        m /= 10.0;
        e += 1;
    } else if m < 1.0 {
        m *= 10.0;
        e -= 1;
    }
    (m, e)
}

/// Writes a finite, non-negative `d`.
fn write_magnitude(
    d: &Decimal,
    notation: Notation,
    places: usize,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if d.is_zero() {
        return write!(f, "{:.*}", places, 0.0);
    }

    let (m, e) = if d.layer() == 0 {
        let x = d.mag();
        if x >= FIXED_MIN {
            let rounded = round_places(x, places);
            if rounded < FIXED_MAX {
                return write!(f, "{rounded:.places$}");
            }
        }
        split_layer0(x)
    } else if d.layer() == 1 && d.mag().abs() < EXPONENT_SHOWN_BELOW {
        let e = d.mag().floor();
        (10f64.powf(d.mag() - e), e as i64)
    } else {
        // Too tall for a mantissa: `e` + the logarithm, or `(e^N)X` past the cap.
        if d.layer() > MAX_ES_IN_A_ROW as i64 {
            return write!(f, "(e^{}){:.*}", d.layer(), places, d.mag());
        }
        f.write_char('e')?;
        let log = d.abs_log10();
        let inner = match notation {
            Notation::Logarithm => Notation::Logarithm,
            _ => Notation::Scientific,
        };
        return write_signed(&log, inner, places, f);
    };

    match notation {
        Notation::Scientific => write_scientific(m, e, places, f),
        Notation::Engineering => write_engineering(m, e, places, f),
        Notation::Standard | Notation::Letters => {
            let (em, ee) = engineering(m, e, places);
            if ee < 3 {
                return write_scientific(m, e, places, f);
            }
            let suffix = if notation == Notation::Standard {
                standard_abbreviation((ee / 3) as u64)
            } else {
                letters_abbreviation((ee / 3) as u64)
            };
            write!(f, "{em:.places$} {suffix}")
        }
        Notation::Logarithm => {
            let log = if d.layer() == 0 {
                d.mag().log10()
            } else {
                d.mag()
            };
            write!(f, "e{log:.places$}")
        }
    }
}

/// Writes a possibly negative value (used for exponents, which may be negative).
fn write_signed(
    d: &Decimal,
    notation: Notation,
    places: usize,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if d.sign() == -1 {
        f.write_char('-')?;
    }
    write_magnitude(&d.abs(), notation, places, f)
}

fn write_scientific(
    mut m: f64,
    mut e: i64,
    places: usize,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if round_places(m, places) >= 10.0 {
        m = 1.0;
        e += 1;
    }
    write!(f, "{m:.places$}e{e}")
}

/// Shifts `(m, e)` so that `e` is a multiple of three and `m` is in `[1, 1000)` after rounding.
fn engineering(mut m: f64, mut e: i64, places: usize) -> (f64, i64) {
    let shift = e.rem_euclid(3);
    m *= power_of_10(shift as i32);
    e -= shift;
    if round_places(m, places) >= 1000.0 {
        m = 1.0;
        e += 3;
    }
    (m, e)
}

fn write_engineering(m: f64, e: i64, places: usize, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let (m, e) = engineering(m, e, places);
    write!(f, "{m:.places$}e{e}")
}

const ABBREVIATIONS: [&str; 11] = ["", "K", "M", "B", "T", "Qa", "Qt", "Sx", "Sp", "Oc", "No"];
const PREFIXES: [[&str; 10]; 3] = [
    ["", "U", "D", "T", "Qa", "Qt", "Sx", "Sp", "O", "N"],
    ["", "Dc", "Vg", "Tg", "Qd", "Qi", "Se", "St", "Og", "Nn"],
    ["", "Ce", "Dn", "Tc", "Qe", "Qu", "Sc", "Si", "Oe", "Ne"],
];
const PREFIXES_2: [&str; 8] = ["", "MI-", "MC-", "NA-", "PC-", "FM-", "AT-", "ZP-"];

/// Returns the illion abbreviation for `1000^thousands`.
///
/// `0` is the empty string, `1` is `K`, `2` is `M`, `11` (`1e33`) is `Dc`, `101` (`1e303`) is
/// `Ce`, and `1001` (`1e3003`) is `MI`. Beyond the first ten, the name is assembled from
/// ones / tens / hundreds prefixes per base-1000 group of `thousands - 1`, with the groups
/// joined by `MI-`, `MC-`, `NA-`, `PC-`, `FM-`, `AT-`, `ZP-`; every `u64` has a name.
///
/// ```
/// use break_eternity::notation::standard_abbreviation;
///
/// assert_eq!(standard_abbreviation(1), "K");
/// assert_eq!(standard_abbreviation(12), "UDc");
/// assert_eq!(standard_abbreviation(21), "Vg");
/// assert_eq!(standard_abbreviation(1002), "MI-U");
/// ```
pub fn standard_abbreviation(thousands: u64) -> String {
    if let Some(short) = ABBREVIATIONS.get(thousands as usize) {
        return (*short).to_string();
    }
    let exp = thousands - 1;
    // Prefixes for each decimal digit of `exp`, least significant first, cycling
    // ones / tens / hundreds.
    let mut prefix: Vec<&str> = Vec::new();
    let mut rest = exp;
    let mut digit = 0usize;
    prefix.push(PREFIXES[0][(rest % 10) as usize]);
    while rest >= 10 {
        rest /= 10;
        digit += 1;
        prefix.push(PREFIXES[digit % 3][(rest % 10) as usize]);
    }
    // A u64 has at most 20 digits, so `groups` is at most 6 and never runs off PREFIXES_2.
    let groups = digit / 3;
    while !prefix.len().is_multiple_of(3) {
        prefix.push("");
    }

    let mut out = String::new();
    for g in (0..=groups).rev() {
        let (ones, tens, hundreds) = (prefix[3 * g], prefix[3 * g + 1], prefix[3 * g + 2]);
        if ones.is_empty() && tens.is_empty() && hundreds.is_empty() {
            continue;
        }
        // "millillion", not "unmillillion": a lone `U` in a joined group is dropped.
        if !(g > 0 && ones == "U" && tens.is_empty() && hundreds.is_empty()) {
            out.push_str(ones);
        }
        out.push_str(tens);
        out.push_str(hundreds);
        out.push_str(PREFIXES_2[g]);
    }
    if out.ends_with('-') {
        out.pop();
    }
    out
}

/// Returns the letters abbreviation for `1000^thousands`: bijective base 26, so `1` is `a`,
/// `26` is `z`, `27` is `aa`. `0` is the empty string.
///
/// ```
/// use break_eternity::notation::letters_abbreviation;
///
/// assert_eq!(letters_abbreviation(1), "a");
/// assert_eq!(letters_abbreviation(27), "aa");
/// assert_eq!(letters_abbreviation(703), "aaa");
/// ```
pub fn letters_abbreviation(thousands: u64) -> String {
    let mut n = thousands;
    let mut letters: Vec<u8> = Vec::new();
    while n > 0 {
        n -= 1;
        letters.push(b'a' + (n % 26) as u8);
        n /= 26;
    }
    letters.reverse();
    String::from_utf8(letters).expect("ASCII letters")
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use Notation::{Engineering, Letters, Logarithm, Scientific, Standard};

    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    fn all(s: &str, places: usize) -> [String; 5] {
        let v = d(s);
        [
            v.to_notation(Scientific, places),
            v.to_notation(Engineering, places),
            v.to_notation(Standard, places),
            v.to_notation(Letters, places),
            v.to_notation(Logarithm, places),
        ]
    }

    #[test]
    fn small_values_are_fixed_point_everywhere() {
        assert_eq!(all("0", 2), ["0.00"; 5]);
        assert_eq!(all("42", 1), ["42.0"; 5]);
        assert_eq!(all("-999.4", 0), ["-999"; 5]);
        assert_eq!(all("0.05", 2), ["0.05"; 5]);
    }

    #[test]
    fn thousands_and_millions() {
        assert_eq!(
            all("1234", 2),
            ["1.23e3", "1.23e3", "1.23 K", "1.23 a", "e3.09"]
        );
        assert_eq!(
            all("12345678", 2),
            ["1.23e7", "12.35e6", "12.35 M", "12.35 b", "e7.09"]
        );
        assert_eq!(
            all("-2.5e9", 1),
            ["-2.5e9", "-2.5e9", "-2.5 B", "-2.5 c", "-e9.4"]
        );
    }

    #[test]
    fn rounding_carries_across_the_threshold() {
        assert_eq!(
            all("999.999", 2),
            ["1.00e3", "1.00e3", "1.00 K", "1.00 a", "e3.00"]
        );
        assert_eq!(d("9.999e5").to_notation(Scientific, 2), "1.00e6");
        assert_eq!(d("999999.5").to_notation(Engineering, 2), "1.00e6");
        assert_eq!(d("999999.5").to_notation(Standard, 2), "1.00 M");
        assert_eq!(d("999.9").to_notation(Standard, 1), "999.9");
    }

    #[test]
    fn tiny_values() {
        assert_eq!(
            all("0.0005", 2),
            ["5.00e-4", "500.00e-6", "5.00e-4", "5.00e-4", "e-3.30"]
        );
        assert_eq!(d("1e-20").to_notation(Scientific, 1), "1.0e-20");
        assert_eq!(d("1e-20").to_notation(Engineering, 1), "10.0e-21");
        assert_eq!(d("1e-20").to_notation(Logarithm, 1), "e-20.0");
    }

    #[test]
    fn illion_table() {
        let names = [
            (1, "K"),
            (2, "M"),
            (10, "No"),
            (11, "Dc"),
            (12, "UDc"),
            (13, "DDc"),
            (21, "Vg"),
            (22, "UVg"),
            (101, "Ce"),
            (102, "UCe"),
            (111, "DcCe"),
            (112, "UDcCe"),
            (1000, "NNnNe"),
            (1001, "MI"),
            (1002, "MI-U"),
            (1012, "MI-UDc"),
            (2001, "DMI"),
            (1_000_001, "MC"),
            (1_001_001, "MC-MI"),
            (2_001_002, "DMC-MI-U"),
        ];
        for (k, name) in names {
            assert_eq!(standard_abbreviation(k), name, "{k}");
        }
        assert_eq!(standard_abbreviation(0), "");
        assert_eq!(
            standard_abbreviation(u64::MAX),
            "ODcAT-SxQdQeFM-QaQdSiPC-TStNA-NSiMC-UQiQuMI-QaDcSc"
        );
        assert_eq!(d("1e33").to_notation(Standard, 2), "1.00 Dc");
        assert_eq!(d("1e303").to_notation(Standard, 2), "1.00 Ce");
        assert_eq!(d("1e3003").to_notation(Standard, 2), "1.00 MI");
        assert_eq!(d("1e3006").to_notation(Standard, 2), "1.00 MI-U");
    }

    #[test]
    fn letters_table() {
        assert_eq!(letters_abbreviation(0), "");
        assert_eq!(letters_abbreviation(26), "z");
        assert_eq!(letters_abbreviation(27), "aa");
        assert_eq!(letters_abbreviation(52), "az");
        assert_eq!(letters_abbreviation(53), "ba");
        assert_eq!(letters_abbreviation(702), "zz");
        assert_eq!(d("1e78").to_notation(Letters, 0), "1 z");
        assert_eq!(d("1e81").to_notation(Letters, 0), "1 aa");
    }

    #[test]
    fn layer_one_and_huge_exponents() {
        assert_eq!(d("1.5e100").to_notation(Scientific, 2), "1.50e100");
        assert_eq!(
            d("1e999999999").to_notation(Scientific, 2),
            "1.00e999999999"
        );
        assert_eq!(d("1e999999999").to_notation(Logarithm, 2), "e999999999.00");
        assert_eq!(
            all("1e1000000000", 2),
            ["e1.00e9", "e1.00e9", "e1.00e9", "e1.00e9", "ee9.00"]
        );
        assert_eq!(d("1e1e15").to_notation(Standard, 2), "e1.00e15");
        assert_eq!(d("1e1e15").to_notation(Logarithm, 2), "ee15.00");
    }

    #[test]
    fn tall_towers() {
        let t3 = d("10^^3"); // 10^10^10
        assert_eq!(t3.to_notation(Scientific, 2), "e1.00e10");
        assert_eq!(t3.to_notation(Logarithm, 2), "ee10.00");
        let t4 = d("10^^4");
        assert_eq!(t4.to_notation(Scientific, 2), "ee1.00e10");
        assert_eq!(t4.to_notation(Logarithm, 2), "eee10.00");
        let deep = Decimal::from_components(1, 7, 20.0);
        assert_eq!(deep.to_notation(Scientific, 2), "(e^7)20.00");
        assert_eq!(deep.to_notation(Standard, 2), "(e^7)20.00");
        assert_eq!((-deep).to_notation(Logarithm, 1), "-(e^7)20.0");
        // Negative mag above layer 1 flips the top of the tower: 10^-(10^20).
        let tiny = Decimal::from_components(1, 2, -20.0);
        assert_eq!(tiny, Decimal::from_components(1, 2, 20.0).recip());
        assert_eq!(tiny.to_notation(Scientific, 2), "e-1.00e20");
    }

    #[test]
    fn specials_and_adapter() {
        assert_eq!(Decimal::inf().to_notation(Standard, 2), "Infinity");
        assert_eq!(Decimal::neg_inf().to_notation(Letters, 2), "-Infinity");
        assert_eq!(Notation::Standard.format(&d("1e6"), 0), "1 M");
        assert_eq!(
            format!("[{}]", d("1e6").display(Notation::default(), 3)),
            "[1.000e6]"
        );
        assert_eq!(d("1").to_notation(Scientific, 500).len(), 102);
    }
}
