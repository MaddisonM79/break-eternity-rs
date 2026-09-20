//! Serde tests for [`Decimal`]: default string form in JSON, component tuple in bincode,
//! the `serde_components` / `serde_string` adapters, and lenient JSON input.
//!
//! Run with: `cargo test --features serde`
#![cfg(feature = "serde")]

use break_eternity::Decimal;
use serde::{Deserialize, Serialize};

fn d(s: &str) -> Decimal {
    s.parse().unwrap()
}

fn samples() -> Vec<Decimal> {
    vec![
        Decimal::zero(),
        d("42"),
        d("-1234.5678"),
        d("1e-300"),
        d("1e50"),
        d("-1e50"),
        Decimal::from_components(1, 2, 5.0),
        Decimal::from_components(1, 5, 1.2345e10),
        Decimal::from_components(-1, 100, 1.5e10),
        Decimal::inf(),
        Decimal::neg_inf(),
    ]
}

#[test]
fn json_uses_the_display_string_and_round_trips() {
    for v in samples() {
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, format!("\"{v}\""), "{v:?}");
        let back: Decimal = serde_json::from_str(&json).unwrap();
        assert!(v.approx_eq(&back, 1e-12), "{v:?} -> {json} -> {back:?}");
    }
    assert_eq!(
        serde_json::to_string(&Decimal::inf()).unwrap(),
        "\"Infinity\""
    );
}

#[test]
fn bincode_uses_components_and_round_trips_exactly() {
    for v in samples() {
        let bytes = bincode::serialize(&v).unwrap();
        assert_eq!(bytes.len(), 1 + 8 + 8, "{v:?}");
        let back: Decimal = bincode::deserialize(&bytes).unwrap();
        assert_eq!(v, back);
    }
}

#[test]
fn json_accepts_numbers_and_component_arrays() {
    assert_eq!(serde_json::from_str::<Decimal>("100").unwrap(), d("100"));
    assert_eq!(serde_json::from_str::<Decimal>("-7").unwrap(), d("-7"));
    assert_eq!(
        serde_json::from_str::<Decimal>("2.5e300").unwrap(),
        d("2.5e300")
    );
    assert_eq!(
        serde_json::from_str::<Decimal>("[1, 1, 100.0]").unwrap(),
        d("1e100")
    );
    assert_eq!(
        serde_json::from_str::<Decimal>("[-1, 9223372036854775807, 0.0]").unwrap(),
        Decimal::neg_inf()
    );
    assert!(serde_json::from_str::<Decimal>("[1, 1]").is_err());
    assert!(serde_json::from_str::<Decimal>("[1, 1, 100.0, 4]").is_err());
    assert!(serde_json::from_str::<Decimal>("[3, 1, 100.0]").is_err());
    assert!(serde_json::from_str::<Decimal>("null").is_err());
    let err = serde_json::from_str::<Decimal>("\"1e\"")
        .unwrap_err()
        .to_string();
    assert!(err.contains("1e"), "{err}");
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Save {
    #[serde(with = "break_eternity::serde_components")]
    compact: Decimal,
    #[serde(with = "break_eternity::serde_string")]
    text: Decimal,
    plain: Decimal,
}

#[test]
fn with_adapters_pin_the_representation() {
    for v in samples() {
        let save = Save {
            compact: v,
            text: v,
            plain: v,
        };
        let json = serde_json::to_string(&save).unwrap();
        let back: Save = serde_json::from_str(&json).unwrap();
        assert_eq!(back.compact, v, "{json}");
        assert!(back.text.approx_eq(&v, 1e-12), "{json}");
        assert!(back.plain.approx_eq(&v, 1e-12), "{json}");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value["compact"].is_array(), "{json}");
        assert!(value["text"].is_string(), "{json}");
        assert!(value["plain"].is_string(), "{json}");

        let bytes = bincode::serialize(&save).unwrap();
        let back: Save = bincode::deserialize(&bytes).unwrap();
        assert_eq!(back.compact, v);
        assert_eq!(back.plain, v);
        assert!(back.text.approx_eq(&v, 1e-12));
    }
    let json = serde_json::to_string(&Save {
        compact: Decimal::inf(),
        text: Decimal::inf(),
        plain: Decimal::inf(),
    })
    .unwrap();
    assert_eq!(
        json,
        r#"{"compact":[1,9223372036854775807,0.0],"text":"Infinity","plain":"Infinity"}"#
    );
    // The string adapter does not take numbers.
    assert!(serde_json::from_str::<Save>(r#"{"compact":[1,0,1.0],"text":5,"plain":"1"}"#).is_err());
}
