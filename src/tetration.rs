//! Tetration and related hyperoperations: tetrate, pentate, slog, ssqrt,
//! iteratedexp, iteratedlog, `layer_add_10`, `layer_add`.

use std::ops::Neg;

use crate::decimal::Decimal;

impl Decimal {
    /// Tetrates the Decimal to the given height.
    ///
    /// Source: <https://andydude.github.io/tetration/archives/tetration2/ident.html>
    pub fn tetrate(&self, height: Option<f64>, payload: Option<Decimal>) -> Decimal {
        let mut height = height.unwrap_or(2.0_f64);
        let mut payload =
            payload.unwrap_or_else(|| Decimal::from_components_no_normalize(1, 0, 1.0));

        if height.is_infinite() && height.is_sign_positive() {
            let neg_ln = self.ln().neg();
            return neg_ln.lambertw().expect("Expected number higher than -1") / neg_ln;
        }

        if height < 0.0 {
            return payload.iteratedlog(*self, -height);
        }

        let old_height = height;
        height = height.trunc();
        let fract_height = old_height - height;

        if fract_height != 0.0 {
            if payload == Decimal::one() {
                height += 1.0;
                payload = Decimal::from_number(fract_height);
            } else if *self == Decimal::from_number(10.0) {
                payload = payload.layer_add_10(Decimal::from_number(fract_height));
            } else {
                payload = payload.layer_add(fract_height, *self);
            }
        }

        for i in 0..height as i64 {
            payload = self.pow(payload);
            // bail if we're NaN
            if !payload.mag.is_finite() {
                return payload;
            }

            if payload.layer - self.layer > 3 {
                return Decimal::from_components_no_normalize(
                    payload.sign,
                    payload.layer + (height as i64 - i - 1),
                    payload.mag,
                );
            }

            if i > 100 {
                return payload;
            }
        }

        payload
    }

    /// Returns the Decimal, iteratively exponentiated.
    ///
    /// Equates to tetrating to the same height.
    pub fn iteratedexp(&self, height: Option<f64>, payload: Option<Decimal>) -> Decimal {
        self.tetrate(height, payload)
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
            if base == Decimal::from_number(10.0) {
                result = result.layer_add_10(Decimal::from_number(-fraction));
            } else {
                result = result.layer_add(-fraction, base);
            }
        }

        result
    }

    /// Returns the super-logarithm of the Decimal.
    pub fn slog(&self, base_opt: Option<Decimal>) -> Decimal {
        if self.mag < 0.0 {
            return Decimal::neg_one();
        }

        let mut result: f64 = 0.0;
        let base = base_opt.unwrap_or_else(|| Decimal::from_number(10.0));
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
                return Decimal::from_number(result + copy.to_number() - 1.0);
            }

            result += 1.0;
            copy = copy.log(base);
        }

        Decimal::from_number(result)
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
            return Decimal::nan();
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
    pub fn ssqrt(&self) -> Decimal {
        if self.sign == 1 && self.layer >= 3 {
            return Decimal::from_components_no_normalize(self.sign, self.layer - 1, self.mag);
        }

        let ln_x = self.ln();
        ln_x / ln_x.lambertw().expect("Expected number higher than -1")
    }

    /// The result of tetrating the Decimal `height` times in a row.
    pub fn pentate(&self, height: Option<f64>, payload: Option<Decimal>) -> Decimal {
        let mut height = height.unwrap_or(2.0_f64);
        let mut payload =
            payload.unwrap_or_else(|| Decimal::from_components_no_normalize(1, 0, 1.0));

        let old_height = height;
        height = height.trunc();
        let fract_height = old_height - height;

        if fract_height != 0.0 {
            if payload == Decimal::one() {
                height += 1.0;
                payload = Decimal::from_number(fract_height);
            } else if *self == Decimal::from_number(10.0) {
                payload = payload.layer_add_10(Decimal::from_number(fract_height));
            } else {
                payload = payload.layer_add(fract_height, *self);
            }
        }

        for i in 0..height as i64 {
            payload = self.tetrate(Some(payload.to_number()), None);
            if !payload.mag.is_finite() {
                return payload;
            }
            if i > 10 {
                return payload;
            }
        }

        payload
    }
}
