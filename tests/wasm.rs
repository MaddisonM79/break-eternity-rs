//! Browser/Node tests for the `wasm` feature's `JsDecimal` class.
//!
//! Run with a wasm32 target and `wasm-bindgen-test-runner` (see `.github/workflows/ci.yml`):
//!
//! ```sh
//! cargo test --target wasm32-unknown-unknown --features wasm --test wasm
//! ```
//!
//! On any other target this file compiles to nothing.
#![cfg(all(target_arch = "wasm32", feature = "wasm"))]

use break_eternity::JsDecimal;
use wasm_bindgen_test::*;

fn d(s: &str) -> JsDecimal {
    JsDecimal::new(s).expect("valid literal")
}

#[wasm_bindgen_test]
fn constructs_and_formats() {
    assert_eq!(d("1.5e100").js_to_string(), "1.5e100");
    assert_eq!(JsDecimal::from_number(42.0).unwrap().js_to_string(), "42");
    assert!(JsDecimal::from_number(f64::NAN).is_err());
    assert!(JsDecimal::new("garbage").is_err());
    assert_eq!(d("-5").to_fixed(2), "-5.00");
    assert_eq!(d("12345").to_precision(2), "1.2e4");
    assert_eq!(d("Infinity").js_to_string(), "Infinity");
}

#[wasm_bindgen_test]
fn arithmetic_and_errors() {
    let a = d("1e100");
    let b = d("2");
    assert_eq!(a.mul(&b).unwrap().js_to_string(), "2e100");
    assert_eq!(a.add(&a).unwrap().js_to_string(), "2e100");
    assert!(a.div(&d("0")).is_err());
    assert!(d("-8").pow(&d("0.5")).is_err());
    assert_eq!(d("-8").cbrt().js_to_string(), "-2");
    assert_eq!(d("2").pow(&d("10")).js_to_string(), "1024");
    assert!(d("-1").ln().is_err());
    assert!(d("0").gamma().is_err());
}

#[wasm_bindgen_test]
fn comparisons() {
    assert_eq!(d("1e100").cmp(&d("1e99")), 1);
    assert!(d("1").lt(&d("2")));
    assert!(d("Infinity").gt(&d("1e1000")));
    assert!(d("1").approx_eq(&d("1.00000000001"), 1e-10));
    assert_eq!(d("5").max(&d("7")).js_to_string(), "7");
}

#[wasm_bindgen_test]
fn hyperoperations_and_helpers() {
    assert_eq!(d("2").tetrate(3.0, None).unwrap().js_to_string(), "16");
    assert_eq!(d("2").pentate(2.0).unwrap().js_to_string(), "4");
    let s = d("1e10").slog(None).unwrap().to_number();
    assert!((s - 2.0).abs() < 1e-9, "{s}");
    assert_eq!(d("256").ssqrt().unwrap().to_number(), 4.0);
    let n = JsDecimal::afford_geometric_series(&d("100"), &d("10"), &d("1.5"), &d("0")).unwrap();
    assert_eq!(n.to_number(), 4.0);
}
