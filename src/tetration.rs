//! Tetration and related hyperoperations: `tetrate`, `iteratedlog`, `slog`, `layer_add_10`,
//! `layer_add`, super-roots, `pentate`, `penta_log`, penta-roots, and a generic increasing
//! inverse search.
//!
//! Ported from `break_eternity.js` 2.1.3. Each operation has a `pub(crate)` `*_raw` kernel
//! with JS semantics (undefined results are the internal NaN sentinel), a `checked_*` method
//! that maps those to [`ArithmeticError`], and a panicking convenience method.

use crate::constants::{
    EXPONENT_LIMIT, FIRST_NEG_LAYER, TETRATION_CONVERGENCE_MAX, TETRATION_CONVERGENCE_MIN,
};
use crate::critical_section::{slog_critical, tetrate_critical};
use crate::decimal::Decimal;
use crate::error::ArithmeticError;
#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
// shadowed by std's inherent methods whenever std is in the crate graph
use crate::math::FloatExt;
use crate::transcendental::LambertBranch;

/// Algorithm choice for the fractional-height path of tetration and its
/// inverse, super-logarithm. Matches the `linear` flag in
/// `break_eternity.js`.
///
/// `Analytic` (the default) uses the JS critical-section interpolation table
/// for bases in `[2, 10]` (rescaled for bases in `(1.444, 2)`). `Linear` uses
/// the older closed-form approximations. Bases above 10 always fall back to
/// linear regardless of mode, and super-roots and penta-roots always use the
/// linear approximation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TetrationMode {
    /// JS-default critical-section interpolation. Matches `break_eternity.js`.
    #[default]
    Analytic,
    /// Older linear approximation. Equivalent to JS `linear=true`.
    Linear,
}

/// Just above `e^(1/e)`: the JS "hotfix" threshold where both fixed points collapse to `e`.
const CONVERGENCE_HOTFIX: f64 = 1.444_667_861_009_099;

fn e() -> Decimal {
    Decimal::from_finite(core::f64::consts::E)
}

impl Decimal {
    // -----------------------------------------------------------------------
    // tetrate
    // -----------------------------------------------------------------------

    /// Tetrates the Decimal to the given height: `self^self^...^self` (`height` copies), or,
    /// when `payload != 1`, iterated exponentiation of `payload` to base `self`.
    ///
    /// `height` defaults to 2 and `payload` to 1. Works with negative and fractional heights;
    /// see [`TetrationMode`] for how fractional heights are interpolated.
    ///
    /// # Panics
    ///
    /// Panics if the result is undefined (for example a negative base with a fractional
    /// height, or bases below `e^-e` at infinite height). Use
    /// [`checked_tetrate`](Self::checked_tetrate) for explicit error handling.
    pub fn tetrate(
        &self,
        height: Option<f64>,
        payload: Option<Decimal>,
        mode: TetrationMode,
    ) -> Decimal {
        self.checked_tetrate(height.unwrap_or(2.0), payload.unwrap_or_else(Decimal::one), mode)
            .unwrap_or_else(|e| {
                panic!(
                    "undefined Decimal tetrate: {e} (base={self:?}, height={height:?}, payload={payload:?}, mode={mode:?})"
                )
            })
    }

    /// Tetrates the Decimal to the given height, returning an error if the result is undefined.
    pub fn checked_tetrate(
        &self,
        height: f64,
        payload: Decimal,
        mode: TetrationMode,
    ) -> Result<Decimal, ArithmeticError> {
        self.tetrate_raw(height, payload, mode)
            .nan_to_err("tetrate")
    }

    /// Upper fixed point of `b^x = x` for `1 < b <= e^(1/e)`, via the `W_-1` branch.
    fn upper_fixed_point(self) -> Decimal {
        let negln = -self.ln_raw();
        negln
            .lambertw_raw(LambertBranch::NonPrincipal)
            .div_raw(negln)
    }

    /// Lower (stable) fixed point of `b^x = x` for `e^-e <= b <= e^(1/e)`, via `W_0`.
    fn lower_fixed_point(self) -> Decimal {
        let negln = -self.ln_raw();
        negln.lambertw_raw(LambertBranch::Principal).div_raw(negln)
    }

    pub(crate) fn tetrate_raw(
        self,
        height: f64,
        mut payload: Decimal,
        mode: TetrationMode,
    ) -> Decimal {
        let one = Decimal::one();
        if self.has_nan_mag() || payload.has_nan_mag() || height.is_nan() {
            return Decimal::nan_sentinel();
        }
        // x^^1 == x^payload
        if height == 1.0 {
            return self.pow_raw(payload);
        }
        // x^^0 == payload
        if height == 0.0 {
            return payload;
        }
        // 1^^x == 1
        if self == one {
            return one;
        }
        // -1^^x == -1^payload
        if self == Decimal::neg_one() {
            return self.pow_raw(payload);
        }

        if height == f64::INFINITY {
            let this_num = self.to_number();
            if (TETRATION_CONVERGENCE_MIN..=TETRATION_CONVERGENCE_MAX).contains(&this_num) {
                let mut lower = self.lower_fixed_point();
                if this_num < 1.0 {
                    return lower;
                }
                let mut upper = self.upper_fixed_point();
                if this_num > CONVERGENCE_HOTFIX {
                    lower = e();
                    upper = e();
                }
                return match payload.cmp(&upper) {
                    core::cmp::Ordering::Equal => upper,
                    core::cmp::Ordering::Less => lower,
                    core::cmp::Ordering::Greater => Decimal::inf(),
                };
            } else if this_num > TETRATION_CONVERGENCE_MAX {
                return Decimal::inf();
            }
            // 0 <= base < e^-e never converges; negative bases go complex.
            return Decimal::nan_sentinel();
        }

        // 0^^x oscillates between 0 and 1 (payload is ignored).
        if self.sign == 0 {
            let mut result = ((height + 1.0) % 2.0).abs();
            if result > 1.0 {
                result = 2.0 - result;
            }
            return Decimal::from_f64(result);
        }

        if height < 0.0 {
            return payload.iteratedlog_raw(self, -height, mode);
        }

        let old_height = height;
        let height = height.trunc();
        let frac_height = old_height - height;

        // Bases in (0, e^(1/e)] converge (or flip-flop) towards a fixed point, so iterate
        // directly instead of using the layer machinery.
        let in_convergence_zone = self.sign > 0
            && (self < one
                || (self <= Decimal::from_finite(TETRATION_CONVERGENCE_MAX)
                    && payload <= self.upper_fixed_point()));
        if in_convergence_zone && (old_height > 10000.0 || mode == TetrationMode::Analytic) {
            let limit_height = height.min(10000.0) as i64;
            if frac_height == 0.0 {
                // Nothing to interpolate. (JS calls layeradd(0) here, which is a numerically
                // noisy no-op for negative payloads; skipping it keeps the exact payload.)
            } else if payload == one {
                payload = self.pow_raw(Decimal::from_f64(frac_height));
            } else if self < one {
                payload = payload
                    .pow_raw(Decimal::from_f64(1.0 - frac_height))
                    .mul_raw(
                        self.pow_raw(payload)
                            .pow_raw(Decimal::from_f64(frac_height)),
                    );
            } else {
                // JS calls layeradd without the linear flag here.
                payload = payload.layer_add_raw(frac_height, self, TetrationMode::Analytic);
            }
            for _ in 0..limit_height {
                let old_payload = payload;
                payload = self.pow_raw(payload);
                if old_payload == payload || payload.has_nan_mag() {
                    return payload;
                }
            }
            if old_height > 10000.0 && old_height.ceil() % 2.0 == 1.0 {
                return self.pow_raw(payload);
            }
            return payload;
        }

        if frac_height != 0.0 {
            if payload == one {
                if self > Decimal::ten() || mode == TetrationMode::Linear {
                    payload = self.pow_raw(Decimal::from_f64(frac_height));
                } else {
                    payload = Decimal::from_f64(tetrate_critical(self.to_number(), frac_height));
                    // The critical-section grid starts at base 2; rescale smaller bases.
                    if self < Decimal::two() {
                        payload = payload.sub_raw(one).mul_raw(self.sub_raw(one)).add_raw(one);
                    }
                }
            } else if self == Decimal::ten() {
                payload = payload.layer_add_10_raw(frac_height, mode);
            } else if self < one {
                payload = payload
                    .pow_raw(Decimal::from_f64(1.0 - frac_height))
                    .mul_raw(
                        self.pow_raw(payload)
                            .pow_raw(Decimal::from_f64(frac_height)),
                    );
            } else {
                payload = payload.layer_add_raw(frac_height, self, mode);
            }
        }

        // `as i64` saturates for absurd heights; the loop bails long before that matters.
        let int_height = height as i64;
        for i in 0..int_height {
            payload = self.pow_raw(payload);
            if !payload.is_finite() {
                return payload;
            }
            // Shortcut: once the payload is 3+ layers above the base, each further
            // exponentiation just adds a layer.
            if payload.layer.saturating_sub(self.layer) > 3 {
                let remaining = int_height - i - 1;
                return Decimal::from_components(
                    payload.sign,
                    payload.layer.saturating_add(remaining),
                    payload.mag,
                );
            }
            if i > 10000 {
                return payload;
            }
        }

        payload
    }

    // -----------------------------------------------------------------------
    // iteratedlog
    // -----------------------------------------------------------------------

