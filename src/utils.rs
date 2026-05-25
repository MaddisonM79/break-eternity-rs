//! Internal utility functions shared across modules.

/// Returns the sign of a float as i8: 1 for positive, -1 for negative, 0 for zero/NaN.
///
/// This differs from [`f64::signum`] in that it returns 0 for 0.0 and NaN.
pub fn sign(num: f64) -> i8 {
    if num.is_nan() {
        return 0;
    }

    if num == 0.0 {
        return 0;
    }

    if num.is_infinite() {
        return if num.is_sign_positive() { 1 } else { -1 };
    }

    if num > 0.0 {
        return 1;
    }

    -1
}

/// Returns `sign(num) * log10(|num|)`.
pub(crate) fn f_maglog10(num: f64) -> f64 {
    sign(num) as f64 * num.abs().log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_cases() {
        assert_eq!(sign(1.5), 1);
        assert_eq!(sign(-1.5), -1);
        assert_eq!(sign(0.0), 0);
        assert_eq!(sign(-0.0), 0);
        assert_eq!(sign(f64::NAN), 0);
        assert_eq!(sign(f64::INFINITY), 1);
        assert_eq!(sign(f64::NEG_INFINITY), -1);
    }
}
