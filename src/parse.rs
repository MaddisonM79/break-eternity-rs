//! String parsing for [`Decimal`].
//!
//! Provides three equivalent entry points: [`Decimal::from_string`], the
//! [`TryFrom<&str>`] impl, and the [`std::str::FromStr`] impl. Together these
//! cover ports of `fromStringInternal` from `break_eternity.js`.

use std::convert::TryFrom;
use std::str::FromStr;

use crate::constants::commas_are_decimal_points;
use crate::constants::ignore_commas;
use crate::decimal::Decimal;
use crate::error::BreakEternityError;
use crate::tetration::TetrationMode;
use crate::utils::f_maglog10;

impl Decimal {
    /// Parses a string into a [`Decimal`].
    ///
    /// Accepts the formats produced by [`Decimal`]'s [`Display`](std::fmt::Display) impl as
    /// well as the additional notations recognized by `break_eternity.js`'s `fromStringInternal`:
    ///
    /// * Plain decimals: `"0"`, `"-5"`, `"3.14"`, `"1000000"`
    /// * Scientific: `"1.23e45"`, `"1.23e+45"`, `"1.23e-45"`, `"1e1000"` (past `f64` range)
    /// * Stacked exponents: `"eN"`, `"eeN"`, `"eeeN"`, …
    /// * Power / tetrate / pentate operators: `"10^N"`, `"10^^N"`, `"10^^^N"`
    /// * Parenthesized large layer: `"(e^N)M"` (the [`Display`](std::fmt::Display) form for very
    ///   high layers)
    /// * `pt`/`p` tetrate shorthands: `"NptM"`, `"NpM"`
    /// * Specials: `"Infinity"`, `"-Infinity"`. `"NaN"` returns an error since NaN is not a
    ///   representable [`Decimal`].
    ///
    /// Surrounding whitespace is trimmed. Malformed input returns
    /// [`BreakEternityError::ParseError`].
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
        Decimal::try_from(s)
    }
}

impl FromStr for Decimal {
    type Err = BreakEternityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Decimal::try_from(s)
    }
}

/// Parses a `&str` slice as an `f64`, mapping parse errors into [`BreakEternityError::ParseError`].
fn parse_f64(s: &str, orig: &str) -> Result<f64, BreakEternityError> {
    s.parse::<f64>()
        .map_err(|error| BreakEternityError::ParseError {
            parsed: orig.to_string(),
            error,
        })
}

impl TryFrom<&str> for Decimal {
    type Error = BreakEternityError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let mut value = s.to_string();
        if ignore_commas() {
            value = value.replace(',', "");
        } else if commas_are_decimal_points() {
            value = value.replace(',', ".");
        }
        let value = value.as_str();

        // -----------------------------------------------------------------------
        // Pentate: "base^^^height" or "base^^^height;payload"
        // -----------------------------------------------------------------------
        let pentation_parts: Vec<&str> = value.split("^^^").collect();
        if pentation_parts.len() == 2 {
            let base = parse_f64(pentation_parts[0], s)?;
            let height = parse_f64(pentation_parts[1], s)?;
            let mut payload = 1.0;
            let height_parts = pentation_parts[1].split(';').collect::<Vec<&str>>();
            if height_parts.len() == 2 {
                let p = parse_f64(height_parts[1], s)?;
                if p.is_finite() {
                    payload = p;
                }
            }
            if base.is_finite() && height.is_finite() {
                return Ok(Decimal::from_finite(base).pentate(
                    Some(height),
                    Some(Decimal::from_finite(payload)),
                    TetrationMode::Analytic,
                ));
            }
        }