    /// Iterated logarithm: the result of applying `log(base)` `times` times in a row.
    ///
    /// Approximately equal to subtracting `times` from the number's `slog` representation.
    /// Equivalent to tetrating to a negative height.
    ///
    /// # Panics
    ///
    /// Panics if the result is undefined. Use [`checked_iteratedlog`](Self::checked_iteratedlog)
    /// for explicit error handling.
    pub fn iteratedlog(&self, base: Decimal, times: f64, mode: TetrationMode) -> Decimal {
        self.checked_iteratedlog(base, times, mode).unwrap_or_else(|e| {
            panic!("undefined Decimal iteratedlog: {e} (self={self:?}, base={base:?}, times={times})")
        })
    }

    /// Iterated logarithm, returning an error if the result is undefined.
    pub fn checked_iteratedlog(
        &self,
        base: Decimal,
        times: f64,
        mode: TetrationMode,
    ) -> Result<Decimal, ArithmeticError> {
        self.iteratedlog_raw(base, times, mode)
            .nan_to_err("iteratedlog")
    }

    pub(crate) fn iteratedlog_raw(self, base: Decimal, times: f64, mode: TetrationMode) -> Decimal {
        if self.has_nan_mag() || base.has_nan_mag() || times.is_nan() {
            return Decimal::nan_sentinel();
        }
        if times < 0.0 {
            return base.tetrate_raw(-times, self, mode);
        }
        if self.is_infinite() {
            return self;
        }

        let mut result = self;
        let full_times = times;
        let mut times = times.trunc();
        let fraction = full_times - times;

        if result.layer - base.layer > 3 {
            let layer_loss = times.min((result.layer - base.layer - 3) as f64);
            times -= layer_loss;
            result.layer -= layer_loss as i64;
        }

        for i in 0..(times as i64) {
            result = result.log_raw(base);
            if !result.is_finite() {
                return result;
            }
            if i > 10000 {
                return result;
            }
        }

        if fraction > 0.0 && fraction < 1.0 {
            if base == Decimal::ten() {
                result = result.layer_add_10_raw(-fraction, mode);
            } else {
                result = result.layer_add_raw(-fraction, base, mode);
            }
        }

        result
    }

    // -----------------------------------------------------------------------
    // slog
    // -----------------------------------------------------------------------

    /// Super-logarithm: the height of the power tower of `base` that equals `self`.
    ///
    /// `base` defaults to 10. The initial estimate from the layer structure is refined against
    /// [`tetrate`](Self::tetrate) with a bracketed secant search (at most 100 probes, usually
    /// under ten) until `tetrate(base, slog(x)) == x` to double precision.
    ///
    /// # Panics
    ///
    /// Panics if the result is undefined (base ≤ 0, base 1, or a value above the fixed point
    /// of a base below `e^(1/e)`). Use [`checked_slog`](Self::checked_slog) for explicit
    /// error handling.
    pub fn slog(&self, base: Option<Decimal>, mode: TetrationMode) -> Decimal {
        let base = base.unwrap_or_else(Decimal::ten);
        self.checked_slog(base, mode).unwrap_or_else(|e| {
            panic!("undefined Decimal slog: {e} (self={self:?}, base={base:?})")
        })
    }

    /// Super-logarithm, returning an error if the result is undefined.
    pub fn checked_slog(
        &self,
        base: Decimal,
        mode: TetrationMode,
    ) -> Result<Decimal, ArithmeticError> {
        if base.sign <= 0 || base == Decimal::one() {
            return Err(ArithmeticError::out_of_domain("slog"));
        }
        self.slog_raw(base, 100, mode).nan_to_err("slog")
    }

    /// Super-logarithm kernel: the layer-structure estimate, refined against `tetrate` with
    /// at most `max_probes` evaluations. See [`refine_slog`].
    pub(crate) fn slog_raw(self, base: Decimal, max_probes: u32, mode: TetrationMode) -> Decimal {
        let initial = self.slog_internal_raw(base, mode);
        if !initial.is_finite() {
            return initial;
        }
        // Below base 1 the only finite estimates are the exact answers 0 (for 1) and -1 (for 0).
        if base < Decimal::one() {
            return initial;
        }
        Decimal::from_f64(refine_slog(
            self,
            base,
            initial.to_number(),
            max_probes,
            mode,
        ))
    }

    /// Initial slog estimate from the layer structure (JS `slog_internal`).
    fn slog_internal_raw(self, base: Decimal, mode: TetrationMode) -> Decimal {
        let one = Decimal::one();
        if self.has_nan_mag() || base.has_nan_mag() {
            return Decimal::nan_sentinel();
        }
        if base.sign <= 0 || base == one {
            return Decimal::nan_sentinel();
        }
        if base < one {
            if self == one {
                return Decimal::zero();
            }
            if self.sign == 0 {
                return Decimal::neg_one();
            }
            return Decimal::nan_sentinel();
        }
        if self.mag < 0.0 || self.sign == 0 {
            return Decimal::neg_one();
        }
        if base < Decimal::from_finite(TETRATION_CONVERGENCE_MAX) {
            let inf_tower = base.lower_fixed_point();
            if self == inf_tower {
                return Decimal::inf();
            }
            if self > inf_tower {
                return Decimal::nan_sentinel();
            }
        }
        if self.is_infinite() {
            return Decimal::inf();
        }

        let mut result: f64 = 0.0;
        let mut copy = self;
        if copy.layer - base.layer > 3 {
            let layer_loss = copy.layer - base.layer - 3;
            result += layer_loss as f64;
            copy.layer -= layer_loss;
        }

        for _ in 0..100 {
            if copy < Decimal::zero() {
                copy = base.pow_raw(copy);
                result -= 1.0;
            } else if copy <= one {
                let frac = match mode {
                    TetrationMode::Linear => copy.to_number() - 1.0,
                    TetrationMode::Analytic => slog_critical(base.to_number(), copy.to_number()),
                };
                return Decimal::from_f64(result + frac);
            } else {
                result += 1.0;
                copy = copy.log_raw(base);
            }
        }

        Decimal::from_f64(result)
    }

    // -----------------------------------------------------------------------
    // layer_add_10 / layer_add / excess_slog
    // -----------------------------------------------------------------------

    /// Adds `diff` to the number's `slog(10)`: for integer `diff` this shifts the layer, and a
    /// fractional residual is interpolated according to `mode`.
    ///
    /// # Panics
    ///
    /// Panics if the result is undefined. Use [`checked_layer_add_10`](Self::checked_layer_add_10)
    /// for explicit error handling.
    pub fn layer_add_10(&self, diff: Decimal, mode: TetrationMode) -> Decimal {
        self.checked_layer_add_10(diff, mode).unwrap_or_else(|e| {
            panic!("undefined Decimal layer_add_10: {e} (self={self:?}, diff={diff:?})")
        })
    }

    /// Adds `diff` to the number's `slog(10)`, returning an error if the result is undefined.
    pub fn checked_layer_add_10(
        &self,
        diff: Decimal,
        mode: TetrationMode,
    ) -> Result<Decimal, ArithmeticError> {
        self.layer_add_10_raw(diff.to_number(), mode)
            .nan_to_err("layer_add_10")
    }

    pub(crate) fn layer_add_10_raw(self, mut diff: f64, mode: TetrationMode) -> Decimal {
        if self.has_nan_mag() || diff.is_nan() {
            return Decimal::nan_sentinel();
        }
        if self.is_infinite() {
            return self;
        }
        let mut result = self;

        if diff >= 1.0 {
            // A "very smol" tower (mag < 0, layer > 0) is effectively zero: zero it first.
            if result.mag < 0.0 && result.layer > 0 {
                result.sign = 0;
                result.mag = 0.0;
                result.layer = 0;
            } else if result.sign == -1 && result.layer == 0 {
                // For inputs like (-3).layer_add_10(1) move the sign onto mag first, so we
                // get 10^-3 rather than -1000.
                result.sign = 1;
                result.mag = -result.mag;
            }
            let layer_add = diff.trunc();
            diff -= layer_add;
            result.layer = result.layer.saturating_add(layer_add as i64);
        }

        if diff <= -1.0 {
            let layer_add = diff.trunc();
            diff -= layer_add;
            result.layer = result.layer.saturating_add(layer_add as i64);
            if result.layer < 0 {
                for _ in 0..100 {
                    result.layer += 1;
                    result.mag = result.mag.log10();
                    if !result.mag.is_finite() {
                        // Hit -inf: the answer is ±infinity, not zero.
                        if result.sign == 0 {
                            result.sign = 1;
                        }
                        if result.layer < 0 {
                            result.layer = 0;
                        }
                        return result.normalize();
                    }
                    if result.layer >= 0 {
                        break;
                    }
                }
            }
        }

        while result.layer < 0 {
            result.layer += 1;
            result.mag = result.mag.log10();
        }
        // If we started with zero we have to repair the layer ourselves before normalizing.
        if result.sign == 0 {
            result.sign = 1;
            if result.mag == 0.0 && result.layer >= 1 {
                result.layer -= 1;
                result.mag = 1.0;
            }
        }
        result.normalize();

        if diff != 0.0 {
            return result.layer_add_raw(diff, Decimal::ten(), mode);
        }

        result
    }

    /// Adds `diff` to the number's `slog(base)` representation.
    ///
    /// # Panics
    ///
    /// Panics if the result is undefined. Use [`checked_layer_add`](Self::checked_layer_add)
    /// for explicit error handling.
    pub fn layer_add(&self, diff: f64, base: Decimal, mode: TetrationMode) -> Decimal {
        self.checked_layer_add(diff, base, mode)
            .unwrap_or_else(|e| {
                panic!(
                    "undefined Decimal layer_add: {e} (self={self:?}, diff={diff}, base={base:?})"
                )
            })
    }

