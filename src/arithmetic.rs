//! Arithmetic trait implementations for [`Decimal`]: Add, Sub, Mul, Div, Rem, Neg,
//! and their assign variants, plus primitive-type overloads via `impl_ops_primitive!`.
//!
//! # Operator overload panics
//!
//! The `Add`/`Sub`/`Mul`/`Div`/`Rem` operator implementations call the `checked_*` variants
//! internally and panic if the result is undefined (analogous to integer overflow-panic in
//! debug builds). For explicit error handling use the `checked_*` methods directly.

use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

use crate::constants::{COMPARE_EPSILON, MAX_FLOAT_PRECISION};
use crate::decimal::Decimal;
use crate::error::{ArithmeticError, ArithmeticErrorKind};
use crate::utils::sign;

// ---------------------------------------------------------------------------
// checked_* arithmetic methods
// ---------------------------------------------------------------------------

impl Decimal {
    /// Adds two Decimals, returning an error if the result is undefined.
    pub fn checked_add(&self, other: &Self) -> Result<Decimal, ArithmeticError> {
        Ok(add_impl(*self, *other))
    }

    /// Subtracts `other` from `self`, returning an error if the result is undefined.
    pub fn checked_sub(&self, other: &Self) -> Result<Decimal, ArithmeticError> {
        Ok(add_impl(*self, -*other))
    }

    /// Multiplies two Decimals, returning an error if the result is undefined.
    pub fn checked_mul(&self, other: &Self) -> Result<Decimal, ArithmeticError> {
        Ok(mul_impl(*self, *other))
    }

    /// Divides `self` by `other`, returning an error if `other` is zero.
    pub fn checked_div(&self, other: &Self) -> Result<Decimal, ArithmeticError> {
        if other.sign == 0 {
            return Err(ArithmeticError {
                kind: ArithmeticErrorKind::DivisionByZero,
                op: "div",
            });
        }
        Ok(mul_impl(*self, other.recip()))
    }

    /// Computes `self % other`, returning an error if `other` is zero.
    pub fn checked_rem(&self, other: &Self) -> Result<Decimal, ArithmeticError> {
        if *other == Decimal::zero() {
            return Err(ArithmeticError {
                kind: ArithmeticErrorKind::DivisionByZero,
                op: "rem",
            });
        }
        Ok(rem_impl(*self, *other))
    }
}

// ---------------------------------------------------------------------------
// Internal pure-logic helpers (no error plumbing)
// ---------------------------------------------------------------------------

fn add_impl(lhs: Decimal, rhs: Decimal) -> Decimal {
    if !lhs.mag.is_finite() {
        return lhs;
    }

    if lhs.sign == 0 {
        return rhs;
    }
    if rhs.sign == 0 {
        return lhs;
    }

    if lhs.sign == -(rhs.sign)
        && lhs.layer == rhs.layer
        && (lhs.mag - rhs.mag).abs() < COMPARE_EPSILON
    {
        return Decimal::zero();
    }

    let a: Decimal;
    let b: Decimal;

    if lhs.layer >= 2 || rhs.layer >= 2 {
        return lhs.maxabs(rhs);
    }

    if lhs.cmpabs(&rhs).is_gt() {
        a = lhs;
        b = rhs;
    } else {
        a = rhs;
        b = lhs;
    }

    if a.layer == 0 && b.layer == 0 {
        return Decimal::from_finite(a.sign as f64 * a.mag + b.sign as f64 * b.mag);
    }

    let layer_a = a.layer * sign(a.mag) as i64;
    let layer_b = b.layer * sign(b.mag) as i64;

    if layer_a - layer_b >= 2 {
        return a;
    }

    if layer_a == 0 && layer_b == -1 {
        if (b.mag - a.mag.log10()).abs() > MAX_FLOAT_PRECISION as f64 {
            return a;
        }
        let mag_diff = 10.0_f64.powf(a.mag.log10() - b.mag);
        let mantissa = b.sign as f64 + (a.sign as f64 * mag_diff);
        return Decimal::from_components(sign(mantissa), 1, b.mag + mantissa.abs().log10());
    }

    if layer_a == 1 && layer_b == 0 {
        if (a.mag - b.mag.log10()).abs() > MAX_FLOAT_PRECISION as f64 {
            return a;
        }
        let mag_diff = 10.0_f64.powf(a.mag - b.mag.log10());
        let mantissa = b.sign as f64 + (a.sign as f64 * mag_diff);
        return Decimal::from_components(sign(mantissa), 1, b.mag.log10() + mantissa.abs().log10());
    }

    if (a.mag - b.mag).abs() > MAX_FLOAT_PRECISION as f64 {
        return a;
    }

    let mag_diff = 10.0_f64.powf(a.mag - b.mag);
    let mantissa = b.sign as f64 + (a.sign as f64 * mag_diff);
    let new_mag = b.mag + mantissa.abs().log10();
    Decimal::from_components(sign(mantissa), 1, new_mag)
}