        // -----------------------------------------------------------------------
        // Tetrate: "base^^height" or "base^^height;payload"
        // -----------------------------------------------------------------------
        let tetration_parts: Vec<&str> = value.split("^^").collect();
        if tetration_parts.len() == 2 {
            let base = parse_f64(tetration_parts[0], s)?;
            let height = parse_f64(tetration_parts[1], s)?;
            let mut payload = 1.0;
            let height_parts = tetration_parts[1].split(';').collect::<Vec<&str>>();
            if height_parts.len() == 2 {
                let p = parse_f64(height_parts[1], s)?;
                if p.is_finite() {
                    payload = p;
                }
            }
            if base.is_finite() && height.is_finite() {
                return Ok(Decimal::from_finite(base).tetrate(
                    Some(height),
                    Some(Decimal::from_finite(payload)),
                    TetrationMode::Analytic,
                ));
            }
        }

        // -----------------------------------------------------------------------
        // Power: "base^exponent"
        // Only applies when both parts parse as finite floats; otherwise fall through.
        // -----------------------------------------------------------------------
        let pow_parts = value.split('^').collect::<Vec<&str>>();
        if pow_parts.len() == 2 {
            if let (Ok(base), Ok(exponent)) =
                (pow_parts[0].parse::<f64>(), pow_parts[1].parse::<f64>())
            {
                if base.is_finite() && exponent.is_finite() {
                    return Ok(Decimal::from_finite(base).pow(Decimal::from_finite(exponent)));
                }
            }
        }

        let value = value.trim().to_lowercase();
        let value = value.as_str();

        // -----------------------------------------------------------------------
        // "NpT(payload)" or "NptM" — tetrate shorthand.
        // Only applies when the height part parses as a finite float.
        // -----------------------------------------------------------------------
        let pt_parts = value.split("pt").collect::<Vec<&str>>();
        if pt_parts.len() == 2 {
            if let Ok(height) = pt_parts[0].parse::<f64>() {
                let base: f64 = 10.0;
                let tmp = pt_parts[1].replace(['(', ')'], "");
                let mut payload = tmp.parse::<f64>().unwrap_or(1.0);
                if !payload.is_finite() {
                    payload = 1.0;
                }
                if height.is_finite() {
                    return Ok(Decimal::from_finite(base).tetrate(
                        Some(height),
                        Some(Decimal::from_finite(payload)),
                        TetrationMode::Analytic,
                    ));
                }
            }
        }

        // -----------------------------------------------------------------------
        // "NpM" — another tetrate shorthand.
        // Only applies when the height part parses as a finite float.
        // -----------------------------------------------------------------------
        let p_parts = value.split('p').collect::<Vec<&str>>();
        if p_parts.len() == 2 {
            if let Ok(height) = p_parts[0].parse::<f64>() {
                let base: f64 = 10.0;
                let tmp = p_parts[1].replace(['(', ')'], "");
                let mut payload = tmp.parse::<f64>().unwrap_or(1.0);
                if !payload.is_finite() {
                    payload = 1.0;
                }
                if height.is_finite() {
                    return Ok(Decimal::from_finite(base).tetrate(
                        Some(height),
                        Some(Decimal::from_finite(payload)),
                        TetrationMode::Analytic,
                    ));
                }
            }
        }

        // -----------------------------------------------------------------------
        // Scientific notation with 'e'
        // -----------------------------------------------------------------------
        let e_parts = value.split('e').collect::<Vec<&str>>();
        let e_count = e_parts.len() - 1;

        if e_count == 0 {
            let n = parse_f64(value, s)?;
            if n.is_finite() {
                return Ok(Decimal::default().set_from_number(n));
            }
            // Non-finite bare literals: accept "Infinity"/"-Infinity", reject "NaN".
            let lower = value.trim();
            if lower == "nan" {
                // NaN is not a representable Decimal — return a parse error.
                return Err(BreakEternityError::ParseError {
                    parsed: s.to_string(),
                    // Fabricate a ParseFloatError by parsing something truly invalid.
                    error: "!nan".parse::<f64>().unwrap_err(),
                });
            }
            if lower == "infinity" {
                return Ok(Decimal::inf());
            }
            if lower == "-infinity" {
                return Ok(Decimal::neg_inf());
            }
            // Any other non-finite result is also an error.
            return Err(BreakEternityError::ParseError {
                parsed: s.to_string(),
                error: format!("!{lower}").parse::<f64>().unwrap_err(),
            });
        } else if e_count == 1 {
            // Try to parse the whole string as a scientific-notation f64 (e.g. "1.5e10").
            // If it doesn't parse, fall through to the more specialised handlers below.
            if let Ok(n) = value.parse::<f64>() {
                if n.is_finite() && n != 0.0 {
                    return Ok(Decimal::default().set_from_number(n));
                }
            }
        }