    /// Adds `diff` to the number's `slog(base)` representation, returning an error if the
    /// result is undefined.
    pub fn checked_layer_add(
        &self,
        diff: f64,
        base: Decimal,
        mode: TetrationMode,
    ) -> Result<Decimal, ArithmeticError> {
        self.layer_add_raw(diff, base, mode).nan_to_err("layer_add")
    }

    pub(crate) fn layer_add_raw(self, diff: f64, base: Decimal, mode: TetrationMode) -> Decimal {
        let one = Decimal::one();
        if self.has_nan_mag() || base.has_nan_mag() || diff.is_nan() {
            return Decimal::nan_sentinel();
        }

        if base > one && base <= Decimal::from_finite(TETRATION_CONVERGENCE_MAX) {
            let (slog_this, range) = excess_slog_raw(self, base, mode);
            let slog_dest = slog_this.to_number() + diff;
            let lower = base.lower_fixed_point();
            let upper = base.upper_fixed_point();
            let slog_zero = match range {
                1 => lower.mul_raw(upper).sqrt_raw(),
                2 => upper.mul_raw(Decimal::two()),
                _ => one,
            };
            let slog_one = base.pow_raw(slog_zero);
            let whole_height = slog_dest.floor();
            let frac_height = slog_dest - whole_height;
            let tower_top = slog_zero
                .pow_raw(Decimal::from_f64(1.0 - frac_height))
                .mul_raw(slog_one.pow_raw(Decimal::from_f64(frac_height)));
            return base.tetrate_raw(whole_height, tower_top, mode);
        }

        let slog_this = self.slog_raw(base, 100, mode).to_number();
        let slog_dest = slog_this + diff;
        if slog_dest >= 0.0 {
            return base.tetrate_raw(slog_dest, one, mode);
        }
        if !slog_dest.is_finite() {
            return Decimal::nan_sentinel();
        }
        if slog_dest >= -1.0 {
            return base.tetrate_raw(slog_dest + 1.0, one, mode).log_raw(base);
        }
        base.tetrate_raw(slog_dest + 2.0, one, mode)
            .log_raw(base)
            .log_raw(base)
    }

    // -----------------------------------------------------------------------
    // Super-roots
    // -----------------------------------------------------------------------

    /// Super square root: the `x` such that `x^^2 = x^x = self`.
    ///
    /// Always uses the linear approximation of tetration (see
    /// [`linear_sroot`](Self::linear_sroot)).
    ///
    /// # Panics
    ///
    /// Panics if the result is undefined. Use [`checked_ssqrt`](Self::checked_ssqrt) for
    /// explicit error handling.
    pub fn ssqrt(&self) -> Decimal {
        self.checked_ssqrt()
            .unwrap_or_else(|e| panic!("undefined Decimal ssqrt: {e} (self={self:?})"))
    }

    /// Super square root, returning an error if the result is undefined.
    pub fn checked_ssqrt(&self) -> Result<Decimal, ArithmeticError> {
        self.linear_sroot_raw(2.0).nan_to_err("ssqrt")
    }

    /// Super-root of the given degree: the `x` such that `x^^degree = self`.
    ///
    /// Only the linear approximation of tetration is used, since starting with the analytic
    /// approximation and switching to linear would give inconsistent results. This only
    /// matters for non-integer degrees.
    ///
    /// # Panics
    ///
    /// Panics if the result is undefined (negative inputs, degrees ≤ 0 other than the
    /// special ranges, or values with no real super-root). Use
    /// [`checked_linear_sroot`](Self::checked_linear_sroot) for explicit error handling.
    pub fn linear_sroot(&self, degree: f64) -> Decimal {
        self.checked_linear_sroot(degree).unwrap_or_else(|e| {
            panic!("undefined Decimal linear_sroot: {e} (self={self:?}, degree={degree})")
        })
    }

    /// Super-root of the given degree, returning an error if the result is undefined.
    pub fn checked_linear_sroot(&self, degree: f64) -> Result<Decimal, ArithmeticError> {
        self.linear_sroot_raw(degree).nan_to_err("linear_sroot")
    }