fn mul_impl(lhs: Decimal, rhs: Decimal) -> Decimal {
    if lhs.sign == 0 || rhs.sign == 0 {
        return Decimal::zero();
    }

    if lhs.layer == rhs.layer && (lhs.mag - -rhs.mag).abs() < COMPARE_EPSILON {
        return Decimal::from_components_unchecked(lhs.sign * rhs.sign, 0, 1.0);
    }

    let a: Decimal;
    let b: Decimal;

    if (lhs.layer > rhs.layer) || (lhs.layer == rhs.layer && lhs.mag.abs() > rhs.mag.abs()) {
        a = lhs;
        b = rhs;
    } else {
        a = rhs;
        b = lhs;
    }

    if a.layer == 0 && b.layer == 0 {
        return Decimal::from_finite(a.sign as f64 * b.sign as f64 * a.mag * b.mag);
    }

    if a.layer >= 3 || (a.layer - b.layer >= 2) {
        return Decimal::from_components(a.sign * b.sign, a.layer, a.mag);
    }

    if a.layer == 1 && b.layer == 0 {
        return Decimal::from_components(a.sign * b.sign, 1, a.mag + b.mag.log10());
    }

    if a.layer == 1 && b.layer == 1 {
        return Decimal::from_components(a.sign * b.sign, 1, a.mag + b.mag);
    }

    if a.layer == 2 && b.layer == 1 {
        let new_mag = Decimal::from_components(sign(a.mag), a.layer - 1, a.mag.abs())
            + Decimal::from_components(sign(b.mag), b.layer - 1, b.mag.abs());
        return Decimal::from_components(
            a.sign * b.sign,
            new_mag.layer + 1,
            new_mag.sign as f64 * new_mag.mag,
        );
    }

    if a.layer == 2 && b.layer == 2 {
        let new_mag = Decimal::from_components(sign(a.mag), a.layer - 1, a.mag.abs())
            + Decimal::from_components(sign(b.mag), b.layer - 1, b.mag.abs());
        return Decimal::from_components(
            a.sign * b.sign,
            new_mag.layer + 1,
            new_mag.sign as f64 * new_mag.mag,
        );
    }

    Decimal::inf()
}

fn rem_impl(lhs: Decimal, rhs: Decimal) -> Decimal {
    if rhs == Decimal::zero() {
        return Decimal::zero();
    }

    if lhs.sign * rhs.sign == -1 {
        return lhs.abs().rem(rhs.abs()).neg();
    }

    if lhs.sign == -1 {
        return lhs.abs().rem(rhs.abs());
    }

    lhs - (lhs / rhs).floor() * rhs
}

// ---------------------------------------------------------------------------
// Decimal op Decimal operator impls
// ---------------------------------------------------------------------------

impl Add<Decimal> for Decimal {
    type Output = Decimal;

    fn add(self, rhs: Decimal) -> Self::Output {
        self.checked_add(&rhs).unwrap_or_else(|e| {
            panic!("undefined Decimal addition: {e} (lhs={self:?}, rhs={rhs:?})")
        })
    }
}

impl Sub<Decimal> for Decimal {
    type Output = Decimal;

    fn sub(self, rhs: Decimal) -> Self::Output {
        self.checked_sub(&rhs).unwrap_or_else(|e| {
            panic!("undefined Decimal subtraction: {e} (lhs={self:?}, rhs={rhs:?})")
        })
    }
}

impl Mul<Decimal> for Decimal {
    type Output = Decimal;

    fn mul(self, rhs: Decimal) -> Self::Output {
        self.checked_mul(&rhs).unwrap_or_else(|e| {
            panic!("undefined Decimal multiplication: {e} (lhs={self:?}, rhs={rhs:?})")
        })
    }
}

impl Div<Decimal> for Decimal {
    type Output = Decimal;

