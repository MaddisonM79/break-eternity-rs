//! Transcendental functions: exp, ln, log, log10, pow, gamma, lambertw and their helpers.

use crate::constants::{COMPARE_EPSILON, OMEGA};
use crate::decimal::Decimal;
use crate::error::{ArithmeticError, ArithmeticErrorKind, BreakEternityError};
use crate::utils::sign;

// ---------------------------------------------------------------------------
// Free-function helpers (module-private)
// ---------------------------------------------------------------------------

/// Stirling-series approximation to the gamma function for real arguments.
pub(crate) fn f_gamma(mut num: f64) -> f64 {
    if !num.is_finite() {
        return num;
    }

    if num < -50.0 {
        if (num - num.trunc()).abs() < COMPARE_EPSILON {
            return f64::NEG_INFINITY;
        }
        return 0.0;
    }

    let mut scal1 = 1.0;
    while num < 10.0 {
        scal1 *= num;
        num += 1.0;
    }

    num += 1.0;
    let mut l = 0.9189385332046727;
    l += (num + 0.5) * num.ln();
    l -= num;
    let num2 = num * num;
    let mut num_p = num;
    l += 1.0 / (12.0 * num_p);
    num_p *= num2;
    l += 1.0 / (360.0 * num_p);
    num_p *= num2;
    l += 1.0 / (1260.0 * num_p);
    num_p *= num2;
    l += 1.0 / (1680.0 * num_p);
    num_p *= num2;
    l += 1.0 / (1188.0 * num_p);
    num_p *= num2;
    l += 691.0 / (360360.0 * num_p);
    num_p *= num2;
    l += 7.0 / (1092.0 * num_p);
    num_p *= num2;
    l += 3617.0 / (122400.0 * num_p);

    l.exp() / scal1
}

/// Scalar Lambert W function (principal branch, `W_0`).
pub(crate) fn f_lambertw(z: f64, tol: Option<f64>) -> Result<f64, BreakEternityError> {
    let tol = tol.unwrap_or(COMPARE_EPSILON);

    let mut w;
    let mut wn;

    if !z.is_finite() {
        return Ok(z);
    }

    if z == 0.0 {
        return Ok(z);
    }

    if (z - 1.0).abs() < COMPARE_EPSILON {
        return Ok(OMEGA);
    }

    if z < 10.0 {
        w = 0.0;
    } else {
        w = z.ln() - z.ln().ln();
    }

    for _ in 0..100 {
        wn = (z * (-w).exp() + w * w) / (w + 1.0);
        if (wn - w).abs() < tol * wn.abs() {
            return Ok(wn);
        }
        w = wn;
    }

    Err(BreakEternityError::IterationFailedConverging { z })
}

/// Decimal-valued Lambert W function (principal branch).
pub(crate) fn d_lambertw(z: Decimal, tol: Option<f64>) -> Result<Decimal, BreakEternityError> {
    let tol = tol.unwrap_or(COMPARE_EPSILON);

    let mut w;
    let mut ew;
    let mut wewz;
    let mut wn;

    if !z.mag.is_finite() {
        return Ok(z);
    }

    if z == Decimal::zero() {
        return Ok(z);
    }

    if z == Decimal::one() {
        return Ok(Decimal::from_finite(OMEGA));
    }

    w = z.ln();

    // Halley's method
    for _ in 0..100 {
        ew = (-w).exp();
        wewz = w - z * ew;
        wn = w - wewz
            / (w + Decimal::from_finite(1.0)
                - (w + Decimal::from_finite(2.0)) * wewz
                    / (Decimal::from_finite(2.0) * w + Decimal::from_finite(2.0)));

        if (wn - w).abs() < Decimal::from_finite(tol) * wn.abs() {
            return Ok(wn);
        }
        w = wn;
    }

    Err(BreakEternityError::IterationFailedConverging { z: z.to_number() })
}

// ---------------------------------------------------------------------------
// impl Decimal — transcendental functions
// ---------------------------------------------------------------------------

impl Decimal {
    /// Returns the absolute log10 of the Decimal.
    pub fn abs_log10(&self) -> Decimal {
        if self.sign == 0 {
            return Decimal::nan_sentinel();
        }

        if self.layer > 0 {
            return Decimal::from_components(sign(self.mag), self.layer - 1, self.mag.abs());
        }

        Decimal::from_components(1, 0, self.mag.log10())
    }