    #[allow(clippy::cognitive_complexity)]
    pub(crate) fn linear_sroot_raw(self, degree: f64) -> Decimal {
        let one = Decimal::one();
        let two = Decimal::two();
        let ten = Decimal::ten();
        let zero = Decimal::zero();
        let linear = TetrationMode::Linear;

        if self.has_nan_mag() || degree.is_nan() {
            return Decimal::nan_sentinel();
        }
        // 1st-degree super root just returns its input.
        if degree == 1.0 {
            return self;
        }
        if self == Decimal::inf() {
            return Decimal::inf();
        }
        if !self.is_finite() {
            return Decimal::nan_sentinel();
        }
        // Using the linear approximation, x^^n = x^n if 0 < n < 1.
        if degree > 0.0 && degree < 1.0 {
            return self.pow_raw(Decimal::from_f64(degree).recip_raw());
        }
        // Under the linear approximation there is a single solution for -2 < degree < -1.
        if degree > -2.0 && degree < -1.0 {
            return Decimal::from_f64(degree + 2.0).pow_raw(self.recip_raw());
        }
        // Degrees in [-1, 0] have no or infinitely many solutions; degrees <= -2 don't work.
        if degree <= 0.0 {
            return Decimal::nan_sentinel();
        }
        // Infinite-degree super-root is x^(1/x) for 1/e <= x <= e, undefined otherwise.
        if degree == f64::INFINITY {
            let this_num = self.to_number();
            if this_num < core::f64::consts::E && this_num > crate::constants::EXPN1 {
                return self.pow_raw(self.recip_raw());
            }
            return Decimal::nan_sentinel();
        }
        // Any super-root of 1 is 1.
        if self == one {
            return one;
        }
        if self < zero {
            return Decimal::nan_sentinel();
        }
        // Treat all numbers of layer <= -2 as zero, because they effectively are.
        if self <= Decimal::from_components(1, 2, -16.0) {
            return if degree % 2.0 == 1.0 {
                self
            } else {
                Decimal::nan_sentinel()
            };
        }

        if self > one {
            // Guess-and-check scaled to the layer of an upper bound: if self > 10^^degree the
            // answer is under iteratedlog(10, degree - 1), otherwise it is under 10.
            let mut upper_bound = ten;
            if self >= ten.tetrate_raw(degree, one, linear) {
                upper_bound = self.iteratedlog_raw(ten, degree - 1.0, linear);
            }
            if degree <= 1.0 {
                upper_bound = self.pow_raw(Decimal::from_f64(degree).recip_raw());
            }
            let mut lower = zero;
            let layer = upper_bound.layer;
            let mut upper = upper_bound.iteratedlog_raw(ten, layer as f64, linear);
            let mut previous = upper;
            let mut guess = upper.div_raw(two);
            let mut loop_going = true;
            let mut guard = 0;
            while loop_going {
                guess = lower.add_raw(upper).div_raw(two);
                if ten
                    .tetrate_raw(layer as f64, guess, linear)
                    .tetrate_raw(degree, one, linear)
                    > self
                {
                    upper = guess;
                } else {
                    lower = guess;
                }
                if guess == previous {
                    loop_going = false;
                } else {
                    previous = guess;
                }
                guard += 1;
                if guard > 10000 {
                    break;
                }
            }
            return ten.tetrate_raw(layer as f64, guess, linear);
        }

        // 0 < self < 1. A tetration of fractional degree can have up to three monotone
        // ranges (see the upstream comments); find the local minimum and maximum, prefer a
        // root in the increasing range, and fall back to the "zero" range. All values here
        // are log10(recip()) of the actual numbers so that layer -1 values stay precise.
        let sentinel = Decimal::from_components(1, 10, 1.0);
        let tet = |x: Decimal| x.tetrate_raw(degree, one, linear);
        let mut stage = 1;
        let mut minimum = sentinel;
        let mut maximum = sentinel;
        let mut lower = sentinel;
        let mut upper = Decimal::from_components(1, 1, -16.0);
        let mut prevspan;
        let mut difference = sentinel;
        let mut upper_bound;
        let mut prev_point;
        let mut next_point;
        let even_degree = degree.ceil() % 2.0 == 0.0;
        let mut range;
        let mut last_valid = sentinel;
        let mut inf_loop_detector;
        let mut previous_upper;
        let mut decreasing_found = false;
        let big = Decimal::from_finite(1e18);
        let mut outer_guard = 0;
        while stage < 4 {
            outer_guard += 1;
            if outer_guard > 100 {
                break;
            }
            if stage == 2 {
                // Minimum found. Even ceiling(degree) means no zero range, so stop here.
                if even_degree {
                    break;
                }
                lower = sentinel;
                upper = minimum;
                stage = 3;
                difference = sentinel;
                last_valid = sentinel;
            }
            inf_loop_detector = false;
            let mut inner_guard = 0;
            while upper != lower {
                inner_guard += 1;
                if inner_guard > 100_000 {
                    break;
                }
                previous_upper = upper;
                let ub = upper.pow10_raw().recip_raw();
                if tet(ub) == one && ub < Decimal::from_finite(0.4) {
                    range = -1;
                    if stage == 3 {
                        last_valid = upper;
                    }
                } else if tet(ub) == ub && !even_degree && ub < Decimal::from_finite(0.4) {
                    range = 0;
                } else if tet(ub) == tet(ub.mul_raw(two)) {
                    // Closer to zero than the next point with a discernible tetration.
                    range = if even_degree { -1 } else { 0 };
                } else {
                    // Approximate derivatives from the neighbouring points.
                    prevspan = upper.mul_raw(Decimal::from_finite(1.2e-16));
                    upper_bound = ub;
                    prev_point = upper.add_raw(prevspan).pow10_raw().recip_raw();
                    let mut distance = upper_bound.sub_raw(prev_point);
                    next_point = upper_bound.add_raw(distance);
                    let mut widen_guard = 0;
                    while tet(prev_point) == tet(upper_bound)
                        || tet(next_point) == tet(upper_bound)
                        || prev_point >= upper_bound
                        || next_point <= upper_bound
                    {
                        prevspan = prevspan.mul_raw(two);
                        prev_point = upper.add_raw(prevspan).pow10_raw().recip_raw();
                        distance = upper_bound.sub_raw(prev_point);
                        next_point = upper_bound.add_raw(distance);
                        widen_guard += 1;
                        if widen_guard > 2000 {
                            break;
                        }
                    }
                    if (stage == 1
                        && tet(next_point) > tet(upper_bound)
                        && tet(prev_point) > tet(upper_bound))
                        || (stage == 3
                            && tet(next_point) < tet(upper_bound)
                            && tet(prev_point) < tet(upper_bound))
                    {
                        last_valid = upper;
                    }
                    if tet(next_point) < tet(upper_bound) {
                        // Derivative is negative: decreasing range.
                        range = -1;
                    } else if even_degree {
                        range = 1;
                    } else if stage == 3 && upper.approx_gt(&minimum, 1e-8) {
                        range = 0;
                    } else {
                        // Widen the bounds until the second derivative is trustworthy.
                        let mut widen_guard = 0;
                        while tet(prev_point).approx_eq(&tet(upper_bound), 1e-8)
                            || tet(next_point).approx_eq(&tet(upper_bound), 1e-8)
                            || prev_point >= upper_bound
                            || next_point <= upper_bound
                        {
                            prevspan = prevspan.mul_raw(two);
                            prev_point = upper.add_raw(prevspan).pow10_raw().recip_raw();
                            distance = upper_bound.sub_raw(prev_point);
                            next_point = upper_bound.add_raw(distance);
                            widen_guard += 1;
                            if widen_guard > 2000 {
                                break;
                            }
                        }
                        if tet(next_point).sub_raw(tet(upper_bound))
                            < tet(upper_bound).sub_raw(tet(prev_point))
                        {
                            // Second derivative negative: zero range.
                            range = 0;
                        } else {
                            range = 1;
                        }
                    }
                }
                if range == -1 {
                    decreasing_found = true;
                }
                if (stage == 1 && range == 1) || (stage == 3 && range != 0) {
                    // The upper bound is too high.
                    if lower == sentinel {
                        upper = upper.mul_raw(two);
                    } else {
                        let cut_off = inf_loop_detector
                            && ((range == 1 && stage == 1) || (range == -1 && stage == 3));
                        upper = upper.add_raw(lower).div_raw(two);
                        if cut_off {
                            break;
                        }
                    }
                } else if lower == sentinel {
                    // Found an actual lower bound.
                    lower = upper;
                    upper = upper.div_raw(two);
                } else {
                    // The upper bound is too low: go to the other half of the range.
                    let cut_off = inf_loop_detector
                        && ((range == 1 && stage == 1) || (range == -1 && stage == 3));
                    lower = lower.sub_raw(difference);
                    upper = upper.sub_raw(difference);
                    if cut_off {
                        break;
                    }
                }
                if lower.sub_raw(upper).div_raw(two).abs()
                    > difference.mul_raw(Decimal::from_finite(1.5))
                {
                    inf_loop_detector = true;
                }
                difference = lower.sub_raw(upper).div_raw(two).abs();
                if upper > big {
                    break;
                }
                if upper == previous_upper {
                    break;
                }
            }
            if upper > big {
                break;
            }
            if !decreasing_found {
                break;
            }
            if last_valid == sentinel {
                // Whatever we're searching for doesn't exist.
                break;
            }
            if stage == 1 {
                minimum = last_valid;
            } else if stage == 3 {
                maximum = last_valid;
            }
            stage += 1;
        }

        // Search the increasing range first.
        let mut lower = minimum;
        let mut upper = Decimal::from_components(1, 1, -18.0);
        let mut previous = upper;
        let mut guess = zero;
        let mut loop_going = true;
        let mut guard = 0;
        while loop_going {
            guess = if lower == sentinel {
                upper.mul_raw(two)
            } else {
                lower.add_raw(upper).div_raw(two)
            };
            if tet(guess.pow10_raw().recip_raw()) > self {
                upper = guess;
            } else {
                lower = guess;
            }
            if guess == previous {
                loop_going = false;
            } else {
                previous = guess;
            }
            if upper > big {
                return Decimal::nan_sentinel();
            }
            guard += 1;
            if guard > 10000 {
                break;
            }
        }
        if !guess.approx_eq(&minimum, 1e-15) {
            return guess.pow10_raw().recip_raw();
        }
        // guess == minimum means no root in the increasing range; try the zero range.
        if maximum == sentinel {
            return Decimal::nan_sentinel();
        }
        let mut lower = sentinel;
        let mut upper = maximum;
        let mut previous = upper;
        let mut guess = zero;
        let mut loop_going = true;
        let mut guard = 0;
        while loop_going {
            guess = if lower == sentinel {
                upper.mul_raw(two)
            } else {
                lower.add_raw(upper).div_raw(two)
            };
            if tet(guess.pow10_raw().recip_raw()) > self {
                upper = guess;
            } else {
                lower = guess;
            }
            if guess == previous {
                loop_going = false;
            } else {
                previous = guess;
            }
            if upper > big {
                return Decimal::nan_sentinel();
            }
            guard += 1;
            if guard > 10000 {
                break;
            }
        }
        guess.pow10_raw().recip_raw()
    }

    // -----------------------------------------------------------------------
    // Pentation
    // -----------------------------------------------------------------------

    /// Pentation: the result of tetrating `height` times in a row. An absurdly strong
    /// operator; `2^^^4.3` and `10^^^2.4` already overflow to infinity.
    ///
    /// `height` defaults to 2 and `payload` to 1. Non-integer heights use a linear
    /// approximation of pentation (there is no analytic one); `mode` still controls the inner
    /// tetrations.
    ///
    /// # Panics
    ///
    /// Panics if the result is undefined. Use [`checked_pentate`](Self::checked_pentate) for
    /// explicit error handling.
    pub fn pentate(
        &self,
        height: Option<f64>,
        payload: Option<Decimal>,
        mode: TetrationMode,
    ) -> Decimal {
        self.checked_pentate(height.unwrap_or(2.0), payload.unwrap_or_else(Decimal::one), mode)
            .unwrap_or_else(|e| {
                panic!(
                    "undefined Decimal pentate: {e} (base={self:?}, height={height:?}, payload={payload:?}, mode={mode:?})"
                )
            })
    }

    /// Pentation, returning an error if the result is undefined.
    pub fn checked_pentate(
        &self,
        height: f64,
        payload: Decimal,
        mode: TetrationMode,
    ) -> Result<Decimal, ArithmeticError> {
        self.pentate_raw(height, payload, mode)
            .nan_to_err("pentate")
    }

    pub(crate) fn pentate_raw(
        self,
        height: f64,
        mut payload: Decimal,
        mode: TetrationMode,
    ) -> Decimal {
        let one = Decimal::one();
        if self.has_nan_mag() || payload.has_nan_mag() || height.is_nan() {
            return Decimal::nan_sentinel();
        }
        let old_height = height;
        let mut height = height.floor();
        let frac_height = old_height - height;
        // Linear approximation for fractional heights.
        if frac_height != 0.0 {
            if payload == one {
                height += 1.0;
                payload = Decimal::from_f64(frac_height);
            } else {
                // penta_log only pentates with payload 1, so this cannot recurse forever.
                let h = payload
                    .penta_log_raw(self, 100, mode)
                    .add_raw(Decimal::from_f64(old_height));
                return self.pentate_raw(h.to_number(), one, mode);
            }
        }

        if height > 0.0 {
            let int_height = height as i64;
            let mut i: i64 = 0;
            let mut prev_payload = Decimal::zero();
            let mut prev_two_payload;
            while i < int_height {
                prev_two_payload = prev_payload;
                prev_payload = payload;
                payload = self.tetrate_raw(payload.to_number(), one, mode);
                i += 1;
                // Once both base and payload are in (0, 1], they stay there and pentation
                // collapses to tetration under the linear approximation.
                if self.sign > 0 && self <= one && payload.sign > 0 && payload <= one {
                    return self.tetrate_raw((int_height - i) as f64, payload, mode);
                }
                // End early once settled on a limit (bases near 0 alternate between two values).
                if payload == prev_payload
                    || (payload == prev_two_payload && i % 2 == int_height % 2)
                {
                    return payload;
                }
                if !payload.is_finite() {
                    return payload;
                }
                if i > 10000 {
                    return payload;
                }
            }
        } else {
            // Negative pentation height is repeated slog.
            let count = (-height) as i64;
            for i in 0..count {
                let prev_payload = payload;
                payload = payload.slog_raw(self, 100, mode);
                if payload == prev_payload {
                    return payload;
                }
                if !payload.is_finite() {
                    return payload;
                }
                if i > 100 {
                    return payload;
                }
            }
        }

        payload
    }

