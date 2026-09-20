//! Arithmetic for [`Decimal`]: `checked_*` methods, operator trait implementations
//! (`Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg`, their assign variants, `Sum`, `Product`),
//! and primitive-type interop.
//!
//! # Operator overload panics
//!
//! The operator implementations call the `checked_*` variants internally and panic if the
//! result is undefined (analogous to integer overflow-panic in debug builds). For explicit
//! error handling use the `checked_*` methods directly.
//!
//! # Infinity
//!
//! `inf + -inf`, `inf - inf`, `inf * 0`, `inf / inf`, and any remainder involving an infinity
//! are [`Undefined`](crate::ArithmeticErrorKind::Undefined). Every other combination follows the usual
//! extended-real rules.

use core::iter::{Product, Sum};
use core::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

use crate::constants::MAX_FLOAT_PRECISION;
use crate::decimal::Decimal;
use crate::error::ArithmeticError;
#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
// shadowed by std's inherent methods whenever std is in the crate graph
use crate::math::FloatExt;
use crate::utils::sign;

// ---------------------------------------------------------------------------
// checked_* arithmetic methods
// ---------------------------------------------------------------------------

impl Decimal {
    /// Adds two Decimals, returning an error if the result is undefined (`inf + -inf`).
    pub fn checked_add(&self, other: &Self) -> Result<Decimal, ArithmeticError> {
        self.add_raw(*other).nan_to_err("add")
    }

    /// Subtracts `other` from `self`, returning an error if the result is undefined (`inf - inf`).
    pub fn checked_sub(&self, other: &Self) -> Result<Decimal, ArithmeticError> {
        self.add_raw(-*other).nan_to_err("sub")
    }

    /// Multiplies two Decimals, returning an error if the result is undefined (`inf * 0`).
    pub fn checked_mul(&self, other: &Self) -> Result<Decimal, ArithmeticError> {
        self.mul_raw(*other).nan_to_err("mul")
    }

    /// Divides `self` by `other`.
    ///
    /// Returns [`DivisionByZero`](crate::ArithmeticErrorKind::DivisionByZero) if `other` is zero and
    /// [`Undefined`](crate::ArithmeticErrorKind::Undefined) for `inf / inf`.
    pub fn checked_div(&self, other: &Self) -> Result<Decimal, ArithmeticError> {
        if other.sign == 0 {
            return Err(ArithmeticError::division_by_zero("div"));
        }
        self.div_raw(*other).nan_to_err("div")
    }

    /// Returns `1 / self`.
    ///
    /// Returns [`DivisionByZero`](crate::ArithmeticErrorKind::DivisionByZero) for zero. The reciprocal
    /// of an infinity is zero.
    pub fn checked_recip(&self) -> Result<Decimal, ArithmeticError> {
        if self.sign == 0 {
            return Err(ArithmeticError::division_by_zero("recip"));
        }
        self.recip_raw().nan_to_err("recip")
    }

    /// Returns the reciprocal of the Decimal.
    ///
    /// # Panics
    ///
    /// Panics if the value is zero. Use [`checked_recip`](Self::checked_recip) for explicit
    /// error handling.
    pub fn recip(&self) -> Decimal {
        self.checked_recip()
            .unwrap_or_else(|e| panic!("undefined Decimal reciprocal: {e} (self={self:?})"))
    }

    /// Computes the truncated remainder `self % other` (the sign follows `self`, like the `%`
    /// operator on primitives).
    ///
    /// Returns [`DivisionByZero`](crate::ArithmeticErrorKind::DivisionByZero) if `other` is zero and
    /// [`Undefined`](crate::ArithmeticErrorKind::Undefined) if either operand is infinite.
    pub fn checked_rem(&self, other: &Self) -> Result<Decimal, ArithmeticError> {
        if other.sign == 0 {
            return Err(ArithmeticError::division_by_zero("rem"));
        }
        if self.is_infinite() || other.is_infinite() {
            return Err(ArithmeticError::undefined("rem"));
        }
        rem_raw(*self, *other, false).nan_to_err("rem")
    }

    /// Computes the floored remainder of `self / other` (the sign follows `other`, as in
    /// number theory and Python's `%`).
    ///
    /// Returns [`DivisionByZero`](crate::ArithmeticErrorKind::DivisionByZero) if `other` is zero and
    /// [`Undefined`](crate::ArithmeticErrorKind::Undefined) if either operand is infinite.
    pub fn checked_rem_floored(&self, other: &Self) -> Result<Decimal, ArithmeticError> {
        if other.sign == 0 {
            return Err(ArithmeticError::division_by_zero("rem_floored"));
        }
        if self.is_infinite() || other.is_infinite() {
            return Err(ArithmeticError::undefined("rem_floored"));
        }
        rem_raw(*self, *other, true).nan_to_err("rem_floored")
    }