    /// Returns log10 of the Decimal.
    ///
    /// Returns a NaN sentinel for non-positive input. Use [`checked_log10`](Self::checked_log10)
    /// for explicit error handling.
    pub fn log10(&self) -> Decimal {
        if self.sign <= 0 {
            return Decimal::nan_sentinel();
        }

        if self.layer > 0 {
            return Decimal::from_components(sign(self.mag), self.layer - 1, self.mag.abs());
        }

        Decimal::from_components(self.sign, 0, self.mag.log10())
    }

    /// Returns log10 of the Decimal, or an error for non-positive input.
    pub fn checked_log10(&self) -> Result<Decimal, ArithmeticError> {
        if self.sign <= 0 {
            return Err(ArithmeticError {
                kind: ArithmeticErrorKind::OutOfDomain,
                op: "log10",
            });
        }
        Ok(self.log10())
    }

    /// Returns the log of the Decimal with the given base.
    ///
    /// Returns a NaN sentinel for out-of-domain inputs. Use [`checked_log`](Self::checked_log)
    /// for explicit error handling.
    pub fn log(&self, base: Decimal) -> Decimal {
        if self.sign <= 0 {
            return Decimal::nan_sentinel();
        }

        if base.sign <= 0 {
            return Decimal::nan_sentinel();
        }

        if base.sign == 1 && base.layer == 0 && (base.mag - 1.0).abs() < COMPARE_EPSILON {
            return Decimal::nan_sentinel();
        }

        if self.layer == 0 && base.layer == 0 {
            return Decimal::from_components(self.sign, 0, self.mag.ln() / base.mag.ln());
        }

        self.log10() / base.log10()
    }

    /// Returns the log of the Decimal with the given base, or an error for out-of-domain inputs.
    pub fn checked_log(&self, base: &Decimal) -> Result<Decimal, ArithmeticError> {
        if self.sign <= 0 {
            return Err(ArithmeticError {
                kind: ArithmeticErrorKind::OutOfDomain,
                op: "log",
            });
        }
        if base.sign <= 0 {
            return Err(ArithmeticError {
                kind: ArithmeticErrorKind::OutOfDomain,
                op: "log",
            });
        }
        if base.sign == 1 && base.layer == 0 && (base.mag - 1.0).abs() < COMPARE_EPSILON {
            return Err(ArithmeticError {
                kind: ArithmeticErrorKind::OutOfDomain,
                op: "log",
            });
        }
        Ok(self.log(*base))
    }

    /// Returns the log2 of the Decimal.
    ///
    /// Returns a NaN sentinel for non-positive input. Use [`checked_log2`](Self::checked_log2)
    /// for explicit error handling.
    pub fn log2(&self) -> Decimal {
        if self.sign <= 0 {
            return Decimal::nan_sentinel();
        }

        if self.layer == 0 {
            return Decimal::from_components(self.sign, 0, self.mag.log2());
        }

        if self.layer == 1 {
            return Decimal::from_components(
                sign(self.mag),
                0,
                self.mag.abs() * std::f64::consts::LOG2_10,
            );
        }

        if self.layer == 2 {
            return Decimal::from_components(
                sign(self.mag),
                1,
                self.mag.abs() + 0.5213902276543247,
            );
        }

        Decimal::from_components(sign(self.mag), self.layer - 1, self.mag.abs())
    }

    /// Returns the log2 of the Decimal, or an error for non-positive input.
    pub fn checked_log2(&self) -> Result<Decimal, ArithmeticError> {
        if self.sign <= 0 {
            return Err(ArithmeticError {
                kind: ArithmeticErrorKind::OutOfDomain,
                op: "log2",
            });
        }
        Ok(self.log2())
    }

