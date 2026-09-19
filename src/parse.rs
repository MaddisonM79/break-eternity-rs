//! String parsing for [`Decimal`].
//!
//! Provides equivalent entry points: [`Decimal::from_string`], [`Decimal::from_string_with_mode`],
//! the [`TryFrom<&str>`] impl, and the [`std::str::FromStr`] impl. Together these port
//! `fromString` from `break_eternity.js` 2.1.3.
//!
//! The parser never panics. Malformed input returns [`BreakEternityError::ParseError`];
//! syntactically valid input whose value is mathematically undefined (for example
//! `"(-2)^^2.5"`) returns [`BreakEternityError::ParseUndefined`].

use std::convert::TryFrom;
use std::str::FromStr;

use crate::constants::commas_are_decimal_points;
use crate::constants::ignore_commas;
use crate::decimal::Decimal;
use crate::error::BreakEternityError;
use crate::tetration::TetrationMode;
use crate::utils::{f_maglog10, sign};

impl Decimal {
    /// Parses a string into a [`Decimal`].
    ///
    /// Accepts the formats produced by [`Decimal`]'s [`Display`](std::fmt::Display) impl as
    /// well as the additional notations recognized by `break_eternity.js`:
    ///
    /// * Plain decimals: `"0"`, `"-5"`, `"3.14"`, `"1,000,000"` (commas are ignored)
    /// * Scientific: `"1.23e45"`, `"1.23e+45"`, `"1.23e-45"`, `"1e1000"` (past `f64` range),
    ///   subnormal literals such as `"2.47e-324"`
    /// * Stacked exponents: `"eN"` (`10^N`), `"eeN"`, `"eeeN"`, …, and `"MeXeY"` (`M·10^(XeY)`)
    /// * Power / tetrate / pentate operators: `"X^Y"`, `"X^^N"`, `"X^^N;P"` (payload),
    ///   `"X^^^N"`, `"X^^^N;P"`
    /// * Parenthesized large layer: `"(e^N)M"` (the [`Display`](std::fmt::Display) form for very
    ///   high layers; negative or fractional `N` is interpreted as `10^^N` with payload `M`)
    /// * Base-10 tetrate shorthands: `"N PT M"`, `"N PT (M)"`, `"NpM"`, and `"MfN"` / `"fN"`
    /// * Specials: `"Infinity"`, `"-Infinity"`. `"NaN"` returns an error since NaN is not a
    ///   representable [`Decimal`].
    ///
    /// Surrounding whitespace is trimmed and letters are case-insensitive. Fractional tetration
    /// heights use [`TetrationMode::Analytic`]; see
    /// [`from_string_with_mode`](Self::from_string_with_mode) to choose.
    ///
    /// # Examples
    ///
    /// ```
    /// use break_eternity::Decimal;
    ///
    /// let d = Decimal::from_string("1.23e45").unwrap();
    /// assert!((d.mantissa() - 1.23).abs() < 1e-12);
    /// assert_eq!(d.exponent(), 45.0);
    ///
    /// // Round-trip with Display
    /// let s = d.to_string();
    /// let d2 = Decimal::from_string(&s).unwrap();
    /// assert_eq!(d, d2);
    /// ```
    pub fn from_string(s: &str) -> Result<Decimal, BreakEternityError> {
        parse(s, TetrationMode::Analytic)
    }

    /// Parses a string like [`from_string`](Self::from_string), using `mode` for any
    /// fractional-height tetration the notation requires (`"10^^2.5"`, `"(e^1.5)5"`, …).
    pub fn from_string_with_mode(
        s: &str,
        mode: TetrationMode,
    ) -> Result<Decimal, BreakEternityError> {
        parse(s, mode)
    }
}

impl FromStr for Decimal {
    type Err = BreakEternityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse(s, TetrationMode::Analytic)
    }
}

impl TryFrom<&str> for Decimal {
    type Error = BreakEternityError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        parse(s, TetrationMode::Analytic)
    }
}

impl TryFrom<String> for Decimal {
    type Error = BreakEternityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        parse(&s, TetrationMode::Analytic)
    }
}

