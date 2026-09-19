//! Transcendental functions: logarithms, powers and roots, `exp`, `gamma`, and Lambert W.
//!
//! Every operation exists in up to three forms:
//!
//! * a `pub(crate)` `*_raw` kernel with `break_eternity.js` semantics that never panics and
//!   reports undefined results with the internal NaN sentinel;
//! * a `checked_*` method that maps domain failures to [`ArithmeticError`];
//! * a plain method that panics on domain failures (mirroring the operator overloads).

use crate::constants::{LAMBERTW_TOLERANCE, OMEGA};
use crate::decimal::Decimal;
use crate::error::ArithmeticError;
use crate::utils::sign;

/// Which real branch of the Lambert W function to evaluate.
///
/// `W_0` (the principal branch) is defined for `z >= -1/e`; `W_-1` is defined for
/// `-1/e <= z < 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LambertBranch {
    /// The principal branch `W_0`.
    #[default]
    Principal,
    /// The non-principal real branch `W_-1`.
    NonPrincipal,
}

/// Parity classification of an exponent, used to decide the sign of a negative base raised
/// to a power.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Parity {
    Odd,
    Even,
    NonInteger,
}

fn exponent_parity(b: Decimal) -> Parity {
    let n = b.to_number();
    if !n.is_finite() {
        return Parity::NonInteger;
    }
    let m = (n % 2.0).abs();
    if m == 1.0 {
        Parity::Odd
    } else if m == 0.0 {
        Parity::Even
    } else {
        Parity::NonInteger
    }
}

// ---------------------------------------------------------------------------
// Free-function helpers (crate-private)
// ---------------------------------------------------------------------------

/// Stirling-series approximation to the gamma function for real arguments.
pub(crate) fn f_gamma(mut num: f64) -> f64 {
    if !num.is_finite() {
        return num;
    }

    if num < -50.0 {
        if num == num.trunc() {
            return f64::NEG_INFINITY;
        }
        return 0.0;
    }

    let mut scal1 = 1.0;
    while num < 10.0 {
        scal1 *= num;
        num += 1.0;
    }

    num -= 1.0;
    let mut l = 0.9189385332046727;
    l += (num + 0.5) * num.ln();
    l -= num;
    let num2 = num * num;
    let mut num_p = num;
    l += 1.0 / (12.0 * num_p);
    num_p *= num2;
    l -= 1.0 / (360.0 * num_p);
    num_p *= num2;
    l += 1.0 / (1260.0 * num_p);
    num_p *= num2;
    l -= 1.0 / (1680.0 * num_p);
    num_p *= num2;
    l += 1.0 / (1188.0 * num_p);
    num_p *= num2;
    l -= 691.0 / (360360.0 * num_p);
    num_p *= num2;
    l += 7.0 / (1092.0 * num_p);
    num_p *= num2;
    l -= 3617.0 / (122400.0 * num_p);

    l.exp() / scal1
}

/// `n!` as an `f64`, exact up to `22!` and correctly rounded well beyond (finite up to `170!`).
fn exact_factorial(n: u32) -> f64 {
    (2..=n).fold(1.0_f64, |acc, k| acc * f64::from(k))
}

/// Scalar Lambert W function. `principal` selects `W_0` (true) or `W_-1` (false).
pub(crate) fn f_lambertw(z: f64, tol: f64, principal: bool) -> Result<f64, ArithmeticError> {
    if !z.is_finite() {
        return Ok(z);
    }

    let mut w;
    if principal {
        if z == 0.0 {
            return Ok(z);
        }
        if z == 1.0 {
            return Ok(OMEGA);
        }
        w = if z < 10.0 { 0.0 } else { z.ln() - z.ln().ln() };
    } else {
        if z == 0.0 {
            return Ok(f64::NEG_INFINITY);
        }
        w = if z <= -0.1 {
            -2.0
        } else {
            (-z).ln() - (-(-z).ln()).ln()
        };
    }

    for _ in 0..100 {
        let wn = (z * (-w).exp() + w * w) / (w + 1.0);
        if wn.is_nan() {
            return Err(ArithmeticError::undefined("lambertw"));
        }
        if (wn - w).abs() < tol * wn.abs() {
            return Ok(wn);
        }
        w = wn;
    }

    Err(ArithmeticError::diverged("lambertw"))
}