    /// Computes the floored remainder of `self / other`. See
    /// [`checked_rem_floored`](Self::checked_rem_floored).
    ///
    /// # Panics
    ///
    /// Panics if `other` is zero or either operand is infinite.
    pub fn rem_floored(&self, other: &Self) -> Decimal {
        self.checked_rem_floored(other).unwrap_or_else(|e| {
            panic!("undefined Decimal floored remainder: {e} (lhs={self:?}, rhs={other:?})")
        })
    }

    // -----------------------------------------------------------------------
    // Raw (JS-semantics) kernels. These never panic and never return an error;
    // undefined results come back as the internal NaN sentinel.
    // -----------------------------------------------------------------------

    pub(crate) fn add_raw(self, rhs: Decimal) -> Decimal {
        add_raw(self, rhs)
    }

    pub(crate) fn sub_raw(self, rhs: Decimal) -> Decimal {
        add_raw(self, -rhs)
    }

    pub(crate) fn mul_raw(self, rhs: Decimal) -> Decimal {
        mul_raw(self, rhs)
    }

    pub(crate) fn div_raw(self, rhs: Decimal) -> Decimal {
        if self.has_nan_mag() || rhs.has_nan_mag() {
            return Decimal::nan_sentinel();
        }
        if self.is_infinite() && rhs.is_infinite() {
            return Decimal::nan_sentinel();
        }
        mul_raw(self, rhs.recip_raw())
    }

    pub(crate) fn recip_raw(self) -> Decimal {
        if self.has_nan_mag() || self.sign == 0 {
            return Decimal::nan_sentinel();
        }
        if self.is_infinite() {
            return Decimal::zero();
        }
        if self.layer == 0 {
            return Decimal::from_components(self.sign, 0, 1.0 / self.mag);
        }
        Decimal::from_components(self.sign, self.layer, -self.mag)
    }
}

// ---------------------------------------------------------------------------
// Internal pure-logic helpers
// ---------------------------------------------------------------------------

fn add_raw(lhs: Decimal, rhs: Decimal) -> Decimal {
    if lhs.has_nan_mag() || rhs.has_nan_mag() {
        return Decimal::nan_sentinel();
    }

    // Infinity + -Infinity = NaN; otherwise an infinite operand dominates.
    if lhs.is_infinite() {
        if rhs.is_infinite() && rhs.sign != lhs.sign {
            return Decimal::nan_sentinel();
        }
        return lhs;
    }
    if rhs.is_infinite() {
        return rhs;
    }

    if lhs.sign == 0 {
        return rhs;
    }
    if rhs.sign == 0 {
        return lhs;
    }

    // Adding a number to its exact negation produces 0, no matter how large.
    if lhs.sign == -rhs.sign && lhs.layer == rhs.layer && lhs.mag == rhs.mag {
        return Decimal::zero();
    }

    // If one of the numbers is layer 2 or higher, just take the bigger number.
    if lhs.layer >= 2 || rhs.layer >= 2 {
        return lhs.maxabs(rhs);
    }

    let (a, b) = if lhs.cmpabs(&rhs).is_gt() {
        (lhs, rhs)
    } else {
        (rhs, lhs)
    };

    if a.layer == 0 && b.layer == 0 {
        return Decimal::from_f64(a.sign as f64 * a.mag + b.sign as f64 * b.mag);
    }

    let layer_a = a.layer * i64::from(sign(a.mag));
    let layer_b = b.layer * i64::from(sign(b.mag));

    // If one of the numbers is 2+ layers higher than the other, just take the bigger number.
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
    Decimal::from_components(sign(mantissa), 1, b.mag + mantissa.abs().log10())
}

