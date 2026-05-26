//! Tetration and related hyperoperations: tetrate, pentate, slog, ssqrt,
//! iteratedexp, iteratedlog, `layer_add_10`, `layer_add`.

use std::ops::Neg;

use crate::critical_section::{slog_critical, tetrate_critical};
use crate::decimal::Decimal;
use crate::error::ArithmeticError;

/// Algorithm choice for the fractional-height path of tetration and its
/// inverse, super-logarithm. Matches the `linear` flag in
/// `break_eternity.js`.
///
/// `Analytic` (the default) uses the JS critical-section interpolation table
/// for bases in `[2, 10]`. `Linear` uses the older closed-form approximations.
/// Bases above 10 always fall back to linear regardless of mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TetrationMode {
    /// JS-default critical-section interpolation. Matches `break_eternity.js`.
    #[default]
    Analytic,
    /// Older linear approximation. Equivalent to JS `linear=true`.
    Linear,
}

impl Decimal {
    /// Tetrates the Decimal to the given height.
    ///
    /// Source: <https://andydude.github.io/tetration/archives/tetration2/ident.html>
    ///
    /// # Panics
    ///
    /// Panics if `lambertw` is out of domain (infinite height case). Use
    /// [`checked_tetrate`](Self::checked_tetrate) for explicit error handling.
    pub fn tetrate(
        &self,
        height: Option<f64>,
        payload: Option<Decimal>,
        mode: TetrationMode,
    ) -> Decimal {
        self.checked_tetrate(
            height.unwrap_or(2.0),
            payload.unwrap_or_else(|| Decimal::from_components_unchecked(1, 0, 1.0)),
            mode,
        )
        .unwrap_or_else(|e| {
            panic!(
                "undefined Decimal tetrate: {e} (base={self:?}, height={height:?}, payload={payload:?}, mode={mode:?})"
            )
        })
    }

    /// Tetrates the Decimal to the given height, returning an error on failure.
    ///
    /// `mode` selects between the JS-default analytic critical-section path
    /// and the older linear approximation for the fractional-height branch.
    /// See [`TetrationMode`].
    pub fn checked_tetrate(
        &self,
        mut height: f64,
        mut payload: Decimal,
        mode: TetrationMode,
    ) -> Result<Decimal, ArithmeticError> {
        if height.is_infinite() && height.is_sign_positive() {
            // Branch on base — matches JS lines 2588-2613. The lambertw
            // shortcut only applies in the convergence zone where the
            // tower has a fixed point.
            let this_num = self.to_number();
            if this_num > 1.444_667_861_009_766_2 {
                // Outside convergence; tower diverges.
                return Ok(Decimal::inf());
            }
            if this_num < 0.065_988_035_845_312_54 {
                // Either oscillates without converging (0..0.066) or
                // would yield complex values (this_num < 0).
                return Ok(Decimal::nan_sentinel());
            }
            let neg_ln = self.ln().neg();
            let w = neg_ln.checked_lambertw()?;
            return Ok(w / neg_ln);
        }

        if height < 0.0 {
            return Ok(payload.iteratedlog(*self, -height, mode));
        }

        let old_height = height;
        height = height.trunc();
        let fract_height = old_height - height;

        if fract_height != 0.0 {
            if payload == Decimal::one() {
                let base_num = self.to_number();
                if matches!(mode, TetrationMode::Analytic) && base_num <= 10.0 {
                    // Analytic: pre-interpolate to the fractional height.
                    // Don't increment `height`; the integer-step loop runs
                    // `height` more times.
                    payload = Decimal::from_finite(tetrate_critical(base_num, fract_height));
                } else {
                    // Linear: payload = base^fract_height, accomplished by
                    // bumping height and letting the loop run once.
                    height += 1.0;
                    payload = Decimal::from_finite(fract_height);
                }
            } else if *self == Decimal::from_finite(10.0) {
                payload = payload.layer_add_10(Decimal::from_finite(fract_height), mode);
            } else {
                payload = payload.layer_add(fract_height, *self, mode);
            }
        }

        for i in 0..height as i64 {
            payload = self.pow(payload);
            // bail if we've hit a non-finite sentinel
            if !payload.mag.is_finite() {
                return Ok(payload);
            }

            if payload.layer - self.layer > 3 {
                return Ok(Decimal::from_components_unchecked(
                    payload.sign,
                    payload.layer + (height as i64 - i - 1),
                    payload.mag,
                ));
            }

            if i > 100 {
                return Ok(payload);
            }
        }

        Ok(payload)
    }

    /// Returns the Decimal, iteratively exponentiated.
    ///
    /// Equates to tetrating to the same height.
    ///
    /// # Deprecated
    ///
    /// Use [`tetrate`](Self::tetrate); `iteratedexp` is identical.
    #[deprecated(note = "use tetrate; iteratedexp is identical")]
    pub fn iteratedexp(
        &self,
        height: Option<f64>,
        payload: Option<Decimal>,
        mode: TetrationMode,
    ) -> Decimal {
        self.tetrate(height, payload, mode)
    }

