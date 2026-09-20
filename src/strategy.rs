//! `proptest` support: [`Arbitrary`] for [`Decimal`] and ready-made strategies. Enable with
//! the `proptest` Cargo feature (implies `std`).
//!
//! ```ignore
//! use break_eternity::strategy::{finite_decimal, DecimalParams};
//! use break_eternity::Decimal;
//! use proptest::prelude::*;
//!
//! proptest! {
//!     #[test]
//!     fn addition_commutes(a in finite_decimal(), b in finite_decimal()) {
//!         prop_assert_eq!(a + b, b + a);
//!     }
//!
//!     #[test]
//!     fn any_shape(d in any::<Decimal>(), p in any_with::<Decimal>(DecimalParams::positive())) {
//!         prop_assert!(d.is_finite());
//!         prop_assert!(p.is_positive());
//!     }
//! }
//! ```
//!
//! The strategies also work outside the macro:
//!
//! ```
//! use break_eternity::strategy::positive_decimal;
//! use proptest::strategy::{Strategy, ValueTree};
//! use proptest::test_runner::TestRunner;
//!
//! let mut runner = TestRunner::deterministic();
//! let d = positive_decimal().new_tree(&mut runner).unwrap().current();
//! assert!(d.is_positive());
//! ```
//!
//! The default distribution is skewed toward the places bugs live: exact small integers, the
//! layer boundaries (`9e15`, `1/9e15`, `10^15.95`), values between `0` and `1` at every
//! layer, and negative values. [`DecimalParams`] narrows it; the free functions are the
//! common presets.

use proptest::arbitrary::Arbitrary;
use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, Union};

use crate::constants::{EXPONENT_LIMIT, FIRST_NEG_LAYER};
use crate::decimal::Decimal;

/// What [`Arbitrary`] for [`Decimal`] generates. Start from a preset and adjust with the
/// `with_*` methods; the struct is `#[non_exhaustive]` so fields can be added later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[allow(clippy::struct_excessive_bools)] // independent switches, not a state machine
pub struct DecimalParams {
    /// Include zero. Default `true`.
    pub zero: bool,
    /// Include negative values. Default `true`.
    pub negative: bool,
    /// Include values with a fractional part, including those between `0` and `1`. When
    /// `false` every value is a whole number. Default `true`.
    pub fractional: bool,
    /// Include `±inf`. Default `false`.
    pub infinite: bool,
    /// Largest layer generated; `0` restricts to plain floats. Default `4`. Values above
    /// layer 4 are structurally identical to layer 4 for every operation in the crate.
    pub max_layer: i64,
}

impl Default for DecimalParams {
    fn default() -> Self {
        Self {
            zero: true,
            negative: true,
            fractional: true,
            infinite: false,
            max_layer: 4,
        }
    }
}

impl DecimalParams {
    /// Finite values of every sign and layer (the default).
    #[must_use]
    pub fn finite() -> Self {
        Self::default()
    }

    /// Finite values plus `±inf`.
    #[must_use]
    pub fn any() -> Self {
        Self {
            infinite: true,
            ..Self::default()
        }
    }

    /// Strictly positive finite values.
    #[must_use]
    pub fn positive() -> Self {
        Self {
            zero: false,
            negative: false,
            ..Self::default()
        }
    }

    /// Finite values with no fractional part: integers at layer 0 and everything above.
    #[must_use]
    pub fn integer() -> Self {
        Self {
            fractional: false,
            ..Self::default()
        }
    }

    /// Plain floats only: `|x|` in `[1/9e15, 9e15)`, plus zero.
    #[must_use]
    pub fn layer0() -> Self {
        Self {
            max_layer: 0,
            ..Self::default()
        }
    }

    /// Sets whether zero is generated.
    #[must_use]
    pub fn with_zero(self, zero: bool) -> Self {
        Self { zero, ..self }
    }

    /// Sets whether negative values are generated.
    #[must_use]
    pub fn with_negative(self, negative: bool) -> Self {
        Self { negative, ..self }
    }

    /// Sets whether non-integers (including values between `0` and `1`) are generated.
    #[must_use]
    pub fn with_fractional(self, fractional: bool) -> Self {
        Self { fractional, ..self }
    }

    /// Sets whether `±inf` is generated.
    #[must_use]
    pub fn with_infinite(self, infinite: bool) -> Self {
        Self { infinite, ..self }
    }

