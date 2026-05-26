//! Regression tests for math correctness fixes landed in the v0.2.x series.
//!
//! Each test locks one previously-buggy formula against a known-good value
//! (most match the JS reference behavior; sqrt-of-tiny is a Rust-only improvement).

use break_eternity::{Decimal, TetrationMode};
use std::convert::TryFrom;

#[test]
fn gamma_integer_values() {
    // gamma(1) = 0! = 1, gamma(2) = 1! = 1, gamma(5) = 4! = 24, gamma(10) = 9! = 362880
    assert!((Decimal::from_finite(1.0).gamma().to_number() - 1.0).abs() < 1e-10);
    assert!((Decimal::from_finite(2.0).gamma().to_number() - 1.0).abs() < 1e-10);
    assert!((Decimal::from_finite(5.0).gamma().to_number() - 24.0).abs() < 1e-9);
    assert!((Decimal::from_finite(10.0).gamma().to_number() - 362880.0).abs() < 1e-3);
}

#[test]
fn gamma_half_integers() {
    // gamma(0.5) = sqrt(π) ≈ 1.7724538509055159
    assert!((Decimal::from_finite(0.5).gamma().to_number() - 1.7724538509055159).abs() < 1e-10);
    // gamma(5.5) ≈ 52.34277778455352
    assert!((Decimal::from_finite(5.5).gamma().to_number() - 52.34277778455352).abs() < 1e-10);
    // gamma(2.5) = 0.75 * sqrt(π) ≈ 1.329340388
    assert!((Decimal::from_finite(2.5).gamma().to_number() - 1.3293403881791415).abs() < 1e-10);
}

#[test]
fn factorial_integer_values() {
    assert!((Decimal::from_finite(0.0).factorial().to_number() - 1.0).abs() < 1e-10);
    assert!((Decimal::from_finite(5.0).factorial().to_number() - 120.0).abs() < 1e-9);
    assert!((Decimal::from_finite(10.0).factorial().to_number() - 3628800.0).abs() < 1e-3);
}

#[test]
fn exp_layer1_negative_is_tiny_positive() {
    // exp(-1e16) should be a tiny positive value (≈ 10^(-4.34e15)), stored
    // at layer 1 with negative mag and sign +1.
    let r = Decimal::from_components(-1, 1, 16.0).exp();
    assert_eq!(
        r.sign(),
        1,
        "exp of huge-negative should be positive (close to 0)"
    );
    assert_eq!(r.layer(), 1);
    assert!(
        r.mag() < 0.0,
        "exp(-1e16) should have negative mag (= tiny positive value), got mag={}",
        r.mag()
    );
}

#[test]
fn atanh_matches_real_math() {
    // atanh(0.5) = 0.5 * ln(3) ≈ 0.549306144
    let r = Decimal::from_finite(0.5).atanh();
    assert!(
        (r.to_number() - 0.5493061443340548).abs() < 1e-10,
        "atanh(0.5) = {}",
        r
    );

    // atanh(-0.3) ≈ -0.30951960
    let r = Decimal::from_finite(-0.3).atanh();
    assert!(
        (r.to_number() - (-0.30951960420311186)).abs() < 1e-10,
        "atanh(-0.3) = {}",
        r
    );

    // atanh(0.9) ≈ 1.47221949
    let r = Decimal::from_finite(0.9).atanh();
    assert!(
        (r.to_number() - 1.4722194895832204).abs() < 1e-10,
        "atanh(0.9) = {}",
        r
    );
}

#[test]
fn pow10_boundary() {
    // 10^(-1) = 0.1; was previously returning 1.0 due to off-by-epsilon boundary
    let r = Decimal::from_finite(10.0).pow(Decimal::from_finite(-1.0));
    assert!((r.to_number() - 0.1).abs() < 1e-12, "10^-1 = {}", r);

    // -10^(-1) = -0.1
    let r = Decimal::from_finite(-10.0).pow(Decimal::from_finite(-1.0));
    assert!((r.to_number() - (-0.1)).abs() < 1e-12, "-10^-1 = {}", r);
}

#[test]
fn pow10_infinity_short_circuits() {
    assert_eq!(Decimal::inf().pow10(), Decimal::inf());
    assert_eq!(Decimal::neg_inf().pow10(), Decimal::zero());
}

#[test]
fn subtraction_preserves_small_residuals() {
    // 1 - (1 - 1e-11) should be ≈ 1e-11 (not collapsed to 0)
    let a = Decimal::from_finite(1.0);
    let b = Decimal::from_finite(1.0 - 1e-11);
    let r = a - b;
    assert!(
        r.to_number() > 0.0,
        "1 - (1-1e-11) should be positive, got {}",
        r
    );
    // Allow generous tolerance — exact result depends on float64 representability
    assert!(
        (r.to_number() - 1e-11).abs() < 1e-13,
        "1 - (1-1e-11) = {}, expected ≈ 1e-11",
        r
    );
}

