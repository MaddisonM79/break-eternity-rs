//! Contract tests for the three-altitude API (see `docs/DESIGN.md`):
//!
//! * a `checked_*` method never panics, and an `Ok` never carries the internal NaN sentinel;
//! * a plain method either panics or returns a real value, never NaN;
//! * when both succeed they agree exactly, and the plain form panics iff the checked form errs;
//! * every value that escapes a public method satisfies the normalization invariant and
//!   survives a `Display` round-trip.
//!
//! These run over stratified random inputs of every layer plus the infinities.

use std::panic::{catch_unwind, AssertUnwindSafe};

use break_eternity::{
    ArithmeticError, Decimal, LambertBranch, TetrationMode, EXPONENT_LIMIT, FIRST_NEG_LAYER,
    LAYER_REDUCTION_THRESHOLD, MAX_SAFE_LAYER,
};
use proptest::prelude::*;

type Checked1 = fn(Decimal) -> Result<Decimal, ArithmeticError>;
type Plain1 = fn(Decimal) -> Decimal;
type Checked2 = fn(Decimal, Decimal) -> Result<Decimal, ArithmeticError>;
type Plain2 = fn(Decimal, Decimal) -> Decimal;

const A: TetrationMode = TetrationMode::Analytic;

// ---------------------------------------------------------------------------
// Operation tables. Each checked op is paired with its plain twin where one exists.
// ---------------------------------------------------------------------------

const UNARY_PAIRS: &[(&str, Checked1, Plain1)] = &[
    ("recip", |a| a.checked_recip(), |a| a.recip()),
    ("sqrt", |a| a.checked_sqrt(), |a| a.sqrt()),
    ("ln", |a| a.checked_ln(), |a| a.ln()),
    ("log10", |a| a.checked_log10(), |a| a.log10()),
    ("log2", |a| a.checked_log2(), |a| a.log2()),
    ("abs_log10", |a| a.checked_abs_log10(), |a| a.abs_log10()),
    ("gamma", |a| a.checked_gamma(), |a| a.gamma()),
    ("factorial", |a| a.checked_factorial(), |a| a.factorial()),
    ("ln_gamma", |a| a.checked_ln_gamma(), |a| a.ln_gamma()),
    ("lambertw", |a| a.checked_lambertw(), |a| a.lambertw()),
    (
        "lambertw_neg1",
        |a| a.checked_lambertw_branch(LambertBranch::NonPrincipal),
        |a| a.lambertw_branch(LambertBranch::NonPrincipal),
    ),
    ("ssqrt", |a| a.checked_ssqrt(), |a| a.ssqrt()),
    (
        "sroot3",
        |a| a.checked_linear_sroot(3.0),
        |a| a.linear_sroot(3.0),
    ),
    ("asin", |a| a.checked_asin(), |a| a.asin()),
    ("acos", |a| a.checked_acos(), |a| a.acos()),
    ("acosh", |a| a.checked_acosh(), |a| a.acosh()),
    ("atanh", |a| a.checked_atanh(), |a| a.atanh()),
    (
        "slog10",
        |a| a.checked_slog(Decimal::ten(), A),
        |a| a.slog(None, A),
    ),
    (
        "slog2",
        |a| a.checked_slog(Decimal::two(), A),
        |a| a.slog(Some(Decimal::two()), A),
    ),
    (
        "tetrate2_5",
        |a| a.checked_tetrate(2.5, Decimal::one(), A),
        |a| a.tetrate(Some(2.5), None, A),
    ),
    (
        "tetrate_neg0_5",
        |a| a.checked_tetrate(-0.5, Decimal::one(), A),
        |a| a.tetrate(Some(-0.5), None, A),
    ),
    (
        "tetrate_inf",
        |a| a.checked_tetrate(f64::INFINITY, Decimal::one(), A),
        |a| a.tetrate(Some(f64::INFINITY), None, A),
    ),
    (
        "pentate2",
        |a| a.checked_pentate(2.0, Decimal::one(), A),
        |a| a.pentate(Some(2.0), None, A),
    ),
    (
        "penta_log10",
        |a| a.checked_penta_log(Decimal::ten(), A),
        |a| a.penta_log(None, A),
    ),
    (
        "layer_add_10_1",
        |a| a.checked_layer_add_10(Decimal::one(), A),
        |a| a.layer_add_10(Decimal::one(), A),
    ),
    (
        "layer_add_10_half",
        |a| a.checked_layer_add_10(Decimal::from_finite(0.5), A),
        |a| a.layer_add_10(Decimal::from_finite(0.5), A),
    ),
    (
        "layer_add_e",
        |a| a.checked_layer_add(1.0, Decimal::from_finite(std::f64::consts::E), A),
        |a| a.layer_add(1.0, Decimal::from_finite(std::f64::consts::E), A),
    ),
    (
        "iteratedlog10_1_5",
        |a| a.checked_iteratedlog(Decimal::ten(), 1.5, A),
        |a| a.iteratedlog(Decimal::ten(), 1.5, A),
    ),
];

