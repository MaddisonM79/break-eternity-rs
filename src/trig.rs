//! Trigonometric and hyperbolic functions.
//!
//! # Precision note
//!
//! For inputs at layer 1 or above, the angle modulo `2π` cannot be resolved within `f64`
//! precision (the magnitude exceeds `2π · 2^53`), so `sin`, `cos`, and `tan` return `0` there
//! rather than a pseudo-random phase. This matches `break_eternity.js`. Callers who need
//! trigonometry at huge arguments must reduce the argument by hand using a separately-tracked
//! modulus. Infinite inputs are treated the same way.
//!
//! Inputs below layer 0 (i.e. smaller than `1/9e15`) are so close to zero that `sin`, `tan`,
//! `asin`, and `atan` return the input unchanged and `cos` returns `1`.

use crate::decimal::Decimal;
use crate::error::ArithmeticError;

impl Decimal {
    /// Returns the sine of the Decimal. Returns `0` at layer ≥ 1; see the
    /// precision note in the crate documentation.
    pub fn sin(&self) -> Decimal {
        if self.mag < 0.0 {
            return *self;
        }
        if self.layer == 0 {
            return Decimal::from_f64((self.sign as f64 * self.mag).sin());
        }
        Decimal::zero()
    }

    /// Returns the cosine of the Decimal. Returns `0` at layer ≥ 1; see the
    /// precision note in the crate documentation.
    pub fn cos(&self) -> Decimal {
        if self.mag < 0.0 {
            return Decimal::one();
        }
        if self.layer == 0 {
            return Decimal::from_f64((self.sign as f64 * self.mag).cos());
        }
        Decimal::zero()
    }

    /// Returns the tangent of the Decimal. Returns `0` at layer ≥ 1; see the
    /// precision note in the crate documentation.
    pub fn tan(&self) -> Decimal {
        if self.mag < 0.0 {
            return *self;
        }
        if self.layer == 0 {
            return Decimal::from_f64((self.sign as f64 * self.mag).tan());
        }
        Decimal::zero()
    }

    /// Returns the arcsine of the Decimal.
    ///
    /// # Panics
    ///
    /// Panics if `|self| > 1`. Use [`checked_asin`](Self::checked_asin) for explicit error
    /// handling.
    pub fn asin(&self) -> Decimal {
        self.checked_asin()
            .unwrap_or_else(|e| panic!("undefined Decimal asin: {e} (self={self:?})"))
    }

    /// Returns the arcsine of the Decimal, or an error if `|self| > 1`.
    pub fn checked_asin(&self) -> Result<Decimal, ArithmeticError> {
        if self.has_nan_mag() {
            return Err(ArithmeticError::undefined("asin"));
        }
        if self.mag < 0.0 {
            return Ok(*self);
        }
        if self.layer == 0 && self.mag <= 1.0 {
            return Ok(Decimal::from_f64((self.sign as f64 * self.mag).asin()));
        }
        Err(ArithmeticError::out_of_domain("asin"))
    }

    /// Returns the arccosine of the Decimal.
    ///
    /// # Panics
    ///
    /// Panics if `|self| > 1`. Use [`checked_acos`](Self::checked_acos) for explicit error
    /// handling.
    pub fn acos(&self) -> Decimal {
        self.checked_acos()
            .unwrap_or_else(|e| panic!("undefined Decimal acos: {e} (self={self:?})"))
    }

    /// Returns the arccosine of the Decimal, or an error if `|self| > 1`.
    pub fn checked_acos(&self) -> Result<Decimal, ArithmeticError> {
        if self.has_nan_mag() {
            return Err(ArithmeticError::undefined("acos"));
        }
        if self.mag < 0.0 {
            return Ok(Decimal::from_f64(self.to_number().acos()));
        }
        if self.layer == 0 && self.mag <= 1.0 {
            return Ok(Decimal::from_f64((self.sign as f64 * self.mag).acos()));
        }
        Err(ArithmeticError::out_of_domain("acos"))
    }

    /// Returns the arctangent of the Decimal. Values beyond `f64` range saturate to `±π/2`.
    pub fn atan(&self) -> Decimal {
        if self.mag < 0.0 {
            return *self;
        }
        if self.layer == 0 {
            return Decimal::from_f64((self.sign as f64 * self.mag).atan());
        }
        Decimal::from_f64((self.sign as f64 * f64::MAX).atan())
    }

    /// Returns the hyperbolic sine of the Decimal.
    pub fn sinh(&self) -> Decimal {
        self.exp().sub_raw((-*self).exp()).div_raw(Decimal::two())
    }

    /// Returns the hyperbolic cosine of the Decimal.
    pub fn cosh(&self) -> Decimal {
        self.exp().add_raw((-*self).exp()).div_raw(Decimal::two())
    }

    /// Returns the hyperbolic tangent of the Decimal.
    pub fn tanh(&self) -> Decimal {
        if self.is_infinite()
            || (self.layer == 0 && self.mag > 20.0)
            || self.layer > 0 && self.mag > 0.0
        {
            // Beyond |x| ≈ 20, tanh(x) is ±1 to full f64 precision.
            return Decimal::from_components_unchecked(self.sign, 0, 1.0);
        }
        self.sinh().div_raw(self.cosh())
    }

