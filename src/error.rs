//! Error types for the crate.

use std::num::ParseFloatError;

/// Identifies the category of an arithmetic error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArithmeticErrorKind {
    /// Operation is mathematically undefined (e.g., 0/0, +inf + -inf).
    Undefined,
    /// Division by zero.
    DivisionByZero,
    /// Negative base with non-integer exponent.
    NegativeBase,
    /// Domain error (e.g., ln of non-positive, lambertw for z < -1/e).
    OutOfDomain,
    /// Numerical iteration diverged.
    IterationDiverged,
}

/// An error produced by a `checked_*` arithmetic operation on [`crate::Decimal`].
#[derive(Debug, thiserror::Error)]
#[error("arithmetic error in {op}: {kind:?}")]
pub struct ArithmeticError {
    /// The kind of arithmetic error.
    pub kind: ArithmeticErrorKind,
    /// The name of the operation that failed.
    pub op: &'static str,
}

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

    /// An arithmetic error from a `checked_*` operation.
    #[error(transparent)]
    Arithmetic(#[from] ArithmeticError),
}