    /// Returns the natural log of the Decimal.
    ///
    /// Returns a NaN sentinel for non-positive input. Use [`checked_ln`](Self::checked_ln)
    /// for explicit error handling.
    pub fn ln(&self) -> Decimal {
        if self.sign <= 0 {
            return Decimal::nan_sentinel();
        }

        if self.layer == 0 {
            return Decimal::from_components(self.sign, 0, self.mag.ln());
        }

        if self.layer == 1 {
            return Decimal::from_components(
                sign(self.mag),
                0,
                self.mag.abs() * std::f64::consts::LN_10,
            );
        }

        if self.layer == 2 {
            return Decimal::from_components(
                sign(self.mag),
                1,
                self.mag.abs() + 0.36221568869946325,
            );
        }

        Decimal::from_components(sign(self.mag), self.layer - 1, self.mag.abs())
    }

    /// Returns the natural log of the Decimal, or an error for non-positive input.
    pub fn checked_ln(&self) -> Result<Decimal, ArithmeticError> {
        if self.sign <= 0 {
            return Err(ArithmeticError {
                kind: ArithmeticErrorKind::OutOfDomain,
                op: "ln",
            });
        }
        Ok(self.ln())
    }

    /// Returns the Decimal to the power of the given exponent.
    pub fn pow(self, exp: Decimal) -> Decimal {
        let a = self;
        let b = exp;

        if a.sign == 0 {
            return if b == Decimal::from_finite(0.0) {
                Decimal::one()
            } else {
                a
            };
        }

        if a.sign == 1 && a.layer == 0 && (a.mag - 1.0).abs() < COMPARE_EPSILON {
            return a;
        }

        if b.sign == 0 {
            return Decimal::one();
        }

        if b.sign == 1 && b.layer == 0 && (b.mag - 1.0).abs() < COMPARE_EPSILON {
            return a;
        }

        let result = (a.abs_log10() * b).pow10();

        if self.sign == -1 && ((b.to_number() % 2.0).abs() - 1.0).abs() < COMPARE_EPSILON {
            return -result;
        }

        result
    }

    /// Returns `self` to the power of `other`, or an error for invalid inputs.
    pub fn checked_pow(&self, other: &Self) -> Result<Decimal, ArithmeticError> {
        // Negative base with non-integer exponent is undefined.
        if self.sign == -1 {
            let exp_val = other.to_number();
            if exp_val.is_finite() && (exp_val - exp_val.round()).abs() > COMPARE_EPSILON {
                return Err(ArithmeticError {
                    kind: ArithmeticErrorKind::NegativeBase,
                    op: "pow",
                });
            }
        }
        Ok(self.pow(*other))
    }

    /// Returns the Decimal raised to the next power of 10.
    pub fn pow10(self) -> Decimal {
        if !self.mag.is_finite() {
            return Decimal::nan_sentinel();
        }

        let mut a = self;

        if a.layer == 0 {
            let new_mag = 10.0_f64.powf(a.sign as f64 * a.mag);
            if new_mag.is_finite() && new_mag.abs() > 0.1 {
                return Decimal::from_components(1, 0, new_mag);
            }
            if a.sign == 0 {
                return Decimal::one();
            }
            a = Decimal::from_components_unchecked(a.sign, a.layer + 1, a.mag.log10());
        }

        if a.sign > 0 && a.mag > 0.0 {
            return Decimal::from_components(a.sign, a.layer + 1, a.mag);
        }

        if a.sign < 0 && a.mag > 0.0 {
            return Decimal::from_components(-a.sign, a.layer + 1, -a.mag);
        }

        Decimal::one()
    }

    /// Returns the exponential function of the Decimal.
    pub fn exp(self) -> Decimal {
        if self.mag < 0.0 {
            return Decimal::one();
        }

        if self.layer == 0 && self.mag <= 709.7 {
            return Decimal::from_finite((self.sign as f64 * self.mag).exp());
        }

        if self.layer == 0 {
            return Decimal::from_components(
                1,
                1,
                self.sign as f64 * std::f64::consts::E.log10() * self.mag,
            );
        }

        if self.layer == 1 {
            return Decimal::from_components(
                1,
                2,
                self.sign as f64 * std::f64::consts::LOG10_E.log10() + self.mag,
            );
        }

        Decimal::from_components(1, self.layer + 1, self.sign as f64 * self.mag)
    }

