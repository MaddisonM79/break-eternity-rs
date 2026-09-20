//! Conversions from [`Decimal`] to primitive integers.
//!
//! `TryFrom<Decimal>` (and `TryFrom<&Decimal>`) is exact: it succeeds only for finite whole
//! numbers inside the target's range, and otherwise reports [`NotInteger`] or [`Overflow`].
//! The `to_*_saturating` methods never fail: they truncate toward zero and clamp to the
//! target's range, which is what a loop counter or a buy count usually wants.
//!
//! Values above `2^53` go through `f64`, so they are exact only as far as an `f64` is.
//!
//! [`NotInteger`]: crate::ArithmeticErrorKind::NotInteger
//! [`Overflow`]: crate::ArithmeticErrorKind::Overflow

use crate::decimal::Decimal;
use crate::error::{ArithmeticError, ArithmeticErrorKind};
#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
// shadowed by std's inherent methods whenever std is in the crate graph
use crate::math::FloatExt;

macro_rules! impl_try_from_decimal {
    ($($t:ty),*) => {$(
        impl TryFrom<Decimal> for $t {
            type Error = ArithmeticError;

            /// Converts a finite whole number in range; fractional values are `NotInteger`,
            /// out-of-range values and infinities are `Overflow`.
            fn try_from(d: Decimal) -> Result<Self, Self::Error> {
                const OP: &str = concat!("TryFrom<Decimal> for ", stringify!($t));
                if !d.is_integer() {
                    return Err(ArithmeticError::new(
                        if d.is_finite() {
                            ArithmeticErrorKind::NotInteger
                        } else {
                            ArithmeticErrorKind::Overflow
                        },
                        OP,
                    ));
                }
                let x = d.to_number();
                // `MAX as f64` may round up to a power of two, so test strictly below MAX + 1.
                if x >= <$t>::MIN as f64 && x < <$t>::MAX as f64 + 1.0 {
                    Ok(x as $t)
                } else {
                    Err(ArithmeticError::new(ArithmeticErrorKind::Overflow, OP))
                }
            }
        }

        impl TryFrom<&Decimal> for $t {
            type Error = ArithmeticError;

            fn try_from(d: &Decimal) -> Result<Self, Self::Error> {
                <$t>::try_from(*d)
            }
        }
    )*};
}

impl_try_from_decimal!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

macro_rules! impl_saturating {
    ($($name:ident => $t:ty),*) => {$(
        #[doc = concat!("Converts to `", stringify!($t), "`, truncating toward zero and clamping to the type's range.")]
        ///
        /// Infinity clamps too; a value between `-1` and `1` is `0`.
        ///
        /// ```
        /// use break_eternity::Decimal;
        ///
        #[doc = concat!("assert_eq!(Decimal::from_finite(7.9).", stringify!($name), "(), 7);")]
        #[doc = concat!("assert_eq!(Decimal::try_from(\"1e100\").unwrap().", stringify!($name), "(), ", stringify!($t), "::MAX);")]
        #[doc = concat!("assert_eq!(Decimal::neg_inf().", stringify!($name), "(), ", stringify!($t), "::MIN);")]
        /// ```
        pub fn $name(&self) -> $t {
            // `as` from f64 saturates and maps NaN to 0.
            self.to_number() as $t
        }
    )*};
}

impl Decimal {
    impl_saturating!(
        to_i32_saturating => i32,
        to_u32_saturating => u32,
        to_i64_saturating => i64,
        to_u64_saturating => u64,
        to_i128_saturating => i128,
        to_u128_saturating => u128,
        to_usize_saturating => usize
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    #[test]
    fn exact_conversions() {
        assert_eq!(i32::try_from(d("42")), Ok(42));
        assert_eq!(i32::try_from(&d("-42")), Ok(-42));
        assert_eq!(u8::try_from(d("255")), Ok(255));
        assert_eq!(i64::try_from(d("4503599627370496")), Ok(1 << 52));
        // Above 2^53 the conversion goes through f64, so it is as exact as f64 is.
        assert_eq!(u128::try_from(d("1e30")), Ok(1e30 as u128));
        assert_eq!(i8::try_from(d("-128")), Ok(i8::MIN));
        assert_eq!(usize::try_from(Decimal::zero()), Ok(0));
    }

    #[test]
    fn rejected_conversions() {
        assert_eq!(
            i32::try_from(d("1.5")).unwrap_err().kind,
            ArithmeticErrorKind::NotInteger
        );
        assert_eq!(
            u8::try_from(d("256")).unwrap_err().kind,
            ArithmeticErrorKind::Overflow
        );
        assert_eq!(
            u8::try_from(d("-1")).unwrap_err().kind,
            ArithmeticErrorKind::Overflow
        );
        assert_eq!(
            i64::try_from(d("9223372036854775808")).unwrap_err().kind,
            ArithmeticErrorKind::Overflow
        );
        assert_eq!(
            i64::try_from(d("1e100")).unwrap_err().kind,
            ArithmeticErrorKind::Overflow
        );
        assert_eq!(
            i64::try_from(Decimal::inf()).unwrap_err().kind,
            ArithmeticErrorKind::Overflow
        );
        assert_eq!(
            i64::try_from(d("1e-20")).unwrap_err().kind,
            ArithmeticErrorKind::NotInteger
        );
        let err = i16::try_from(d("70000")).unwrap_err();
        assert_eq!(err.op, "TryFrom<Decimal> for i16");
    }

    #[test]
    fn saturating() {
        assert_eq!(d("7.9").to_i64_saturating(), 7);
        assert_eq!(d("-7.9").to_i64_saturating(), -7);
        assert_eq!(d("-7.9").to_u64_saturating(), 0);
        assert_eq!(d("0.3").to_u32_saturating(), 0);
        assert_eq!(d("1e100").to_i32_saturating(), i32::MAX);
        assert_eq!(d("-1e100").to_i128_saturating(), i128::MIN);
        assert_eq!(Decimal::inf().to_u128_saturating(), u128::MAX);
        assert_eq!(Decimal::neg_inf().to_usize_saturating(), 0);
        assert_eq!(d("10^^3").to_u64_saturating(), u64::MAX);
    }
}
