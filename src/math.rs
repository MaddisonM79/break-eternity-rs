//! Float math shim for `no_std` builds.
//!
//! With the `std` feature (the default) this module is empty and the crate calls the
//! inherent `f64` methods. Without it, [`FloatExt`] supplies the same method names on
//! `f64`, backed by [`libm`], so the kernels compile unchanged. Inherent methods always win
//! method resolution, so the trait is only ever reached for methods `core` lacks.

#[cfg(not(feature = "std"))]
#[allow(dead_code)]
pub(crate) trait FloatExt: Sized {
    fn floor(self) -> Self;
    fn ceil(self) -> Self;
    fn round(self) -> Self;
    fn trunc(self) -> Self;
    fn fract(self) -> Self;
    fn sqrt(self) -> Self;
    fn cbrt(self) -> Self;
    fn powi(self, n: i32) -> Self;
    fn powf(self, y: Self) -> Self;
    fn exp(self) -> Self;
    fn ln(self) -> Self;
    fn log10(self) -> Self;
    fn log2(self) -> Self;
    fn sin(self) -> Self;
    fn cos(self) -> Self;
    fn tan(self) -> Self;
    fn asin(self) -> Self;
    fn acos(self) -> Self;
    fn atan(self) -> Self;
    fn sinh(self) -> Self;
    fn cosh(self) -> Self;
    fn tanh(self) -> Self;
    fn asinh(self) -> Self;
    fn acosh(self) -> Self;
    fn atanh(self) -> Self;
}

#[cfg(not(feature = "std"))]
impl FloatExt for f64 {
    #[inline]
    fn floor(self) -> f64 {
        libm::floor(self)
    }
    #[inline]
    fn ceil(self) -> f64 {
        libm::ceil(self)
    }
    #[inline]
    fn round(self) -> f64 {
        libm::round(self)
    }
    #[inline]
    fn trunc(self) -> f64 {
        libm::trunc(self)
    }
    #[inline]
    fn fract(self) -> f64 {
        self - libm::trunc(self)
    }
    #[inline]
    fn sqrt(self) -> f64 {
        libm::sqrt(self)
    }
    #[inline]
    fn cbrt(self) -> f64 {
        libm::cbrt(self)
    }
    /// Exponentiation by squaring, the same scheme the `powi` intrinsic lowers to.
    fn powi(self, n: i32) -> f64 {
        let mut base = self;
        let mut exp = n.unsigned_abs();
        let mut acc = 1.0;
        while exp > 0 {
            if exp & 1 == 1 {
                acc *= base;
            }
            base *= base;
            exp >>= 1;
        }
        if n < 0 {
            1.0 / acc
        } else {
            acc
        }
    }
    #[inline]
    fn powf(self, y: f64) -> f64 {
        libm::pow(self, y)
    }
    #[inline]
    fn exp(self) -> f64 {
        libm::exp(self)
    }
    #[inline]
    fn ln(self) -> f64 {
        libm::log(self)
    }
    #[inline]
    fn log10(self) -> f64 {
        libm::log10(self)
    }
    #[inline]
    fn log2(self) -> f64 {
        libm::log2(self)
    }
    #[inline]
    fn sin(self) -> f64 {
        libm::sin(self)
    }
    #[inline]
    fn cos(self) -> f64 {
        libm::cos(self)
    }
    #[inline]
    fn tan(self) -> f64 {
        libm::tan(self)
    }
    #[inline]
    fn asin(self) -> f64 {
        libm::asin(self)
    }
    #[inline]
    fn acos(self) -> f64 {
        libm::acos(self)
    }
    #[inline]
    fn atan(self) -> f64 {
        libm::atan(self)
    }
    #[inline]
    fn sinh(self) -> f64 {
        libm::sinh(self)
    }
    #[inline]
    fn cosh(self) -> f64 {
        libm::cosh(self)
    }
    #[inline]
    fn tanh(self) -> f64 {
        libm::tanh(self)
    }
    #[inline]
    fn asinh(self) -> f64 {
        libm::asinh(self)
    }
    #[inline]
    fn acosh(self) -> f64 {
        libm::acosh(self)
    }
    #[inline]
    fn atanh(self) -> f64 {
        libm::atanh(self)
    }
}

/// With `std` linked for the test harness the inherent methods shadow the trait, so the
/// libm paths are exercised here through fully qualified calls against the std answers.
#[cfg(all(test, not(feature = "std")))]
mod tests {
    use super::FloatExt;

    const SAMPLES: [f64; 9] = [0.0, 0.5, 1.0, 2.0, 15.954, 308.25, 1e15, -3.75, -0.25];

    fn close(a: f64, b: f64) -> bool {
        (a.is_nan() && b.is_nan()) || a == b || ((a - b) / b).abs() < 1e-14
    }

    #[test]
    fn libm_agrees_with_std() {
        for &x in &SAMPLES {
            assert!(close(FloatExt::floor(x), f64::floor(x)), "floor {x}");
            assert!(close(FloatExt::ceil(x), f64::ceil(x)), "ceil {x}");
            assert!(close(FloatExt::round(x), f64::round(x)), "round {x}");
            assert!(close(FloatExt::trunc(x), f64::trunc(x)), "trunc {x}");
            assert!(close(FloatExt::fract(x), f64::fract(x)), "fract {x}");
            assert!(close(FloatExt::sqrt(x), f64::sqrt(x)), "sqrt {x}");
            assert!(close(FloatExt::cbrt(x), f64::cbrt(x)), "cbrt {x}");
            assert!(close(FloatExt::exp(x), f64::exp(x)), "exp {x}");
            assert!(close(FloatExt::ln(x), f64::ln(x)), "ln {x}");
            assert!(close(FloatExt::log10(x), f64::log10(x)), "log10 {x}");
            assert!(close(FloatExt::log2(x), f64::log2(x)), "log2 {x}");
            assert!(close(FloatExt::sin(x), f64::sin(x)), "sin {x}");
            assert!(close(FloatExt::cos(x), f64::cos(x)), "cos {x}");
            assert!(close(FloatExt::tan(x), f64::tan(x)), "tan {x}");
            assert!(close(FloatExt::asin(x), f64::asin(x)), "asin {x}");
            assert!(close(FloatExt::acos(x), f64::acos(x)), "acos {x}");
            assert!(close(FloatExt::atan(x), f64::atan(x)), "atan {x}");
            assert!(close(FloatExt::sinh(x), f64::sinh(x)), "sinh {x}");
            assert!(close(FloatExt::cosh(x), f64::cosh(x)), "cosh {x}");
            assert!(close(FloatExt::tanh(x), f64::tanh(x)), "tanh {x}");
            assert!(close(FloatExt::asinh(x), f64::asinh(x)), "asinh {x}");
            assert!(close(FloatExt::acosh(x), f64::acosh(x)), "acosh {x}");
            assert!(close(FloatExt::atanh(x), f64::atanh(x)), "atanh {x}");
            for n in [-3, -1, 0, 1, 2, 7, 31] {
                assert!(close(FloatExt::powi(x, n), f64::powi(x, n)), "powi {x} {n}");
            }
            for &y in &SAMPLES {
                assert!(close(FloatExt::powf(x, y), f64::powf(x, y)), "powf {x} {y}");
            }
        }
    }
}