        // -----------------------------------------------------------------------
        // Multiple leading 'e' prefix: "eeeeeM" (layer = number of es, mag = M).
        // This is the format produced by Display for layers 2..=MAX_ES_IN_A_ROW.
        // -----------------------------------------------------------------------
        if value.starts_with("ee") || value.strip_prefix('-').is_some_and(|v| v.starts_with("ee")) {
            let (sign_char, rest) = if let Some(stripped) = value.strip_prefix('-') {
                (-1i8, stripped)
            } else {
                (1i8, value)
            };
            // Count leading 'e's.
            let layer = rest.bytes().take_while(|&b| b == b'e').count() as i64;
            let mag_str = &rest[layer as usize..];
            if let Ok(mag) = mag_str.parse::<f64>() {
                let mut dec = Decimal::from_components_unchecked(sign_char, layer, mag);
                dec.normalize();
                return Ok(dec);
            }
        }

        // -----------------------------------------------------------------------
        // Parenthesized large-layer: "(e^N)M" or "-(e^N)M".
        // This is the format produced by Display for layers > MAX_ES_IN_A_ROW.
        // -----------------------------------------------------------------------
        {
            let (sign_char, rest) = if value.starts_with("-(e^") {
                (-1i8, &value[1..])
            } else {
                (1i8, value)
            };
            if rest.starts_with("(e^") {
                if let Some(close) = rest.find(')') {
                    let layer_str = &rest[3..close]; // between "(e^" and ")"
                    let mag_str = &rest[close + 1..];
                    if let (Ok(layer_f), Ok(mag)) =
                        (layer_str.parse::<f64>(), mag_str.parse::<f64>())
                    {
                        let mut dec =
                            Decimal::from_components_unchecked(sign_char, layer_f as i64, mag);
                        dec.normalize();
                        return Ok(dec);
                    }
                }
            }
        }

        // -----------------------------------------------------------------------
        // "e^N" notation
        // -----------------------------------------------------------------------
        let new_parts = value.split("e^").collect::<Vec<&str>>();
        if new_parts.len() == 2 {
            let mut dec = Decimal {
                sign: 1,
                ..Default::default()
            };
            if new_parts[0].starts_with('-') {
                dec.sign = -1;
            }

            let mut layer_string = String::new();
            for (i, c) in new_parts[1].chars().enumerate() {
                if c.is_numeric()
                    || c == '+'
                    || c == '-'
                    || c == '.'
                    || c == 'e'
                    || c == ','
                    || c == '/'
                {
                    layer_string.push(c);
                } else {
                    let layer = parse_f64(&layer_string, s)?;
                    dec.layer = layer as i64;
                    let mag = parse_f64(&new_parts[1][i + 1..], s)?;
                    dec.mag = mag;
                    dec.normalize();
                    return Ok(dec);
                }
            }
        }

        // -----------------------------------------------------------------------
        // Multi-e: "MeNeP..." — mantissa followed by multiple e-separated exponents
        // -----------------------------------------------------------------------
        let mut dec = Decimal::default();

        if e_count < 1 {
            return Ok(dec);
        }

        let mantissa = parse_f64(e_parts[0], s)?;
        if mantissa == 0.0 {
            return Ok(dec);
        }

        let exponent_str = e_parts.last().unwrap();
        let mut exponent = parse_f64(exponent_str, s)?;

        if e_count >= 2 {
            let me = parse_f64(e_parts[e_parts.len() - 2], s)?;
            if me.is_finite() {
                exponent *= crate::utils::sign(me) as f64;
                exponent += f_maglog10(me);
            }
        }

