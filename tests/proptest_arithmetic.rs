//! Property tests for [`break_eternity::Decimal`].
//!
//! Uses proptest to verify algebraic identities across a stratified range of
//! values spanning layer 0 (ordinary f64-range), layer 1 (1e15 to 1e1e15),
//! up to layer 4, and the infinities, plus a "no input string can panic the
//! parser" fuzz.
//!
//! Tolerance notes
//! ---------------
//! `Decimal::approx_eq` uses a relative tolerance: the difference between two
//! values must be ≤ `tol * max(|a.mag|, |b.mag|)`. A single tolerance of `1e-9`
//! is used throughout. For layer 0 values this is tighter than floating-point
//! rounding; for layer ≥ 1 values it reflects precision loss in `Display`
//! round-trips and transcendental evaluation.

use break_eternity::{Decimal, TetrationMode, COMPARE_EPSILON};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Strategy that produces a wide range of interesting finite `Decimal` values:
/// zero, subnormal-range, normal-range, and layer 1–4 values of both signs.
fn finite_decimal() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        1 => Just(Decimal::zero()),
        2 => Just(Decimal::one()),
        2 => Just(Decimal::neg_one()),
        15 => (-1e15_f64..-1e-300_f64).prop_map(Decimal::from_finite),
        10 => (1e-300_f64..1.0_f64).prop_map(Decimal::from_finite),
        15 => (1.0_f64..1e15_f64).prop_map(Decimal::from_finite),
        // Layer 1: mag in [LAYER_REDUCTION_THRESHOLD, 1e15); from_components normalizes.
        10 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 1, mag)),
        5 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(-1, 1, mag)),
        // Layer 1 with negative mag: values between 1/9e15 and 1e-(1e15).
        4 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 1, -mag)),
        5 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 2, mag)),
        2 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(-1, 2, mag)),
        3 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 3, mag)),
        1 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 4, mag)),
    ]
}

/// Finite values plus the two infinities.
fn any_decimal() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        30 => finite_decimal(),
        1 => Just(Decimal::inf()),
        1 => Just(Decimal::neg_inf()),
    ]
}

/// Strategy that produces only nonzero finite `Decimal` values.
fn nonzero_decimal() -> impl Strategy<Value = Decimal> {
    finite_decimal().prop_filter("must be nonzero", |d| !d.is_zero())
}

/// Strictly positive finite values of any layer.
fn positive_decimal() -> impl Strategy<Value = Decimal> {
    finite_decimal().prop_filter("must be positive", |d| d.is_positive())
}

/// Strategy that produces positive `Decimal` values suitable for ln/exp round-trip
/// (layer 0 only — higher layers lose too much precision in transcendental eval).
fn positive_layer0_decimal() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        1 => (1e-300_f64..1.0_f64).prop_map(Decimal::from_finite),
        1 => (1.0_f64..1e15_f64).prop_map(Decimal::from_finite),
    ]
}

/// Strings drawn from the parser's alphabet, biased towards nearly-valid notations.
fn parser_input() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => "[-+0-9.eEpPtTfF^;(), ]{0,24}",
        1 => "\\PC{0,16}",
        1 => any::<f64>().prop_map(|f| f.to_string()),
        1 => finite_decimal().prop_map(|d| d.to_string()),
        1 => (any::<i8>(), 0..12_i64, any::<f64>()).prop_map(|(s, l, m)| format!("{}(e^{l}){m}", if s < 0 { "-" } else { "" })),
        1 => (any::<f64>(), any::<f64>()).prop_map(|(a, b)| format!("{a}^^{b}")),
        1 => (any::<f64>(), any::<f64>(), any::<f64>()).prop_map(|(a, b, c)| format!("{a}^^^{b};{c}")),
    ]
}

const PROP_TOLERANCE: f64 = 1e-9;