    /// Penta-logarithm: the height you would have to pentate `base` to in order to reach `self`.
    /// Grows incredibly slowly. `base` defaults to 10.
    ///
    /// # Panics
    ///
    /// Panics if the result is undefined (base ≤ 1, or values at or below the pentation limit
    /// between -1 and -2). Use [`checked_penta_log`](Self::checked_penta_log) for explicit
    /// error handling.
    pub fn penta_log(&self, base: Option<Decimal>, mode: TetrationMode) -> Decimal {
        let base = base.unwrap_or_else(Decimal::ten);
        self.checked_penta_log(base, mode).unwrap_or_else(|e| {
            panic!("undefined Decimal penta_log: {e} (self={self:?}, base={base:?})")
        })
    }

    /// Penta-logarithm, returning an error if the result is undefined.
    pub fn checked_penta_log(
        &self,
        base: Decimal,
        mode: TetrationMode,
    ) -> Result<Decimal, ArithmeticError> {
        if base <= Decimal::one() {
            return Err(ArithmeticError::out_of_domain("penta_log"));
        }
        self.penta_log_raw(base, 100, mode).nan_to_err("penta_log")
    }

    pub(crate) fn penta_log_raw(
        self,
        base: Decimal,
        iterations: u32,
        mode: TetrationMode,
    ) -> Decimal {
        let one = Decimal::one();
        if self.has_nan_mag() || base.has_nan_mag() {
            return Decimal::nan_sentinel();
        }
        // Bases below 1 oscillate, so the logarithm doesn't make sense.
        if base <= one {
            return Decimal::nan_sentinel();
        }
        if self == one {
            return Decimal::zero();
        }
        if self == Decimal::inf() {
            return Decimal::inf();
        }
        let mut value = one;
        let mut result: f64 = 0.0;
        let mut step_size: f64 = 1.0;

        // There is some x in (-2, -1) where base^^x == x; that is base^^^(-inf).
        if self < Decimal::neg_one() {
            if self <= Decimal::from_finite(-2.0) {
                return Decimal::nan_sentinel();
            }
            let limit_check = base.tetrate_raw(self.to_number(), one, mode);
            if self == limit_check {
                return Decimal::neg_inf();
            }
            if self > limit_check {
                return Decimal::nan_sentinel();
            }
        }

        if self > one {
            while value < self {
                result += 1.0;
                value = base.tetrate_raw(value.to_number(), one, mode);
                if result > 1000.0 || value.has_nan_mag() {
                    return Decimal::nan_sentinel();
                }
            }
        } else {
            let mut guard = 0;
            while value > self {
                result -= 1.0;
                value = value.slog_raw(base, 100, mode);
                guard += 1;
                if guard > 100 || value.has_nan_mag() {
                    return Decimal::nan_sentinel();
                }
            }
        }
        for _ in 1..iterations {
            let new_decimal = base.pentate_raw(result, one, mode);
            if new_decimal == self {
                break;
            }
            let currently_rose = new_decimal > self;
            step_size = step_size.abs() * if currently_rose { -1.0 } else { 1.0 };
            result += step_size;
            step_size /= 2.0;
            if step_size == 0.0 {
                break;
            }
        }
        Decimal::from_f64(result)
    }

    /// Penta-root of the given degree: the `x` such that `x^^^degree = self`. Always uses the
    /// linear approximation of tetration.
    ///
    /// # Panics
    ///
    /// Panics if the result is undefined. Use
    /// [`checked_linear_penta_root`](Self::checked_linear_penta_root) for explicit error handling.
    pub fn linear_penta_root(&self, degree: f64) -> Decimal {
        self.checked_linear_penta_root(degree).unwrap_or_else(|e| {
            panic!("undefined Decimal linear_penta_root: {e} (self={self:?}, degree={degree})")
        })
    }

    /// Penta-root of the given degree, returning an error if the result is undefined.
    pub fn checked_linear_penta_root(&self, degree: f64) -> Result<Decimal, ArithmeticError> {
        self.linear_penta_root_raw(degree)
            .nan_to_err("linear_penta_root")
    }

    pub(crate) fn linear_penta_root_raw(self, degree: f64) -> Decimal {
        let one = Decimal::one();
        if self.has_nan_mag() || degree.is_nan() {
            return Decimal::nan_sentinel();
        }
        if degree == 1.0 {
            return self;
        }
        if degree < 0.0 {
            return Decimal::nan_sentinel();
        }
        if self == Decimal::inf() {
            return Decimal::inf();
        }
        if !self.is_finite() {
            return Decimal::nan_sentinel();
        }
        // Using the linear approximation, x^^^n = x^n if 0 < n < 1.
        if degree > 0.0 && degree < 1.0 {
            return self.pow_raw(Decimal::from_f64(degree).recip_raw());
        }
        if self == one {
            return one;
        }
        if self < Decimal::zero() {
            return Decimal::nan_sentinel();
        }
        // Below 1 the penta-root and super-root coincide under the linear approximation.
        if self < one {
            return self.linear_sroot_raw(degree);
        }

        InverseSearch::new(move |v: Decimal| v.pentate_raw(degree, one, TetrationMode::Linear))
            .invert_raw(self)
    }
}

// ---------------------------------------------------------------------------
// excess_slog
// ---------------------------------------------------------------------------

/// A strange version of slog for bases between 1 and `e^(1/e)` that can handle values above
/// `base^^inf`. Returns the slog-like value and a range code: 0 below the lower fixed point of
/// `b^x = x` (ordinary slog), 1 between the two fixed points, 2 above the upper one.
/// `log10` applied `depth` times, as an `f64`, extended monotonically: any value that is
/// zero or negative at some step (and so has no further logarithm) maps to `-inf`, since it is
/// below every value that survives all `depth` steps. `NaN` only for the NaN sentinel.
///
/// Peeling `k <= layer` logarithms off a positive-`mag` value is just `layer - k`, so this is
/// O(1) in the layer even when the layer is `9e15`.
fn iterated_log10(v: Decimal, depth: i64) -> f64 {
    if v.has_nan_mag() {
        return f64::NAN;
    }
    if depth == 0 {
        return v.to_number();
    }
    if v.sign <= 0 {
        return f64::NEG_INFINITY;
    }
    if v.layer > 0 && v.mag < 0.0 {
        // Between 0 and 1: one logarithm makes it negative, a second is undefined.
        return if depth == 1 {
            Decimal::from_components(-1, v.layer - 1, -v.mag).to_number()
        } else {
            f64::NEG_INFINITY
        };
    }
    if v.layer >= depth {
        return Decimal::from_components(1, v.layer - depth, v.mag).to_number();
    }
    // `layer` logarithms bring it to a plain float (`mag` itself, or the layer-0 value); the
    // rest are ordinary f64 logs, which hit a non-positive value within a handful of steps.
    let mut x = v.mag;
    let mut remaining = depth - v.layer;
    while remaining > 0 {
        if x <= 0.0 {
            return f64::NEG_INFINITY;
        }
        x = x.log10();
        remaining -= 1;
    }
    x
}

