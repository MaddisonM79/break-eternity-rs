//! Incremental-game helpers for geometric and arithmetic cost series.
//!
//! These mirror the static helpers in `break_eternity.js` (`affordGeometricSeries`,
//! `sumGeometricSeries`, `affordArithmeticSeries`, `sumArithmeticSeries`,
//! `efficiencyOfPurchase`). Each has a `checked_*` form that reports degenerate inputs as an
//! [`ArithmeticError`], and a plain form that panics on them.
//!
//! The two `afford_*` inverses are settled against their `sum_*` counterparts (see [`settle`]),
//! so `sum(n) <= resources < sum(n + 1)` holds for the `n` they return.

use crate::decimal::Decimal;
use crate::error::ArithmeticError;

impl Decimal {
    /// How many items can be bought with `resources_available` when the price starts at
    /// `price_start`, multiplies by `price_ratio` per purchase, and `current_owned` have already
    /// been bought.
    ///
    /// The result `n` agrees with [`sum_geometric_series`](Self::sum_geometric_series):
    /// `sum(n) <= resources_available < sum(n + 1)`, so a budget of exactly `sum(n)` affords
    /// `n`, never `n - 1`. (The closed-form inverse alone lands a rounding step short of the
    /// sum in roughly half of all exact-boundary cases; the count is nudged by up to two steps
    /// to match the sum, which is the authority.)
    ///
    /// A ratio of exactly 1 degenerates to `floor(resources / price_start)`. A ratio below 1
    /// makes the series converge to `start / (1 - ratio)`; a budget that covers that total
    /// affords every further purchase, and the result is infinity.
    ///
    /// # Panics
    ///
    /// Panics on degenerate inputs (a non-positive price or ratio). Use
    /// [`checked_afford_geometric_series`](Self::checked_afford_geometric_series) for explicit
    /// error handling.
    pub fn afford_geometric_series(
        resources_available: Decimal,
        price_start: Decimal,
        price_ratio: Decimal,
        current_owned: Decimal,
    ) -> Decimal {
        Self::checked_afford_geometric_series(
            resources_available,
            price_start,
            price_ratio,
            current_owned,
        )
        .unwrap_or_else(|e| panic!("undefined afford_geometric_series: {e}"))
    }

    /// Fallible form of [`afford_geometric_series`](Self::afford_geometric_series).
    pub fn checked_afford_geometric_series(
        resources_available: Decimal,
        price_start: Decimal,
        price_ratio: Decimal,
        current_owned: Decimal,
    ) -> Result<Decimal, ArithmeticError> {
        let op = "afford_geometric_series";
        if price_start.sign <= 0 || price_ratio.sign <= 0 {
            return Err(ArithmeticError::out_of_domain(op));
        }
        if resources_available.sign <= 0 {
            return Ok(Decimal::zero());
        }
        let one = Decimal::one();
        let actual_start = price_start.mul_raw(price_ratio.pow_raw(current_owned));
        let estimate = if price_ratio == one {
            resources_available.div_raw(actual_start).floor()
        } else {
            // floor(log10(resources / start * (ratio - 1) + 1) / log10(ratio))
            let inner = resources_available
                .div_raw(actual_start)
                .mul_raw(price_ratio.sub_raw(one))
                .add_raw(one);
            if inner.sign <= 0 {
                // A shrinking price whose infinite total, start / (1 - ratio), fits in the
                // budget: every further purchase is affordable.
                return Ok(Decimal::inf());
            }
            inner.log10_raw().div_raw(price_ratio.log10_raw()).floor()
        };
        let estimate = estimate.nan_to_err(op)?;
        Ok(settle(estimate, resources_available, |n| {
            Self::checked_sum_geometric_series(n, price_start, price_ratio, current_owned)
        }))
    }

    /// Total cost of buying `num_items` items when the price starts at `price_start`,
    /// multiplies by `price_ratio` per purchase, and `current_owned` have already been bought.
    ///
    /// # Panics
    ///
    /// Panics on degenerate inputs. Use
    /// [`checked_sum_geometric_series`](Self::checked_sum_geometric_series) for explicit error
    /// handling.
    pub fn sum_geometric_series(
        num_items: Decimal,
        price_start: Decimal,
        price_ratio: Decimal,
        current_owned: Decimal,
    ) -> Decimal {
        Self::checked_sum_geometric_series(num_items, price_start, price_ratio, current_owned)
            .unwrap_or_else(|e| panic!("undefined sum_geometric_series: {e}"))
    }

