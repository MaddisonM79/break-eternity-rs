//! `TryFrom<&str>` implementation for [`Decimal`].

use std::convert::TryFrom;

use crate::constants::commas_are_decimal_points;
use crate::constants::ignore_commas;
use crate::decimal::Decimal;
use crate::error::BreakEternityError;
use crate::utils::f_maglog10;

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

        let pentation_parts: Vec<&str> = value.split("^^^").collect();
        if pentation_parts.len() == 2 {
            let base = pentation_parts[0].parse::<f64>();
            if let Err(parse_error) = base {
                return Err(BreakEternityError::ParseError {
                    parsed: s.to_string(),
                    error: parse_error,
                });
            }
            let base = base.unwrap();
            let height = pentation_parts[1].parse::<f64>();
            if let Err(parse_error) = height {
                return Err(BreakEternityError::ParseError {
                    parsed: s.to_string(),
                    error: parse_error,
                });
            }
            let height = height.unwrap();
            let mut payload = 1.0;
            let height_parts = pentation_parts[1].split(';').collect::<Vec<&str>>();
            if height_parts.len() == 2 {
                let payload_parsed = height_parts[1].parse::<f64>();
                if let Err(parse_error) = payload_parsed {
                    return Err(BreakEternityError::ParseError {
                        parsed: s.to_string(),
                        error: parse_error,
                    });
                }
                payload = payload_parsed.unwrap();
                if !payload.is_finite() {
                    payload = 1.0;
                }
            }

            if base.is_finite() && height.is_finite() {
                return Ok(Decimal::from_number(base)
                    .pentate(Some(height), Some(Decimal::from_number(payload))));
            }
        }

        let tetration_parts: Vec<&str> = value.split("^^").collect();
        if tetration_parts.len() == 2 {
            let base = tetration_parts[0].parse::<f64>();
            if let Err(parse_error) = base {
                return Err(BreakEternityError::ParseError {
                    parsed: s.to_string(),
                    error: parse_error,
                });
            }
            let base = base.unwrap();
            let height = tetration_parts[1].parse::<f64>();
            if let Err(parse_error) = height {
                return Err(BreakEternityError::ParseError {
                    parsed: s.to_string(),
                    error: parse_error,
                });
            }
            let height = height.unwrap();
            let mut payload = 1.0;
            let height_parts = tetration_parts[1].split(';').collect::<Vec<&str>>();
            if height_parts.len() == 2 {
                let payload_parsed = height_parts[1].parse::<f64>();
                if let Err(parse_error) = payload_parsed {
                    return Err(BreakEternityError::ParseError {
                        parsed: s.to_string(),
                        error: parse_error,
                    });
                }
                payload = payload_parsed.unwrap();
                if !payload.is_finite() {
                    payload = 1.0;
                }
            }

            if base.is_finite() && height.is_finite() {
                return Ok(Decimal::from_number(base)
                    .tetrate(Some(height), Some(Decimal::from_number(payload))));
            }
        }

        let pow_parts = value.split('^').collect::<Vec<&str>>();
        if pow_parts.len() == 2 {
            let base = pow_parts[0].parse::<f64>();
            if let Err(parse_error) = base {
                return Err(BreakEternityError::ParseError {
                    parsed: s.to_string(),
                    error: parse_error,
                });
            }
            let base = base.unwrap();
            let exponent = pow_parts[1].parse::<f64>();
            if let Err(parse_error) = exponent {
                return Err(BreakEternityError::ParseError {
                    parsed: s.to_string(),
                    error: parse_error,
                });
            }
            let exponent = exponent.unwrap();
            if base.is_finite() && exponent.is_finite() {
                return Ok(Decimal::from_number(base).pow(Decimal::from_number(exponent)));
            }
        }

        let value = value.trim().to_lowercase();
        let value = value.as_str();

        let mut pt_parts = value.split("pt").collect::<Vec<&str>>();
        if pt_parts.len() == 2 {
            let base: f64 = 10.0;
            let height = pt_parts[0].parse::<f64>();
            if let Err(parse_error) = height {
                return Err(BreakEternityError::ParseError {
                    parsed: s.to_string(),
                    error: parse_error,
                });
            }
            let height = height.unwrap();
            let tmp = pt_parts[1].replace(['(', ')'], "");
            pt_parts[1] = tmp.as_str();

            let payload = pt_parts[1].parse::<f64>();
            if let Err(parse_error) = payload {
                return Err(BreakEternityError::ParseError {
                    parsed: s.to_string(),
                    error: parse_error,
                });
            }
            let mut payload = payload.unwrap();
            if !payload.is_finite() {
                payload = 1.0;
            }
            if base.is_finite() && height.is_finite() {
                // tetrate again
                return Ok(Decimal::from_number(base)
                    .tetrate(Some(height), Some(Decimal::from_number(payload))));
            }
        }

        let mut p_parts = value.split('p').collect::<Vec<&str>>();
        if p_parts.len() == 2 {
            let base: f64 = 10.0;
            let height = p_parts[0].parse::<f64>();
            if let Err(parse_error) = height {
                return Err(BreakEternityError::ParseError {
                    parsed: s.to_string(),
                    error: parse_error,
                });
            }
            let height = height.unwrap();
            let tmp = p_parts[1].replace(['(', ')'], "");
            p_parts[1] = tmp.as_str();

            let payload = p_parts[1].parse::<f64>();
            if let Err(parse_error) = payload {
                return Err(BreakEternityError::ParseError {
                    parsed: s.to_string(),
                    error: parse_error,
                });
            }
            let mut payload = payload.unwrap();
            if !payload.is_finite() {
                payload = 1.0;
            }
            if base.is_finite() && height.is_finite() {
                // another tetrate
                return Ok(Decimal::from_number(base)
                    .tetrate(Some(height), Some(Decimal::from_number(payload))));
            }
        }

        let e_parts = value.split('e').collect::<Vec<&str>>();
        let e_count = e_parts.len() - 1;

        if e_count == 0 {
            let number_attempt = value.parse::<f64>();
            if let Err(parse_error) = number_attempt {
                return Err(BreakEternityError::ParseError {
                    parsed: s.to_string(),
                    error: parse_error,
                });
            }
            let number_attempt = number_attempt.unwrap();
            if number_attempt.is_finite() {
                return Ok(Decimal::default().set_from_number(number_attempt));
            }
        } else if e_count == 1 {
            let number_attempt = value.parse::<f64>();
            if let Err(parse_error) = number_attempt {
                return Err(BreakEternityError::ParseError {
                    parsed: s.to_string(),
                    error: parse_error,
                });
            }
            let number_attempt = number_attempt.unwrap();
            if number_attempt.is_finite() && number_attempt != 0.0 {
                return Ok(Decimal::default().set_from_number(number_attempt));
            }
        }

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
                    let layer = layer_string.parse::<f64>();
                    if let Err(parse_error) = layer {
                        return Err(BreakEternityError::ParseError {
                            parsed: s.to_string(),
                            error: parse_error,
                        });
                    }
                    dec.layer = layer.unwrap() as i64;
                    let mag = new_parts[1][i + 1..].parse::<f64>();
                    if let Err(parse_error) = mag {
                        return Err(BreakEternityError::ParseError {
                            parsed: s.to_string(),
                            error: parse_error,
                        });
                    }
                    let mag = mag.unwrap();
                    dec.mag = mag;
                    dec.normalize();
                    return Ok(dec);
                }
            }
        }

        let mut dec = Decimal::default();

        if e_count < 1 {
            return Ok(dec);
        }
        let mantissa = e_parts[0].parse::<f64>();
        if let Err(parse_error) = mantissa {
            return Err(BreakEternityError::ParseError {
                parsed: s.to_string(),
                error: parse_error,
            });
        }
        let mantissa = mantissa.unwrap();
        if mantissa == 0.0 {
            return Ok(dec);
        }
        let exponent = e_parts.last().unwrap().parse::<f64>();
        if let Err(parse_error) = exponent {
            return Err(BreakEternityError::ParseError {
                parsed: s.to_string(),
                error: parse_error,
            });
        }
        let mut exponent = exponent.unwrap();
        if e_count >= 2 {
            let me = e_parts[e_parts.len() - 2].parse::<f64>();
            if let Err(parse_error) = me {
                return Err(BreakEternityError::ParseError {
                    parsed: s.to_string(),
                    error: parse_error,
                });
            }
            let me = me.unwrap();
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
                    Decimal::from_components(1, 2, exponent) * Decimal::from_number(mantissa)
                );
            }
            // mantissa is way too small
            dec.mag = exponent;
        }

        dec.normalize();
        Ok(dec)
    }
}