/// Refines a super-logarithm estimate so that `tetrate(base, h) == target` to double
/// precision.
///
/// Upstream walks `h` with a step that doubles until the probe crosses the target and halves
/// afterwards, spending ~100 `tetrate` calls per `slog`. This is a secant search on the same
/// root, safeguarded by bisection once a bracket exists, and typically converges in under ten
/// probes. The residual is measured after `depth` iterated logarithms so it is smooth in `h`
/// across layer changes and never overflows: `depth` is the target's layer plus one (one for a
/// plain float, and one for the tiny values stored with a negative `mag`).
///
/// Negative targets are measured through the negated values, since `tetrate` is negative
/// exactly on heights in `(-2, -1)` and stays monotone there. An infinite residual still has
/// a direction and updates the bracket; only an undefined probe (`tetrate` returned NaN)
/// carries none, and the search halves the step back toward the last finite probe.
/// `max_probes` bounds the total `tetrate` calls.
fn refine_slog(
    target: Decimal,
    base: Decimal,
    estimate: f64,
    max_probes: u32,
    mode: TetrationMode,
) -> f64 {
    let depth = if target.sign == 0 {
        0
    } else if target.layer == 0 || target.mag < 0.0 {
        1
    } else {
        target.layer + 1
    };
    let negative = target.sign < 0;
    let measure = |v: Decimal| -> f64 {
        if negative {
            -iterated_log10(-v, depth)
        } else {
            iterated_log10(v, depth)
        }
    };
    let goal = measure(target);
    if !goal.is_finite() {
        return estimate;
    }
    let one = Decimal::one();
    let probes = core::cell::Cell::new(0u32);
    let probe = |h: f64| -> f64 {
        probes.set(probes.get() + 1);
        measure(base.tetrate_raw(h, one, mode)) - goal
    };

    // The estimate itself can sit where tetrate is undefined; look nearby for solid ground.
    let mut a = estimate;
    let mut fa = probe(a);
    if !fa.is_finite() {
        let mut found = false;
        for offset in [0.001, -0.001, 0.01, -0.01, 0.1, -0.1] {
            fa = probe(estimate + offset);
            if fa.is_finite() {
                a = estimate + offset;
                found = true;
                break;
            }
        }
        if !found {
            return estimate;
        }
    }
    if fa == 0.0 {
        return a;
    }

    // Bracket ends with their residuals: `lo` negative, `hi` positive.
    let mut lo = if fa < 0.0 { Some((a, fa)) } else { None };
    let mut hi = if fa > 0.0 { Some((a, fa)) } else { None };
    // Second point in the direction the residual says the root lies.
    let mut b = a + 0.001 * -fa.signum();
    let mut fb = probe(b);

    while probes.get() < max_probes {
        if fb == 0.0 {
            return b;
        }
        if fb < 0.0 {
            lo = Some((b, fb));
        } else if fb > 0.0 {
            hi = Some((b, fb));
        }

        let bracket = match (lo, hi) {
            (Some((l, _)), Some((h, _))) => Some((l.min(h), l.max(h))),
            _ => None,
        };
        if let (Some((l, fl)), Some((h, fh))) = (lo, hi) {
            // Relative only: heights near 0 (huge bases) still have plenty of resolution.
            if (h - l).abs() <= 2.0 * f64::EPSILON * h.abs().max(l.abs()) {
                // Adjacent floats straddle the root (or the root is below resolution, as for
                // slog(1e-1000) = -1 + 1e-1000): take the side that is closer in residual.
                return if fl.abs() <= fh.abs() { l } else { h };
            }
        }

        let candidate = if fb.is_nan() {
            // No direction from this probe: pull back toward the last finite one.
            0.5 * (a + b)
        } else if fa.is_finite() && fb.is_finite() && fb != fa {
            let secant = b - fb * (b - a) / (fb - fa);
            match bracket {
                Some((l, h)) if !(secant > l && secant < h && secant.is_finite()) => 0.5 * (l + h),
                None if !secant.is_finite() => b + 2.0 * (b - a),
                _ => secant,
            }
        } else {
            match bracket {
                Some((l, h)) => 0.5 * (l + h),
                // Flat, single point, or infinite residual: march the way it points.
                None => b + (b - a).abs().max(0.001) * 2.0 * -fb.signum(),
            }
        };

        if (candidate - b).abs() <= 2.0 * f64::EPSILON * candidate.abs().max(b.abs()) {
            return if fb.is_nan() { candidate } else { b };
        }
        if !fb.is_nan() {
            a = b;
            fa = fb;
        }
        b = candidate;
        fb = probe(b);
    }

    // Out of probes. With a bracket, the side closer in residual is the better answer: for
    // a huge base the root can be 1e-1000, below f64 resolution, and `lo` is then exactly 0.
    match (lo, hi) {
        (Some((l, fl)), Some((h, fh))) => {
            if fl.abs() <= fh.abs() {
                l
            } else {
                h
            }
        }
        _ if fb.is_nan() => a,
        _ => b,
    }
}

fn excess_slog_raw(value: Decimal, base: Decimal, mode: TetrationMode) -> (Decimal, u8) {
    let one = Decimal::one();
    let two = Decimal::two();
    let base_num = base.to_number();
    if base_num == 1.0 || base_num <= 0.0 {
        return (Decimal::nan_sentinel(), 0);
    }
    if base_num > TETRATION_CONVERGENCE_MAX {
        return (value.slog_raw(base, 100, mode), 0);
    }
    let mut lower = base.lower_fixed_point();
    let mut upper = if base_num > 1.0 {
        base.upper_fixed_point()
    } else {
        Decimal::inf()
    };
    if base_num > CONVERGENCE_HOTFIX {
        lower = e();
        upper = e();
    }
    if value < lower {
        return (value.slog_raw(base, 100, mode), 0);
    }
    if value == lower {
        return (Decimal::inf(), 0);
    }
    if value == upper {
        return (Decimal::neg_inf(), 2);
    }

    if value > upper {
        let slog_zero = upper.mul_raw(two);
        let slog_one = base.pow_raw(slog_zero);
        let mut estimate: f64 = 0.0;
        if value >= slog_zero && value < slog_one {
            estimate = 0.0;
        } else if value >= slog_one {
            let mut payload = slog_one;
            estimate = 1.0;
            let mut guard = 0;
            while payload < value {
                payload = base.pow_raw(payload);
                estimate += 1.0;
                if payload.layer > 3 {
                    let layers_left = ((value.layer - payload.layer + 1) as f64).floor();
                    payload = base.tetrate_raw(layers_left, payload, mode);
                    estimate += layers_left;
                }
                guard += 1;
                if guard > 10000 || !payload.is_finite() {
                    return (Decimal::nan_sentinel(), 0);
                }
            }
            if payload > value {
                estimate -= 1.0;
            }
        } else if value < slog_zero {
            let mut payload = slog_zero;
            estimate = 0.0;
            let mut guard = 0;
            while payload > value {
                payload = payload.log_raw(base);
                estimate -= 1.0;
                guard += 1;
                if guard > 10000 || !payload.is_finite() {
                    return (Decimal::nan_sentinel(), 0);
                }
            }
        }
        let mut frac_height: f64 = 0.0;
        let mut step_size: f64 = 0.5;
        let mut guess = Decimal::zero();
        while step_size > 1e-16 {
            let tested = frac_height + step_size;
            // Weighted geometric average.
            let tower_top = slog_zero
                .pow_raw(Decimal::from_f64(1.0 - tested))
                .mul_raw(slog_one.pow_raw(Decimal::from_f64(tested)));
            guess = base.tetrate_raw(estimate, tower_top, TetrationMode::Analytic);
            if guess == value {
                return (Decimal::from_f64(estimate + tested), 2);
            } else if guess < value {
                frac_height += step_size;
            }
            step_size /= 2.0;
        }
        if !guess.approx_eq(&value, 1e-7) {
            return (Decimal::nan_sentinel(), 0);
        }
        return (Decimal::from_f64(estimate + frac_height), 2);
    }

    if value < upper && value > lower {
        // Geometric mean of the two fixed points is arbitrarily chosen as slog 0.
        let slog_zero = lower.mul_raw(upper).sqrt_raw();
        let slog_one = base.pow_raw(slog_zero);
        let mut estimate: f64 = 0.0;
        if value <= slog_zero && value > slog_one {
            estimate = 0.0;
        } else if value <= slog_one {
            let mut payload = slog_one;
            estimate = 1.0;
            let mut guard = 0;
            while payload > value {
                payload = base.pow_raw(payload);
                estimate += 1.0;
                guard += 1;
                if guard > 10000 || !payload.is_finite() {
                    return (Decimal::nan_sentinel(), 0);
                }
            }
            if payload < value {
                estimate -= 1.0;
            }
        } else if value > slog_zero {
            let mut payload = slog_zero;
            estimate = 0.0;
            let mut guard = 0;
            while payload < value {
                payload = payload.log_raw(base);
                estimate -= 1.0;
                guard += 1;
                if guard > 10000 || !payload.is_finite() {
                    return (Decimal::nan_sentinel(), 0);
                }
            }
        }
        let mut frac_height: f64 = 0.0;
        let mut step_size: f64 = 0.5;
        let mut guess = Decimal::zero();
        while step_size > 1e-16 {
            let tested = frac_height + step_size;
            let tower_top = slog_zero
                .pow_raw(Decimal::from_f64(1.0 - tested))
                .mul_raw(slog_one.pow_raw(Decimal::from_f64(tested)));
            guess = base.tetrate_raw(estimate, tower_top, TetrationMode::Analytic);
            if guess == value {
                return (Decimal::from_f64(estimate + tested), 1);
            } else if guess > value {
                frac_height += step_size;
            }
            step_size /= 2.0;
        }
        if !guess.approx_eq(&value, 1e-7) {
            return (Decimal::nan_sentinel(), 0);
        }
        return (Decimal::from_f64(estimate + frac_height), 1);
    }

    let _ = one;
    (Decimal::nan_sentinel(), 0)
}

// ---------------------------------------------------------------------------
// Increasing inverse search
// ---------------------------------------------------------------------------

/// Which family of values the inverse search walks through.
#[derive(Clone, Copy)]
enum SearchRange {
    /// Plain layer-0 values.
    Direct,
    /// `10^v` (layer-1 values).
    Pow10,
    /// `10^^v` (whole layers).
    Tetrate,
    /// `10^^(10^v)` (exponents of layers).
    TetratePow10,
}

/// A numeric inverse of a continuous, strictly monotone `Decimal -> Decimal` function, found
/// by binary search. Port of `Decimal.increasingInverse` from `break_eternity.js`.
///
/// The search automatically picks a scale (plain values, exponents, layers, ...) so it works
/// across the whole representable range. The resulting inverse calls the original function
/// many times, so it is noticeably slower than a closed form.
///
/// If the function is increasing but not strictly, ranges where it is constant map to the
/// value closest to zero. Discontinuous functions may give erroneous results for inputs that
/// are not in the function's range.
///
/// # Example
///
/// ```
/// use break_eternity::{Decimal, InverseSearch};
///
/// let cube = InverseSearch::new(|x: Decimal| x.cube());
/// let root = cube.invert(Decimal::from(27)).unwrap();
/// assert!(root.approx_eq(&Decimal::from(3), 1e-12));
/// ```
pub struct InverseSearch<F> {
    func: F,
    decreasing: bool,
    iterations: u32,
    min_x: Decimal,
    max_x: Decimal,
    min_y: Decimal,
    max_y: Decimal,
}

