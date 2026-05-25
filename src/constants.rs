//! Compile-time and lazily-initialized constants for the crate.

use std::sync::OnceLock;

/// Maximum number of digits of precision to assume in a float.
pub const MAX_FLOAT_PRECISION: usize = 17;

/// Exponent limit to increase a layer (9e15 is about the largest safe number).
///
/// If above this value, increase a layer.
pub const EXPONENT_LIMIT: f64 = 9e15;

/// Layer reduction threshold.
///
/// If below this value, reduce a layer. Approximately 15.954.
pub const LAYER_REDUCTION_THRESHOLD: f64 = 15.954242509439325;

/// At layer 0, smaller non-zero numbers than this become layer 1 numbers with negative mag.
///
/// After that the pattern continues as normal.
pub const FIRST_NEG_LAYER: f64 = 1_f64 / 9e15;

/// Largest exponent that can appear in a number.
///
/// Not all mantissas are valid here.
pub const NUMBER_EXP_MAX: i32 = 308;

/// Smallest exponent that can appear in a number.
///
/// Not all mantissas are valid here.
pub const NUMBER_EXP_MIN: i32 = -324;

/// Amount of Es that will be printed in a string representation of a Decimal.
pub const MAX_ES_IN_A_ROW: u64 = 5;

/// Maximum number powers of 10 that will be cached.
pub const MAX_POWERS_OF_TEN: usize = (NUMBER_EXP_MAX - NUMBER_EXP_MIN + 1) as usize;

/// 2*PI
pub const TWO_PI: f64 = std::f64::consts::TAU;

/// exp(-1)
pub const EXPN1: f64 = 0.36787944117144233;

/// W(1, 0)
pub const OMEGA: f64 = 0.5671432904097838;

/// Relative tolerance used for Decimal equality comparisons.
pub const COMPARE_EPSILON: f64 = 1e-10;

// ---------------------------------------------------------------------------
// OnceLock-backed lazy statics
// ---------------------------------------------------------------------------

static POWERS_OF_TEN_CELL: OnceLock<Vec<f64>> = OnceLock::new();

/// Returns a reference to the cached table of powers of 10 from `10^(NUMBER_EXP_MIN+1)`
/// through `10^NUMBER_EXP_MAX`.
pub(crate) fn powers_of_ten() -> &'static Vec<f64> {
    POWERS_OF_TEN_CELL.get_or_init(|| {
        let mut v: Vec<f64> = Vec::new();
        for i in (NUMBER_EXP_MIN + 1)..=NUMBER_EXP_MAX {
            v.push(format!("1e{i}").parse().unwrap());
        }
        v
    })
}

static IGNORE_COMMAS_CELL: OnceLock<bool> = OnceLock::new();

/// Returns whether commas in string representations should be ignored.
pub(crate) fn ignore_commas() -> bool {
    *IGNORE_COMMAS_CELL.get_or_init(|| true)
}

static COMMAS_ARE_DECIMAL_POINTS_CELL: OnceLock<bool> = OnceLock::new();

/// Returns whether commas are used as decimal points.
pub(crate) fn commas_are_decimal_points() -> bool {
    *COMMAS_ARE_DECIMAL_POINTS_CELL.get_or_init(|| false)
}

// ---------------------------------------------------------------------------
// Helpers that only need constants (no Decimal dependency)
// ---------------------------------------------------------------------------

/// Returns 10^exp using the precomputed power-of-ten table.
pub(crate) fn power_of_10(exp: i32) -> f64 {
    powers_of_ten()[(exp + 323) as usize]
}

// ---------------------------------------------------------------------------
// Unused public re-exports kept for API compatibility (the original file
// exported POWERS_OF_TEN, IGNORE_COMMAS, COMMAS_ARE_DECIMAL_POINTS as
// lazy_static statics — those are now private OnceLock cells, so nothing
// external depended on them directly).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_of_10_spot_checks() {
        assert_eq!(power_of_10(0), 1.0);
        assert_eq!(power_of_10(1), 10.0);
        assert_eq!(power_of_10(2), 100.0);
        assert_eq!(power_of_10(-1), 0.1);
    }

    #[test]
    fn compare_epsilon_value() {
        assert_eq!(COMPARE_EPSILON, 1e-10);
    }
}