fn mul_raw(lhs: Decimal, rhs: Decimal) -> Decimal {
    if lhs.has_nan_mag() || rhs.has_nan_mag() {
        return Decimal::nan_sentinel();
    }

    // Infinity * 0 = NaN; otherwise an infinite operand dominates with the product sign.
    if lhs.is_infinite() || rhs.is_infinite() {
        if lhs.sign == 0 || rhs.sign == 0 {
            return Decimal::nan_sentinel();
        }
        return if lhs.sign * rhs.sign > 0 {
            Decimal::inf()
        } else {
            Decimal::neg_inf()
        };
    }

    if lhs.sign == 0 || rhs.sign == 0 {
        return Decimal::zero();
    }

    // Multiplying a number by its exact reciprocal yields ±1, no matter how large.
    if lhs.layer == rhs.layer && lhs.mag == -rhs.mag {
        return Decimal::from_components_unchecked(lhs.sign * rhs.sign, 0, 1.0);
    }

    // Which number is bigger in terms of its multiplicative distance from 1?
    let (a, b) =
        if lhs.layer > rhs.layer || (lhs.layer == rhs.layer && lhs.mag.abs() > rhs.mag.abs()) {
            (lhs, rhs)
        } else {
            (rhs, lhs)
        };

    if a.layer == 0 && b.layer == 0 {
        return Decimal::from_f64(a.sign as f64 * b.sign as f64 * a.mag * b.mag);
    }

    // If one of the numbers is layer 3 or higher, or 2+ layers bigger than the other,
    // just take the bigger number.
    if a.layer >= 3 || (a.layer - b.layer >= 2) {
        return Decimal::from_components(a.sign * b.sign, a.layer, a.mag);
    }

    if a.layer == 1 && b.layer == 0 {
        return Decimal::from_components(a.sign * b.sign, 1, a.mag + b.mag.log10());
    }

    if a.layer == 1 && b.layer == 1 {
        return Decimal::from_components(a.sign * b.sign, 1, a.mag + b.mag);
    }

    // a.layer == 2 && (b.layer == 1 || b.layer == 2)
    let new_mag = add_raw(
        Decimal::from_components(sign(a.mag), a.layer - 1, a.mag.abs()),
        Decimal::from_components(sign(b.mag), b.layer - 1, b.mag.abs()),
    );
    Decimal::from_components(
        a.sign * b.sign,
        new_mag.layer.saturating_add(1),
        new_mag.sign as f64 * new_mag.mag,
    )
}

/// Remainder with JS `mod` semantics: truncated by default, floored when `floored` is set.
/// Adapted from OmegaNum.js.
fn rem_raw(lhs: Decimal, rhs: Decimal, floored: bool) -> Decimal {
    if lhs.has_nan_mag() || rhs.has_nan_mag() {
        return Decimal::nan_sentinel();
    }
    if lhs.is_infinite() || rhs.is_infinite() {
        return Decimal::nan_sentinel();
    }

    let abs_rhs = rhs.abs();
    if lhs.sign == 0 || rhs.sign == 0 {
        return Decimal::zero();
    }

    if floored {
        let mut abs_mod = rem_raw(lhs.abs(), abs_rhs, false);
        if (lhs.sign == -1) != (rhs.sign == -1) && abs_mod.sign != 0 {
            abs_mod = abs_rhs.sub_raw(abs_mod);
        }
        return abs_mod.mul_raw(Decimal::from_components_unchecked(rhs.sign, 0, 1.0));
    }

    // To avoid precision issues, if both numbers fit in an f64, use the native operator.
    let num_lhs = lhs.to_number();
    let num_rhs = abs_rhs.to_number();
    if num_lhs.is_finite() && num_rhs.is_finite() && num_lhs != 0.0 && num_rhs != 0.0 {
        return Decimal::from_f64(num_lhs % num_rhs);
    }

    if lhs.sub_raw(abs_rhs) == lhs {
        // rhs is too small to register against lhs.
        return Decimal::zero();
    }
    if abs_rhs.sub_raw(lhs) == abs_rhs {
        // lhs is too small to register against rhs.
        return lhs;
    }
    if lhs.sign == -1 {
        return -rem_raw(lhs.abs(), abs_rhs, false);
    }

    lhs.sub_raw(lhs.div_raw(abs_rhs).floor().mul_raw(abs_rhs))
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

    /// Division of two Decimals.
    ///
    /// # Panics
    ///
    /// Panics if `rhs` is zero or the quotient is undefined (`inf / inf`).
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

impl Neg for &Decimal {
    type Output = Decimal;

    fn neg(self) -> Decimal {
        -*self
    }
}

// ---------------------------------------------------------------------------
// Reference combinations for Decimal op &Decimal and &Decimal op Decimal
// ---------------------------------------------------------------------------

macro_rules! impl_ref_ops {
    ($trait:ident, $method:ident) => {
        impl $trait<&Decimal> for Decimal {
            type Output = Decimal;
            fn $method(self, rhs: &Decimal) -> Decimal {
                $trait::$method(self, *rhs)
            }
        }

        impl $trait<Decimal> for &Decimal {
            type Output = Decimal;
            fn $method(self, rhs: Decimal) -> Decimal {
                $trait::$method(*self, rhs)
            }
        }

        impl $trait<&Decimal> for &Decimal {
            type Output = Decimal;
            fn $method(self, rhs: &Decimal) -> Decimal {
                $trait::$method(*self, *rhs)
            }
        }
    };
}

impl_ref_ops!(Add, add);
impl_ref_ops!(Sub, sub);
impl_ref_ops!(Mul, mul);
impl_ref_ops!(Div, div);
impl_ref_ops!(Rem, rem);

// ---------------------------------------------------------------------------
// Assign variants
// ---------------------------------------------------------------------------

macro_rules! impl_assign_ops {
    ($trait:ident, $method:ident, $op:tt) => {
        impl $trait<Decimal> for Decimal {
            fn $method(&mut self, rhs: Decimal) {
                *self = *self $op rhs;
            }
        }

        impl $trait<&Decimal> for Decimal {
            fn $method(&mut self, rhs: &Decimal) {
                *self = *self $op *rhs;
            }
        }
    };
}

impl_assign_ops!(AddAssign, add_assign, +);
impl_assign_ops!(SubAssign, sub_assign, -);
impl_assign_ops!(MulAssign, mul_assign, *);
impl_assign_ops!(DivAssign, div_assign, /);
impl_assign_ops!(RemAssign, rem_assign, %);

// ---------------------------------------------------------------------------
// Sum / Product
// ---------------------------------------------------------------------------

impl Sum for Decimal {
    fn sum<I: Iterator<Item = Decimal>>(iter: I) -> Decimal {
        iter.fold(Decimal::zero(), |acc, x| acc + x)
    }
}

impl<'a> Sum<&'a Decimal> for Decimal {
    fn sum<I: Iterator<Item = &'a Decimal>>(iter: I) -> Decimal {
        iter.fold(Decimal::zero(), |acc, x| acc + *x)
    }
}