impl<F: Fn(Decimal) -> Decimal> InverseSearch<F> {
    /// Wraps `func`, assumed strictly increasing over all finite values, with 120 search
    /// iterations (enough to reach full `f64` precision).
    pub fn new(func: F) -> Self {
        let max = Decimal::layer_safe_max();
        Self {
            func,
            decreasing: false,
            iterations: 120,
            min_x: -max,
            max_x: max,
            min_y: -max,
            max_y: max,
        }
    }

    /// Marks the function as strictly decreasing instead of strictly increasing.
    #[must_use]
    pub fn decreasing(mut self, decreasing: bool) -> Self {
        self.decreasing = decreasing;
        self
    }

    /// Sets the number of search iterations before giving up and returning the best estimate.
    #[must_use]
    pub fn iterations(mut self, iterations: u32) -> Self {
        self.iterations = iterations;
        self
    }

    /// Restricts the function's domain: the inverse never evaluates `func` outside
    /// `[min_x, max_x]`.
    #[must_use]
    pub fn domain(mut self, min_x: Decimal, max_x: Decimal) -> Self {
        self.min_x = min_x;
        self.max_x = max_x;
        self
    }

    /// Restricts the function's range: inputs to the inverse outside `[min_y, max_y]` are
    /// reported as out of domain.
    #[must_use]
    pub fn range(mut self, min_y: Decimal, max_y: Decimal) -> Self {
        self.min_y = min_y;
        self.max_y = max_y;
        self
    }

    /// Finds `x` such that `func(x) == value`.
    ///
    /// Returns [`OutOfDomain`](crate::ArithmeticErrorKind::OutOfDomain) if `value` lies outside
    /// the configured range or the search runs off the configured domain.
    pub fn invert(&self, value: Decimal) -> Result<Decimal, ArithmeticError> {
        let r = self.invert_raw(value);
        if r.has_nan_mag() {
            Err(ArithmeticError::out_of_domain("increasing_inverse"))
        } else {
            Ok(r)
        }
    }

    fn apply(range: SearchRange, reciprocal: bool, negative: bool, v: f64) -> Decimal {
        let ten = Decimal::ten();
        let one = Decimal::one();
        let log10_max = f64::MAX.log10();
        let mut r = match range {
            SearchRange::Direct => Decimal::from_f64(v),
            SearchRange::Pow10 => Decimal::from_f64(v).pow10_raw(),
            SearchRange::Tetrate => ten.tetrate_raw(v, one, TetrationMode::Analytic),
            SearchRange::TetratePow10 => {
                if v > log10_max {
                    // Beyond the representable range in this direction.
                    let extreme = if reciprocal {
                        Decimal::zero()
                    } else {
                        Decimal::inf()
                    };
                    return if negative { -extreme } else { extreme };
                }
                ten.tetrate_raw(10.0_f64.powf(v), one, TetrationMode::Analytic)
            }
        };
        if reciprocal {
            r = r.recip_raw();
        }
        if negative {
            r = -r;
        }
        r
    }

    /// Evaluates `func` at `probe` and reports whether `value` is *below* it, flipped for
    /// decreasing functions. `None` means the probe is outside `[min_x, max_x]`.
    fn value_below(&self, probe: Decimal, value: Decimal) -> bool {
        let check = (self.func)(probe);
        let below = value < check;
        if self.decreasing {
            !below
        } else {
            below
        }
    }

    /// Like [`value_below`](Self::value_below) but answers "is `value` above `func(probe)`".
    fn value_above(&self, probe: Decimal, value: Decimal) -> bool {
        let check = (self.func)(probe);
        let above = value > check;
        if self.decreasing {
            !above
        } else {
            above
        }
    }

    /// Decides one branch of the range-selection tree: if the whole domain is on one side of
    /// `limit` the answer is forced, otherwise `func` is probed at `limit`.
    fn decide(
        &self,
        limit: Decimal,
        value: Decimal,
        below_when_max_lt: bool,
        probe_below: bool,
    ) -> bool {
        if self.max_x < limit {
            return below_when_max_lt;
        }
        if self.min_x > limit {
            return !below_when_max_lt;
        }
        if probe_below {
            self.value_below(limit, value)
        } else {
            self.value_above(limit, value)
        }
    }