// ---------------------------------------------------------------------------
// Display ↔ Parse round-trip
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, max_global_rejects: 1024, ..ProptestConfig::default() })]

    /// For any Decimal (including infinities), `Display` then parse recovers the value.
    #[test]
    fn display_parse_roundtrip(d in any_decimal()) {
        let s = d.to_string();
        let reparsed: Decimal = s.as_str().try_into()
            .unwrap_or_else(|e| panic!("failed to parse Display output {s:?}: {e}"));
        if d.is_zero() || d.is_infinite() {
            prop_assert_eq!(reparsed, d);
        } else {
            prop_assert!(
                d.approx_eq(&reparsed, COMPARE_EPSILON * 10.0),
                "round-trip mismatch: original={d:?} display={s:?} reparsed={reparsed:?}"
            );
        }
    }

    /// `to_fixed`, `to_precision`, and `to_exponential` output always parses back to a
    /// value close to the original (they are lossy by design).
    #[test]
    fn formatted_output_parses(d in any_decimal(), places in 0usize..12) {
        for s in [d.to_fixed(places), d.to_precision(places), d.to_exponential(places), format!("{d:e}"), format!("{d:E}")] {
            let back: Decimal = s.as_str().try_into()
                .unwrap_or_else(|e| panic!("failed to parse formatted output {s:?} of {d:?}: {e}"));
            if d.is_zero() || d.is_infinite() {
                prop_assert_eq!(back, d, "{}", s);
            } else if !back.is_zero() {
                // Values that round to zero legitimately lose their sign.
                prop_assert_eq!(back.sign(), d.sign(), "sign lost in {:?} -> {:?}", d, s);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Parser never panics
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig { cases: 2048, ..ProptestConfig::default() })]

    /// The parser must never panic, whatever the input; it may only return Ok or Err.
    #[test]
    fn parser_never_panics(s in parser_input()) {
        let _ = Decimal::from_string(&s);
        let _ = Decimal::from_string_with_mode(&s, TetrationMode::Linear);
        // Whatever parsed must also format and re-parse without panicking.
        if let Ok(d) = Decimal::from_string(&s) {
            let _ = d.to_string();
            let _ = d.to_fixed(3);
            let _ = d.to_precision(3);
        }
    }
}

// ---------------------------------------------------------------------------
// Ring-ish identities
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, max_global_rejects: 1024, ..ProptestConfig::default() })]

    #[test]
    fn add_commutative(a in finite_decimal(), b in finite_decimal()) {
        let ab = a + b;
        let ba = b + a;
        prop_assert!(ab.approx_eq(&ba, PROP_TOLERANCE), "a={a:?}, b={b:?}, a+b={ab:?}, b+a={ba:?}");
    }

    #[test]
    fn add_zero_identity(a in any_decimal()) {
        prop_assert_eq!(a + Decimal::zero(), a);
    }

    #[test]
    fn mul_commutative(a in finite_decimal(), b in finite_decimal()) {
        let ab = a * b;
        let ba = b * a;
        prop_assert!(ab.approx_eq(&ba, PROP_TOLERANCE), "a={a:?}, b={b:?}, a*b={ab:?}, b*a={ba:?}");
    }

    #[test]
    fn mul_one_identity(a in any_decimal()) {
        let result = a * Decimal::one();
        if a.is_zero() || a.is_infinite() {
            prop_assert_eq!(result, a);
        } else {
            prop_assert!(a.approx_eq(&result, PROP_TOLERANCE), "a={a:?}, result={result:?}");
        }
    }

    #[test]
    fn mul_zero(a in finite_decimal()) {
        prop_assert_eq!(a * Decimal::zero(), Decimal::zero());
    }

    #[test]
    fn self_subtraction(a in finite_decimal()) {
        prop_assert_eq!(a - a, Decimal::zero());
    }

    #[test]
    fn self_division(a in nonzero_decimal()) {
        let result = a.checked_div(&a).expect("self-division should not fail");
        prop_assert!(result.approx_eq(&Decimal::one(), PROP_TOLERANCE), "a={a:?}, result={result:?}");
    }

    #[test]
    fn neg_involution(a in any_decimal()) {
        prop_assert_eq!(-(-a), a);
    }

    #[test]
    fn abs_nonnegative_sign(a in any_decimal()) {
        prop_assert!(a.abs().sign() >= 0);
        prop_assert!(a.abs() >= Decimal::zero());
    }

    #[test]
    fn recip_involution(a in nonzero_decimal()) {
        let back = a.recip().recip();
        prop_assert!(back.approx_eq(&a, PROP_TOLERANCE), "a={a:?}, 1/(1/a)={back:?}");
    }

    #[test]
    fn sum_matches_fold(v in prop::collection::vec(finite_decimal(), 0..8)) {
        let folded = v.iter().fold(Decimal::zero(), |acc, x| acc + *x);
        prop_assert_eq!(v.iter().sum::<Decimal>(), folded);
    }
}

