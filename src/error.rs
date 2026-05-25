//! Error types for the crate.

use std::num::ParseFloatError;

/// Error type for all errors in this crate.
#[derive(Debug, thiserror::Error)]
pub enum BreakEternityError {
    /// An error that occurs when `f_gamma` and `lambertw` fails to converge a number (more than 100 iterations).
    #[error("Iteration failed to converge: {z}")]
    IterationFailedConverging {
        /// The number that failed to converge.
        z: f64,
    },

    /// An error that occurs when a String cannot be parsed to a Decimal.
    #[error("Error while parsing \"{parsed}\": {error}")]
    ParseError {
        /// The string that failed to parse.
        parsed: String,
        /// The error that occurred.
        error: ParseFloatError,
    },

    /// An error that occurs when lambertw is called with a number less than -1.
    #[error("lambertw is undefined for results < -1")]
    LambertWError,
}