    /// Fallible form of [`sum_geometric_series`](Self::sum_geometric_series).
    pub fn checked_sum_geometric_series(
        num_items: Decimal,
        price_start: Decimal,
        price_ratio: Decimal,
        current_owned: Decimal,
    ) -> Result<Decimal, ArithmeticError> {
        let op = "sum_geometric_series";
        if price_ratio.sign <= 0 {
            return Err(ArithmeticError::out_of_domain(op));
        }
        let one = Decimal::one();
        let actual_start = price_start.mul_raw(price_ratio.pow_raw(current_owned));
        if price_ratio == one {
            return actual_start.mul_raw(num_items).nan_to_err(op);
        }
        // start * ratio^owned * (1 - ratio^n) / (1 - ratio)
        actual_start
            .mul_raw(one.sub_raw(price_ratio.pow_raw(num_items)))
            .div_raw(one.sub_raw(price_ratio))
            .nan_to_err(op)
    }

    /// How many items can be bought with `resources_available` when the price starts at
    /// `price_start`, increases by `price_add` per purchase, and `current_owned` have already
    /// been bought.
    ///
    /// The result `n` agrees with [`sum_arithmetic_series`](Self::sum_arithmetic_series):
    /// `sum(n) <= resources_available < sum(n + 1)`. See
    /// [`afford_geometric_series`](Self::afford_geometric_series) for why that needs saying.
    ///
    /// # Panics
    ///
    /// Panics on degenerate inputs (a non-positive `price_add`). Use
    /// [`checked_afford_arithmetic_series`](Self::checked_afford_arithmetic_series) for explicit
    /// error handling.
    pub fn afford_arithmetic_series(
        resources_available: Decimal,
        price_start: Decimal,
        price_add: Decimal,
        current_owned: Decimal,
    ) -> Decimal {
        Self::checked_afford_arithmetic_series(
            resources_available,
            price_start,
            price_add,
            current_owned,
        )
        .unwrap_or_else(|e| panic!("undefined afford_arithmetic_series: {e}"))
    }

    /// Fallible form of [`afford_arithmetic_series`](Self::afford_arithmetic_series).
    pub fn checked_afford_arithmetic_series(
        resources_available: Decimal,
        price_start: Decimal,
        price_add: Decimal,
        current_owned: Decimal,
    ) -> Result<Decimal, ArithmeticError> {
        let op = "afford_arithmetic_series";
        if price_add.sign <= 0 {
            return Err(ArithmeticError::out_of_domain(op));
        }
        if resources_available.sign <= 0 {
            return Ok(Decimal::zero());
        }
        // n = (-(a - d/2) + sqrt((a - d/2)^2 + 2dS)) / d, floored
        let actual_start = price_start.add_raw(current_owned.mul_raw(price_add));
        let b = actual_start.sub_raw(price_add.div_raw(Decimal::two()));
        let b2 = b.sqr();
        let root = b2
            .add_raw(
                price_add
                    .mul_raw(resources_available)
                    .mul_raw(Decimal::two()),
            )
            .sqrt_raw();
        let estimate = (-b)
            .add_raw(root)
            .div_raw(price_add)
            .floor()
            .nan_to_err(op)?;
        Ok(settle(estimate, resources_available, |n| {
            Self::checked_sum_arithmetic_series(n, price_start, price_add, current_owned)
        }))
    }

    /// Total cost of buying `num_items` items when the price starts at `price_start`,
    /// increases by `price_add` per purchase, and `current_owned` have already been bought.
    pub fn sum_arithmetic_series(
        num_items: Decimal,
        price_start: Decimal,
        price_add: Decimal,
        current_owned: Decimal,
    ) -> Decimal {
        Self::checked_sum_arithmetic_series(num_items, price_start, price_add, current_owned)
            .unwrap_or_else(|e| panic!("undefined sum_arithmetic_series: {e}"))
    }

    /// Fallible form of [`sum_arithmetic_series`](Self::sum_arithmetic_series).
    pub fn checked_sum_arithmetic_series(
        num_items: Decimal,
        price_start: Decimal,
        price_add: Decimal,
        current_owned: Decimal,
    ) -> Result<Decimal, ArithmeticError> {
        // (n/2) * (2a + (n - 1)d)
        let actual_start = price_start.add_raw(current_owned.mul_raw(price_add));
        num_items
            .div_raw(Decimal::two())
            .mul_raw(
                actual_start
                    .mul_raw(Decimal::two())
                    .add_raw(num_items.sub_raw(Decimal::one()).mul_raw(price_add)),
            )
            .nan_to_err("sum_arithmetic_series")
    }