#[test]
fn sqrt_of_tiny_layer1_value() {
    // sqrt(1e-20) = 1e-10; previously returned NaN at layer-1 with negative mag.
    let tiny = Decimal::from_finite(1.0) / Decimal::try_from("1e20").unwrap();
    let r = tiny.sqrt();
    assert!(
        r.mag().is_finite(),
        "sqrt(1e-20) returned non-finite mag: {:?}",
        r
    );
    assert!(
        (r.to_number() - 1e-10).abs() < 1e-20,
        "sqrt(1e-20) = {}, expected 1e-10",
        r
    );
}

#[test]
fn gamma_large_layer0_path() {
    // The `layer == 0 && mag >= 24` branch of gamma() uses a Stirling series.
    // The -1/(360·t^3) term was missing; values below come from `math.gamma`.
    let cases: &[(f64, f64)] = &[
        (24.0, 2.5852016738884976e22),
        (25.0, 6.204484017332394e23),
        (50.0, 6.082818640342675e62),
        (50.5, 4.290462912351975e63),
        (100.0, 9.332621544394415e155),
    ];
    for &(x, expected) in cases {
        let got = Decimal::from_finite(x).gamma().to_number();
        let rel_err = (got - expected).abs() / expected;
        assert!(
            rel_err < 1e-12,
            "gamma({}) = {}, expected {}, rel_err = {}",
            x,
            got,
            expected,
            rel_err
        );
    }
}

#[test]
fn sqrt_huge_tiny_inputs() {
    // sqrt(10^-100) = 10^-50, sqrt(10^-1000) = 10^-500. Both stay at layer 1.
    let v = Decimal::try_from("1e-100").unwrap().sqrt();
    let expected = Decimal::try_from("1e-50").unwrap();
    assert!(
        v.approx_eq(&expected, 1e-12),
        "sqrt(1e-100) = {}, expected 1e-50",
        v
    );

    let v = Decimal::try_from("1e-1000").unwrap().sqrt();
    let expected = Decimal::try_from("1e-500").unwrap();
    assert!(
        v.approx_eq(&expected, 1e-12),
        "sqrt(1e-1000) = {}, expected 1e-500",
        v
    );
}

#[test]
fn pow10_boundary_just_below() {
    // Locks the `>= 0.1` boundary against a future regression toward `> 0.1`.
    // 10^-1.0001 ≈ 0.0999769 — must route through the layer-bump branch and
    // still produce the right answer, not collapse to 1.0.
    let r = Decimal::from_finite(10.0).pow(Decimal::from_finite(-1.0001));
    let expected = 10.0_f64.powf(-1.0001);
    assert!(
        (r.to_number() - expected).abs() < 1e-12,
        "10^-1.0001 = {}, expected {}",
        r,
        expected
    );
}

#[test]
fn sub_huge_close_values_preserved() {
    // Locks the strict-equality additive-inverse check against any future
    // attempt to reintroduce an epsilon tolerance. Layer-1 close values
    // must produce a meaningful positive, not collapse to zero.
    let a = Decimal::from_components(1, 1, 100.0);
    let b = Decimal::from_components(1, 1, 99.99999999999);
    let r = a - b;
    assert_ne!(
        r,
        Decimal::zero(),
        "10^100 - 10^99.99999999999 must not collapse to 0"
    );
    assert_eq!(r.sign(), 1, "result must be positive");
}

#[test]
fn gamma_negative_threshold() {
    // The `f_gamma` cutoff at n < -50 deliberately returns 0 for non-integer
    // negatives below -50 (matches JS). gamma(-50.5) is actually ~-1.45e-65,
    // but JS chose to return 0 there. Lock both sides of the boundary.
    let r = Decimal::from_finite(-49.5).gamma().to_number();
    // gamma(-49.5) ≈ 7.32e-64; allow generous tolerance — Stirling near a
    // negative half-integer is asymptotic.
    assert!(r.abs() > 1e-65 && r.abs() < 1e-62, "gamma(-49.5) = {}", r);

    let r = Decimal::from_finite(-50.5).gamma().to_number();
    assert_eq!(r, 0.0, "gamma(-50.5) deliberately returns 0 (matches JS)");
}

#[test]
fn slog_analytic_matches_js_reference() {
    // Critical-section + refinement-loop slog. Values from break_eternity.js@2.1.3.
    let cases: &[(&str, f64)] = &[
        ("2", 0.3929124010088671),
        ("100", 1.392912401008867),
        ("1e308", 2.5021325804854673),
        ("1e1000", 2.587852386325334),
        ("ee100", 3.392912401008867),
        ("1e1e15", 3.096027070908571),
        ("(e^10)100", 11.392912401008866),
    ];
    for &(input, expected) in cases {
        let x = Decimal::try_from(input).unwrap();
        let got = x.slog(None, TetrationMode::Analytic).to_number();
        assert!(
            (got - expected).abs() < 1e-8,
            "slog10({}) analytic = {}, expected {}, diff = {}",
            input,
            got,
            expected,
            (got - expected).abs()
        );
    }
}

