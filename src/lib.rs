#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::similar_names)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::float_cmp)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::return_self_not_must_use)]
#![crate_name = "break_eternity"]

//! A numerical library to represent numbers as large as 10^^1e308 and as 'small' as 10^-(10^^1e308).
//!
//! # Examples
//!
//! ```
//! use break_eternity::Decimal;
//!
//! let a = Decimal::from_finite(1.0);
//! let b = Decimal::from_finite(2.0);
//!
//! let c = a + b;
//! assert_eq!(c, Decimal::from_finite(3.0));
//! ```

mod arithmetic;
mod constants;
mod decimal;
mod error;
mod format;
mod parse;
#[cfg(feature = "serde")]
mod serde_impl;
mod tetration;
mod transcendental;
mod utils;

pub use constants::{
    COMPARE_EPSILON, EXPN1, EXPONENT_LIMIT, FIRST_NEG_LAYER, LAYER_REDUCTION_THRESHOLD,
    MAX_ES_IN_A_ROW, MAX_FLOAT_PRECISION, MAX_POWERS_OF_TEN, NUMBER_EXP_MAX, NUMBER_EXP_MIN, OMEGA,
    TWO_PI,
};
pub use decimal::Decimal;
pub use error::{ArithmeticError, ArithmeticErrorKind, BreakEternityError};
pub use format::{decimal_places, to_fixed};
pub use utils::sign;