    /// Efficiency score of a purchase that costs `cost` and raises income from `current_rps`
    /// by `delta_rps`: `cost / current_rps + cost / delta_rps`. Lower is better.
    ///
    /// # Panics
    ///
    /// Panics if either rate is zero. Use
    /// [`checked_efficiency_of_purchase`](Self::checked_efficiency_of_purchase) for explicit
    /// error handling.
    pub fn efficiency_of_purchase(
        cost: Decimal,
        current_rps: Decimal,
        delta_rps: Decimal,
    ) -> Decimal {
        Self::checked_efficiency_of_purchase(cost, current_rps, delta_rps)
            .unwrap_or_else(|e| panic!("undefined efficiency_of_purchase: {e}"))
    }

    /// Fallible form of [`efficiency_of_purchase`](Self::efficiency_of_purchase).
    pub fn checked_efficiency_of_purchase(
        cost: Decimal,
        current_rps: Decimal,
        delta_rps: Decimal,
    ) -> Result<Decimal, ArithmeticError> {
        let op = "efficiency_of_purchase";
        if current_rps.sign == 0 || delta_rps.sign == 0 {
            return Err(ArithmeticError::division_by_zero(op));
        }
        cost.div_raw(current_rps)
            .add_raw(cost.div_raw(delta_rps))
            .nan_to_err(op)
    }
}