/// Decimal-valued Lambert W function via Halley's method. `principal` selects the branch.
pub(crate) fn d_lambertw(
    z: Decimal,
    tol: f64,
    principal: bool,
) -> Result<Decimal, ArithmeticError> {
    if !z.mag.is_finite() {
        return Ok(z);
    }

    let one = Decimal::one();
    let two = Decimal::two();
    let tol_d = Decimal::from_finite(tol);

    let mut w;
    if principal {
        if z.sign == 0 {
            return Ok(Decimal::zero());
        }
        if z == one {
            return Ok(Decimal::from_finite(OMEGA));
        }
        w = z.ln_raw();
    } else {
        if z.sign == 0 {
            return Ok(Decimal::neg_inf());
        }
        w = (-z).ln_raw();
    }

    for _ in 0..100 {
        let ew = (-w).exp();
        let wewz = w.sub_raw(z.mul_raw(ew));
        let denom = w.add_raw(one).sub_raw(
            w.add_raw(two)
                .mul_raw(wewz)
                .div_raw(two.mul_raw(w).add_raw(two)),
        );
        let wn = w.sub_raw(wewz.div_raw(denom));
        if wn.has_nan_mag() {
            return Err(ArithmeticError::undefined("lambertw"));
        }
        if wn.sub_raw(w).abs() < wn.abs().mul_raw(tol_d) {
            return Ok(wn);
        }
        w = wn;
    }

    Err(ArithmeticError::diverged("lambertw"))
}

// ---------------------------------------------------------------------------
// impl Decimal — logarithms
// ---------------------------------------------------------------------------

impl Decimal {
    pub(crate) fn abs_log10_raw(self) -> Decimal {
        if self.has_nan_mag() || self.sign == 0 {
            return Decimal::nan_sentinel();
        }
        if self.is_infinite() {
            return Decimal::inf();
        }
        if self.layer > 0 {
            return Decimal::from_components(sign(self.mag), self.layer - 1, self.mag.abs());
        }
        Decimal::from_components(1, 0, self.mag.log10())
    }

    /// Returns `log10(|self|)`, or an error for zero.
    pub fn checked_abs_log10(&self) -> Result<Decimal, ArithmeticError> {
        if self.sign == 0 {
            return Err(ArithmeticError::out_of_domain("abs_log10"));
        }
        self.abs_log10_raw().nan_to_err("abs_log10")
    }

    /// Returns `log10(|self|)`.
    ///
    /// # Panics
    ///
    /// Panics for zero. Use [`checked_abs_log10`](Self::checked_abs_log10) for explicit error
    /// handling.
    pub fn abs_log10(&self) -> Decimal {
        self.checked_abs_log10()
            .unwrap_or_else(|e| panic!("undefined Decimal abs_log10: {e} (self={self:?})"))
    }

    /// "Positive log10": returns `log10(self)` for positive values and `0` for zero or negative
    /// values. Never fails, which makes it convenient for scaling formulas in game logic.
    pub fn p_log10(&self) -> Decimal {
        if self.sign <= 0 {
            return Decimal::zero();
        }
        self.log10_raw()
    }

    pub(crate) fn log10_raw(self) -> Decimal {
        if self.has_nan_mag() || self.sign <= 0 {
            return Decimal::nan_sentinel();
        }
        if self.is_infinite() {
            return Decimal::inf();
        }
        if self.layer > 0 {
            return Decimal::from_components(sign(self.mag), self.layer - 1, self.mag.abs());
        }
        Decimal::from_components(self.sign, 0, self.mag.log10())
    }

    /// Returns log10 of the Decimal.
    ///
    /// # Panics
    ///
    /// Panics for non-positive input. Use [`checked_log10`](Self::checked_log10) for explicit
    /// error handling.
    pub fn log10(&self) -> Decimal {
        self.checked_log10()
            .unwrap_or_else(|e| panic!("undefined Decimal log10: {e} (self={self:?})"))
    }

    /// Returns log10 of the Decimal, or an error for non-positive input.
    pub fn checked_log10(&self) -> Result<Decimal, ArithmeticError> {
        if self.sign <= 0 {
            return Err(ArithmeticError::out_of_domain("log10"));
        }
        self.log10_raw().nan_to_err("log10")
    }

    pub(crate) fn log_raw(self, base: Decimal) -> Decimal {
        if self.has_nan_mag() || base.has_nan_mag() || self.sign <= 0 || base.sign <= 0 {
            return Decimal::nan_sentinel();
        }
        if base == Decimal::one() {
            return Decimal::nan_sentinel();
        }
        if self.layer == 0 && base.layer == 0 {
            return Decimal::from_components(1, 0, self.mag.ln() / base.mag.ln());
        }
        self.log10_raw().div_raw(base.log10_raw())
    }

    /// Returns the log of the Decimal with the given base.
    ///
    /// # Panics
    ///
    /// Panics for out-of-domain inputs (non-positive value, non-positive base, or base 1).
    /// Use [`checked_log`](Self::checked_log) for explicit error handling.
    pub fn log(&self, base: Decimal) -> Decimal {
        self.checked_log(&base)
            .unwrap_or_else(|e| panic!("undefined Decimal log: {e} (self={self:?}, base={base:?})"))
    }