impl Product for Decimal {
    fn product<I: Iterator<Item = Decimal>>(iter: I) -> Decimal {
        iter.fold(Decimal::one(), |acc, x| acc * x)
    }
}

impl<'a> Product<&'a Decimal> for Decimal {
    fn product<I: Iterator<Item = &'a Decimal>>(iter: I) -> Decimal {
        iter.fold(Decimal::one(), |acc, x| acc * *x)
    }
}

// ---------------------------------------------------------------------------
// Primitive-type conversions and overloads
// ---------------------------------------------------------------------------

/// Implements `From<$prim_type> for Decimal` for integer types (always finite).
macro_rules! impl_from_integer {
    ($($prim_type:ty),*) => {$(
        impl From<$prim_type> for Decimal {
            fn from(prim: $prim_type) -> Self {
                Decimal::from_finite(prim as f64)
            }
        }
    )*};
}

/// Implements arithmetic operators and comparisons between `Decimal` and a primitive type.
///
/// `$convert` turns the primitive into a `Decimal`; `$try_convert` does the same fallibly
/// (returning `None` for NaN or infinite floats) and drives the comparison impls.
macro_rules! impl_ops_primitive {
    ($prim_type:ty, $convert:expr, $try_convert:expr) => {
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
        impl PartialEq<$prim_type> for Decimal {
            fn eq(&self, other: &$prim_type) -> bool {
                $try_convert(*other).is_some_and(|d: Decimal| *self == d)
            }
        }
        impl PartialEq<Decimal> for $prim_type {
            fn eq(&self, other: &Decimal) -> bool {
                other == self
            }
        }
        impl PartialOrd<$prim_type> for Decimal {
            fn partial_cmp(&self, other: &$prim_type) -> Option<core::cmp::Ordering> {
                $try_convert(*other).map(|d: Decimal| self.cmp(&d))
            }
        }
        impl PartialOrd<Decimal> for $prim_type {
            fn partial_cmp(&self, other: &Decimal) -> Option<core::cmp::Ordering> {
                $try_convert(*self).map(|d: Decimal| d.cmp(other))
            }
        }
    };
}

impl_from_integer!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
// Note: f32/f64 do NOT get From; only TryFrom is provided (in decimal.rs).