        if !mantissa.is_finite() {
            dec.sign = if e_parts[0] == "-" { -1 } else { 1 };
            dec.layer = e_count as i64;
            dec.mag = exponent;
        } else if e_count == 1 {
            dec.sign = crate::utils::sign(mantissa);
            dec.layer = 1;
            dec.mag = exponent + mantissa.abs().log10();
        } else {
            dec.sign = crate::utils::sign(mantissa);
            dec.layer = e_count as i64;
            if e_count == 2 {
                return Ok(
                    Decimal::from_components(1, 2, exponent) * Decimal::from_finite(mantissa)
                );
            }
            // mantissa is way too small at this level
            dec.mag = exponent;
        }

        dec.normalize();
        Ok(dec)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use super::*;

    #[test]
    fn parse_simple_number() {
        let d = Decimal::try_from("42").unwrap();
        assert_eq!(d.to_number(), 42.0);
    }

    #[test]
    fn parse_scientific() {
        let d = Decimal::try_from("1e5").unwrap();
        assert!((d.to_number() - 1e5).abs() < 1.0);
    }

    #[test]
    fn parse_infinity_strings() {
        let d = Decimal::try_from("Infinity").unwrap();
        assert_eq!(d, Decimal::inf());
        let d2 = Decimal::try_from("-Infinity").unwrap();
        assert_eq!(d2, Decimal::neg_inf());
    }

    #[test]
    fn parse_nan_returns_error() {
        assert!(Decimal::try_from("NaN").is_err());
    }

    #[test]
    fn parse_parenthesized_large_layer() {
        // (e^100)15000000000 is the Display format for layer=100, mag=1.5e10
        let result = Decimal::try_from("(e^100)15000000000");
        assert!(result.is_ok(), "failed to parse: {result:?}");
    }