    /// Returns `self` iteratively exponentiated, returning an error on failure.
    ///
    /// Equates to [`checked_tetrate`](Self::checked_tetrate).
    pub fn checked_iteratedexp(
        &self,
        height: f64,
        payload: Decimal,
        mode: TetrationMode,
    ) -> Result<Decimal, ArithmeticError> {
        self.checked_tetrate(height, payload, mode)
    }

    /// Iterated log: The result of applying log(base) 'times' times in a row.
    ///
    /// Approximately equal to subtracting (times) from the number's slog representation.
    /// Equates to tetrating to a negative height.
    pub fn iteratedlog(&self, base: Decimal, mut times: f64, mode: TetrationMode) -> Decimal {
        if times < 0.0 {
            return base
                .checked_tetrate(-times, *self, mode)
                .unwrap_or_else(|_| Decimal::nan_sentinel());
        }

        let mut result = *self;
        let full_times = times;
        times = times.trunc();
        let fraction = full_times - times;

        if result.layer - base.layer > 3 {
            let layer_loss = times.min((result.layer - base.layer - 3) as f64);
            times -= layer_loss;
            result.layer -= layer_loss as i64;
        }

        for i in 0..times as i64 {
            result = result.log(base);
            if !result.mag.is_finite() {
                return result;
            }
            if i > 100 {
                return result;
            }
        }

        if fraction > 0.0 && fraction < 1.0 {
            if base == Decimal::from_finite(10.0) {
                result = result.layer_add_10(Decimal::from_finite(-fraction), mode);
            } else {
                result = result.layer_add(-fraction, base, mode);
            }
        }

        result
    }

    /// Iterated log returning an error on failure.
    pub fn checked_iteratedlog(
        &self,
        base: Decimal,
        times: f64,
        mode: TetrationMode,
    ) -> Result<Decimal, ArithmeticError> {
        Ok(self.iteratedlog(base, times, mode))
    }

    /// Returns the super-logarithm of the Decimal.
    pub fn slog(&self, base_opt: Option<Decimal>, mode: TetrationMode) -> Decimal {
        self.checked_slog(base_opt.map_or(10.0, |b| b.to_number()), mode)
            .unwrap_or_else(|e| panic!("undefined Decimal slog: {e} (self={self:?})"))
    }

    /// Returns the super-logarithm, returning an error on failure.
    ///
    /// `mode` selects between the JS-default analytic critical-section
    /// interpolation (with a 100-iteration refinement wrapper) and the older
    /// linear closed-form approximation. See [`TetrationMode`].
    pub fn checked_slog(
        &self,
        base_f64: f64,
        mode: TetrationMode,
    ) -> Result<Decimal, ArithmeticError> {
        let initial = self.slog_internal(base_f64, mode).to_number();

        if !initial.is_finite() {
            return Ok(Decimal::from_finite(initial));
        }

        // JS slog refinement (break_eternity.js lines 2793-2830): start from
        // slog_internal, take a step ±0.001, double the step until tetrate(r)
        // overshoots, then halve. 100 iterations.
        let base = Decimal::from_finite(base_f64);
        let one = Decimal::from_finite(1.0);
        let mut step_size = 0.001_f64;
        let mut has_changed_directions_once = false;
        let mut previously_rose = false;
        let mut result = initial;

        for i in 1..100 {
            let new_decimal = base
                .checked_tetrate(result, one, mode)
                .unwrap_or_else(|_| Decimal::nan_sentinel());

            // If tetrate produced a non-finite sentinel, we've stepped into a
            // region the chain can't represent. Preserve the most recent
            // finite estimate and stop refining.
            if !new_decimal.mag().is_finite() {
                break;
            }

            let currently_rose = new_decimal > *self;

            if i > 1 && previously_rose != currently_rose {
                has_changed_directions_once = true;
            }
            previously_rose = currently_rose;

            if has_changed_directions_once {
                step_size /= 2.0;
            } else {
                step_size *= 2.0;
            }

            step_size = step_size.abs() * if currently_rose { -1.0 } else { 1.0 };
            result += step_size;

            if step_size == 0.0 {
                break;
            }
        }

        Ok(Decimal::from_finite(result))
    }

