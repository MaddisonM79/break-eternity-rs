//! Tests for `checked_*` arithmetic methods and operator-overload panic behavior.

use std::convert::TryFrom;

use break_eternity::{ArithmeticErrorKind, Decimal};

// ---------------------------------------------------------------------------
// checked_div
// ---------------------------------------------------------------------------

#[test]
fn checked_div_by_zero() {
    let zero = Decimal::from_finite(0.0);
    let nonzero = Decimal::from_finite(5.0);

    let result = nonzero.checked_div(&zero);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        ArithmeticErrorKind::DivisionByZero
    );
}

#[test]
fn checked_div_zero_by_zero() {
    let zero = Decimal::from_finite(0.0);
    let result = zero.checked_div(&zero);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        ArithmeticErrorKind::DivisionByZero
    );
}

#[test]
fn checked_div_happy_path() {
    let a = Decimal::from_finite(10.0);
    let b = Decimal::from_finite(2.0);
    let result = a.checked_div(&b).unwrap();
    assert_eq!(result, Decimal::from_finite(5.0));
}

// ---------------------------------------------------------------------------
// checked_ln
// ---------------------------------------------------------------------------

#[test]
fn checked_ln_negative_errors() {
    let neg = Decimal::from_finite(-1.0);
    let result = neg.checked_ln();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, ArithmeticErrorKind::OutOfDomain);
}

#[test]
fn checked_ln_zero_errors() {
    let zero = Decimal::from_finite(0.0);
    let result = zero.checked_ln();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, ArithmeticErrorKind::OutOfDomain);
}

#[test]
fn checked_ln_positive_ok() {
    let e = Decimal::from_finite(std::f64::consts::E);
    let result = e.checked_ln().unwrap();
    // ln(e) ≈ 1
    assert!(result.approx_eq(&Decimal::from_finite(1.0), 1e-9));
}

// ---------------------------------------------------------------------------
// checked_log10
// ---------------------------------------------------------------------------

#[test]
fn checked_log10_non_positive_errors() {
    assert!(Decimal::from_finite(0.0).checked_log10().is_err());
    assert!(Decimal::from_finite(-5.0).checked_log10().is_err());
}

#[test]
fn checked_log10_ok() {
    let d = Decimal::from_finite(100.0);
    let result = d.checked_log10().unwrap();
    assert!(result.approx_eq(&Decimal::from_finite(2.0), 1e-9));
}

// ---------------------------------------------------------------------------
// checked_sqrt
// ---------------------------------------------------------------------------

#[test]
fn checked_sqrt_negative_errors() {
    let neg = Decimal::from_finite(-4.0);
    let result = neg.checked_sqrt();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, ArithmeticErrorKind::OutOfDomain);
}

#[test]
fn checked_sqrt_ok() {
    let d = Decimal::from_finite(9.0);
    let result = d.checked_sqrt().unwrap();
    assert!(result.approx_eq(&Decimal::from_finite(3.0), 1e-9));
}

// ---------------------------------------------------------------------------
// checked_pow
// ---------------------------------------------------------------------------

#[test]
fn checked_pow_negative_base_non_integer_exponent_errors() {
    let base = Decimal::from_finite(-2.0);
    let exp = Decimal::from_finite(0.5);
    let result = base.checked_pow(&exp);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, ArithmeticErrorKind::NegativeBase);
}

#[test]
fn checked_pow_negative_base_integer_exponent_ok() {
    let base = Decimal::from_finite(-2.0);
    let exp = Decimal::from_finite(3.0);
    let result = base.checked_pow(&exp).unwrap();
    // (-2)^3 = -8
    assert!(result.approx_eq(&Decimal::from_finite(-8.0), 1e-9));
}

// ---------------------------------------------------------------------------
// TryFrom<f64>
// ---------------------------------------------------------------------------

#[test]
fn try_from_nan_errors() {
    let result = Decimal::try_from(f64::NAN);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, ArithmeticErrorKind::Undefined);
}

#[test]
fn try_from_positive_infinity_errors() {
    let result = Decimal::try_from(f64::INFINITY);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, ArithmeticErrorKind::Undefined);
}

#[test]
fn try_from_negative_infinity_errors() {
    let result = Decimal::try_from(f64::NEG_INFINITY);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, ArithmeticErrorKind::Undefined);
}

#[test]
fn try_from_finite_ok() {
    let d = Decimal::try_from(1.23456789_f64).unwrap();
    assert!(d.approx_eq(&Decimal::from_finite(1.23456789), 1e-12));
}

// ---------------------------------------------------------------------------
// Operator overload panics
// ---------------------------------------------------------------------------

#[test]
fn operator_div_by_zero_panics() {
    let zero = Decimal::from_finite(0.0);
    let nonzero = Decimal::from_finite(1.0);

    let result = std::panic::catch_unwind(|| {
        let _ = nonzero / zero;
    });
    assert!(
        result.is_err(),
        "division by zero should panic via operator overload"
    );

    // Verify the panic message contains structured context.
    let panic_val = result.unwrap_err();
    if let Some(msg) = panic_val.downcast_ref::<String>() {
        assert!(
            msg.contains("DivisionByZero") || msg.contains("division"),
            "panic message should identify the error: {msg}"
        );
    }
}

#[test]
fn checked_rem_by_zero_errors() {
    let zero = Decimal::from_finite(0.0);
    let five = Decimal::from_finite(5.0);
    let result = five.checked_rem(&zero);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        ArithmeticErrorKind::DivisionByZero
    );
}

// ---------------------------------------------------------------------------
// Reference operator combinations
// ---------------------------------------------------------------------------

#[test]
// These tests intentionally exercise the &Decimal op &Decimal / Decimal op &Decimal /
// &Decimal op Decimal overloads. Because Decimal is Copy, clippy warns that the
// references are unnecessary — but that's exactly what we're testing here.
#[allow(clippy::op_ref)]
fn ref_ops() {
    let a = Decimal::from_finite(6.0);
    let b = Decimal::from_finite(2.0);

    // &Decimal op &Decimal
    assert_eq!(&a + &b, Decimal::from_finite(8.0));
    assert_eq!(&a - &b, Decimal::from_finite(4.0));
    assert_eq!(&a * &b, Decimal::from_finite(12.0));
    assert_eq!(&a / &b, Decimal::from_finite(3.0));

    // Decimal op &Decimal
    assert_eq!(a + &b, Decimal::from_finite(8.0));
    assert_eq!(a - &b, Decimal::from_finite(4.0));

    // &Decimal op Decimal
    assert_eq!(&a + b, Decimal::from_finite(8.0));
    assert_eq!(&a - b, Decimal::from_finite(4.0));
}