/// Nudges a closed-form purchase count until it agrees with the series sum it inverts.
///
/// The inverse (a log or a square root) and the sum (a power) round differently in the last
/// bit, so a budget of exactly `sum(n)` comes back from the closed form as `n - 1` about half
/// the time, and `sum(n)` can land a hair above a budget the closed form accepted. Walk at most
/// two steps in each direction until `sum(count) <= resources < sum(count + 1)`, which is the
/// check a caller would otherwise have to make by hand.
///
/// Two steps cover every case the closed form can produce: its error is a rounding step in the
/// log domain, well under one item. Where the sum cannot distinguish `count` from `count + 1`
/// (counts above layer 0) the walk is a no-op, and a sum that fails leaves `count` untouched.
fn settle(
    mut count: Decimal,
    resources: Decimal,
    sum: impl Fn(Decimal) -> Result<Decimal, ArithmeticError>,
) -> Decimal {
    if count.is_infinite() {
        return count;
    }
    let one = Decimal::one();
    for _ in 0..2 {
        match sum(count) {
            Ok(total) if count.sign() > 0 && total > resources => count = count.sub_raw(one),
            _ => break,
        }
    }
    for _ in 0..2 {
        let next = count.add_raw(one);
        if next == count {
            break;
        }
        match sum(next) {
            Ok(total) if total <= resources => count = next,
            _ => break,
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(x: f64) -> Decimal {
        Decimal::from_finite(x)
    }

    #[test]
    fn geometric_series() {
        // Prices 10, 15, 22.5, 33.75 ... : 100 resources afford 4 (10+15+22.5+33.75 = 81.25).
        assert_eq!(
            Decimal::afford_geometric_series(d(100.0), d(10.0), d(1.5), d(0.0)),
            d(4.0)
        );
        assert!(
            Decimal::sum_geometric_series(d(4.0), d(10.0), d(1.5), d(0.0))
                .approx_eq(&d(81.25), 1e-12)
        );
        // Owning 2 already shifts the start to 22.5.
        assert!(
            Decimal::sum_geometric_series(d(1.0), d(10.0), d(1.5), d(2.0))
                .approx_eq(&d(22.5), 1e-12)
        );
        // Ratio 1 is a flat price.
        assert_eq!(
            Decimal::afford_geometric_series(d(100.0), d(10.0), d(1.0), d(0.0)),
            d(10.0)
        );
        assert_eq!(
            Decimal::sum_geometric_series(d(3.0), d(10.0), d(1.0), d(0.0)),
            d(30.0)
        );
        assert_eq!(
            Decimal::afford_geometric_series(d(0.0), d(10.0), d(1.5), d(0.0)),
            Decimal::zero()
        );
        assert!(Decimal::checked_afford_geometric_series(d(1.0), d(0.0), d(1.5), d(0.0)).is_err());
        // Works far past f64.
        let big: Decimal = "1e500".parse().unwrap();
        let n = Decimal::afford_geometric_series(big, d(1.0), d(1.15), d(0.0));
        assert_eq!(n, d(8223.0), "{n}");
    }

    #[test]
    fn afford_agrees_with_sum_at_exact_boundaries() {
        // The closed form alone returns n - 1 for a budget of exactly sum(n) in about half of
        // these; the settle step restores the contract sum(n) <= budget < sum(n + 1).
        let zero = Decimal::zero();
        for (start, ratio) in [
            (8e13, 2.0),
            (10.0, 1.5),
            (10.0, 1.15),
            (1.0, 1.07),
            (3.0, 3.0),
        ] {
            let (start, ratio) = (d(start), d(ratio));
            for n in 1..=300u32 {
                let n = Decimal::from(n);
                let budget = Decimal::sum_geometric_series(n, start, ratio, zero);
                let got = Decimal::afford_geometric_series(budget, start, ratio, zero);
                assert_eq!(
                    got, n,
                    "geometric start={start} ratio={ratio} budget={budget}"
                );
                let next = Decimal::sum_geometric_series(n.add_raw(d(1.0)), start, ratio, zero);
                assert!(next > budget, "sum({n}) == sum({n} + 1) at {budget}");
            }
        }
        for (start, add) in [(10.0, 2.0), (0.7, 0.3), (8e13, 1e12), (1.0, 1e-3)] {
            let (start, add) = (d(start), d(add));
            for n in 1..=300u32 {
                let n = Decimal::from(n);
                let budget = Decimal::sum_arithmetic_series(n, start, add, zero);
                let got = Decimal::afford_arithmetic_series(budget, start, add, zero);
                assert_eq!(got, n, "arithmetic start={start} add={add} budget={budget}");
            }
        }
        // A flat price with a non-representable step: 3 * 0.7 = 2.0999999999999996.
        assert_eq!(
            Decimal::afford_geometric_series(d(3.0 * 0.7), d(0.7), d(1.0), d(0.0)),
            d(3.0)
        );
        // One unit short of the boundary is still n - 1.
        assert_eq!(
            Decimal::afford_geometric_series(d(5.6e14 - 1.0), d(8e13), d(2.0), d(0.0)),
            d(2.0)
        );
        // Owned offsets shift the start; the contract holds there too.
        let budget = Decimal::sum_geometric_series(d(7.0), d(8e13), d(2.0), d(5.0));
        assert_eq!(
            Decimal::afford_geometric_series(budget, d(8e13), d(2.0), d(5.0)),
            d(7.0)
        );
    }

    #[test]
    fn shrinking_prices_saturate_to_infinity() {
        // Prices 10, 5, 2.5, ... sum to 20; a budget of 20 or more affords every purchase.
        assert_eq!(
            Decimal::afford_geometric_series(d(20.0), d(10.0), d(0.5), d(0.0)),
            Decimal::inf()
        );
        assert_eq!(
            Decimal::afford_geometric_series(d(1e6), d(10.0), d(0.5), d(0.0)),
            Decimal::inf()
        );
        // Just under the total is a large finite count: sum(n) <= 19.99 < sum(n + 1).
        let n = Decimal::afford_geometric_series(d(19.99), d(10.0), d(0.5), d(0.0));
        assert!(n.is_finite() && n >= d(9.0), "{n}");
        assert!(Decimal::sum_geometric_series(n, d(10.0), d(0.5), d(0.0)) <= d(19.99));
        // Not even the first purchase.
        assert_eq!(
            Decimal::afford_geometric_series(d(9.0), d(10.0), d(0.5), d(0.0)),
            d(0.0)
        );
    }

    #[test]
    fn arithmetic_series() {
        // Prices 10, 12, 14, 16: 52 resources afford exactly 4.
        assert_eq!(
            Decimal::afford_arithmetic_series(d(52.0), d(10.0), d(2.0), d(0.0)),
            d(4.0)
        );
        assert_eq!(
            Decimal::afford_arithmetic_series(d(51.0), d(10.0), d(2.0), d(0.0)),
            d(3.0)
        );
        assert_eq!(
            Decimal::sum_arithmetic_series(d(4.0), d(10.0), d(2.0), d(0.0)),
            d(52.0)
        );
        assert_eq!(
            Decimal::sum_arithmetic_series(d(2.0), d(10.0), d(2.0), d(1.0)),
            d(26.0)
        );
        assert!(Decimal::checked_afford_arithmetic_series(d(1.0), d(1.0), d(0.0), d(0.0)).is_err());
    }

    #[test]
    fn efficiency() {
        assert_eq!(
            Decimal::efficiency_of_purchase(d(100.0), d(10.0), d(5.0)),
            d(30.0)
        );
        assert!(Decimal::checked_efficiency_of_purchase(d(1.0), d(0.0), d(1.0)).is_err());
    }
}
