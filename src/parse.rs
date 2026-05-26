//! `TryFrom<&str>` implementation for [`Decimal`].

use std::convert::TryFrom;

use crate::constants::commas_are_decimal_points;
use crate::constants::ignore_commas;
use crate::decimal::Decimal;
use crate::error::BreakEternityError;
use crate::tetration::TetrationMode;
use crate::utils::f_maglog10;

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
                return Ok(Decimal::from_finite(base)
                    .pentate(Some(height), Some(Decimal::from_finite(payload)), TetrationMode::Analytic));
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
                return Ok(Decimal::from_finite(base)
                    .tetrate(Some(height), Some(Decimal::from_finite(payload)), TetrationMode::Analytic));
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
                    return Ok(Decimal::from_finite(base)
                        .tetrate(Some(height), Some(Decimal::from_finite(payload)), TetrationMode::Analytic));
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
                    return Ok(Decimal::from_finite(base)
                        .tetrate(Some(height), Some(Decimal::from_finite(payload)), TetrationMode::Analytic));
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
}
