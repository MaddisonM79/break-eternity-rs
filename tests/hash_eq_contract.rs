//! Tests that [`Decimal`]'s `Eq` and `Hash` implementations satisfy their contract:
//! values that compare equal must hash to the same value.

use std::collections::{HashMap, HashSet};

use break_eternity::Decimal;

#[test]
fn hashmap_insert_and_lookup() {
    let key = Decimal::from_finite(1.0);
    let mut map: HashMap<Decimal, &str> = HashMap::new();
    map.insert(key, "one");

    // A freshly-constructed equal value must retrieve the same entry.
    let lookup_key = Decimal::from_finite(1.0);
    assert_eq!(map.get(&lookup_key), Some(&"one"));
}

#[test]
fn equal_values_same_hash() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn hash_of(d: &Decimal) -> u64 {
        let mut h = DefaultHasher::new();
        d.hash(&mut h);
        h.finish()
    }

    let a = Decimal::from_finite(42.0);
    let b = Decimal::from_finite(42.0);
    assert_eq!(a, b, "precondition: values must be equal");
    assert_eq!(
        hash_of(&a),
        hash_of(&b),
        "equal Decimals must have equal hashes"
    );
}

#[test]
fn zero_neg_zero_canonical() {
    // After normalization -0.0 is canonicalized to 0.0, so these must be equal and share a hash.
    let pos_zero = Decimal::from_finite(0.0);
    let neg_zero = Decimal::from_finite(-0.0);
    assert_eq!(pos_zero, neg_zero);

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h1 = DefaultHasher::new();
    pos_zero.hash(&mut h1);
    let mut h2 = DefaultHasher::new();
    neg_zero.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
}

#[test]
fn hashset_dedup() {
    let values: Vec<Decimal> = vec![
        Decimal::from_finite(1.0),
        Decimal::from_finite(2.0),
        Decimal::from_finite(1.0), // duplicate
        Decimal::from_finite(3.0),
        Decimal::from_finite(2.0), // duplicate
    ];

    let set: HashSet<Decimal> = values.into_iter().collect();
    assert_eq!(set.len(), 3, "duplicates should be collapsed");
}

#[test]
fn large_layer_roundtrip() {
    // Construct a large-layer value and verify equality + HashMap round-trip.
    let big = Decimal::from_components(1, 5, 1.2345e10);
    let big2 = Decimal::from_components(1, 5, 1.2345e10);
    assert_eq!(big, big2);

    let mut map = HashMap::new();
    map.insert(big, 99u32);
    assert_eq!(map.get(&big2), Some(&99u32));
}

#[test]
fn negative_values_in_hashmap() {
    let neg = Decimal::from_finite(-7.5);
    let mut map = HashMap::new();
    map.insert(neg, "negative");
    assert_eq!(map.get(&Decimal::from_finite(-7.5)), Some(&"negative"));
}