    /// Restricts to layers up to `max_layer`.
    #[must_use]
    pub fn with_max_layer(self, max_layer: i64) -> Self {
        Self { max_layer, ..self }
    }
}

impl Arbitrary for Decimal {
    type Parameters = DecimalParams;
    type Strategy = BoxedStrategy<Decimal>;

    fn arbitrary_with(p: Self::Parameters) -> Self::Strategy {
        let mut arms: Vec<(u32, BoxedStrategy<Decimal>)> = Vec::new();
        let mut arm = |weight: u32, s: BoxedStrategy<Decimal>| arms.push((weight, s));

        if p.zero {
            arm(2, Just(Decimal::zero()).boxed());
        }
        // Exact small integers, where most game logic lives.
        arm(6, (1_i64..=1000).prop_map(Decimal::from).boxed());
        arm(
            4,
            (1.0_f64..1e15)
                .prop_map(|x| Decimal::from_finite(x.floor()))
                .boxed(),
        );
        // The top of layer 0 (everything here is an integer: it is above 2^52). Values at or
        // above 9e15 normalize to layer 1, so only cross over when that layer is allowed.
        let top = if p.max_layer >= 1 {
            9.1e15
        } else {
            EXPONENT_LIMIT
        };
        arm(
            2,
            (8.9e15_f64..top)
                .prop_map(|x| Decimal::from_finite(x.floor()))
                .boxed(),
        );
        if p.fractional {
            arm(4, (1.0_f64..1e15).prop_map(Decimal::from_finite).boxed());
            arm(
                4,
                (FIRST_NEG_LAYER..1.0)
                    .prop_map(Decimal::from_finite)
                    .boxed(),
            );
        }
        if p.max_layer >= 1 {
            // Layer 1: from_components normalizes mag below the threshold back down.
            arm(6, layer(1, 16.0..1e15).boxed());
            if p.fractional {
                // Both sides of the layer-0 / layer-1 boundary.
                arm(
                    1,
                    (1e-17_f64..1.3e-16).prop_map(Decimal::from_finite).boxed(),
                );
                arm(1, layer(1, 15.9..16.0).boxed());
                arm(3, layer_neg(1, 16.0..1e15).boxed());
            }
        }
        for l in 2..=p.max_layer.min(4) {
            arm(3, layer(l, 16.0..1e15).boxed());
            if p.fractional {
                arm(1, layer_neg(l, 16.0..1e15).boxed());
            }
        }
        if p.max_layer > 4 {
            arm(
                1,
                (5_i64..=p.max_layer)
                    .prop_map(|l| Decimal::from_components(1, l, 100.0))
                    .boxed(),
            );
        }
        if p.infinite {
            arm(1, Just(Decimal::inf()).boxed());
        }

        let magnitude = Union::new_weighted(arms);
        if p.negative {
            (magnitude, any::<bool>())
                .prop_map(|(d, neg)| if neg { -d } else { d })
                .boxed()
        } else {
            magnitude.boxed()
        }
    }
}

fn layer(l: i64, mag: core::ops::Range<f64>) -> impl Strategy<Value = Decimal> {
    mag.prop_map(move |mag| Decimal::from_components(1, l, mag))
}

fn layer_neg(l: i64, mag: core::ops::Range<f64>) -> impl Strategy<Value = Decimal> {
    mag.prop_map(move |mag| Decimal::from_components(1, l, -mag))
}

/// Finite values of every sign and layer: `any::<Decimal>()`.
pub fn finite_decimal() -> BoxedStrategy<Decimal> {
    Decimal::arbitrary_with(DecimalParams::finite())
}

/// Finite values plus `±inf`.
pub fn any_decimal() -> BoxedStrategy<Decimal> {
    Decimal::arbitrary_with(DecimalParams::any())
}

/// Strictly positive finite values.
pub fn positive_decimal() -> BoxedStrategy<Decimal> {
    Decimal::arbitrary_with(DecimalParams::positive())
}

/// Finite whole numbers.
pub fn integer_decimal() -> BoxedStrategy<Decimal> {
    Decimal::arbitrary_with(DecimalParams::integer())
}

/// Plain floats only (layer 0).
pub fn layer0_decimal() -> BoxedStrategy<Decimal> {
    Decimal::arbitrary_with(DecimalParams::layer0())
}