#[test]
fn slog_linear_preserves_old_behavior() {
    // Linear mode keeps the closed-form `result + copy - 1` approximation —
    // the answers are coarser than analytic but stable, and useful when the
    // caller knows the analytic surface isn't a good fit.
    let two = Decimal::from_finite(2.0);
    let linear = two.slog(None, TetrationMode::Linear).to_number();
    // log10(2) is the linear-mode answer: slog_internal returns result+copy-1
    // where for self=2: result=1, copy=log10(2), so 1 + log10(2) - 1 = log10(2).
    assert!(
        (linear - std::f64::consts::LOG10_2).abs() < 1e-12,
        "slog10(2) linear = {}, expected ≈ log10(2)",
        linear
    );
}

#[test]
fn slog_linear_round_trip_through_tetrate() {
    // The public `tetrate` is linear; pair it with linear slog so the
    // round-trip property holds. The analytic slog ↔ analytic tetrate
    // round-trip will be tested once tetrate's analytic path is exposed.
    for &n in &[2.0_f64, 5.0, 10.0, 100.0, 1e6] {
        let x = Decimal::from_finite(n);
        let s = x.slog(None, TetrationMode::Linear).to_number();
        let back = Decimal::from_finite(10.0)
            .tetrate(
                Some(s),
                Some(Decimal::from_finite(1.0)),
                TetrationMode::Linear,
            )
            .to_number();
        let rel_err = ((back - n) / n).abs();
        assert!(
            rel_err < 1e-6,
            "linear round-trip: slog10({}) = {}, tetrate(10, {}) = {}, rel_err = {}",
            n,
            s,
            s,
            back,
            rel_err
        );
    }
}

#[test]
fn tetrate_analytic_matches_js_reference() {
    // Fractional-height tetrate, analytic mode. JS values from
    // break_eternity.js@2.1.3 (linear=false). Bases above 10 fall back to
    // linear in JS too — these all use bases in [2, 10].
    let cases: &[(f64, f64, f64)] = &[
        (2.0, 2.5, 6.721399494148862),
        (std::f64::consts::E, 2.5, 179.11551957319872),
    ];
    for &(base_f64, height, expected) in cases {
        let base = Decimal::from_finite(base_f64);
        let got = base
            .tetrate(Some(height), None, TetrationMode::Analytic)
            .to_number();
        let rel_err = (got - expected).abs() / expected;
        assert!(
            rel_err < 1e-10,
            "tetrate({}, {}, Analytic) = {}, expected {}, rel_err = {}",
            base_f64,
            height,
            got,
            expected,
            rel_err
        );
    }
}

#[test]
fn tetrate_analytic_diverges_from_linear() {
    // For fractional heights with base <= 10, analytic and linear should
    // disagree by a measurable amount. This locks the wire-up — if a future
    // refactor accidentally routes both modes through the same path, this
    // test catches it.
    let two = Decimal::from_finite(2.0);
    let analytic = two
        .tetrate(Some(2.5), None, TetrationMode::Analytic)
        .to_number();
    let linear = two
        .tetrate(Some(2.5), None, TetrationMode::Linear)
        .to_number();
    assert!(
        (analytic - linear).abs() > 0.01,
        "tetrate(2, 2.5) analytic={} vs linear={} should differ noticeably",
        analytic,
        linear
    );
}

#[test]
fn slog_analytic_round_trip_through_tetrate_analytic() {
    // Issue #16 acceptance criterion: with both ops on Analytic, slog should
    // invert tetrate to ~1e-8 precision. The slog refinement loop converges
    // against the analytic tetrate (now both reachable publicly).
    for &n in &[2.0_f64, 5.0, 10.0, 100.0, 1e6] {
        let x = Decimal::from_finite(n);
        let s = x.slog(None, TetrationMode::Analytic).to_number();
        let back = Decimal::from_finite(10.0)
            .tetrate(
                Some(s),
                Some(Decimal::from_finite(1.0)),
                TetrationMode::Analytic,
            )
            .to_number();
        let rel_err = ((back - n) / n).abs();
        assert!(
            rel_err < 1e-8,
            "analytic round-trip: slog10({}) = {}, tetrate(10, {}, analytic) = {}, rel_err = {}",
            n,
            s,
            s,
            back,
            rel_err
        );
    }
}