impl_ops_primitive!(i8, Decimal::from, |x: i8| Some(Decimal::from(x)));
impl_ops_primitive!(i16, Decimal::from, |x: i16| Some(Decimal::from(x)));
impl_ops_primitive!(i32, Decimal::from, |x: i32| Some(Decimal::from(x)));
impl_ops_primitive!(i64, Decimal::from, |x: i64| Some(Decimal::from(x)));
impl_ops_primitive!(i128, Decimal::from, |x: i128| Some(Decimal::from(x)));
impl_ops_primitive!(isize, Decimal::from, |x: isize| Some(Decimal::from(x)));
impl_ops_primitive!(u8, Decimal::from, |x: u8| Some(Decimal::from(x)));
impl_ops_primitive!(u16, Decimal::from, |x: u16| Some(Decimal::from(x)));
impl_ops_primitive!(u32, Decimal::from, |x: u32| Some(Decimal::from(x)));
impl_ops_primitive!(u64, Decimal::from, |x: u64| Some(Decimal::from(x)));
impl_ops_primitive!(u128, Decimal::from, |x: u128| Some(Decimal::from(x)));
impl_ops_primitive!(usize, Decimal::from, |x: usize| Some(Decimal::from(x)));
impl_ops_primitive!(f32, |x: f32| Decimal::from_finite(x as f64), |x: f32| {
    Decimal::try_from(x).ok()
});
impl_ops_primitive!(f64, Decimal::from_finite, |x: f64| Decimal::try_from(x)
    .ok());

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ArithmeticErrorKind;

    #[test]
    fn infinity_arithmetic_rules() {
        let inf = Decimal::inf();
        let ninf = Decimal::neg_inf();
        assert_eq!(inf + inf, inf);
        assert_eq!(inf + Decimal::one(), inf);
        assert_eq!(ninf - Decimal::one(), ninf);
        assert_eq!(inf * Decimal::two(), inf);
        assert_eq!(inf * ninf, ninf);
        assert_eq!(ninf * ninf, inf);
        assert_eq!(Decimal::one() / inf, Decimal::zero());
        assert_eq!(inf / Decimal::two(), inf);
        assert_eq!(inf.recip(), Decimal::zero());
        assert_eq!(
            inf.checked_add(&ninf).unwrap_err().kind,
            ArithmeticErrorKind::Undefined
        );
        assert_eq!(
            inf.checked_sub(&inf).unwrap_err().kind,
            ArithmeticErrorKind::Undefined
        );
        assert_eq!(
            inf.checked_mul(&Decimal::zero()).unwrap_err().kind,
            ArithmeticErrorKind::Undefined
        );
        assert_eq!(
            inf.checked_div(&inf).unwrap_err().kind,
            ArithmeticErrorKind::Undefined
        );
        assert_eq!(
            inf.checked_rem(&Decimal::two()).unwrap_err().kind,
            ArithmeticErrorKind::Undefined
        );
        assert_eq!(
            Decimal::zero().checked_recip().unwrap_err().kind,
            ArithmeticErrorKind::DivisionByZero
        );
    }

    #[test]
    fn remainder_uses_native_precision_when_possible() {
        let big: Decimal = "1e20".parse().unwrap();
        assert_eq!(big % Decimal::from(7), Decimal::two());
        assert_eq!(Decimal::from(-10) % Decimal::from(3), Decimal::from(-1));
        assert_eq!(Decimal::from(10) % Decimal::from(-3), Decimal::from(1));
        assert_eq!(
            Decimal::from(-10).rem_floored(&Decimal::from(3)),
            Decimal::from(2)
        );
        assert_eq!(
            Decimal::from(10).rem_floored(&Decimal::from(-3)),
            Decimal::from(-2)
        );
        assert_eq!(
            Decimal::from(9).rem_floored(&Decimal::from(3)),
            Decimal::zero()
        );
        let huge: Decimal = "1e1000".parse().unwrap();
        assert_eq!(huge % Decimal::from(7), Decimal::zero());
        assert_eq!(Decimal::from(7) % huge, Decimal::from(7));
    }

    #[test]
    fn exact_reciprocal_product_is_one() {
        let a: Decimal = "1e20".parse().unwrap();
        let b: Decimal = "1e-20".parse().unwrap();
        assert_eq!(a * b, Decimal::one());
        // Nearly-reciprocal values must not be snapped to 1.
        let c = Decimal::from_components(1, 1, -20.00000000005);
        assert_ne!(a * c, Decimal::one());
    }

    #[test]
    fn sum_and_product() {
        let v = [Decimal::one(), Decimal::two(), Decimal::ten()];
        assert_eq!(v.iter().sum::<Decimal>(), Decimal::from(13));
        assert_eq!(v.into_iter().product::<Decimal>(), Decimal::from(20));
    }

    #[test]
    fn primitive_comparisons() {
        assert_eq!(Decimal::from(2), 2.0);
        assert_eq!(2.0, Decimal::from(2));
        assert!(Decimal::from(2) < 3_u8);
        assert!(3_i64 > Decimal::from(2));
        assert!(Decimal::from(2).partial_cmp(&f64::NAN).is_none());
        assert!(Decimal::from(i128::MAX) > Decimal::from(u64::MAX));
    }
}