    /// Inner slog kernel — JS `slog_internal`. Returns an initial estimate
    /// that [`checked_slog`](Self::checked_slog) refines further.
    fn slog_internal(&self, base_f64: f64, mode: TetrationMode) -> Decimal {
        if self.mag < 0.0 {
            return Decimal::neg_one();
        }

        let mut result: f64 = 0.0;
        let base = Decimal::from_finite(base_f64);
        let mut copy = *self;

        if copy.layer - base.layer > 3 {
            let layer_loss = copy.layer - base.layer - 3;
            result += layer_loss as f64;
            copy.layer -= layer_loss;
        }

        for _ in 0..100 {
            if copy < Decimal::zero() {
                copy = base.pow(copy);
                result -= 1.0;
            }

            if copy <= Decimal::one() {
                let frac = match mode {
                    TetrationMode::Linear => copy.to_number() - 1.0,
                    TetrationMode::Analytic => slog_critical(base_f64, copy.to_number()),
                };
                return Decimal::from_finite(result + frac);
            }

            result += 1.0;
            copy = copy.log(base);
        }

        Decimal::from_finite(result)
    }

    /// Adds or removes layers from a Decimal.
    ///
    /// `mode` selects the fractional-residual algorithm: `Analytic` recurses
    /// through [`layer_add`](Self::layer_add) to reach the JS-equivalent
    /// critical-section path, `Linear` uses the inline closed-form math.
    pub fn layer_add_10(&self, diff: Decimal, mode: TetrationMode) -> Decimal {
        let mut diff = diff.to_number();
        let mut result = *self;

        if diff >= 1.0 {
            // Bugfix A (JS): if `result` is a "very smol" tower (mag < 0,
            // layer > 0), zero it before layer-bumping up.
            if result.mag < 0.0 && result.layer > 0 {
                result.sign = 0;
                result.mag = 0.0;
                result.layer = 0;
            } else if result.sign == -1 && result.layer == 0 {
                // Bugfix B (JS): for inputs like `-3.layer_add_10(1)` move
                // the sign onto mag before bumping layer, so we get
                // 10^(-3) = 0.001 rather than -1000.
                result.sign = 1;
                result.mag = -result.mag;
            }

            let layer_add = diff.trunc();
            diff -= layer_add;
            result.layer += layer_add as i64;
        }

        if diff <= -1.0 {
            let layer_add = diff.trunc();
            diff -= layer_add;
            result.layer += layer_add as i64;
            if result.layer < 0 {
                for _ in 0..100 {
                    result.layer += 1;
                    result.mag = result.mag.log10();
                    if !result.mag.is_finite() {
                        // Bugfix C (JS): mag is -inf — produce ±infinity
                        // rather than returning an unnormalized sentinel.
                        if result.sign == 0 {
                            result.sign = 1;
                        }
                        if result.layer < 0 {
                            result.layer = 0;
                        }
                        result.normalize();
                        return result;
                    }

                    if result.layer >= 0 {
                        break;
                    }
                }
            }
        }

        match mode {
            TetrationMode::Linear => {
                // Existing inline math for the fractional residual.
                if diff > 0.0 {
                    let mut subtract_layers_later: i64 = 0;
                    while result.mag.is_finite() && result.mag < 10.0 {
                        result.mag = 10.0_f64.powf(result.mag);
                        subtract_layers_later += 1;
                    }

                    if result.mag > 1e10_f64 {
                        result.mag = result.mag.log10();
                        result.layer += 1;
                    }

                    let diff_to_next_slog = (1e10_f64.ln() / result.mag.ln()).log10();
                    if diff_to_next_slog < diff {
                        result.mag = 1e10_f64.log10();
                        result.layer += 1;
                        diff -= diff_to_next_slog;
                    }

                    result.mag = result.mag.powf(10.0_f64.powf(diff));

                    while subtract_layers_later > 0 {
                        result.mag = result.mag.log10();
                        subtract_layers_later -= 1;
                    }
                }

                if diff < 0.0 {
                    let mut subtract_layers_later: i64 = 0;

                    while result.mag.is_finite() && result.mag < 10.0 {
                        result.mag = 10.0_f64.powf(result.mag);
                        subtract_layers_later += 1;
                    }

                    if result.mag > 1e10_f64 {
                        result.mag = result.mag.log10();
                        result.layer += 1;
                    }

                    let diff_to_next_slog = (1.0 / result.mag.log10()).log10();
                    if diff_to_next_slog > diff {
                        result.mag = 1e10_f64;
                        result.layer -= 1;
                        diff -= diff_to_next_slog;
                    }

                    result.mag = result.mag.powf(10.0_f64.powf(diff));

                    while subtract_layers_later > 0 {
                        result.mag = result.mag.log10();
                        subtract_layers_later -= 1;
                    }
                }

                while result.layer < 0 {
                    result.layer += 1;
                    result.mag = result.mag.log10();
                }

                // Bugfix D (JS): if we entered with sign=0 the layer-bumping
                // arithmetic above leaves a sign=0 result whose mag/layer
                // would otherwise be wrong after normalize. Snap to sign=1
                // and collapse a stray (mag=0, layer>=1) state.
                if result.sign == 0 {
                    result.sign = 1;
                    if result.mag == 0.0 && result.layer >= 1 {
                        result.layer -= 1;
                        result.mag = 1.0;
                    }
                }

                result.normalize();
                result
            }
            TetrationMode::Analytic => {
                // JS-style: normalize, then recurse into layer_add for the
                // fractional residual. This routes back through slog/tetrate
                // so the analytic critical-section path is reached.
                while result.layer < 0 {
                    result.layer += 1;
                    result.mag = result.mag.log10();
                }

                // Bugfix D (JS): same as Linear branch above.
                if result.sign == 0 {
                    result.sign = 1;
                    if result.mag == 0.0 && result.layer >= 1 {
                        result.layer -= 1;
                        result.mag = 1.0;
                    }
                }

                result.normalize();
                if diff != 0.0 {
                    return result.layer_add(diff, Decimal::from_finite(10.0), mode);
                }
                result
            }
        }
    }

