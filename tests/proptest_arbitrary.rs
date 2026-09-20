//! The `proptest` feature: `Arbitrary for Decimal` honours its parameters and only produces
//! normalized values.
//!
//! Run with: `cargo test --features proptest --test proptest_arbitrary`
#![cfg(feature = "proptest")]

use break_eternity::strategy::{
    any_decimal, finite_decimal, integer_decimal, layer0_decimal, positive_decimal, DecimalParams,
};
use break_eternity::{Decimal, EXPONENT_LIMIT, FIRST_NEG_LAYER, LAYER_REDUCTION_THRESHOLD};
use proptest::prelude::*;

fn assert_normalized(d: Decimal) -> Result<(), TestCaseError> {
    prop_assert!(d.is_finite() || d.is_infinite(), "{d:?}");
    if d.is_infinite() {
        return Ok(());
    }
    if d.is_zero() {
        prop_assert_eq!((d.sign(), d.layer(), d.mag()), (0, 0, 0.0));
        return Ok(());
    }
    prop_assert!(d.sign() == 1 || d.sign() == -1, "{d:?}");
    if d.layer() == 0 {
        prop_assert!(
            d.mag() >= FIRST_NEG_LAYER && d.mag() < EXPONENT_LIMIT,
            "{d:?}"
        );
    } else {
        prop_assert!(
            d.mag().abs() >= LAYER_REDUCTION_THRESHOLD && d.mag().abs() < EXPONENT_LIMIT,
            "{d:?}"
        );
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    #[test]
    fn default_is_finite_and_normalized(d in any::<Decimal>()) {
        prop_assert!(d.is_finite());
        assert_normalized(d)?;
    }

    #[test]
    fn presets(
        f in finite_decimal(),
        a in any_decimal(),
        p in positive_decimal(),
        i in integer_decimal(),
        z in layer0_decimal(),
    ) {
        prop_assert!(f.is_finite());
        assert_normalized(a)?;
        prop_assert!(p.is_positive() && p.is_finite(), "{p:?}");
        prop_assert!(i.is_integer(), "{i:?}");
        prop_assert_eq!(z.layer(), 0, "{:?}", z);
    }

    #[test]
    fn params_are_honoured(
        d in any_with::<Decimal>(
            DecimalParams::default()
                .with_zero(false)
                .with_negative(false)
                .with_fractional(false)
                .with_max_layer(2),
        ),
        deep in any_with::<Decimal>(DecimalParams::default().with_max_layer(50)),
    ) {
        prop_assert!(d.is_positive(), "{d:?}");
        prop_assert!(d.is_integer(), "{d:?}");
        prop_assert!(d.layer() <= 2, "{d:?}");
        prop_assert!(deep.layer() <= 50, "{deep:?}");
    }
}

#[test]
fn every_arm_is_reachable() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    let mut runner = TestRunner::deterministic();
    let strategy = any_decimal();
    let mut seen_layers = std::collections::BTreeSet::new();
    let mut seen_inf = false;
    let mut seen_neg = false;
    let mut seen_frac = false;
    for _ in 0..2000 {
        let d = strategy.new_tree(&mut runner).unwrap().current();
        if d.is_infinite() {
            seen_inf = true;
            continue;
        }
        seen_layers.insert(d.layer());
        seen_neg |= d.is_negative();
        seen_frac |= !d.is_zero() && d.abs() < Decimal::one();
    }
    assert!(seen_inf && seen_neg && seen_frac);
    assert_eq!(
        seen_layers.into_iter().collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4]
    );
}