    pub(crate) fn invert_raw(&self, value: Decimal) -> Decimal {
        let zero = Decimal::zero();
        if value.has_nan_mag()
            || self.max_x < self.min_x
            || value < self.min_y
            || value > self.max_y
        {
            return Decimal::nan_sentinel();
        }

        let exp_limit = Decimal::from_finite(EXPONENT_LIMIT);
        let first_neg = Decimal::from_finite(FIRST_NEG_LAYER);
        let e9e15 = exp_limit.pow10_raw();
        let tower =
            Decimal::ten().tetrate_raw(EXPONENT_LIMIT, Decimal::one(), TetrationMode::Analytic);

        // Is the inverse positive?
        let positive = if self.max_x < zero {
            false
        } else if self.min_x > zero {
            true
        } else {
            let val_check = (self.func)(zero);
            if val_check == value {
                return zero;
            }
            let above = value > val_check;
            if self.decreasing {
                !above
            } else {
                above
            }
        };

        let (range, reciprocal) = if positive {
            // Below 1/9e15?
            if self.decide(first_neg, value, true, true) {
                // Above 1/e9e15?
                if self.decide(e9e15.recip_raw(), value, false, false) {
                    (SearchRange::Pow10, true)
                } else if self.decide(tower.recip_raw(), value, false, false) {
                    (SearchRange::Tetrate, true)
                } else {
                    (SearchRange::TetratePow10, true)
                }
            } else if self.decide(exp_limit, value, true, true) {
                (SearchRange::Direct, false)
            } else if self.decide(e9e15, value, true, true) {
                (SearchRange::Pow10, false)
            } else if self.decide(tower, value, true, true) {
                (SearchRange::Tetrate, false)
            } else {
                (SearchRange::TetratePow10, false)
            }
        } else {
            // Above -1/9e15?
            if self.decide(-first_neg, value, false, false) {
                // Below -1/e9e15?
                if self.decide(-e9e15.recip_raw(), value, true, true) {
                    (SearchRange::Pow10, true)
                } else if self.decide(-tower.recip_raw(), value, true, true) {
                    (SearchRange::Tetrate, true)
                } else {
                    (SearchRange::TetratePow10, true)
                }
            } else if self.decide(-exp_limit, value, false, false) {
                (SearchRange::Direct, false)
            } else if self.decide(-e9e15, value, false, false) {
                (SearchRange::Pow10, false)
            } else if self.decide(-tower, value, false, false) {
                (SearchRange::Tetrate, false)
            } else {
                (SearchRange::TetratePow10, false)
            }
        };
        let negative = !positive;

        let search_increasing = (positive != reciprocal) != self.decreasing;

        let mut step_size = 0.001_f64;
        let mut has_changed_directions_once = false;
        let mut previously_rose = false;
        let mut result = 1.0_f64;
        for i in 1..self.iterations {
            let mut critical = false;
            let old_result = result;
            let mut applied = Self::apply(range, reciprocal, negative, result);
            // Never call func outside its domain.
            if applied > self.max_x {
                applied = self.max_x;
                critical = true;
            }
            if applied < self.min_x {
                applied = self.min_x;
                critical = true;
            }
            let new_decimal = (self.func)(applied);
            if new_decimal == value && !critical {
                break;
            }
            let currently_rose = if search_increasing {
                new_decimal > value
            } else {
                new_decimal < value
            };
            if i > 1 && previously_rose != currently_rose {
                has_changed_directions_once = true;
            }
            previously_rose = currently_rose;
            if has_changed_directions_once {
                step_size /= 2.0;
            } else {
                step_size *= 2.0;
            }
            // Trying to leave the domain: no inverse exists.
            if (currently_rose != search_increasing && applied == self.max_x)
                || (currently_rose == search_increasing && applied == self.min_x)
            {
                return Decimal::nan_sentinel();
            }
            step_size = step_size.abs() * if currently_rose { -1.0 } else { 1.0 };
            result += step_size;
            if step_size == 0.0 || old_result == result {
                break;
            }
        }
        Self::apply(range, reciprocal, negative, result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The refinement makes slog the inverse of tetrate to double precision, for every
    /// target shape (plain, layer 1..3, tiny, negative) and both modes.
    #[test]
    fn slog_inverts_tetrate() {
        let bases = [
            Decimal::two(),
            Decimal::from_finite(core::f64::consts::E),
            Decimal::from_finite(3.5),
            Decimal::ten(),
            Decimal::from_finite(1e10),
        ];
        let targets: [Decimal; 11] = [
            "0.5",
            "1",
            "2",
            "10",
            "12345.678",
            "1e10",
            "1e100",
            "1e1e15",
            "10^^3",
            "10^^4",
            "-1",
        ]
        .map(|s| s.parse().unwrap());
        for mode in [TetrationMode::Analytic, TetrationMode::Linear] {
            for base in bases {
                for x in targets {
                    let Ok(h) = x.checked_slog(base, mode) else {
                        continue;
                    };
                    let back = base.tetrate(Some(h.to_number()), None, mode);
                    // One ulp of a height near 2 moves a layer-1 mag by ~1e-12 relative; for
                    // a negative target the height sits at -2 + tiny and loses more.
                    let tol = if x.is_negative() { 1e-6 } else { 1e-9 };
                    assert!(
                        back.approx_eq(&x, tol),
                        "slog_{base:?}({x:?}) = {h:?} ({mode:?}) but tetrate gives {back:?}"
                    );
                }
            }
        }
        // Zero is exact: slog(1) is 0 and slog(0) is -1.
        assert_eq!(
            Decimal::one().slog(None, TetrationMode::Analytic),
            Decimal::zero()
        );
        assert_eq!(
            Decimal::zero().slog(None, TetrationMode::Linear),
            Decimal::neg_one()
        );
        // Roots below f64 resolution land on the boundary rather than short of it.
        for s in ["1e-20", "1e-1000"] {
            let tiny: Decimal = s.parse().unwrap();
            for mode in [TetrationMode::Analytic, TetrationMode::Linear] {
                let h = tiny.slog(None, mode).to_number();
                assert!((-1.0..-1.0 + 1e-15).contains(&h), "slog({s}) = {h}");
            }
        }
        // A huge base: slog(10) is 1e-1000, i.e. 0, and layer_add(1) gives back the base.
        let base: Decimal = "ee1000".parse().unwrap();
        assert_eq!(
            Decimal::ten().slog(Some(base), TetrationMode::Analytic),
            Decimal::zero()
        );
        assert_eq!(
            Decimal::ten().layer_add(1.0, base, TetrationMode::Analytic),
            base
        );
    }

    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    #[test]
    fn integer_tetration() {
        let two = Decimal::two();
        assert_eq!(
            two.tetrate(Some(3.0), None, TetrationMode::Analytic),
            Decimal::from(16)
        );
        assert_eq!(
            two.tetrate(Some(4.0), None, TetrationMode::Analytic),
            Decimal::from(65536)
        );
        assert_eq!(
            Decimal::ten().tetrate(Some(1.0), None, TetrationMode::Analytic),
            Decimal::ten()
        );
        assert_eq!(
            Decimal::ten().tetrate(Some(0.0), None, TetrationMode::Analytic),
            Decimal::one()
        );
    }

    #[test]
    fn convergence_zone_fractional_heights() {
        // Reference values from break_eternity.js 2.1.3 (see issue #18).
        let cases = [
            ("0", 0.5),
            ("0.5", 0.6540408600420695),
            ("0.1", 0.3289989197992152),
            ("1e-10", 1.0053153644435384e-10),
            ("1e-300", 1e-300),
        ];
        for (base, expected) in cases {
            let got = d(base).tetrate(Some(2.5), None, TetrationMode::Analytic);
            assert!(
                got.approx_eq(&Decimal::from_finite(expected), 1e-8),
                "tetrate({base}, 2.5) = {got}, expected {expected}"
            );
        }
    }

    #[test]
    fn infinite_height_fixed_points() {
        let sqrt2 = Decimal::from_finite(core::f64::consts::SQRT_2);
        let t = sqrt2.tetrate(Some(f64::INFINITY), None, TetrationMode::Analytic);
        assert!(t.approx_eq(&Decimal::two(), 1e-9), "{t}");
        // Above the upper fixed point (4) the tower diverges.
        let above = sqrt2.tetrate(
            Some(f64::INFINITY),
            Some(Decimal::from(5)),
            TetrationMode::Analytic,
        );
        assert_eq!(above, Decimal::inf());
        assert_eq!(
            Decimal::two().tetrate(Some(f64::INFINITY), None, TetrationMode::Analytic),
            Decimal::inf()
        );
        assert!(Decimal::from_finite(0.01)
            .checked_tetrate(f64::INFINITY, Decimal::one(), TetrationMode::Analytic)
            .is_err());
    }

    #[test]
    fn layer_overflow_saturates_to_infinity() {
        assert_eq!(
            Decimal::ten().tetrate(Some(1e19), None, TetrationMode::Analytic),
            Decimal::inf()
        );
        assert_eq!(
            Decimal::ten().tetrate(Some(1e300), None, TetrationMode::Analytic),
            Decimal::inf()
        );
        assert_eq!(
            Decimal::ten().pentate(Some(2.5), None, TetrationMode::Analytic),
            Decimal::inf()
        );
        assert_eq!(
            Decimal::inf().tetrate(Some(3.0), None, TetrationMode::Analytic),
            Decimal::inf()
        );
        assert_eq!(
            Decimal::inf().slog(None, TetrationMode::Analytic),
            Decimal::inf()
        );
    }

    #[test]
    fn checked_forms_do_not_leak_nan() {
        assert!(Decimal::two()
            .checked_tetrate(-2.0, Decimal::one(), TetrationMode::Analytic)
            .is_err());
        assert!(Decimal::from(-2)
            .checked_tetrate(2.5, Decimal::one(), TetrationMode::Analytic)
            .is_err());
        assert!(d("1e1000")
            .checked_slog(Decimal::one(), TetrationMode::Analytic)
            .is_err());
        assert!(Decimal::neg_one().checked_ssqrt().is_err());
        assert!(Decimal::ten()
            .checked_slog(Decimal::zero(), TetrationMode::Analytic)
            .is_err());
    }

    #[test]
    fn slog_and_tetrate_round_trip() {
        for h in [0.5_f64, 1.25, 2.0, 2.75, 3.5] {
            let t = Decimal::ten().tetrate(Some(h), None, TetrationMode::Analytic);
            let s = t.slog(None, TetrationMode::Analytic).to_number();
            assert!((s - h).abs() < 1e-8, "slog(10^^{h}) = {s}");
        }
        let t = Decimal::from_finite(1.7).tetrate(Some(2.5), None, TetrationMode::Analytic);
        let s = t
            .slog(Some(Decimal::from_finite(1.7)), TetrationMode::Analytic)
            .to_number();
        assert!((s - 2.5).abs() < 1e-7, "slog_1.7 = {s}");
    }

    #[test]
    fn super_roots() {
        assert!(Decimal::from(256)
            .ssqrt()
            .approx_eq(&Decimal::from(4), 1e-9));
        assert!(Decimal::from(27).ssqrt().approx_eq(&Decimal::from(3), 1e-9));
        let big = d("1e100");
        let r = big.ssqrt();
        assert!(r.pow(r).approx_eq(&big, 1e-6), "{r}");
        // x^^3 = 3^3^3 = 7625597484987
        let r3 = d("7625597484987").linear_sroot(3.0);
        assert!(r3.approx_eq(&Decimal::from(3), 1e-9), "{r3}");
        // Values below 1 have a real ssqrt down to (1/e)^(1/e).
        let small = Decimal::from_finite(0.8);
        let rs = small.ssqrt();
        assert!(rs.pow(rs).approx_eq(&small, 1e-6), "{rs}");
        assert!(Decimal::from_finite(0.5).checked_ssqrt().is_err());
    }

    #[test]
    fn pentation_and_inverses() {
        assert_eq!(
            Decimal::two().pentate(Some(2.0), None, TetrationMode::Analytic),
            Decimal::from(4)
        );
        assert_eq!(
            Decimal::two().pentate(Some(3.0), None, TetrationMode::Analytic),
            Decimal::from(65536)
        );
        let p = Decimal::two().pentate(Some(3.0), None, TetrationMode::Analytic);
        let l = p.penta_log(Some(Decimal::two()), TetrationMode::Analytic);
        assert!(l.approx_eq(&Decimal::from(3), 1e-9), "{l}");
        let frac = Decimal::two().pentate(Some(2.5), None, TetrationMode::Linear);
        let back = frac.penta_log(Some(Decimal::two()), TetrationMode::Linear);
        assert!(back.approx_eq(&Decimal::from_finite(2.5), 1e-6), "{back}");
        let r = Decimal::from(65536).linear_penta_root(3.0);
        assert!(r.approx_eq(&Decimal::two(), 1e-9), "{r}");
        assert!(Decimal::two()
            .checked_penta_log(Decimal::one(), TetrationMode::Analytic)
            .is_err());
    }

    #[test]
    fn increasing_inverse_across_scales() {
        let square = InverseSearch::new(|x: Decimal| x.sqr())
            .domain(Decimal::zero(), Decimal::layer_safe_max());
        assert!(square
            .invert(Decimal::from(49))
            .unwrap()
            .approx_eq(&Decimal::from(7), 1e-12));
        let huge = d("1e400");
        assert!(square.invert(huge).unwrap().approx_eq(&d("1e200"), 1e-9));
        let tiny = d("1e-400");
        assert!(square.invert(tiny).unwrap().approx_eq(&d("1e-200"), 1e-9));
        let neg = InverseSearch::new(|x: Decimal| -x).decreasing(true);
        assert!(neg
            .invert(Decimal::from(5))
            .unwrap()
            .approx_eq(&Decimal::from(-5), 1e-12));
        assert!(square.invert(Decimal::from(-1)).is_err());
    }

    #[test]
    fn layer_add_on_small_bases() {
        // Adding then removing layers is the identity, also for bases in (1, e^(1/e)].
        let base = Decimal::from_finite(1.3);
        let x = Decimal::from_finite(1.5);
        let y = x.layer_add(1.0, base, TetrationMode::Analytic);
        assert!(y.approx_eq(&base.pow(x), 1e-6), "{y} vs {}", base.pow(x));
        let back = y.layer_add(-1.0, base, TetrationMode::Analytic);
        assert!(back.approx_eq(&x, 1e-6), "{back}");
        // Above the upper fixed point the excess_slog machinery is used.
        let big = Decimal::from(100);
        let up = big.layer_add(1.0, base, TetrationMode::Analytic);
        assert!(up.approx_eq(&base.pow(big), 1e-6), "{up}");
    }
}
