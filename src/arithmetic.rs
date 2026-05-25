//! Arithmetic trait implementations for [`Decimal`]: Add, Sub, Mul, Div, Rem, Neg,
//! and their assign variants, plus primitive-type overloads via `impl_ops_primitive!`.

use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

use crate::constants::{COMPARE_EPSILON, MAX_FLOAT_PRECISION};
use crate::decimal::Decimal;
use crate::utils::sign;

// ---------------------------------------------------------------------------
// Decimal op Decimal
// ---------------------------------------------------------------------------

impl Add<Decimal> for Decimal {
    type Output = Decimal;

    fn add(self, rhs: Decimal) -> Self::Output {
        if !self.mag.is_finite() {
            return self;
        }

        if self.sign == 0 {
            return rhs;
        }
        if rhs.sign == 0 {
            return self;
        }

        if self.sign == -(rhs.sign)
            && self.layer == rhs.layer
            && (self.mag - rhs.mag).abs() < COMPARE_EPSILON
        {
            return Decimal::zero();
        }

        let a: Decimal;
        let b: Decimal;

        if self.layer >= 2 || rhs.layer >= 2 {
            return self.maxabs(rhs);
        }

        if self.cmpabs(&rhs) > 0 {
            a = self;
            b = rhs;
        } else {
            a = rhs;
            b = self;
        }

        if a.layer == 0 && b.layer == 0 {
            return Decimal::from_number(a.sign as f64 * a.mag + b.sign as f64 * b.mag);
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
            return Decimal::from_components(
                sign(mantissa),
                1,
                b.mag.log10() + mantissa.abs().log10(),
            );
        }

        if (a.mag - b.mag).abs() > MAX_FLOAT_PRECISION as f64 {
            return a;
        }

        let mag_diff = 10.0_f64.powf(a.mag - b.mag);
        let mantissa = b.sign as f64 + (a.sign as f64 * mag_diff);
        let new_mag = b.mag + mantissa.abs().log10();
        Decimal::from_components(sign(mantissa), 1, new_mag)
    }
}

impl Sub<Decimal> for Decimal {
    type Output = Decimal;

    fn sub(self, rhs: Decimal) -> Self::Output {
        self + -rhs
    }
}

impl Mul<Decimal> for Decimal {
    type Output = Decimal;

    fn mul(self, rhs: Decimal) -> Self::Output {
        if self.sign == 0 || rhs.sign == 0 {
            return Decimal::zero();
        }

        if self.layer == rhs.layer && (self.mag - -rhs.mag).abs() < COMPARE_EPSILON {
            return Decimal::from_components_no_normalize(self.sign * rhs.sign, 0, 1.0);
        }

        let a: Decimal;
        let b: Decimal;

        if (self.layer > rhs.layer) || (self.layer == rhs.layer && self.mag.abs() > rhs.mag.abs()) {
            a = self;
            b = rhs;
        } else {
            a = rhs;
            b = self;
        }

        if a.layer == 0 && b.layer == 0 {
            return Decimal::from_number(a.sign as f64 * b.sign as f64 * a.mag * b.mag);
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
}

impl Div<Decimal> for Decimal {
    type Output = Decimal;

    /// Division of two decimals by multiplying the denominator by the reciprocal of the numerator.
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Decimal) -> Self::Output {
        self * rhs.recip()
    }
}

impl Rem<Decimal> for Decimal {
    type Output = Decimal;

    fn rem(self, rhs: Decimal) -> Self::Output {
        if rhs == Decimal::zero() {
            return Decimal::zero();
        }

        if self.sign * rhs.sign == -1 {
            return self.abs().rem(rhs.abs()).neg();
        }

        if self.sign == -1 {
            return self.abs().rem(rhs.abs());
        }

        self - (self / rhs).floor() * rhs
    }
}

impl Neg for Decimal {
    type Output = Decimal;

    fn neg(self) -> Decimal {
        Decimal::from_components_no_normalize(-self.sign, self.layer, self.mag)
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
// Primitive-type overloads
// ---------------------------------------------------------------------------

macro_rules! impl_from_primitive {
    ($prim_type:ty) => {
        impl From<$prim_type> for Decimal {
            fn from(prim: $prim_type) -> Self {
                Decimal::from_number(prim as f64)
            }
        }
    };
}

macro_rules! impl_ops_primitive {
    ($prim_type:ty) => {
        impl Add<$prim_type> for Decimal {
            type Output = Decimal;

            fn add(self, rhs: $prim_type) -> Self::Output {
                self + Decimal::from_number(rhs as f64)
            }
        }

        impl Add<Decimal> for $prim_type {
            type Output = Decimal;

            fn add(self, rhs: Decimal) -> Self::Output {
                Decimal::from_number(self as f64) + rhs
            }
        }

        impl Sub<$prim_type> for Decimal {
            type Output = Decimal;

            fn sub(self, rhs: $prim_type) -> Self::Output {
                self - Decimal::from_number(rhs as f64)
            }
        }

        impl Sub<Decimal> for $prim_type {
            type Output = Decimal;

            fn sub(self, rhs: Decimal) -> Self::Output {
                Decimal::from_number(self as f64) - rhs
            }
        }

        impl Mul<$prim_type> for Decimal {
            type Output = Decimal;

            fn mul(self, rhs: $prim_type) -> Self::Output {
                self * Decimal::from_number(rhs as f64)
            }
        }

        impl Mul<Decimal> for $prim_type {
            type Output = Decimal;

            fn mul(self, rhs: Decimal) -> Self::Output {
                Decimal::from_number(self as f64) * rhs
            }
        }

        impl Div<$prim_type> for Decimal {
            type Output = Decimal;

            fn div(self, rhs: $prim_type) -> Self::Output {
                self / Decimal::from_number(rhs as f64)
            }
        }

        impl Div<Decimal> for $prim_type {
            type Output = Decimal;

            fn div(self, rhs: Decimal) -> Self::Output {
                Decimal::from_number(self as f64) / rhs
            }
        }

        impl Rem<$prim_type> for Decimal {
            type Output = Decimal;

            fn rem(self, rhs: $prim_type) -> Self::Output {
                self % Decimal::from_number(rhs as f64)
            }
        }

        impl Rem<Decimal> for $prim_type {
            type Output = Decimal;

            fn rem(self, rhs: Decimal) -> Self::Output {
                Decimal::from_number(self as f64) % rhs
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

impl_from_primitive!(i8);
impl_from_primitive!(i16);
impl_from_primitive!(i32);
impl_from_primitive!(i64);
impl_from_primitive!(u8);
impl_from_primitive!(u16);
impl_from_primitive!(u32);
impl_from_primitive!(u64);
impl_from_primitive!(f32);
impl_from_primitive!(f64);

impl_ops_primitive!(i8);
impl_ops_primitive!(i16);
impl_ops_primitive!(i32);
impl_ops_primitive!(i64);
impl_ops_primitive!(u8);
impl_ops_primitive!(u16);
impl_ops_primitive!(u32);
impl_ops_primitive!(u64);
impl_ops_primitive!(f32);
impl_ops_primitive!(f64);