/// Parses a `&str` slice as an `f64`, mapping parse errors into [`BreakEternityError::ParseError`].
fn parse_f64(s: &str, orig: &str) -> Result<f64, BreakEternityError> {
    s.trim()
        .parse::<f64>()
        .map_err(|error| BreakEternityError::ParseError {
            parsed: orig.to_string(),
            error,
        })
}

/// Lenient float parse in the spirit of JS `parseFloat`: `None` if the text is not a number.
fn lenient_f64(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

fn parse_error(orig: &str) -> BreakEternityError {
    BreakEternityError::ParseError {
        parsed: orig.to_string(),
        error: "!".parse::<f64>().unwrap_err(),
    }
}

/// Maps the internal NaN sentinel to [`BreakEternityError::ParseUndefined`].
fn finish(d: Decimal, orig: &str) -> Result<Decimal, BreakEternityError> {
    if d.has_nan_mag() {
        Err(BreakEternityError::ParseUndefined {
            parsed: orig.to_string(),
        })
    } else {
        Ok(d)
    }
}

/// Strips parentheses and whitespace from a payload fragment and parses it, defaulting to 1.
fn payload_or_one(s: &str) -> f64 {
    let cleaned: String = s.chars().filter(|c| *c != '(' && *c != ')').collect();
    match lenient_f64(&cleaned) {
        Some(p) if p.is_finite() => p,
        _ => 1.0,
    }
}

/// Handles the `N PT M`, `NpM` shorthands: base-10 tetration with the height first.
fn tetrate_shorthand(
    height_part: &str,
    payload_part: &str,
    mode: TetrationMode,
) -> Option<Decimal> {
    let mut height_part = height_part.trim();
    let mut negative = false;
    if let Some(rest) = height_part.strip_prefix('-') {
        negative = true;
        height_part = rest;
    }
    let height = lenient_f64(height_part)?;
    if !height.is_finite() {
        return None;
    }
    let payload = payload_or_one(payload_part);
    let mut result = Decimal::ten().tetrate_raw(height, Decimal::from_f64(payload), mode);
    if negative {
        result = -result;
    }
    Some(result)
}

fn parse(s: &str, mode: TetrationMode) -> Result<Decimal, BreakEternityError> {
    let mut value = s.trim().to_string();
    if ignore_commas() {
        value = value.replace(',', "");
    } else if commas_are_decimal_points() {
        value = value.replace(',', ".");
    }
    let value = value.to_lowercase();
    let value = value.as_str();

    if value.is_empty() {
        return Err(parse_error(s));
    }

    // -----------------------------------------------------------------------
    // Specials
    // -----------------------------------------------------------------------
    match value {
        "nan" | "+nan" | "-nan" => return Err(parse_error(s)),
        "infinity" | "+infinity" | "inf" | "+inf" => return Ok(Decimal::inf()),
        "-infinity" | "-inf" => return Ok(Decimal::neg_inf()),
        _ => {}
    }

    // -----------------------------------------------------------------------
    // Pentate: "base^^^height" or "base^^^height;payload"
    // -----------------------------------------------------------------------
    let pentation_parts: Vec<&str> = value.split("^^^").collect();
    if pentation_parts.len() == 2 {
        let base = lenient_f64(pentation_parts[0]);
        let mut height_parts = pentation_parts[1].splitn(2, ';');
        let height = height_parts.next().and_then(lenient_f64);
        let payload = height_parts.next().map_or(1.0, payload_or_one);
        if let (Some(base), Some(height)) = (base, height) {
            if base.is_finite() && height.is_finite() {
                let r =
                    Decimal::from_f64(base).pentate_raw(height, Decimal::from_f64(payload), mode);
                return finish(r, s);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Tetrate: "base^^height" or "base^^height;payload"
    // -----------------------------------------------------------------------
    let tetration_parts: Vec<&str> = value.split("^^").collect();
    if tetration_parts.len() == 2 {
        let base = lenient_f64(tetration_parts[0]);
        let mut height_parts = tetration_parts[1].splitn(2, ';');
        let height = height_parts.next().and_then(lenient_f64);
        let payload = height_parts.next().map_or(1.0, payload_or_one);
        if let (Some(base), Some(height)) = (base, height) {
            if base.is_finite() && height.is_finite() {
                let r =
                    Decimal::from_f64(base).tetrate_raw(height, Decimal::from_f64(payload), mode);
                return finish(r, s);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Power: "base^exponent" (only when both parts are finite floats)
    // -----------------------------------------------------------------------
    let pow_parts: Vec<&str> = value.split('^').collect();
    if pow_parts.len() == 2 {
        if let (Some(base), Some(exponent)) = (lenient_f64(pow_parts[0]), lenient_f64(pow_parts[1]))
        {
            if base.is_finite() && exponent.is_finite() {
                let r = Decimal::from_f64(base).pow_raw(Decimal::from_f64(exponent));
                return finish(r, s);
            }
        }
    }

    // -----------------------------------------------------------------------
    // "N PT M" / "N PT (M)" — base-10 tetrate shorthand.
    // -----------------------------------------------------------------------
    let pt_parts: Vec<&str> = value.split("pt").collect();
    if pt_parts.len() == 2 {
        if let Some(r) = tetrate_shorthand(pt_parts[0], pt_parts[1], mode) {
            return finish(r, s);
        }
    }

    // -----------------------------------------------------------------------
    // "NpM" — the same with a bare p.
    // -----------------------------------------------------------------------
    let p_parts: Vec<&str> = value.split('p').collect();
    if p_parts.len() == 2 {
        if let Some(r) = tetrate_shorthand(p_parts[0], p_parts[1], mode) {
            return finish(r, s);
        }
    }

    // -----------------------------------------------------------------------
    // "MfN" / "fN" — payload first, height after the f.
    // -----------------------------------------------------------------------
    let f_parts: Vec<&str> = value.split('f').collect();
    if f_parts.len() == 2 {
        let mut payload_part = f_parts[0].trim();
        let mut negative = false;
        if let Some(rest) = payload_part.strip_prefix('-') {
            negative = true;
            payload_part = rest;
        }
        let payload = payload_or_one(payload_part);
        let height_cleaned: String = f_parts[1]
            .chars()
            .filter(|c| *c != '(' && *c != ')')
            .collect();
        if let Some(height) = lenient_f64(&height_cleaned) {
            if height.is_finite() {
                let mut r = Decimal::ten().tetrate_raw(height, Decimal::from_f64(payload), mode);
                if negative {
                    r = -r;
                }
                return finish(r, s);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Scientific notation with 'e'
    // -----------------------------------------------------------------------
    let e_parts: Vec<&str> = value.split('e').collect();
    let e_count = e_parts.len() - 1;

    if e_count == 0 {
        let n = parse_f64(value, s)?;
        if !n.is_finite() {
            return Err(parse_error(s));
        }
        return Ok(Decimal::from_f64(n));
    }

    if e_count == 1 {
        // Ordinary floats parse directly. Very small values ("2e-3000") round to zero and
        // subnormals ("1e-310") lose precision, so those fall through to the string path.
        if let Ok(n) = value.parse::<f64>() {
            if n.is_finite() && n.abs() > 1e-307 {
                return Ok(Decimal::from_f64(n));
            }
        }
    }

    // -----------------------------------------------------------------------
    // "(e^N)X" and "-(e^N)X": the Display form for layers above MAX_ES_IN_A_ROW.
    // Negative or fractional N is interpreted as 10^^N with payload X.
    // -----------------------------------------------------------------------
    let caret_parts: Vec<&str> = value.split("e^").collect();
    if caret_parts.len() == 2 {
        let negative = caret_parts[0].starts_with('-');
        let tail = caret_parts[1];
        let layer_end = tail
            .char_indices()
            .find(|(_, c)| !matches!(c, '+' | '-' | '.' | '/' | ',' | 'e' | '0'..='9'))
            .map(|(i, c)| (i, c.len_utf8()));
        if let Some((end, width)) = layer_end {
            let layer_f = parse_f64(&tail[..end], s)?;
            let mag = parse_f64(&tail[end + width..], s)?;
            let mut r = if layer_f < 0.0 || layer_f.fract() != 0.0 {
                Decimal::ten().tetrate_raw(layer_f, Decimal::from_f64(mag), mode)
            } else {
                // `as i64` saturates; normalize turns anything past the safe range into inf.
                Decimal::from_components(1, layer_f as i64, mag)
            };
            if negative {
                r = -r;
            }
            return finish(r, s);
        }
    }

    // -----------------------------------------------------------------------
    // "MeX", "eX", "eeX", "MeXeY", ... — mantissa followed by e-separated exponents.
    // -----------------------------------------------------------------------
    let mantissa_str = e_parts[0].trim();
    let mantissa = match mantissa_str {
        "" | "+" => None,
        "-" => Some(-1.0_f64).filter(|_| false).or(None),
        other => Some(parse_f64(other, s)?),
    };
    let explicit_sign: i8 = if mantissa_str == "-" { -1 } else { 1 };

    if mantissa == Some(0.0) {
        return Ok(Decimal::zero());
    }

    let mut exponent = parse_f64(e_parts[e_parts.len() - 1], s)?;

    // Numbers like AeBeC and AeeeeBeC.
    if e_count >= 2 {
        if let Some(me) = lenient_f64(e_parts[e_parts.len() - 2]) {
            if me.is_finite() {
                exponent *= sign(me) as f64;
                exponent += f_maglog10(me);
            }
        }
    }

    let result = match mantissa {
        // "eX", "eeX", ...: N es then the innermost exponent.
        None => Decimal::from_components(explicit_sign, e_count as i64, exponent),
        Some(m) if !m.is_finite() => return Err(parse_error(s)),
        // "MeX": 10^(X + log10(M)).
        Some(m) if e_count == 1 => Decimal::from_components(sign(m), 1, exponent + m.abs().log10()),
        // "MeeX": M * 10^10^X.
        Some(m) if e_count == 2 => {
            Decimal::from_components(1, 2, exponent).mul_raw(Decimal::from_f64(m))
        }
        // At eee and above the mantissa is too small to be recognizable.
        Some(m) => Decimal::from_components(sign(m), e_count as i64, exponent),
    };

    finish(result, s)
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use super::*;

    fn p(s: &str) -> Decimal {
        Decimal::try_from(s).unwrap_or_else(|e| panic!("failed to parse {s:?}: {e}"))
    }

    #[test]
    fn parse_simple_number() {
        assert_eq!(p("42").to_number(), 42.0);
        assert_eq!(p("  12  ").to_number(), 12.0);
        assert_eq!(p("+5").to_number(), 5.0);
        assert_eq!(p(".5").to_number(), 0.5);
        assert_eq!(p("5.").to_number(), 5.0);
        assert_eq!(p("1,000,000").to_number(), 1e6);
    }

    #[test]
    fn parse_scientific() {
        assert!((p("1e5").to_number() - 1e5).abs() < 1.0);
        assert_eq!(p("1E5"), p("1e5"));
        assert_eq!(p("-1e5").to_number(), -1e5);
        assert_eq!(p("1e1000").to_string(), "1e1000");
        assert_eq!(p("1e-400").to_string(), "1e-400");
        assert_eq!(p("0e5"), Decimal::zero());
        // Subnormal literals keep their precision.
        assert!((p("2.47e-324").mantissa() - 2.47).abs() < 1e-9);
    }

    #[test]
    fn parse_stacked_exponents() {
        assert_eq!(p("e3").to_number(), 1000.0);
        assert_eq!(p("-e3").to_number(), -1000.0);
        assert_eq!(p("e-3").to_number(), 0.001);
        assert_eq!(p("ee3").to_string(), "1e1000");
        assert_eq!(p("eee3").to_string(), "ee1000");
        assert_eq!(p("1e1e3").to_string(), "1e1000");
        assert_eq!(p("e1e3").to_string(), "1e1000");
        assert_eq!(
            p("2ee3"),
            Decimal::from_components(1, 2, 3.0) * Decimal::two()
        );
        assert!(p("eeeee12345000000").layer() >= 5);
    }

    #[test]
    fn parse_operators() {
        assert_eq!(p("10^3").to_number(), 1000.0);
        assert_eq!(p("2^10").to_number(), 1024.0);
        assert_eq!(p("2^^3").to_number(), 16.0);
        assert_eq!(p("2^^2;3").to_number(), 256.0);
        assert_eq!(p("2^^^2").to_number(), 4.0);
        assert_eq!(p("2^^^2;2").to_number(), 65536.0);
        assert_eq!(p("10^^3").to_string(), "1e10000000000");
        assert!(Decimal::try_from("(-2)^^2.5").is_err());
    }

    #[test]
    fn parse_tetrate_shorthands() {
        assert_eq!(p("3pt2"), p("ee100"));
        assert_eq!(p("3PT2"), p("ee100"));
        assert_eq!(p("3 PT 2"), p("ee100"));
        assert_eq!(p("3 PT (2)"), p("ee100"));
        assert_eq!(p("3pt(2)"), p("ee100"));
        assert_eq!(p("-3pt2"), -p("ee100"));
        assert_eq!(p("2p3"), p("1e1000"));
        assert_eq!(p("1PT3").to_number(), 1000.0);
        assert_eq!(p("2f3"), p("ee100"));
        assert_eq!(p("f2").to_number(), 1e10);
    }

    #[test]
    fn parse_parenthesized_large_layer() {
        let result = Decimal::try_from("(e^100)15000000000");
        assert!(result.is_ok(), "failed to parse: {result:?}");
        assert_eq!(p("(e^3)2"), p("ee100"));
        assert_eq!(p("-(e^7)15.5").sign(), -1);
        // Negative or fractional layers are tetration heights.
        assert_eq!(
            p("(e^-1)5"),
            Decimal::ten().tetrate(Some(-1.0), Some(Decimal::from(5)), TetrationMode::Analytic)
        );
        assert_eq!(
            p("(e^1.5)5"),
            Decimal::ten().tetrate(Some(1.5), Some(Decimal::from(5)), TetrationMode::Analytic)
        );
        assert_eq!(p("(e^1e300)5"), Decimal::inf());
    }

    #[test]
    fn parse_infinity_strings() {
        assert_eq!(p("Infinity"), Decimal::inf());
        assert_eq!(p("-Infinity"), Decimal::neg_inf());
        assert_eq!(p("inf"), Decimal::inf());
        assert_eq!(p("-inf"), Decimal::neg_inf());
    }

    #[test]
    fn parse_nan_returns_error() {
        assert!(Decimal::try_from("NaN").is_err());
        assert!(Decimal::try_from("nan").is_err());
    }

    #[test]
    fn from_string_garbage_errors() {
        for s in [
            "", "abc", "1e", "e", "1_000", "1e5e", "--5", "(e^5", "1..2", "e^", "pt", "f",
        ] {
            assert!(Decimal::try_from(s).is_err(), "{s:?} should fail");
        }
    }

    #[test]
    fn parse_f64_helper_errors_gracefully() {
        let result = parse_f64("not_a_number", "original");
        assert!(result.is_err());
        if let Err(BreakEternityError::ParseError { parsed, .. }) = result {
            assert_eq!(parsed, "original");
        } else {
            panic!("expected ParseError");
        }
    }

    #[test]
    fn from_string_with_mode_threads_mode() {
        let a = Decimal::from_string_with_mode("10^^2.5", TetrationMode::Analytic).unwrap();
        let l = Decimal::from_string_with_mode("10^^2.5", TetrationMode::Linear).unwrap();
        assert_ne!(a, l);
        assert_eq!(
            a,
            Decimal::ten().tetrate(Some(2.5), None, TetrationMode::Analytic)
        );
        assert_eq!(
            l,
            Decimal::ten().tetrate(Some(2.5), None, TetrationMode::Linear)
        );
    }

    #[test]
    fn non_ascii_does_not_panic() {
        for s in ["(e^é)5", "５", "1e５", "ｅ5", "—5", "e^ü)3"] {
            let _ = Decimal::try_from(s);
        }
    }
}