    /// Division of two Decimals by multiplying the dividend by the reciprocal of the divisor.
    ///
    /// # Panics
    ///
    /// Panics if `rhs` is zero.
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Decimal) -> Self::Output {
        self.checked_div(&rhs).unwrap_or_else(|e| {
            panic!("undefined Decimal division: {e} (lhs={self:?}, rhs={rhs:?})")
        })
    }
}

impl Rem<Decimal> for Decimal {
    type Output = Decimal;

    fn rem(self, rhs: Decimal) -> Self::Output {
        self.checked_rem(&rhs).unwrap_or_else(|e| {
            panic!("undefined Decimal remainder: {e} (lhs={self:?}, rhs={rhs:?})")
        })
    }
}

impl Neg for Decimal {
    type Output = Decimal;

    fn neg(self) -> Decimal {
        Decimal::from_components_unchecked(-self.sign, self.layer, self.mag)
    }
}

// ---------------------------------------------------------------------------
// Reference combinations for Decimal op &Decimal and &Decimal op Decimal
// ---------------------------------------------------------------------------

impl Add<&Decimal> for Decimal {
    type Output = Decimal;
    fn add(self, rhs: &Decimal) -> Decimal {
        self + *rhs
    }
}

impl Add<Decimal> for &Decimal {
    type Output = Decimal;
    fn add(self, rhs: Decimal) -> Decimal {
        *self + rhs
    }
}

impl Add<&Decimal> for &Decimal {
    type Output = Decimal;
    fn add(self, rhs: &Decimal) -> Decimal {
        *self + *rhs
    }
}

impl Sub<&Decimal> for Decimal {
    type Output = Decimal;
    fn sub(self, rhs: &Decimal) -> Decimal {
        self - *rhs
    }
}

impl Sub<Decimal> for &Decimal {
    type Output = Decimal;
    fn sub(self, rhs: Decimal) -> Decimal {
        *self - rhs
    }
}

impl Sub<&Decimal> for &Decimal {
    type Output = Decimal;
    fn sub(self, rhs: &Decimal) -> Decimal {
        *self - *rhs
    }
}

impl Mul<&Decimal> for Decimal {
    type Output = Decimal;
    fn mul(self, rhs: &Decimal) -> Decimal {
        self * *rhs
    }
}

impl Mul<Decimal> for &Decimal {
    type Output = Decimal;
    fn mul(self, rhs: Decimal) -> Decimal {
        *self * rhs
    }
}

impl Mul<&Decimal> for &Decimal {
    type Output = Decimal;
    fn mul(self, rhs: &Decimal) -> Decimal {
        *self * *rhs
    }
}

impl Div<&Decimal> for Decimal {
    type Output = Decimal;
    fn div(self, rhs: &Decimal) -> Decimal {
        self / *rhs
    }
}

impl Div<Decimal> for &Decimal {
    type Output = Decimal;
    fn div(self, rhs: Decimal) -> Decimal {
        *self / rhs
    }
}

impl Div<&Decimal> for &Decimal {
    type Output = Decimal;
    fn div(self, rhs: &Decimal) -> Decimal {
        *self / *rhs
    }
}

impl Rem<&Decimal> for Decimal {
    type Output = Decimal;
    fn rem(self, rhs: &Decimal) -> Decimal {
        self % *rhs
    }
}

impl Rem<Decimal> for &Decimal {
    type Output = Decimal;
    fn rem(self, rhs: Decimal) -> Decimal {
        *self % rhs
    }
}

impl Rem<&Decimal> for &Decimal {
    type Output = Decimal;
    fn rem(self, rhs: &Decimal) -> Decimal {
        *self % *rhs
    }
}

// ---------------------------------------------------------------------------
// Assign variants
// ---------------------------------------------------------------------------

impl AddAssign<Decimal> for Decimal {
    fn add_assign(&mut self, rhs: Decimal) {
        *self = *self + rhs;
    }
}

impl SubAssign<Decimal> for Decimal {
    fn sub_assign(&mut self, rhs: Decimal) {
        *self = *self - rhs;
    }
}

impl MulAssign<Decimal> for Decimal {
    fn mul_assign(&mut self, rhs: Decimal) {
        *self = *self * rhs;
    }
}

impl DivAssign<Decimal> for Decimal {
    fn div_assign(&mut self, rhs: Decimal) {
        *self = *self / rhs;
    }
}

impl RemAssign<Decimal> for Decimal {
    fn rem_assign(&mut self, rhs: Decimal) {
        *self = *self % rhs;
    }
}