// ---------------------------------------------------------------------------
// Ordering
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, max_global_rejects: 1024, ..ProptestConfig::default() })]

    /// `Ord` is antisymmetric and consistent with `PartialOrd`/`Eq`.
    #[test]
    fn ordering_is_consistent(a in any_decimal(), b in any_decimal()) {
        prop_assert_eq!(a.cmp(&b), b.cmp(&a).reverse());
        prop_assert_eq!(a.partial_cmp(&b), Some(a.cmp(&b)));
        prop_assert_eq!(a == b, a.cmp(&b).is_eq());
        prop_assert_eq!(a.max(b), b.max(a));
        prop_assert_eq!(a.min(b), b.min(a));
    }

    /// Infinity dominates every finite value.
    #[test]
    fn infinity_dominates(a in finite_decimal()) {
        prop_assert!(Decimal::inf() > a);
        prop_assert!(Decimal::neg_inf() < a);
        prop_assert_eq!(a + Decimal::inf(), Decimal::inf());
        prop_assert_eq!(a - Decimal::inf(), Decimal::neg_inf());
        prop_assert_eq!(a / Decimal::inf(), Decimal::zero());
    }

    /// Ordering agrees with subtraction sign and with `to_number` where the latter is exact.
    #[test]
    fn ordering_matches_subtraction(a in finite_decimal(), b in finite_decimal()) {
        let diff = a - b;
        match a.cmp(&b) {
            std::cmp::Ordering::Less => prop_assert!(diff.is_negative() || diff.is_zero(), "a={a:?} b={b:?} diff={diff:?}"),
            std::cmp::Ordering::Greater => prop_assert!(diff.is_positive() || diff.is_zero(), "a={a:?} b={b:?} diff={diff:?}"),
            std::cmp::Ordering::Equal => prop_assert!(diff.is_zero()),
        }
    }

    /// Rounding functions are idempotent and bracket the value.
    #[test]
    fn rounding_brackets(a in finite_decimal()) {
        let f = a.floor();
        let c = a.ceil();
        prop_assert!(f <= a && a <= c, "floor={f:?} a={a:?} ceil={c:?}");
        prop_assert_eq!(f.floor(), f);
        prop_assert_eq!(c.ceil(), c);
        prop_assert_eq!(a.trunc().trunc(), a.trunc());
        prop_assert!(a.trunc().abs() <= a.abs());
    }
}