    #[test]
    fn parse_multi_e_prefix() {
        // eeeee<mag> is the Display format for layer=5
        let result = Decimal::try_from("eeeee12345000000");
        assert!(result.is_ok(), "failed to parse: {result:?}");
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

    // -----------------------------------------------------------------------
    // from_string / FromStr coverage
    // -----------------------------------------------------------------------

    /// Approximate equality used by round-trip tests. Display is lossy at
    /// layers where mantissa/exponent are recomputed (layer 0 large, layer 1).
    fn approx_eq(a: Decimal, b: Decimal) -> bool {
        if a == b {
            return true;
        }
        if a.sign() != b.sign() || a.layer() != b.layer() {
            return false;
        }
        let am = a.mag();
        let bm = b.mag();
        if am == bm {
            return true;
        }
        let scale = am.abs().max(bm.abs()).max(1.0);
        (am - bm).abs() / scale < 1e-12
    }

    #[test]
    fn from_string_simple() {
        let d = Decimal::from_string("12345").unwrap();
        assert_eq!(d.exponent(), 4.0);
        assert!((d.mantissa() - 1.2345).abs() < 1e-12);
    }

    #[test]
    fn from_string_negative() {
        let d = Decimal::from_string("-5").unwrap();
        assert_eq!(d.to_number(), -5.0);
    }

    #[test]
    fn from_string_scientific() {
        let d = Decimal::from_string("1.23e45").unwrap();
        assert_eq!(d.exponent(), 45.0);
        assert!((d.mantissa() - 1.23).abs() < 1e-12);
    }

    #[test]
    fn from_string_scientific_signs() {
        let positive = Decimal::from_string("1.23e+45").unwrap();
        let negative_exp = Decimal::from_string("1.23e-45").unwrap();
        assert_eq!(positive.exponent(), 45.0);
        assert_eq!(negative_exp.exponent(), -45.0);
    }

    #[test]
    fn from_string_beyond_f64_range() {
        // 1e1000 overflows f64 but fits comfortably as a layer-1 Decimal.
        let d = Decimal::from_string("1e1000").unwrap();
        assert_eq!(d.exponent(), 1000.0);
        assert_eq!(d.layer(), 1);
        assert!(d.sign() > 0);
    }

    #[test]
    fn from_string_whitespace_trimmed() {
        let d = Decimal::from_string("  42  ").unwrap();
        assert_eq!(d.to_number(), 42.0);
    }

    #[test]
    fn from_string_garbage_errors() {
        assert!(Decimal::from_string("garbage").is_err());
    }

    #[test]
    fn fromstr_trait_works() {
        let d: Decimal = "1.23e45".parse().unwrap();
        assert_eq!(d.exponent(), 45.0);

        // Specials via FromStr
        assert_eq!("Infinity".parse::<Decimal>().unwrap(), Decimal::inf());
        assert_eq!("-Infinity".parse::<Decimal>().unwrap(), Decimal::neg_inf());
        assert!("NaN".parse::<Decimal>().is_err());
        assert!("not a number".parse::<Decimal>().is_err());
    }

    #[test]
    fn from_string_pow_notation() {
        // "10^N" parses through the power branch.
        let d = Decimal::from_string("10^50").unwrap();
        assert_eq!(d.exponent(), 50.0);
    }

    #[test]
    fn from_string_tetration_notation() {
        // "10^^N" parses through the tetration branch.
        let result = Decimal::from_string("10^^5");
        assert!(result.is_ok(), "10^^5 should parse: {result:?}");
    }

    #[test]
    fn round_trip_plain() {
        let d = Decimal::from_finite(-2.5);
        let s = d.to_string();
        let d2 = Decimal::from_string(&s).unwrap();
        assert!(approx_eq(d, d2), "round-trip failed: {d:?} -> {s:?} -> {d2:?}");
    }

    #[test]
    fn round_trip_large_exponent() {
        let d = Decimal::from_mantissa_exponent(1.234, 400.0);
        let s = d.to_string();
        let d2 = Decimal::from_string(&s).unwrap();
        assert!(approx_eq(d, d2), "round-trip failed: {d:?} -> {s:?} -> {d2:?}");
    }

    #[test]
    fn round_trip_layer_2() {
        // mag past EXPONENT_LIMIT pushes into layer 2.
        let d = Decimal::from_mantissa_exponent(1.5, 1e16);
        assert!(d.layer() >= 2, "expected layer >= 2, got {}", d.layer());
        let s = d.to_string();
        let d2 = Decimal::from_string(&s).unwrap();
        assert!(approx_eq(d, d2), "round-trip failed: {d:?} -> {s:?} -> {d2:?}");
    }

    #[test]
    fn round_trip_eeeee_layer() {
        // Force layer 5 directly so Display emits "eeeee<mag>".
        let d = Decimal::from_components(1, 5, 1.234e7);
        assert_eq!(d.layer(), 5);
        let s = d.to_string();
        assert!(s.starts_with("eeeee"), "unexpected Display: {s}");
        let d2 = Decimal::from_string(&s).unwrap();
        assert!(approx_eq(d, d2), "round-trip failed: {d:?} -> {s:?} -> {d2:?}");
    }

    #[test]
    fn round_trip_parenthesized_layer() {
        // Layer above MAX_ES_IN_A_ROW serializes as "(e^N)mag".
        let d = Decimal::from_components(1, 100, 1.5e10);
        assert_eq!(d.layer(), 100);
        let s = d.to_string();
        assert!(s.starts_with("(e^100)"), "unexpected Display: {s}");
        let d2 = Decimal::from_string(&s).unwrap();
        assert!(approx_eq(d, d2), "round-trip failed: {d:?} -> {s:?} -> {d2:?}");
    }

    #[test]
    fn round_trip_specials() {
        assert_eq!(
            Decimal::from_string(&Decimal::inf().to_string()).unwrap(),
            Decimal::inf()
        );
        assert_eq!(
            Decimal::from_string(&Decimal::neg_inf().to_string()).unwrap(),
            Decimal::neg_inf()
        );
        assert_eq!(
            Decimal::from_string(&Decimal::zero().to_string()).unwrap(),
            Decimal::zero()
        );
    }
}
