//! Serde round-trip tests for [`Decimal`].
//!
//! Run with: `cargo test --features serde`

#[cfg(feature = "serde")]
mod roundtrip {
    use break_eternity::Decimal;

    fn roundtrip(d: Decimal) -> Decimal {
        let serialized = serde_json::to_string(&d).expect("serialization must not fail");
        let deserialized: Decimal = serde_json::from_str(&serialized)
            .unwrap_or_else(|e| panic!("deserialization failed for {serialized:?}: {e}"));
        deserialized
    }

    fn assert_roundtrip(d: Decimal) {
        let rt = roundtrip(d);
        assert!(
            d.approx_eq(&rt, 1e-9),
            "round-trip failed:\n  original:   {d:?}\n  after rt:   {rt:?}"
        );
    }

    #[test]
    fn layer_0_positive() {
        assert_roundtrip(Decimal::from_finite(42.0));
    }

    #[test]
    fn layer_0_negative() {
        assert_roundtrip(Decimal::from_finite(-1234.5678));
    }

    #[test]
    fn layer_0_zero() {
        let rt = roundtrip(Decimal::zero());
        assert_eq!(rt, Decimal::zero());
    }

    #[test]
    fn layer_1_value() {
        // 1e50 — layer 1 after normalization
        assert_roundtrip(Decimal::from_finite(1e50));
    }

    #[test]
    fn layer_2_value() {
        // Construct a layer-2 Decimal: 10^(10^5) ≈ 10^^2-ish
        let d = Decimal::from_components(1, 2, 5.0);
        assert_roundtrip(d);
    }

    #[test]
    fn layer_5_value() {
        let d = Decimal::from_components(1, 5, 1.2345e10);
        assert_roundtrip(d);
    }

    #[test]
    fn layer_100_value() {
        let d = Decimal::from_components(1, 100, 1.5e10);
        assert_roundtrip(d);
    }

    #[test]
    fn infinity_roundtrip() {
        let rt = roundtrip(Decimal::inf());
        assert_eq!(rt, Decimal::inf());
    }

    #[test]
    fn neg_infinity_roundtrip() {
        let rt = roundtrip(Decimal::neg_inf());
        assert_eq!(rt, Decimal::neg_inf());
    }
}