    /// Returns the log of the Decimal with the given base, or an error for out-of-domain inputs.
    pub fn checked_log(&self, base: &Decimal) -> Result<Decimal, ArithmeticError> {
        if self.sign <= 0 || base.sign <= 0 || *base == Decimal::one() {
            return Err(ArithmeticError::out_of_domain("log"));
        }
        self.log_raw(*base).nan_to_err("log")
    }

    pub(crate) fn log2_raw(self) -> Decimal {
        if self.has_nan_mag() || self.sign <= 0 {
            return Decimal::nan_sentinel();
        }
        if self.is_infinite() {
            return Decimal::inf();
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

    /// Returns the log2 of the Decimal.
    ///
    /// # Panics
    ///
    /// Panics for non-positive input. Use [`checked_log2`](Self::checked_log2) for explicit
    /// error handling.
    pub fn log2(&self) -> Decimal {
        self.checked_log2()
            .unwrap_or_else(|e| panic!("undefined Decimal log2: {e} (self={self:?})"))
    }

    /// Returns the log2 of the Decimal, or an error for non-positive input.
    pub fn checked_log2(&self) -> Result<Decimal, ArithmeticError> {
        if self.sign <= 0 {
            return Err(ArithmeticError::out_of_domain("log2"));
        }
        self.log2_raw().nan_to_err("log2")
    }

    pub(crate) fn ln_raw(self) -> Decimal {
        if self.has_nan_mag() || self.sign <= 0 {
            return Decimal::nan_sentinel();
        }
        if self.is_infinite() {
            return Decimal::inf();
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

    /// Returns the natural log of the Decimal.
    ///
    /// # Panics
    ///
    /// Panics for non-positive input. Use [`checked_ln`](Self::checked_ln) for explicit error
    /// handling.
    pub fn ln(&self) -> Decimal {
        self.checked_ln()
            .unwrap_or_else(|e| panic!("undefined Decimal ln: {e} (self={self:?})"))
    }

    /// Returns the natural log of the Decimal, or an error for non-positive input.
    pub fn checked_ln(&self) -> Result<Decimal, ArithmeticError> {
        if self.sign <= 0 {
            return Err(ArithmeticError::out_of_domain("ln"));
        }
        self.ln_raw().nan_to_err("ln")
    }

    // -----------------------------------------------------------------------
    // Powers
    // -----------------------------------------------------------------------

    /// `self ^ b` with `break_eternity.js` semantics, plus an exact `powf` fast path when
    /// both operands are layer 0 and the result is a normal `f64`.
    pub(crate) fn pow_raw(self, b: Decimal) -> Decimal {
        let a = self;
        if a.has_nan_mag() || b.has_nan_mag() {
            return Decimal::nan_sentinel();
        }

        // 0^b: 1 for b == 0, 0 for b > 0, infinite for b < 0.
        if a.sign == 0 {
            return match b.sign.cmp(&0) {
                std::cmp::Ordering::Equal => Decimal::one(),
                std::cmp::Ordering::Less => Decimal::inf(),
                std::cmp::Ordering::Greater => Decimal::zero(),
            };
        }
        // 1^b == 1
        if a.sign == 1 && a.layer == 0 && a.mag == 1.0 {
            return Decimal::one();
        }
        // a^0 == 1
        if b.sign == 0 {
            return Decimal::one();
        }
        // a^1 == a
        if b.sign == 1 && b.layer == 0 && b.mag == 1.0 {
            return a;
        }

        if b.is_infinite() {
            if a.sign == -1 {
                return Decimal::nan_sentinel();
            }
            let a_gt_1 = a > Decimal::one();
            return if (b.sign == 1) == a_gt_1 {
                Decimal::inf()
            } else {
                Decimal::zero()
            };
        }

        if a.is_infinite() {
            let magnitude = if b.sign > 0 {
                Decimal::inf()
            } else {
                Decimal::zero()
            };
            if a.sign == 1 {
                return magnitude;
            }
            return match exponent_parity(b) {
                Parity::Odd => -magnitude,
                Parity::Even => magnitude,
                Parity::NonInteger => Decimal::nan_sentinel(),
            };
        }

        // Exact fast path for ordinary floats.
        if a.layer == 0 && b.layer == 0 {
            let r = (a.sign as f64 * a.mag).powf(b.sign as f64 * b.mag);
            if r.is_normal() {
                return Decimal::from_f64(r);
            }
        }

        let result = a.abs_log10_raw().mul_raw(b).pow10_raw();

        if a.sign == -1 {
            return match exponent_parity(b) {
                Parity::Odd => -result,
                Parity::Even => result,
                Parity::NonInteger => Decimal::nan_sentinel(),
            };
        }

        result
    }

    /// Returns `self` raised to the power `exp`.
    ///
    /// Layer-0 operands use `f64::powf` directly when the result fits, so integer powers of
    /// ordinary floats are exact (`2^10 == 1024`). A negative base with a non-integer exponent
    /// is undefined.
    ///
    /// # Panics
    ///
    /// Panics if the result is undefined (negative base with non-integer exponent, or `0`
    /// raised to a negative power). Use [`checked_pow`](Self::checked_pow) for explicit error
    /// handling.
    pub fn pow(self, exp: Decimal) -> Decimal {
        self.checked_pow(&exp)
            .unwrap_or_else(|e| panic!("undefined Decimal pow: {e} (base={self:?}, exp={exp:?})"))
    }

    /// Returns `self` to the power of `other`, or an error for invalid inputs.
    ///
    /// * `0` raised to a negative power is [`DivisionByZero`](crate::ArithmeticErrorKind::DivisionByZero).
    /// * A negative base with a non-integer (or infinite) exponent is
    ///   [`NegativeBase`](crate::ArithmeticErrorKind::NegativeBase).
    pub fn checked_pow(&self, other: &Self) -> Result<Decimal, ArithmeticError> {
        if self.sign == 0 && other.sign < 0 {
            return Err(ArithmeticError::division_by_zero("pow"));
        }
        if self.sign == -1
            && other.sign != 0
            && !(other.layer == 0 && other.mag == 1.0)
            && exponent_parity(*other) == Parity::NonInteger
        {
            return Err(ArithmeticError::negative_base("pow"));
        }
        self.pow_raw(*other).nan_to_err("pow")
    }

    /// Returns `base ^ self`.
    ///
    /// # Panics
    ///
    /// Panics if the result is undefined; see [`pow`](Self::pow).
    pub fn pow_base(&self, base: Decimal) -> Decimal {
        base.pow(*self)
    }

    /// Returns `base ^ self`, or an error for invalid inputs; see [`checked_pow`](Self::checked_pow).
    pub fn checked_pow_base(&self, base: &Decimal) -> Result<Decimal, ArithmeticError> {
        base.checked_pow(self)
    }

    pub(crate) fn pow10_raw(self) -> Decimal {
        if self.has_nan_mag() {
            return Decimal::nan_sentinel();
        }
        if self.is_infinite() {
            return if self.sign == 1 {
                Decimal::inf()
            } else {
                Decimal::zero()
            };
        }

        let mut a = self;

        if a.layer == 0 {
            let new_mag = 10.0_f64.powf(a.sign as f64 * a.mag);
            if new_mag.is_finite() && new_mag.abs() >= 0.1 {
                return Decimal::from_components(1, 0, new_mag);
            }
            if a.sign == 0 {
                return Decimal::one();
            }
            a = Decimal::from_components_unchecked(a.sign, 1, a.mag.log10());
        }

        if a.sign > 0 && a.mag >= 0.0 {
            return Decimal::from_components(a.sign, a.layer.saturating_add(1), a.mag);
        }

        if a.sign < 0 && a.mag >= 0.0 {
            return Decimal::from_components(-a.sign, a.layer.saturating_add(1), -a.mag);
        }

        Decimal::one()
    }

    /// Returns `10 ^ self`.
    pub fn pow10(self) -> Decimal {
        self.pow10_raw()
    }

    /// Returns `e ^ self`.
    pub fn exp(self) -> Decimal {
        if self.has_nan_mag() {
            return Decimal::nan_sentinel();
        }
        if self.is_infinite() {
            return if self.sign == 1 {
                Decimal::inf()
            } else {
                Decimal::zero()
            };
        }

        if self.mag < 0.0 {
            return Decimal::one();
        }

        if self.layer == 0 && self.mag <= 709.7 {
            return Decimal::from_f64((self.sign as f64 * self.mag).exp());
        }

        if self.layer == 0 {
            return Decimal::from_components(
                1,
                1,
                self.sign as f64 * std::f64::consts::LOG10_E * self.mag,
            );
        }

        if self.layer == 1 {
            return Decimal::from_components(
                1,
                2,
                self.sign as f64 * (std::f64::consts::LOG10_E.log10() + self.mag),
            );
        }

        Decimal::from_components(1, self.layer.saturating_add(1), self.sign as f64 * self.mag)
    }

    /// Returns the Decimal squared.
    pub fn sqr(&self) -> Decimal {
        self.mul_raw(*self)
    }

    pub(crate) fn sqrt_raw(self) -> Decimal {
        if self.has_nan_mag() || self.sign == -1 {
            return Decimal::nan_sentinel();
        }
        if self.is_infinite() {
            return Decimal::inf();
        }

        if self.layer == 0 {
            return Decimal::from_f64(self.mag.sqrt());
        }

        if self.layer == 1 {
            // For positive mag the value is large (10^mag) and sqrt promotes to layer 2.
            // For negative mag the value is tiny (10^mag with mag < 0): sqrt(10^mag) = 10^(mag/2).
            if self.mag >= 0.0 {
                return Decimal::from_components(
                    1,
                    2,
                    self.mag.log10() - std::f64::consts::LOG10_2,
                );
            }
            return Decimal::from_components(1, 1, self.mag / 2.0);
        }

        let mut result =
            Decimal::from_components_unchecked(1, self.layer - 1, self.mag).div_raw(Decimal::two());
        result.layer = result.layer.saturating_add(1);
        result.normalize();
        result
    }

    /// Returns the square root of the Decimal.
    ///
    /// # Panics
    ///
    /// Panics for negative input. Use [`checked_sqrt`](Self::checked_sqrt) for explicit error
    /// handling.
    pub fn sqrt(&self) -> Decimal {
        self.checked_sqrt()
            .unwrap_or_else(|e| panic!("undefined Decimal sqrt: {e} (self={self:?})"))
    }

    /// Returns the square root of the Decimal, or an error for negative input.
    pub fn checked_sqrt(&self) -> Result<Decimal, ArithmeticError> {
        if self.sign < 0 {
            return Err(ArithmeticError::out_of_domain("sqrt"));
        }
        self.sqrt_raw().nan_to_err("sqrt")
    }

    /// Returns the Decimal cubed.
    pub fn cube(&self) -> Decimal {
        self.pow_raw(Decimal::from_finite(3.0))
    }

    /// Returns the real cube root of the Decimal. Negative inputs give negative results.
    pub fn cbrt(&self) -> Decimal {
        let third = Decimal::from_finite(1.0 / 3.0);
        if self.sign < 0 {
            return -self.abs().pow_raw(third);
        }
        self.pow_raw(third)
    }

    /// Returns the `n`-th root of the Decimal, `self ^ (1/n)`.
    ///
    /// Odd integer roots of negative numbers are real and negative; other roots of negative
    /// numbers are undefined.
    ///
    /// # Panics
    ///
    /// Panics if `n` is zero or the root is undefined. Use [`checked_root`](Self::checked_root)
    /// for explicit error handling.
    pub fn root(self, n: Decimal) -> Decimal {
        self.checked_root(&n)
            .unwrap_or_else(|e| panic!("undefined Decimal root: {e} (self={self:?}, n={n:?})"))
    }

    /// Returns the `n`-th root of the Decimal, or an error if it is undefined.
    pub fn checked_root(&self, n: &Decimal) -> Result<Decimal, ArithmeticError> {
        if n.sign == 0 {
            return Err(ArithmeticError::division_by_zero("root"));
        }
        if self.sign < 0 && exponent_parity(*n) == Parity::Odd {
            return self.abs().checked_root(n).map(|r| -r);
        }
        self.checked_pow(&n.recip_raw())
            .map_err(|e| ArithmeticError::new(e.kind, "root"))
    }

    // -----------------------------------------------------------------------
    // Gamma and factorial
    // -----------------------------------------------------------------------

    /// True if the value is a pole of the gamma function (zero or a negative integer).
    fn is_gamma_pole(&self) -> bool {
        self.sign == 0 || (self.sign == -1 && self.layer == 0 && self.mag.fract() == 0.0)
    }

    pub(crate) fn gamma_raw(self) -> Decimal {
        if self.has_nan_mag() {
            return Decimal::nan_sentinel();
        }
        if self.is_infinite() {
            return if self.sign == 1 {
                Decimal::inf()
            } else {
                Decimal::nan_sentinel()
            };
        }
        if self.is_gamma_pole() {
            return Decimal::nan_sentinel();
        }

        if self.mag < 0.0 {
            return self.recip_raw();
        }

        if self.layer == 0 {
            let x = self.sign as f64 * self.mag;
            // Exact for positive integers up to 171! (the Stirling series is off by a few ulps).
            if (1.0..=171.0).contains(&x) && x.fract() == 0.0 {
                return Decimal::from_f64(exact_factorial(x as u32 - 1));
            }
            if x < 24.0 {
                return Decimal::from_f64(f_gamma(x));
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
            if l2 == l {
                return Decimal::from_finite(l).exp();
            }

            l = l2;
            np *= n2;
            lm = 360.0 * np;
            let adj = 1.0 / lm;
            let l2 = l - adj;
            if l2 == l {
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
            return self.mul_raw(self.ln_raw().sub_raw(Decimal::one())).exp();
        }

        self.exp()
    }

    /// Returns the gamma function of the Decimal.
    ///
    /// # Panics
    ///
    /// Panics at the poles (zero and the negative integers) and for negative infinity. Use
    /// [`checked_gamma`](Self::checked_gamma) for explicit error handling.
    pub fn gamma(&self) -> Decimal {
        self.checked_gamma()
            .unwrap_or_else(|e| panic!("undefined Decimal gamma: {e} (self={self:?})"))
    }

    /// Returns the gamma function of the Decimal, or an error at its poles.
    pub fn checked_gamma(&self) -> Result<Decimal, ArithmeticError> {
        if self.is_gamma_pole() || (self.is_infinite() && self.sign == -1) {
            return Err(ArithmeticError::out_of_domain("gamma"));
        }
        self.gamma_raw().nan_to_err("gamma")
    }

    pub(crate) fn factorial_raw(self) -> Decimal {
        if self.has_nan_mag() {
            return Decimal::nan_sentinel();
        }
        if self.mag < 0.0 || self.layer == 0 || self.is_infinite() {
            return self.add_raw(Decimal::one()).gamma_raw();
        }
        if self.layer == 1 {
            return self.mul_raw(self.ln_raw().sub_raw(Decimal::one())).exp();
        }
        self.exp()
    }

    /// Returns the factorial of the Decimal, extended to real arguments via the gamma function.
    ///
    /// # Panics
    ///
    /// Panics for negative integers. Use [`checked_factorial`](Self::checked_factorial) for
    /// explicit error handling.
    pub fn factorial(&self) -> Decimal {
        self.checked_factorial()
            .unwrap_or_else(|e| panic!("undefined Decimal factorial: {e} (self={self:?})"))
    }

    /// Returns the factorial of the Decimal, or an error for negative integers.
    pub fn checked_factorial(&self) -> Result<Decimal, ArithmeticError> {
        if self.sign == -1
            && (self.layer > 0 || self.mag.fract() == 0.0)
            && !self.is_infinite()
            && self.mag >= 0.0
        {
            // Negative integer (or a negative value so large its fractional part is lost).
            return Err(ArithmeticError::out_of_domain("factorial"));
        }
        if self.is_infinite() && self.sign == -1 {
            return Err(ArithmeticError::out_of_domain("factorial"));
        }
        self.factorial_raw().nan_to_err("factorial")
    }

    /// Returns the natural logarithm of the gamma function of the Decimal.
    ///
    /// # Panics
    ///
    /// Panics where [`gamma`](Self::gamma) is undefined or non-positive.
    pub fn ln_gamma(&self) -> Decimal {
        self.checked_ln_gamma()
            .unwrap_or_else(|e| panic!("undefined Decimal ln_gamma: {e} (self={self:?})"))
    }

    /// Returns `ln(gamma(self))`, or an error where gamma is undefined or non-positive.
    pub fn checked_ln_gamma(&self) -> Result<Decimal, ArithmeticError> {
        let g = self
            .checked_gamma()
            .map_err(|e| ArithmeticError::new(e.kind, "ln_gamma"))?;
        g.checked_ln()
            .map_err(|e| ArithmeticError::new(e.kind, "ln_gamma"))
    }

    // -----------------------------------------------------------------------
    // Lambert W
    // -----------------------------------------------------------------------

    /// Lambert W with NaN-sentinel error reporting, for internal iteration loops.
    pub(crate) fn lambertw_raw(self, branch: LambertBranch) -> Decimal {
        self.checked_lambertw_branch(branch)
            .unwrap_or_else(|_| Decimal::nan_sentinel())
    }

    /// Returns the principal branch `W_0` of the Lambert W function (the solution of
    /// `W * e^W = self`).
    ///
    /// # Panics
    ///
    /// Panics for `self < -1/e` or if the iteration fails to converge. Use
    /// [`checked_lambertw`](Self::checked_lambertw) for explicit error handling.
    pub fn lambertw(&self) -> Decimal {
        self.lambertw_branch(LambertBranch::Principal)
    }

    /// Returns the principal branch `W_0` of the Lambert W function, or an error for
    /// `self < -1/e` ([`OutOfDomain`](crate::ArithmeticErrorKind::OutOfDomain)) or on
    /// non-convergence ([`IterationDiverged`](crate::ArithmeticErrorKind::IterationDiverged)).
    pub fn checked_lambertw(&self) -> Result<Decimal, ArithmeticError> {
        self.checked_lambertw_branch(LambertBranch::Principal)
    }

    /// Returns the requested real branch of the Lambert W function.
    ///
    /// # Panics
    ///
    /// Panics outside the branch's domain or on non-convergence. Use
    /// [`checked_lambertw_branch`](Self::checked_lambertw_branch) for explicit error handling.
    pub fn lambertw_branch(&self, branch: LambertBranch) -> Decimal {
        self.checked_lambertw_branch(branch).unwrap_or_else(|e| {
            panic!("undefined Decimal lambertw: {e} (self={self:?}, branch={branch:?})")
        })
    }

    /// Returns the requested real branch of the Lambert W function, or an error.
    ///
    /// `W_0` is defined for `self >= -1/e`; `W_-1` is defined for `-1/e <= self < 0`
    /// (with `W_-1(0) = -inf`).
    pub fn checked_lambertw_branch(
        &self,
        branch: LambertBranch,
    ) -> Result<Decimal, ArithmeticError> {
        if self.has_nan_mag() {
            return Err(ArithmeticError::undefined("lambertw"));
        }
        if *self < Decimal::from_finite(-0.3678794411710499) {
            return Err(ArithmeticError::out_of_domain("lambertw"));
        }

        match branch {
            LambertBranch::Principal => {
                if self.is_infinite() {
                    return Ok(Decimal::inf());
                }
                if self.abs() < Decimal::from_finite(1e-300) {
                    return Ok(*self);
                }
                if self.mag < 0.0 || self.layer == 0 {
                    return f_lambertw(self.to_number(), LAMBERTW_TOLERANCE, true)
                        .map(Decimal::from_f64);
                }
                if *self < Decimal::from_components_unchecked(1, 3, 15.0) {
                    return d_lambertw(*self, LAMBERTW_TOLERANCE, true);
                }
                // Numbers this large would sometimes fail to converge; ln() is close enough.
                Ok(self.ln_raw())
            }
            LambertBranch::NonPrincipal => {
                if self.sign == 1 {
                    return Err(ArithmeticError::out_of_domain("lambertw"));
                }
                if self.sign == 0 {
                    return Ok(Decimal::neg_inf());
                }
                if self.layer == 0 {
                    return f_lambertw(self.to_number(), LAMBERTW_TOLERANCE, false)
                        .map(Decimal::from_f64);
                }
                if self.layer == 1 {
                    return d_lambertw(*self, LAMBERTW_TOLERANCE, false);
                }
                let w = (-*self)
                    .recip_raw()
                    .checked_lambertw_branch(LambertBranch::Principal)?;
                Ok(-w)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ArithmeticErrorKind;

    fn close(a: Decimal, b: f64) -> bool {
        a.approx_eq(&Decimal::from_finite(b), 1e-9)
    }

    #[test]
    fn pow_has_no_epsilon_short_circuit() {
        let base = Decimal::from_finite(1.0 + 1e-11);
        let r = base.pow(Decimal::from_finite(1e15));
        // (1 + 1e-11)^1e15 = e^(1e4) ≈ 10^4342.94
        assert!(
            r.log10().approx_eq(&Decimal::from_finite(4342.94), 1e-4),
            "{r}"
        );
        let r2 = Decimal::from_finite(7.0).pow(Decimal::from_finite(1.0 + 1e-11));
        assert!(r2 > Decimal::from_finite(7.0), "{r2}");
    }

    #[test]
    fn pow_layer0_fast_path_is_exact() {
        assert_eq!(Decimal::two().pow(Decimal::from(10)), Decimal::from(1024));
        assert_eq!(Decimal::from(-2).pow(Decimal::from(3)), Decimal::from(-8));
        assert_eq!(Decimal::from(-2).pow(Decimal::from(4)), Decimal::from(16));
        assert_eq!(
            Decimal::from(10).pow(Decimal::from(-1)),
            Decimal::from_finite(0.1)
        );
        assert_eq!(
            Decimal::from_finite(1.15).pow(Decimal::from(100)),
            Decimal::from_finite(1.15_f64.powf(100.0))
        );
    }

    #[test]
    fn pow_negative_base_semantics() {
        assert_eq!(
            Decimal::from(-8)
                .checked_pow(&Decimal::from_finite(1.0 / 3.0))
                .unwrap_err()
                .kind,
            ArithmeticErrorKind::NegativeBase
        );
        assert_eq!(Decimal::from(-8).cbrt(), Decimal::from(-2));
        assert_eq!(Decimal::from(-27).root(Decimal::from(3)), Decimal::from(-3));
        assert_eq!(
            Decimal::from(-16)
                .checked_root(&Decimal::from(4))
                .unwrap_err()
                .kind,
            ArithmeticErrorKind::NegativeBase
        );
        assert_eq!(
            Decimal::zero()
                .checked_pow(&Decimal::neg_one())
                .unwrap_err()
                .kind,
            ArithmeticErrorKind::DivisionByZero
        );
        assert_eq!(Decimal::zero().pow(Decimal::zero()), Decimal::one());
        assert_eq!(Decimal::zero().pow(Decimal::two()), Decimal::zero());
    }

    #[test]
    fn pow_with_infinities() {
        let inf = Decimal::inf();
        assert_eq!(Decimal::two().pow(inf), inf);
        assert_eq!(Decimal::from_finite(0.5).pow(inf), Decimal::zero());
        assert_eq!(Decimal::two().pow(Decimal::neg_inf()), Decimal::zero());
        assert_eq!(inf.pow(Decimal::two()), inf);
        assert_eq!(inf.pow(Decimal::neg_one()), Decimal::zero());
        assert_eq!(Decimal::neg_inf().pow(Decimal::from(3)), Decimal::neg_inf());
        assert_eq!(Decimal::neg_inf().pow(Decimal::from(2)), inf);
        assert_eq!(Decimal::one().pow(inf), Decimal::one());
    }

    #[test]
    fn logs_of_infinity() {
        assert_eq!(Decimal::inf().ln(), Decimal::inf());
        assert_eq!(Decimal::inf().log10(), Decimal::inf());
        assert_eq!(Decimal::inf().log2(), Decimal::inf());
        assert_eq!(Decimal::inf().exp(), Decimal::inf());
        assert_eq!(Decimal::neg_inf().exp(), Decimal::zero());
        assert_eq!(Decimal::inf().sqrt(), Decimal::inf());
        assert_eq!(Decimal::inf().pow10(), Decimal::inf());
        assert_eq!(Decimal::neg_inf().pow10(), Decimal::zero());
        assert!(Decimal::neg_inf().checked_ln().is_err());
        assert_eq!(Decimal::from(8).log(Decimal::two()), Decimal::from(3));
        assert!(Decimal::from(8).checked_log(&Decimal::one()).is_err());
    }

    #[test]
    fn gamma_poles_are_errors() {
        for x in [0.0, -1.0, -2.0, -50.0, -51.0] {
            assert_eq!(
                Decimal::from_finite(x).checked_gamma().unwrap_err().kind,
                ArithmeticErrorKind::OutOfDomain,
                "gamma({x})"
            );
        }
        assert!(Decimal::from(-1).checked_factorial().is_err());
        assert!(Decimal::neg_inf().checked_gamma().is_err());
        assert!(close(Decimal::from_finite(0.5).gamma(), 1.7724538509055159));
        assert!(close(
            Decimal::from_finite(-0.5).gamma(),
            -3.5449077018110318
        ));
        assert!(close(
            Decimal::from_finite(-1.5).gamma(),
            2.3632718012073544
        ));
        assert_eq!(Decimal::from(5).factorial(), Decimal::from(120));
        assert_eq!(
            Decimal::from(20).factorial(),
            Decimal::from(2_432_902_008_176_640_000_u64)
        );
        assert_eq!(Decimal::from(6).gamma(), Decimal::from(120));
        assert!(Decimal::from(170)
            .factorial()
            .approx_eq(&"7.257415615307994e306".parse().unwrap(), 1e-12));
        assert!(Decimal::from(171).factorial() > Decimal::maximum());
        assert_eq!(Decimal::inf().gamma(), Decimal::inf());
    }

    #[test]
    fn lambertw_branches() {
        assert!(close(Decimal::one().lambertw(), OMEGA));
        assert!(close(
            Decimal::from_finite(-0.3).lambertw(),
            -0.4894022271802149
        ));
        assert!(close(
            Decimal::from_finite(-0.3).lambertw_branch(LambertBranch::NonPrincipal),
            -1.781_337_023_421_628
        ));
        assert!(close(
            Decimal::from_finite(-0.1).lambertw_branch(LambertBranch::NonPrincipal),
            -3.577152063957297
        ));
        assert_eq!(
            Decimal::from_finite(-0.5)
                .checked_lambertw()
                .unwrap_err()
                .kind,
            ArithmeticErrorKind::OutOfDomain
        );
        assert_eq!(
            Decimal::one()
                .checked_lambertw_branch(LambertBranch::NonPrincipal)
                .unwrap_err()
                .kind,
            ArithmeticErrorKind::OutOfDomain
        );
        assert_eq!(
            Decimal::zero().lambertw_branch(LambertBranch::NonPrincipal),
            Decimal::neg_inf()
        );
        assert_eq!(Decimal::inf().lambertw(), Decimal::inf());
        let big: Decimal = "1e100".parse().unwrap();
        assert!(close(big.lambertw(), 224.84310644511848));
        // W_-1 on a tiny negative number of layer 1 and layer 2.
        let tiny = Decimal::from_finite(-1e-20);
        let w = tiny.lambertw_branch(LambertBranch::NonPrincipal);
        assert!(close(w, -49.96298427667447), "{w}");
        let tinier: Decimal = "-ee-20".parse().unwrap();
        let w2 = tinier.lambertw_branch(LambertBranch::NonPrincipal);
        assert!(close(w2, -2.3025850929940526e20), "{w2}");
    }

    #[test]
    fn p_log10_never_fails() {
        assert_eq!(Decimal::zero().p_log10(), Decimal::zero());
        assert_eq!(Decimal::from(-5).p_log10(), Decimal::zero());
        assert_eq!(Decimal::from(1000).p_log10(), Decimal::from(3));
    }
}
