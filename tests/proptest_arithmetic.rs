//! Property tests for [`break_eternity::Decimal`] arithmetic.
//!
//! Uses proptest to verify algebraic identities across a stratified range of
//! values spanning layer 0 (ordinary f64-range), layer 1 (1e15 to 1e1e15),
//! and up to layer 4.
//!
//! Tolerance notes
//! ---------------
//! `Decimal::approx_eq` uses a relative tolerance: the difference between two
//! values must be ≤ `tol * max(|a.mag|, |b.mag|)`. A single tolerance of `1e-9`
//! is used throughout. For layer 0 values this is tighter than floating-point
//! rounding; for layer ≥ 1 values it reflects precision loss in `Display`
//! round-trips and transcendental evaluation.

use break_eternity::{Decimal, COMPARE_EPSILON};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategy
// ---------------------------------------------------------------------------

/// Strategy that produces a wide range of interesting `Decimal` values:
/// zero, subnormal-range, normal-range, and layer 1–4 values.
fn finite_decimal() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        // Zero (1 weight)
        1 => Just(Decimal::zero()),
        // Layer 0 — negative side
        15 => (-1e15_f64..-1e-300_f64).prop_map(Decimal::from_finite),
        // Layer 0 — positive side, small
        10 => (1e-300_f64..1.0_f64).prop_map(Decimal::from_finite),
        // Layer 0 — positive side, moderate to large
        15 => (1.0_f64..1e15_f64).prop_map(Decimal::from_finite),
        // Layer 1: mag in [LAYER_REDUCTION_THRESHOLD, 1e15) — positive
        // from_components normalizes so we just need mag > ~15.954
        10 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 1, mag)),
        // Layer 1: negative
        5 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(-1, 1, mag)),
        // Layer 2: positive
        5 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 2, mag)),
        // Layer 3: positive
        3 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 3, mag)),
        // Layer 4: positive
        1 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 4, mag)),
    ]
}

/// Strategy that produces only nonzero `Decimal` values.
fn nonzero_decimal() -> impl Strategy<Value = Decimal> {
    finite_decimal().prop_filter("must be nonzero", |d| *d != Decimal::zero())
}

/// Strategy that produces positive `Decimal` values suitable for ln/exp round-trip
/// (layer 0 only — higher layers lose too much precision in transcendental eval).
fn positive_layer0_decimal() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        // Small positives
        1 => (1e-300_f64..1.0_f64).prop_map(Decimal::from_finite),
        // Moderate to large layer 0 positives
        1 => (1.0_f64..1e15_f64).prop_map(Decimal::from_finite),
    ]
}

// ---------------------------------------------------------------------------
// Tolerance constant
// ---------------------------------------------------------------------------

const PROP_TOLERANCE: f64 = 1e-9;

// ---------------------------------------------------------------------------
// Property: Display ↔ Parse round-trip
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        max_global_rejects: 1024,
        ..ProptestConfig::default()
    })]

    /// For any finite Decimal, `Display` then `TryFrom<&str>` recovers the original
    /// value within tolerance (tolerance is needed because `Display` uses limited
    /// decimal precision).
    #[test]
    fn display_parse_roundtrip(d in finite_decimal()) {
        let s = d.to_string();
        let reparsed: Decimal = s.as_str().try_into()
            .unwrap_or_else(|e| panic!("failed to parse Display output {s:?}: {e}"));
        // Exact equality for zero.
        if d == Decimal::zero() {
            prop_assert_eq!(reparsed, Decimal::zero());
        } else {
            prop_assert!(
                d.approx_eq(&reparsed, COMPARE_EPSILON * 10.0),
                "round-trip mismatch: original={d:?} display={s:?} reparsed={reparsed:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Property: Addition commutativity  a + b ≈ b + a
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        max_global_rejects: 1024,
        ..ProptestConfig::default()
    })]

    #[test]
    fn add_commutative(a in finite_decimal(), b in finite_decimal()) {
        let ab = a + b;
        let ba = b + a;
        prop_assert!(
            ab.approx_eq(&ba, PROP_TOLERANCE),
            "add not commutative: a={a:?}, b={b:?}, a+b={ab:?}, b+a={ba:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Property: Additive identity  a + 0 == a  (exact)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        max_global_rejects: 1024,
        ..ProptestConfig::default()
    })]

    #[test]
    fn add_zero_identity(a in finite_decimal()) {
        let result = a + Decimal::zero();
        prop_assert!(
            result == a,
            "a + 0 != a: a={:?}, result={:?}",
            a,
            result
        );
    }
}