/// Unary operations that are total: they must never panic and never return NaN.
const UNARY_TOTAL: &[(&str, Plain1)] = &[
    ("abs", |a| a.abs()),
    ("neg", |a| -a),
    ("floor", |a| a.floor()),
    ("ceil", |a| a.ceil()),
    ("round", |a| a.round()),
    ("trunc", |a| a.trunc()),
    ("exp", |a| a.exp()),
    ("pow10", |a| a.pow10()),
    ("sqr", |a| a.sqr()),
    ("cube", |a| a.cube()),
    ("cbrt", |a| a.cbrt()),
    ("sin", |a| a.sin()),
    ("cos", |a| a.cos()),
    ("tan", |a| a.tan()),
    ("atan", |a| a.atan()),
    ("sinh", |a| a.sinh()),
    ("cosh", |a| a.cosh()),
    ("tanh", |a| a.tanh()),
    ("asinh", |a| a.asinh()),
    ("p_log10", |a| a.p_log10()),
];

const BINARY_PAIRS: &[(&str, Checked2, Plain2)] = &[
    ("add", |a, b| a.checked_add(&b), |a, b| a + b),
    ("sub", |a, b| a.checked_sub(&b), |a, b| a - b),
    ("mul", |a, b| a.checked_mul(&b), |a, b| a * b),
    ("div", |a, b| a.checked_div(&b), |a, b| a / b),
    ("rem", |a, b| a.checked_rem(&b), |a, b| a % b),
    (
        "rem_floored",
        |a, b| a.checked_rem_floored(&b),
        |a, b| a.rem_floored(&b),
    ),
    ("pow", |a, b| a.checked_pow(&b), |a, b| a.pow(b)),
    (
        "pow_base",
        |a, b| a.checked_pow_base(&b),
        |a, b| a.pow_base(b),
    ),
    ("root", |a, b| a.checked_root(&b), |a, b| a.root(b)),
    ("log", |a, b| a.checked_log(&b), |a, b| a.log(b)),
    (
        "tetrate_payload",
        |a, b| a.checked_tetrate(2.0, b, A),
        |a, b| a.tetrate(Some(2.0), Some(b), A),
    ),
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn finite_decimal() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        1 => Just(Decimal::zero()),
        2 => Just(Decimal::one()),
        2 => Just(Decimal::neg_one()),
        2 => Just(Decimal::two()),
        2 => Just(Decimal::ten()),
        2 => Just(Decimal::from_finite(0.5)),
        2 => Just(Decimal::from_finite(1.3)),
        2 => Just(Decimal::maximum()),
        2 => Just(Decimal::minimum()),
        12 => (-1e15_f64..-1e-300_f64).prop_map(Decimal::from_finite),
        8 => (1e-300_f64..1.0_f64).prop_map(Decimal::from_finite),
        12 => (1.0_f64..1e15_f64).prop_map(Decimal::from_finite),
        8 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 1, mag)),
        4 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(-1, 1, mag)),
        4 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 1, -mag)),
        4 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 2, mag)),
        2 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(-1, 2, mag)),
        2 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 2, -mag)),
        2 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 3, mag)),
        1 => (16.0_f64..1e15_f64).prop_map(|mag| Decimal::from_components(1, 7, mag)),
        1 => (0_i64..MAX_SAFE_LAYER).prop_map(|layer| Decimal::from_components(1, layer, 100.0)),
    ]
}

fn any_decimal() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        40 => finite_decimal(),
        1 => Just(Decimal::inf()),
        1 => Just(Decimal::neg_inf()),
    ]
}

/// Runs `f`, swallowing the panic message, and reports whether it panicked.
fn quietly<T>(f: impl FnOnce() -> T) -> Result<T, ()> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|_| ())
}

/// Asserts the public normalization invariant and Display round-trip for a returned value.
fn assert_well_formed(op: &str, input: &str, v: Decimal) -> Result<(), TestCaseError> {
    let ctx = || format!("{op}({input}) -> {v:?}");
    prop_assert!(v.is_finite() || v.is_infinite(), "NaN escaped: {}", ctx());
    prop_assert!(matches!(v.sign(), -1 | 0 | 1), "bad sign: {}", ctx());
    prop_assert!(v.layer() >= 0, "negative layer: {}", ctx());
    if v.is_zero() {
        prop_assert!(
            v.layer() == 0 && v.mag() == 0.0,
            "unnormalized zero: {}",
            ctx()
        );
    } else if v.is_infinite() {
        prop_assert!(
            v.layer() > MAX_SAFE_LAYER,
            "infinity with small layer: {}",
            ctx()
        );
    } else if v.layer() == 0 {
        prop_assert!(
            v.mag() >= FIRST_NEG_LAYER && v.mag() < EXPONENT_LIMIT,
            "layer-0 mag out of range: {}",
            ctx()
        );
    } else {
        prop_assert!(
            v.layer() <= MAX_SAFE_LAYER,
            "finite layer past safe max: {}",
            ctx()
        );
        prop_assert!(
            v.mag().abs() >= LAYER_REDUCTION_THRESHOLD && v.mag().abs() < EXPONENT_LIMIT,
            "layer>0 mag out of range: {}",
            ctx()
        );
    }
    let s = v.to_string();
    let back = s
        .parse::<Decimal>()
        .map_err(|e| TestCaseError::fail(format!("{} did not re-parse {s:?}: {e}", ctx())))?;
    if v.is_zero() || v.is_infinite() {
        prop_assert_eq!(back, v, "{}", ctx());
    } else {
        prop_assert!(
            back.approx_eq(&v, 1e-9),
            "Display round-trip drifted: {} -> {:?}",
            ctx(),
            back
        );
    }
    Ok(())
}