    /// Adds `diff` to the Decimal's slog(base) representation.
    ///
    /// `mode` propagates into the inner slog and tetrate calls. With
    /// `Analytic`, the operation matches JS `layeradd(diff, base, false)`.
    pub fn layer_add(&self, diff: f64, base: Decimal, mode: TetrationMode) -> Decimal {
        let slog_this = self.slog(Some(base), mode).to_number();
        let slog_dest = slog_this + diff;

        if slog_dest >= 0.0 {
            return base
                .checked_tetrate(slog_dest, Decimal::one(), mode)
                .unwrap_or_else(|_| Decimal::nan_sentinel());
        }

        if !slog_dest.is_finite() {
            return Decimal::nan_sentinel();
        }

        if slog_dest >= -1.0 {
            return base
                .checked_tetrate(slog_dest + 1.0, Decimal::one(), mode)
                .unwrap_or_else(|_| Decimal::nan_sentinel())
                .log(base);
        }

        base.checked_tetrate(slog_dest + 2.0, Decimal::one(), mode)
            .unwrap_or_else(|_| Decimal::nan_sentinel())
            .log(base)
            .log(base)
    }

    /// Returns the super square root of the Decimal.
    ///
    /// Essentially "what number, tetrated to height 2, equals this?"
    ///
    /// # Panics
    ///
    /// Panics if `lambertw` is out of domain. Use [`checked_ssqrt`](Self::checked_ssqrt) instead.
    pub fn ssqrt(&self) -> Decimal {
        self.checked_ssqrt()
            .unwrap_or_else(|e| panic!("undefined Decimal ssqrt: {e} (self={self:?})"))
    }

    /// Returns the super square root, returning an error on failure.
    pub fn checked_ssqrt(&self) -> Result<Decimal, ArithmeticError> {
        if self.sign == 1 && self.layer >= 3 {
            return Ok(Decimal::from_components_unchecked(
                self.sign,
                self.layer - 1,
                self.mag,
            ));
        }

        let ln_x = self.ln();
        let w = ln_x.checked_lambertw()?;
        Ok(ln_x / w)
    }

    /// The result of tetrating the Decimal `height` times in a row.
    ///
    /// # Panics
    ///
    /// Panics if an inner [`tetrate`](Self::tetrate) is out of domain.
    /// Use [`checked_pentate`](Self::checked_pentate) for explicit error handling.
    pub fn pentate(
        &self,
        height: Option<f64>,
        payload: Option<Decimal>,
        mode: TetrationMode,
    ) -> Decimal {
        self.checked_pentate(
            height.unwrap_or(2.0),
            payload.unwrap_or_else(|| Decimal::from_components_unchecked(1, 0, 1.0)),
            mode,
        )
        .unwrap_or_else(|e| {
            panic!(
                "undefined Decimal pentate: {e} (base={self:?}, height={height:?}, payload={payload:?}, mode={mode:?})"
            )
        })
    }

    /// The result of tetrating the Decimal `height` times in a row, returning an error on failure.
    pub fn checked_pentate(
        &self,
        mut height: f64,
        mut payload: Decimal,
        mode: TetrationMode,
    ) -> Result<Decimal, ArithmeticError> {
        let old_height = height;
        height = height.trunc();
        let fract_height = old_height - height;

        if fract_height != 0.0 {
            if payload == Decimal::one() {
                height += 1.0;
                payload = Decimal::from_finite(fract_height);
            } else if *self == Decimal::from_finite(10.0) {
                payload = payload.layer_add_10(Decimal::from_finite(fract_height), mode);
            } else {
                payload = payload.layer_add(fract_height, *self, mode);
            }
        }

        for i in 0..height as i64 {
            payload = self.checked_tetrate(payload.to_number(), Decimal::one(), mode)?;
            if !payload.mag.is_finite() {
                return Ok(payload);
            }
            if i > 10 {
                return Ok(payload);
            }
        }

        Ok(payload)
    }
}