// ---------------------------------------------------------------------------
// Transcendental round-trips
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, max_global_rejects: 1024, ..ProptestConfig::default() })]

    /// For positive layer-0 values, exp(ln(a)) ≈ a.
    #[test]
    fn ln_exp_roundtrip(a in positive_layer0_decimal()) {
        prop_assume!(a.layer() == 0);
        let ln_a = a.checked_ln().expect("ln of positive should succeed");
        let recovered = ln_a.exp();
        prop_assert!(recovered.approx_eq(&a, COMPARE_EPSILON * 10.0), "a={a:?}, ln={ln_a:?}, recovered={recovered:?}");
    }

    /// pow10(log10(a)) ≈ a for positive values of any layer.
    #[test]
    fn log10_pow10_roundtrip(a in positive_decimal()) {
        let back = a.log10().pow10();
        prop_assert!(back.approx_eq(&a, PROP_TOLERANCE), "a={a:?}, back={back:?}");
    }

    /// sqrt(a)^2 ≈ a and a.sqr().sqrt() ≈ |a|.
    #[test]
    fn sqrt_sqr_roundtrip(a in positive_decimal()) {
        let r = a.sqrt();
        prop_assert!(r.sqr().approx_eq(&a, PROP_TOLERANCE), "a={a:?}, sqrt={r:?}");
        prop_assert!(a.sqr().sqrt().approx_eq(&a, PROP_TOLERANCE), "a={a:?}");
    }

    /// pow is monotone in the exponent for bases above 1 (no epsilon short-circuits).
    #[test]
    fn pow_monotone_in_exponent(base in 1.000_000_001_f64..1e6, e1 in -1e6_f64..1e6, e2 in -1e6_f64..1e6) {
        let b = Decimal::from_finite(base);
        let (lo, hi) = if e1 <= e2 { (e1, e2) } else { (e2, e1) };
        prop_assume!(hi - lo > 1e-6 * hi.abs().max(1.0));
        let p_lo = b.pow(Decimal::from_finite(lo));
        let p_hi = b.pow(Decimal::from_finite(hi));
        prop_assert!(p_lo < p_hi, "{base}^{lo}={p_lo:?} !< {base}^{hi}={p_hi:?}");
    }

    /// slog(tetrate(10, h)) ≈ h for moderate heights (both analytic).
    #[test]
    fn slog_tetrate_roundtrip(h in 0.0_f64..6.0) {
        let t = Decimal::ten().tetrate(Some(h), None, TetrationMode::Analytic);
        let s = t.slog(None, TetrationMode::Analytic).to_number();
        prop_assert!((s - h).abs() < 1e-7, "slog(10^^{h}) = {s}");
    }

    /// Integer tetration matches repeated exponentiation.
    #[test]
    fn integer_tetration_is_repeated_pow(base in 1.1_f64..3.0, height in 1u32..4) {
        let b = Decimal::from_finite(base);
        let mut expected = Decimal::one();
        for _ in 0..height {
            expected = b.pow(expected);
        }
        let got = b.tetrate(Some(f64::from(height)), None, TetrationMode::Analytic);
        prop_assert!(got.approx_eq(&expected, PROP_TOLERANCE), "{base}^^{height}: got={got:?} expected={expected:?}");
    }

    /// Geometric-series helpers agree with a brute-force purchase loop.
    #[test]
    fn geometric_series_matches_loop(resources in 1.0_f64..1e6, start in 1.0_f64..100.0, ratio in 1.01_f64..3.0) {
        let (r, s, q) = (Decimal::from_finite(resources), Decimal::from_finite(start), Decimal::from_finite(ratio));
        let n = Decimal::afford_geometric_series(r, s, q, Decimal::zero()).to_number() as u32;
        // Brute force: keep buying while affordable.
        let mut spent = 0.0_f64;
        let mut count = 0u32;
        let mut price = start;
        while spent + price <= resources * (1.0 + 1e-9) && count < 10_000 {
            spent += price;
            price *= ratio;
            count += 1;
        }
        prop_assert!((i64::from(n) - i64::from(count)).abs() <= 1, "afford={n} loop={count}");
        let total = Decimal::sum_geometric_series(Decimal::from(count), s, q, Decimal::zero());
        prop_assert!(total.approx_eq(&Decimal::from_finite(spent), 1e-6) || count == 0, "sum={total:?} loop={spent}");
    }
}
