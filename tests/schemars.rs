//! The `schemars` feature: `JsonSchema` for `Decimal`.
//!
//! Run with: `cargo test --features schemars --test schemars`
#![cfg(feature = "schemars")]

use break_eternity::Decimal;
use regex_lite::Regex;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(JsonSchema, Serialize, Deserialize)]
struct Upgrade {
    name: String,
    cost: Decimal,
    multiplier: Decimal,
}

fn decimal_def() -> Value {
    let schema = schema_for!(Upgrade);
    let json = serde_json::to_value(&schema).unwrap();
    assert_eq!(json["properties"]["cost"]["$ref"], "#/$defs/Decimal");
    assert_eq!(json["properties"]["multiplier"]["$ref"], "#/$defs/Decimal");
    json["$defs"]["Decimal"].clone()
}

fn pattern() -> Regex {
    let def = decimal_def();
    let pattern = def["anyOf"][0]["pattern"].as_str().unwrap();
    assert_eq!(pattern, break_eternity::DECIMAL_PATTERN);
    Regex::new(pattern).expect("pattern must be a valid regex")
}

#[test]
fn schema_is_a_named_string_first_union() {
    let def = decimal_def();
    assert_eq!(def["title"], "Decimal");
    assert!(def["description"]
        .as_str()
        .unwrap()
        .contains("break_eternity"));
    let branches = def["anyOf"].as_array().unwrap();
    assert_eq!(branches.len(), 3);
    assert_eq!(branches[0]["type"], "string");
    assert!(branches[0]["pattern"].is_string());
    assert!(branches[0]["examples"].is_array());
    assert_eq!(branches[1]["type"], "number");
    assert_eq!(branches[2]["type"], "array");
    assert_eq!(branches[2]["prefixItems"].as_array().unwrap().len(), 3);

    // A bare `Decimal` schema works too, and is the same definition.
    let top = serde_json::to_value(schema_for!(Decimal)).unwrap();
    assert_eq!(top["title"], "Decimal");
    assert_eq!(top["anyOf"], def["anyOf"]);
}

#[test]
fn serialized_values_match_the_pattern() {
    let re = pattern();
    let values: [Decimal; 14] = [
        Decimal::zero(),
        Decimal::one(),
        Decimal::from(-42),
        Decimal::from_finite(3.5),
        Decimal::from_finite(1e-8),
        Decimal::from_finite(1e21),
        "1e-20".parse().unwrap(),
        "1.5e100".parse().unwrap(),
        "-2.5e-300".parse().unwrap(),
        "ee10".parse().unwrap(),
        "-eee5.5".parse().unwrap(),
        "(e^7)10".parse().unwrap(),
        Decimal::inf(),
        Decimal::neg_inf(),
    ];
    for v in values {
        let json = serde_json::to_value(v).unwrap();
        let s = json.as_str().expect("human-readable form is a string");
        assert!(re.is_match(s), "Display form {s:?} must match the pattern");
        assert_eq!(serde_json::from_value::<Decimal>(json).unwrap(), v);
    }
    for example in decimal_def()["anyOf"][0]["examples"].as_array().unwrap() {
        let s = example.as_str().unwrap();
        assert!(re.is_match(s), "example {s:?} must match the pattern");
        assert!(s.parse::<Decimal>().is_ok(), "example {s:?} must parse");
    }
}

#[test]
fn pattern_accepts_every_parser_notation() {
    let re = pattern();
    let accepted = [
        "0",
        "-5",
        "3.14",
        "1,000,000",
        ".5",
        "5.",
        "+7",
        "1.23e45",
        "1.23E+45",
        "1.23e-45",
        "1e1000",
        "2.47e-324",
        "1e100.5",
        "e10",
        "ee10",
        "eee10",
        "eeeee10",
        "5e3e2",
        "-ee-100.5",
        "ee-100",
        "(e^7)10",
        "-(e^7)10",
        "(e^1000)33.3",
        "(e^2.5)10",
        "2^10",
        "2^^3",
        "2^^3;1.5",
        "10^^5",
        "2^^^2",
        "3^^^2;2",
        "1e5^2",
        "3 PT 4",
        "3 PT (4)",
        "3pt4",
        "3p4",
        "10 pt 2",
        "2f3",
        "f3",
        "f(3)",
        "-2f3",
        "Infinity",
        "-Infinity",
        "inf",
        "-inf",
        "+Infinity",
        "INFINITY",
        "  42  ",
        " 1e5 ",
    ];
    for s in accepted {
        assert!(re.is_match(s), "{s:?} should match the pattern");
        assert!(s.parse::<Decimal>().is_ok(), "{s:?} should parse");
    }
}

#[test]
fn pattern_rejects_text_outside_the_grammar() {
    let re = pattern();
    let rejected = [
        "",
        "abc",
        "lots",
        "1e1O0",
        "10x",
        "NaN",
        "nan",
        "1_000",
        "0x10",
        "1 000",
        "é",
        "1e5 apples",
        "e",
        "^",
        "(e^5",
        "--5",
        "1..2",
        "..",
        "2^^",
        ";5",
        "f",
        "pt",
        "p",
    ];
    for s in rejected {
        assert!(!re.is_match(s), "{s:?} should not match the pattern");
    }
    // Everything the pattern rejects, the parser rejects too (no false positives the other way).
    for s in rejected {
        assert!(s.parse::<Decimal>().is_err(), "{s:?} should not parse");
    }
}

#[test]
fn lenient_forms_are_covered_by_the_other_branches() {
    // These load through serde and are valid against the number / array branches.
    let cases = [
        json!(100),
        json!(-2.5),
        json!(1e300),
        json!([1, 1, 100.0]),
        json!([-1, 0, 5.0]),
    ];
    for v in cases {
        let d: Decimal = serde_json::from_value(v.clone()).unwrap();
        assert!(d.is_finite(), "{v}");
    }
}