// ---------------------------------------------------------------------------
// Primitive-type conversions and overloads
// ---------------------------------------------------------------------------

/// Implements `From<$prim_type> for Decimal` for integer types (always finite).
macro_rules! impl_from_integer {
    ($prim_type:ty) => {
        impl From<$prim_type> for Decimal {
            fn from(prim: $prim_type) -> Self {
                Decimal::from_finite(prim as f64)
            }
        }
    };
}

/// Implements arithmetic operators between `Decimal` and a primitive type.
macro_rules! impl_ops_primitive {
    ($prim_type:ty, $convert:expr) => {
        impl Add<$prim_type> for Decimal {
            type Output = Decimal;
            fn add(self, rhs: $prim_type) -> Self::Output {
                self + $convert(rhs)
            }
        }
        impl Add<Decimal> for $prim_type {
            type Output = Decimal;
            fn add(self, rhs: Decimal) -> Self::Output {
                $convert(self) + rhs
            }
        }
        impl Sub<$prim_type> for Decimal {
            type Output = Decimal;
            fn sub(self, rhs: $prim_type) -> Self::Output {
                self - $convert(rhs)
            }
        }
        impl Sub<Decimal> for $prim_type {
            type Output = Decimal;
            fn sub(self, rhs: Decimal) -> Self::Output {
                $convert(self) - rhs
            }
        }
        impl Mul<$prim_type> for Decimal {
            type Output = Decimal;
            fn mul(self, rhs: $prim_type) -> Self::Output {
                self * $convert(rhs)
            }
        }
        impl Mul<Decimal> for $prim_type {
            type Output = Decimal;
            fn mul(self, rhs: Decimal) -> Self::Output {
                $convert(self) * rhs
            }
        }
        impl Div<$prim_type> for Decimal {
            type Output = Decimal;
            fn div(self, rhs: $prim_type) -> Self::Output {
                self / $convert(rhs)
            }
        }
        impl Div<Decimal> for $prim_type {
            type Output = Decimal;
            fn div(self, rhs: Decimal) -> Self::Output {
                $convert(self) / rhs
            }
        }
        impl Rem<$prim_type> for Decimal {
            type Output = Decimal;
            fn rem(self, rhs: $prim_type) -> Self::Output {
                self % $convert(rhs)
            }
        }
        impl Rem<Decimal> for $prim_type {
            type Output = Decimal;
            fn rem(self, rhs: Decimal) -> Self::Output {
                $convert(self) % rhs
            }
        }
        impl AddAssign<$prim_type> for Decimal {
            fn add_assign(&mut self, rhs: $prim_type) {
                *self = *self + rhs;
            }
        }
        impl SubAssign<$prim_type> for Decimal {
            fn sub_assign(&mut self, rhs: $prim_type) {
                *self = *self - rhs;
            }
        }
        impl MulAssign<$prim_type> for Decimal {
            fn mul_assign(&mut self, rhs: $prim_type) {
                *self = *self * rhs;
            }
        }
        impl DivAssign<$prim_type> for Decimal {
            fn div_assign(&mut self, rhs: $prim_type) {
                *self = *self / rhs;
            }
        }
        impl RemAssign<$prim_type> for Decimal {
            fn rem_assign(&mut self, rhs: $prim_type) {
                *self = *self % rhs;
            }
        }
    };
}

// Integer types: always finite, so From is infallible.
impl_from_integer!(i8);
impl_from_integer!(i16);
impl_from_integer!(i32);
impl_from_integer!(i64);
impl_from_integer!(u8);
impl_from_integer!(u16);
impl_from_integer!(u32);
impl_from_integer!(u64);
// Note: f32/f64 do NOT get From; only TryFrom is provided (in decimal.rs).
// The impl_ops_primitive! for floats uses a helper that calls from_finite.

impl_ops_primitive!(i8, Decimal::from);
impl_ops_primitive!(i16, Decimal::from);
impl_ops_primitive!(i32, Decimal::from);
impl_ops_primitive!(i64, Decimal::from);
impl_ops_primitive!(u8, Decimal::from);
impl_ops_primitive!(u16, Decimal::from);
impl_ops_primitive!(u32, Decimal::from);
impl_ops_primitive!(u64, Decimal::from);
impl_ops_primitive!(f32, |x: f32| Decimal::from_finite(x as f64));
impl_ops_primitive!(f64, Decimal::from_finite);