// ---------------------------------------------------------------------------
// Property: Multiplication commutativity  a * b ≈ b * a
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        max_global_rejects: 1024,
        ..ProptestConfig::default()
    })]

    #[test]
    fn mul_commutative(a in finite_decimal(), b in finite_decimal()) {
        let ab = a * b;
        let ba = b * a;
        prop_assert!(
            ab.approx_eq(&ba, PROP_TOLERANCE),
            "mul not commutative: a={a:?}, b={b:?}, a*b={ab:?}, b*a={ba:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Property: Multiplicative identity  a * 1.0 ≈ a
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        max_global_rejects: 1024,
        ..ProptestConfig::default()
    })]

    #[test]
    fn mul_one_identity(a in finite_decimal()) {
        let one = Decimal::from_finite(1.0);
        let result = a * one;
        // For zero, must be exactly zero.
        if a == Decimal::zero() {
            prop_assert_eq!(result, Decimal::zero());
        } else {
            prop_assert!(
                a.approx_eq(&result, PROP_TOLERANCE),
                "a * 1 != a: a={a:?}, result={result:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Property: Multiplication by zero  a * 0 == 0  (exact)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        max_global_rejects: 1024,
        ..ProptestConfig::default()
    })]

    #[test]
    fn mul_zero(a in finite_decimal()) {
        let result = a * Decimal::zero();
        prop_assert!(
            result == Decimal::zero(),
            "a * 0 != 0: a={:?}, result={:?}",
            a,
            result
        );
    }
}

// ---------------------------------------------------------------------------
// Property: Self-subtraction  a - a == 0  (exact)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        max_global_rejects: 1024,
        ..ProptestConfig::default()
    })]

    #[test]
    fn self_subtraction(a in finite_decimal()) {
        let result = a - a;
        prop_assert!(
            result == Decimal::zero(),
            "a - a != 0: a={:?}, result={:?}",
            a,
            result
        );
    }
}

// ---------------------------------------------------------------------------
// Property: Self-division  a / a ≈ 1  (for nonzero a)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        max_global_rejects: 1024,
        ..ProptestConfig::default()
    })]

    #[test]
    fn self_division(a in nonzero_decimal()) {
        let result = a.checked_div(&a).expect("self-division should not fail");
        let one = Decimal::from_finite(1.0);
        prop_assert!(
            result.approx_eq(&one, PROP_TOLERANCE),
            "a / a != 1: a={a:?}, result={result:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Property: Negation involution  -(-a) == a  (exact)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        max_global_rejects: 1024,
        ..ProptestConfig::default()
    })]

    #[test]
    fn neg_involution(a in finite_decimal()) {
        let double_neg = -(-a);
        prop_assert!(
            double_neg == a,
            "-(-a) != a: a={:?}, double_neg={:?}",
            a,
            double_neg
        );
    }
}

// ---------------------------------------------------------------------------
// Property: abs sign  a.abs().sign() >= 0
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        max_global_rejects: 1024,
        ..ProptestConfig::default()
    })]

    #[test]
    fn abs_nonnegative_sign(a in finite_decimal()) {
        let abs_val = a.abs();
        prop_assert!(
            abs_val.sign() >= 0,
            "abs produced negative sign: a={a:?}, abs={abs_val:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Property: ln/exp round-trip  exp(ln(a)) ≈ a  (layer 0 positives only)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        max_global_rejects: 1024,
        ..ProptestConfig::default()
    })]

    /// For positive layer-0 values, exp(ln(a)) ≈ a.
    ///
    /// Layer ≥ 1 is excluded: `ln` of a layer-1 value drops a layer, and then
    /// `exp` of a moderate value may not reconstruct the original magnitude with
    /// enough precision.
    #[test]
    fn ln_exp_roundtrip(a in positive_layer0_decimal()) {
        // Sanity: only layer-0 values enter here.
        prop_assume!(a.layer() == 0);

        let ln_a = a.checked_ln().expect("ln of positive should succeed");
        let recovered = ln_a.exp();

        prop_assert!(
            recovered.approx_eq(&a, COMPARE_EPSILON * 10.0),
            "exp(ln(a)) != a: a={a:?}, ln={ln_a:?}, recovered={recovered:?}"
        );
    }
}
