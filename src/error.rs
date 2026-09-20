//! Error types for the crate.

use alloc::string::String;
use core::num::ParseFloatError;

/// Identifies the category of an arithmetic error.
///
/// Marked `#[non_exhaustive]`: new kinds may be added in minor releases, so match with a
/// wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
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
    /// The value does not fit the target type (integer conversions; infinities included).
    Overflow,
    /// The value has a fractional part where a whole number is required.
    NotInteger,
}

/// An error produced by a `checked_*` arithmetic operation on [`crate::Decimal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("arithmetic error in {op}: {kind:?}")]
pub struct ArithmeticError {
    /// The kind of arithmetic error.
    pub kind: ArithmeticErrorKind,
    /// The name of the operation that failed.
    pub op: &'static str,
}

impl ArithmeticError {
    /// Creates a new error for operation `op` of the given kind.
    #[must_use]
    pub const fn new(kind: ArithmeticErrorKind, op: &'static str) -> Self {
        Self { kind, op }
    }

    pub(crate) const fn undefined(op: &'static str) -> Self {
        Self::new(ArithmeticErrorKind::Undefined, op)
    }

    pub(crate) const fn division_by_zero(op: &'static str) -> Self {
        Self::new(ArithmeticErrorKind::DivisionByZero, op)
    }

    pub(crate) const fn negative_base(op: &'static str) -> Self {
        Self::new(ArithmeticErrorKind::NegativeBase, op)
    }

    pub(crate) const fn out_of_domain(op: &'static str) -> Self {
        Self::new(ArithmeticErrorKind::OutOfDomain, op)
    }

    pub(crate) const fn diverged(op: &'static str) -> Self {
        Self::new(ArithmeticErrorKind::IterationDiverged, op)
    }
}

/// Error type for all errors in this crate.
#[derive(Debug, thiserror::Error)]
pub enum BreakEternityError {
    /// An error that occurs when a String cannot be parsed to a Decimal.
    #[error("Error while parsing \"{parsed}\": {error}")]
    ParseError {
        /// The string that failed to parse.
        parsed: String,
        /// The error that occurred.
        error: ParseFloatError,
    },

    /// A string that parsed but denotes a mathematically undefined value (for example
    /// `"(-2)^^2.5"`).
    #[error("\"{parsed}\" is syntactically valid but has no defined value")]
    ParseUndefined {
        /// The string that was parsed.
        parsed: String,
    },

    /// An arithmetic error from a `checked_*` operation.
    #[error(transparent)]
    Arithmetic(#[from] ArithmeticError),
}