fn check_pair<T: std::fmt::Debug>(
    op: &str,
    input: &str,
    checked: Result<Result<Decimal, ArithmeticError>, ()>,
    plain: Result<Decimal, ()>,
    _marker: T,
) -> Result<(), TestCaseError> {
    let checked =
        checked.map_err(|()| TestCaseError::fail(format!("checked {op}({input}) panicked")))?;
    match (checked, plain) {
        (Ok(c), Ok(p)) => {
            assert_well_formed(op, input, c)?;
            prop_assert_eq!(c, p, "checked and plain {}({}) disagree", op, input);
        }
        (Err(_), Err(())) => {}
        (Ok(c), Err(())) => {
            return Err(TestCaseError::fail(format!(
                "plain {op}({input}) panicked but checked returned {c:?}"
            )));
        }
        (Err(e), Ok(p)) => {
            return Err(TestCaseError::fail(format!(
                "checked {op}({input}) returned {e} but plain returned {p:?}"
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, max_global_rejects: 1024, ..ProptestConfig::default() })]

    /// Unary checked/plain pairs: never NaN, never an unexpected panic, always agree.
    #[test]
    fn unary_contract(a in any_decimal()) {
        std::panic::set_hook(Box::new(|_| {}));
        let input = format!("{a:?}");
        for (op, checked, plain) in UNARY_PAIRS {
            let c = quietly(|| checked(a));
            let p = quietly(|| plain(a));
            check_pair(op, &input, c, p, ())?;
        }
        for (op, f) in UNARY_TOTAL {
            let v = quietly(|| f(a))
                .map_err(|()| TestCaseError::fail(format!("total op {op}({input}) panicked")))?;
            assert_well_formed(op, &input, v)?;
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, max_global_rejects: 1024, ..ProptestConfig::default() })]

    /// Binary checked/plain pairs over every combination of layers and signs.
    #[test]
    fn binary_contract(a in any_decimal(), b in any_decimal()) {
        std::panic::set_hook(Box::new(|_| {}));
        let input = format!("{a:?}, {b:?}");
        for (op, checked, plain) in BINARY_PAIRS {
            let c = quietly(|| checked(a, b));
            let p = quietly(|| plain(a, b));
            check_pair(op, &input, c, p, ())?;
        }
        // Comparisons and selections are total.
        let _ = a.cmp(&b);
        prop_assert_eq!(a.partial_cmp(&b), Some(a.cmp(&b)));
        prop_assert_eq!(a.approx_eq(&b, 1e-9), b.approx_eq(&a, 1e-9));
        assert_well_formed("max", &input, a.max(b))?;
        assert_well_formed("min", &input, a.min(b))?;
        assert_well_formed("maxabs", &input, a.maxabs(b))?;
        assert_well_formed("clamp", &input, a.clamp(b.min(a), b.max(a)))?;
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    /// The game helpers are total over sane inputs and never return NaN.
    #[test]
    fn series_contract(
        resources in any_decimal(),
        start in finite_decimal(),
        step in finite_decimal(),
        owned in finite_decimal(),
    ) {
        std::panic::set_hook(Box::new(|_| {}));
        let input = format!("{resources:?}, {start:?}, {step:?}, {owned:?}");
        let ops: [(&str, fn(Decimal, Decimal, Decimal, Decimal) -> Result<Decimal, ArithmeticError>); 4] = [
            ("afford_geometric", Decimal::checked_afford_geometric_series),
            ("sum_geometric", Decimal::checked_sum_geometric_series),
            ("afford_arithmetic", Decimal::checked_afford_arithmetic_series),
            ("sum_arithmetic", Decimal::checked_sum_arithmetic_series),
        ];
        for (op, f) in ops {
            let r = quietly(|| f(resources, start, step, owned))
                .map_err(|()| TestCaseError::fail(format!("{op}({input}) panicked")))?;
            if let Ok(v) = r {
                assert_well_formed(op, &input, v)?;
            }
        }
    }
}