    /// Returns the inverse hyperbolic sine of the Decimal.
    pub fn asinh(&self) -> Decimal {
        if self.sign == 0 {
            return Decimal::zero();
        }
        // asinh is odd; evaluate on |x| to avoid cancellation for negative inputs.
        let x = self.abs();
        let r = x
            .add_raw(x.sqr().add_raw(Decimal::one()).sqrt_raw())
            .ln_raw();
        if self.sign < 0 {
            -r
        } else {
            r
        }
    }

    /// Returns the inverse hyperbolic cosine of the Decimal.
    ///
    /// # Panics
    ///
    /// Panics if `self < 1`. Use [`checked_acosh`](Self::checked_acosh) for explicit error
    /// handling.
    pub fn acosh(&self) -> Decimal {
        self.checked_acosh()
            .unwrap_or_else(|e| panic!("undefined Decimal acosh: {e} (self={self:?})"))
    }

    /// Returns the inverse hyperbolic cosine of the Decimal, or an error if `self < 1`.
    pub fn checked_acosh(&self) -> Result<Decimal, ArithmeticError> {
        if self.has_nan_mag() {
            return Err(ArithmeticError::undefined("acosh"));
        }
        if *self < Decimal::one() {
            return Err(ArithmeticError::out_of_domain("acosh"));
        }
        self.add_raw(self.sqr().sub_raw(Decimal::one()).sqrt_raw())
            .ln_raw()
            .nan_to_err("acosh")
    }

    /// Returns the inverse hyperbolic tangent of the Decimal.
    ///
    /// # Panics
    ///
    /// Panics if `|self| >= 1`. Use [`checked_atanh`](Self::checked_atanh) for explicit error
    /// handling.
    pub fn atanh(&self) -> Decimal {
        self.checked_atanh()
            .unwrap_or_else(|e| panic!("undefined Decimal atanh: {e} (self={self:?})"))
    }

    /// Returns the inverse hyperbolic tangent of the Decimal, or an error if `|self| >= 1`.
    pub fn checked_atanh(&self) -> Result<Decimal, ArithmeticError> {
        if self.has_nan_mag() {
            return Err(ArithmeticError::undefined("atanh"));
        }
        if self.abs() >= Decimal::one() {
            return Err(ArithmeticError::out_of_domain("atanh"));
        }
        let one = Decimal::one();
        // atanh(x) = ln((1+x)/(1-x)) / 2
        self.add_raw(one)
            .div_raw(one.sub_raw(*self))
            .ln_raw()
            .div_raw(Decimal::two())
            .nan_to_err("atanh")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Decimal, b: f64) -> bool {
        a.approx_eq(&Decimal::from_finite(b), 1e-9)
    }

    #[test]
    fn layer0_matches_f64() {
        let x = 0.7_f64;
        let d = Decimal::from_finite(x);
        assert!(close(d.sin(), x.sin()));
        assert!(close(d.cos(), x.cos()));
        assert!(close(d.tan(), x.tan()));
        assert!(close(d.asin(), x.asin()));
        assert!(close(d.acos(), x.acos()));
        assert!(close(d.atan(), x.atan()));
        assert!(close(d.sinh(), x.sinh()));
        assert!(close(d.cosh(), x.cosh()));
        assert!(close(d.tanh(), x.tanh()));
        assert!(close(d.asinh(), x.asinh()));
        assert!(close(
            Decimal::from_finite(-3.0).asinh(),
            (-3.0_f64).asinh()
        ));
        assert!(close(
            Decimal::from_finite(-1e10).asinh(),
            (-1e10_f64).asinh()
        ));
        assert!(close(Decimal::from_finite(2.0).acosh(), 2.0_f64.acosh()));
        assert!(close(d.atanh(), x.atanh()));
    }

    #[test]
    fn domain_errors() {
        assert!(Decimal::two().checked_asin().is_err());
        assert!(Decimal::two().checked_acos().is_err());
        assert!(Decimal::from_finite(0.5).checked_acosh().is_err());
        assert!(Decimal::one().checked_atanh().is_err());
        assert!(Decimal::inf().checked_asin().is_err());
    }

    #[test]
    fn huge_and_infinite_inputs() {
        let huge: Decimal = "1e1000".parse().unwrap();
        assert_eq!(huge.sin(), Decimal::zero());
        assert_eq!(huge.cos(), Decimal::zero());
        assert_eq!(huge.tanh(), Decimal::one());
        assert_eq!((-huge).tanh(), Decimal::neg_one());
        assert!(close(huge.atan(), std::f64::consts::FRAC_PI_2));
        assert_eq!(Decimal::inf().tanh(), Decimal::one());
        assert_eq!(Decimal::inf().sinh(), Decimal::inf());
        assert_eq!(Decimal::neg_inf().sinh(), Decimal::neg_inf());
        assert_eq!(Decimal::inf().cosh(), Decimal::inf());
        assert_eq!(Decimal::inf().asinh(), Decimal::inf());
        assert_eq!(Decimal::inf().acosh(), Decimal::inf());
        assert!(close(Decimal::from_finite(400.0).tanh(), 1.0));
    }
}