    /// Returns the gamma function of the Decimal.
    pub fn gamma(&self) -> Decimal {
        if self.mag < 0.0 {
            return self.recip();
        }

        if self.layer == 0 {
            if self < &Decimal::from_components_unchecked(1, 0, 24.0) {
                return Decimal::from_finite(f_gamma(self.sign as f64 * self.mag));
            }

            let t = self.mag - 1.0;
            let mut l = 0.9189385332046727;
            l += (t + 0.5) * t.ln();
            l -= t;
            let n2 = t * t;
            let mut np = t;
            let mut lm = 12.0 * np;
            let adj = 1.0 / lm;
            let l2 = l + adj;
            if (l2 - l).abs() < COMPARE_EPSILON {
                return Decimal::from_finite(l).exp();
            }

            l = l2;
            np *= n2;
            lm = 1260.0 * np;
            let mut lt = 1.0 / lm;
            l += lt;
            np *= n2;
            lm = 1680.0 * np;
            lt = 1.0 / lm;
            l -= lt;
            return Decimal::from_finite(l).exp();
        }

        if self.layer == 1 {
            return (*self * (self.ln() - Decimal::from_finite(1.0))).exp();
        }

        self.exp()
    }

    /// Returns the gamma function of the Decimal, or an error on failure.
    pub fn checked_gamma(&self) -> Result<Decimal, ArithmeticError> {
        Ok(self.gamma())
    }

    /// Returns the factorial of the Decimal.
    pub fn factorial(&self) -> Decimal {
        if self.mag < 0.0 {
            return (*self + Decimal::from_finite(1.0)).gamma();
        }

        if self.layer == 0 {
            return (*self + Decimal::from_finite(1.0)).gamma();
        }

        if self.layer == 1 {
            return (*self * self.ln() - Decimal::from_finite(1.0)).exp();
        }

        self.exp()
    }

    /// Returns the factorial of the Decimal, or an error on failure.
    pub fn checked_factorial(&self) -> Result<Decimal, ArithmeticError> {
        Ok(self.factorial())
    }

    /// Returns the natural logarithm of the gamma function of the Decimal.
    pub fn ln_gamma(&self) -> Decimal {
        self.gamma().ln()
    }

    /// Returns the product logarithm (Lambert W) of the Decimal.
    ///
    /// Returns `Err` for `z < -1/e` or if iteration fails to converge.
    pub fn lambertw(&self) -> Result<Decimal, BreakEternityError> {
        if self < &Decimal::from_finite(-0.3678794411710499) {
            return Err(BreakEternityError::LambertWError);
        }

        if self.mag < 0.0 {
            return Ok(Decimal::from_finite(f_lambertw(self.to_number(), None)?));
        }

        if self.layer == 0 {
            return Ok(Decimal::from_finite(f_lambertw(
                self.sign as f64 * self.mag,
                None,
            )?));
        }

        if self.layer == 1 || self.layer == 2 {
            return d_lambertw(*self, None);
        }

        Ok(Decimal::from_components_unchecked(
            self.sign,
            self.layer - 1,
            self.mag,
        ))
    }

    /// Returns the product logarithm (Lambert W) of the Decimal, or an `ArithmeticError`.
    ///
    /// Returns `Err(OutOfDomain)` for `z < -1/e`, or `Err(IterationDiverged)` if the
    /// iteration fails to converge.
    pub fn checked_lambertw(&self) -> Result<Decimal, ArithmeticError> {
        self.lambertw().map_err(|e| match e {
            BreakEternityError::LambertWError => ArithmeticError {
                kind: ArithmeticErrorKind::OutOfDomain,
                op: "lambertw",
            },
            BreakEternityError::IterationFailedConverging { .. } => ArithmeticError {
                kind: ArithmeticErrorKind::IterationDiverged,
                op: "lambertw",
            },
            _ => ArithmeticError {
                kind: ArithmeticErrorKind::Undefined,
                op: "lambertw",
            },
        })
    }

    /// Returns the square root of the Decimal, or an error for negative input.
    pub fn checked_sqrt(&self) -> Result<Decimal, ArithmeticError> {
        if self.sign < 0 && self.layer == 0 {
            return Err(ArithmeticError {
                kind: ArithmeticErrorKind::OutOfDomain,
                op: "sqrt",
            });
        }
        Ok(self.sqrt())
    }
}