#[test]
fn iteratedlog_propagates_mode() {
    // Lock the iteratedlog mode-propagation chain. If a future refactor
    // accidentally drops `mode` from layer_add_10 / layer_add inside
    // iteratedlog, the two modes will silently collapse to one path —
    // this test fails first.
    let x = Decimal::try_from("ee100").unwrap();
    let ten = Decimal::from_finite(10.0);
    let a = x.iteratedlog(ten, 2.5, TetrationMode::Analytic);
    let l = x.iteratedlog(ten, 2.5, TetrationMode::Linear);
    assert!(
        !a.approx_eq(&l, 1e-10),
        "iteratedlog(ee100, 10, 2.5) modes should diverge: analytic={:?}, linear={:?}",
        a,
        l
    );
}

#[test]
fn pentate_propagates_mode() {
    // Pentate at fractional height calls into tetrate, which has the
    // analytic critical-section branch. Mode must reach it.
    let two = Decimal::from_finite(2.0);
    let a = two.pentate(Some(2.5), None, TetrationMode::Analytic);
    let l = two.pentate(Some(2.5), None, TetrationMode::Linear);
    assert!(
        !a.approx_eq(&l, 1e-10),
        "pentate(2, 2.5) modes should diverge: analytic={:?}, linear={:?}",
        a,
        l
    );
}

#[test]
fn layer_add_propagates_mode() {
    // layer_add calls slog internally with the supplied mode. Mode-aware
    // slog produces different answers analytic vs linear, so layer_add
    // should too.
    let x = Decimal::from_finite(100.0);
    let ten = Decimal::from_finite(10.0);
    let a = x.layer_add(0.5, ten, TetrationMode::Analytic);
    let l = x.layer_add(0.5, ten, TetrationMode::Linear);
    assert!(
        !a.approx_eq(&l, 1e-10),
        "layer_add(100, 0.5, base=10) modes should diverge: analytic={:?}, linear={:?}",
        a,
        l
    );
}

#[test]
fn tetrate_above_base_10_analytic_equals_linear() {
    // The critical-section table covers bases in [2, 10]. For base > 10
    // the analytic path must fall through to the linear approximation —
    // locks the `base_num <= 10.0` guard in checked_tetrate.
    let eleven = Decimal::from_finite(11.0);
    let a = eleven.tetrate(Some(2.5), None, TetrationMode::Analytic);
    let l = eleven.tetrate(Some(2.5), None, TetrationMode::Linear);
    assert!(
        a.approx_eq(&l, 1e-12),
        "tetrate(11, 2.5): base > 10 must use linear path in both modes; got analytic={:?}, linear={:?}",
        a, l
    );
}

#[test]
fn layer_add_10_sign_move_b() {
    // JS bugfix B: (-3).layer_add_10(1) should equal 10^(-3) = 0.001, not -1000.
    // The negative sign moves to mag before the layer-bump.
    for mode in [TetrationMode::Analytic, TetrationMode::Linear] {
        let r = Decimal::from_finite(-3.0)
            .layer_add_10(Decimal::from_finite(1.0), mode)
            .to_number();
        assert!(
            (r - 0.001).abs() < 1e-12,
            "(-3).layer_add_10(1, {:?}) = {}, expected 0.001",
            mode,
            r
        );
    }
}

#[test]
fn layer_add_10_display_doesnt_panic_on_zero_negative() {
    // JS bugfix C: 0.layer_add_10(-2) used to return an unnormalized
    // (sign=0, layer=-1, mag=-inf) Decimal whose Display impl panicked
    // with `capacity overflow`. Now should produce a normalized output
    // (JS returns -Infinity).
    for mode in [TetrationMode::Analytic, TetrationMode::Linear] {
        let r = Decimal::from_finite(0.0).layer_add_10(Decimal::from_finite(-2.0), mode);
        // The key safety property: Display must not panic.
        let _ = format!("{}", r);
    }
}

#[test]
fn tetrate_positive_infinity_doesnt_panic() {
    // JS bugfix: tetrate(base, +Infinity, 1) should return Infinity for
    // base > 1.444 (no fixed point, tower diverges) instead of panicking
    // with `lambertw OutOfDomain`.
    let r = Decimal::from_finite(2.0).tetrate(
        Some(f64::INFINITY),
        Some(Decimal::from_finite(1.0)),
        TetrationMode::Analytic,
    );
    assert_eq!(
        r,
        Decimal::inf(),
        "tetrate(2, +Inf) should be Inf, got {:?}",
        r
    );

    // For base in the convergence zone, lambertw applies and returns a fixed point.
    let r = Decimal::from_finite(1.2).tetrate(
        Some(f64::INFINITY),
        Some(Decimal::from_finite(1.0)),
        TetrationMode::Analytic,
    );
    assert!(
        r.to_number().is_finite() && r.to_number() > 1.0,
        "tetrate(1.2, +Inf) should converge to a fixed point > 1, got {:?}",
        r
    );
}
