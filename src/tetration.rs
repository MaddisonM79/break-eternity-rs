//! Tetration and related hyperoperations: tetrate, pentate, slog, ssqrt,
//! iteratedexp, iteratedlog, `layer_add_10`, `layer_add`.

use std::ops::Neg;

use crate::decimal::Decimal;
use crate::error::ArithmeticError;

impl Decimal {
    /// Tetrates the Decimal to the given height.
    ///
    /// Source: <https://andydude.github.io/tetration/archives/tetration2/ident.html>
    ///
    /// # Panics
    ///
    /// Panics if `lambertw` is out of domain (infinite height case). Use
    /// [`checked_tetrate`](Self::checked_tetrate) for explicit error handling.
    pub fn tetrate(&self, height: Option<f64>, payload: Option<Decimal>) -> Decimal {
        self.checked_tetrate(
            height.unwrap_or(2.0),
            payload.unwrap_or_else(|| Decimal::from_components_unchecked(1, 0, 1.0)),
        )
        .unwrap_or_else(|e| {
            panic!(
                "undefined Decimal tetrate: {e} (base={self:?}, height={height:?}, payload={payload:?})"
            )
        })
    }

    /// Tetrates the Decimal to the given height, returning an error on failure.
    pub fn checked_tetrate(
        &self,
        mut height: f64,
        mut payload: Decimal,
    ) -> Result<Decimal, ArithmeticError> {
        if height.is_infinite() && height.is_sign_positive() {
            let neg_ln = self.ln().neg();
            let w = neg_ln.checked_lambertw()?;
            return Ok(w / neg_ln);
        }

        if height < 0.0 {
            return Ok(payload.iteratedlog(*self, -height));
        }

        let old_height = height;
        height = height.trunc();
        let fract_height = old_height - height;

        if fract_height != 0.0 {
            if payload == Decimal::one() {
                height += 1.0;
                payload = Decimal::from_finite(fract_height);
            } else if *self == Decimal::from_finite(10.0) {
                payload = payload.layer_add_10(Decimal::from_finite(fract_height));
            } else {
                payload = payload.layer_add(fract_height, *self);
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
    pub fn iteratedexp(&self, height: Option<f64>, payload: Option<Decimal>) -> Decimal {
        self.tetrate(height, payload)
    }

    /// Returns `self` iteratively exponentiated, returning an error on failure.
    ///
    /// Equates to [`checked_tetrate`](Self::checked_tetrate).
    pub fn checked_iteratedexp(
        &self,
        height: f64,
        payload: Decimal,
    ) -> Result<Decimal, ArithmeticError> {
        self.checked_tetrate(height, payload)
    }

    /// Iterated log: The result of applying log(base) 'times' times in a row.
    ///
    /// Approximately equal to subtracting (times) from the number's slog representation.
    /// Equates to tetrating to a negative height.
    pub fn iteratedlog(&self, base: Decimal, mut times: f64) -> Decimal {
        if times < 0.0 {
            return base.tetrate(Some(-times), Some(*self));
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
                result = result.layer_add_10(Decimal::from_finite(-fraction));
            } else {
                result = result.layer_add(-fraction, base);
            }
        }

        result
    }

    /// Iterated log returning an error on failure.
    pub fn checked_iteratedlog(
        &self,
        base: Decimal,
        times: f64,
    ) -> Result<Decimal, ArithmeticError> {
        Ok(self.iteratedlog(base, times))
    }

    /// Returns the super-logarithm of the Decimal.
    pub fn slog(&self, base_opt: Option<Decimal>) -> Decimal {
        self.checked_slog(base_opt.map_or(10.0, |b| b.to_number()))
            .unwrap_or_else(|e| panic!("undefined Decimal slog: {e} (self={self:?})"))
    }

    /// Returns the super-logarithm, returning an error on failure.
    pub fn checked_slog(&self, base_f64: f64) -> Result<Decimal, ArithmeticError> {
        if self.mag < 0.0 {
            return Ok(Decimal::neg_one());
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
                return Ok(Decimal::from_finite(result + copy.to_number() - 1.0));
            }

            result += 1.0;
            copy = copy.log(base);
        }

        Ok(Decimal::from_finite(result))
    }

    /// Adds or removes layers from a Decimal using linear approximation.
    pub fn layer_add_10(&self, diff: Decimal) -> Decimal {
        let mut diff = diff.to_number();
        let mut result = *self;

        if diff >= 1.0 {
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
                        return result;
                    }

                    if result.layer >= 0 {
                        break;
                    }
                }
            }
        }

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

        result.normalize();
        result
    }

    /// Adds `diff` to the Decimal's slog(base) representation.
    pub fn layer_add(&self, diff: f64, base: Decimal) -> Decimal {
        let slog_this = self.slog(Some(base)).to_number();
        let slog_dest = slog_this + diff;

        if slog_dest >= 0.0 {
            return base.tetrate(Some(slog_dest), None);
        }

        if !slog_dest.is_finite() {
            return Decimal::nan_sentinel();
        }

        if slog_dest >= -1.0 {
            return base.tetrate(Some(slog_dest + 1.0), None).log(base);
        }

        base.tetrate(Some(slog_dest + 2.0), None)
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
    pub fn pentate(&self, height: Option<f64>, payload: Option<Decimal>) -> Decimal {
        self.checked_pentate(
            height.unwrap_or(2.0),
            payload.unwrap_or_else(|| Decimal::from_components_unchecked(1, 0, 1.0)),
        )
        .unwrap_or_else(|e| {
            panic!(
                "undefined Decimal pentate: {e} (base={self:?}, height={height:?}, payload={payload:?})"
            )
        })
    }

    /// The result of tetrating the Decimal `height` times in a row, returning an error on failure.
    pub fn checked_pentate(
        &self,
        mut height: f64,
        mut payload: Decimal,
    ) -> Result<Decimal, ArithmeticError> {
        let old_height = height;
        height = height.trunc();
        let fract_height = old_height - height;

        if fract_height != 0.0 {
            if payload == Decimal::one() {
                height += 1.0;
                payload = Decimal::from_finite(fract_height);
            } else if *self == Decimal::from_finite(10.0) {
                payload = payload.layer_add_10(Decimal::from_finite(fract_height));
            } else {
                payload = payload.layer_add(fract_height, *self);
            }
        }

        for i in 0..height as i64 {
            payload = self.checked_tetrate(payload.to_number(), Decimal::one())?;
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
