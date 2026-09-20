//! The `bevy_reflect` feature: `Decimal` is an opaque reflected type.
//!
//! Run with: `cargo test --features bevy_reflect,serde --test bevy_reflect`
#![cfg(feature = "bevy_reflect")]

use bevy_reflect::prelude::*;
use bevy_reflect::{FromReflect, PartialReflect, TypePath, TypeRegistry, Typed};
use break_eternity::Decimal;

#[test]
fn opaque_reflection_round_trips() {
    let d: Decimal = "1e100".parse().unwrap();
    assert_eq!(Decimal::type_path(), "break_eternity::decimal::Decimal");
    assert!(matches!(
        Decimal::type_info(),
        bevy_reflect::TypeInfo::Opaque(_)
    ));

    let boxed: Box<dyn Reflect> = Box::new(d);
    let back = boxed.downcast_ref::<Decimal>().copied().unwrap();
    assert_eq!(back, d);

    let cloned = d.reflect_clone().unwrap();
    assert_eq!(
        Decimal::from_reflect(cloned.as_partial_reflect()).unwrap(),
        d
    );
    assert_eq!(d.reflect_partial_eq(&Decimal::from(5)), Some(false));
    assert_eq!(d.reflect_partial_eq(&d), Some(true));

    let mut applied = Decimal::zero();
    applied.apply(&d);
    assert_eq!(applied, d);
}

#[test]
fn registers_type_data() {
    let mut registry = TypeRegistry::default();
    registry.register::<Decimal>();
    let registration = registry.get(core::any::TypeId::of::<Decimal>()).unwrap();
    assert!(registration.data::<ReflectDefault>().is_some());
    #[cfg(feature = "serde")]
    {
        assert!(registration
            .data::<bevy_reflect::ReflectSerialize>()
            .is_some());
        assert!(registration
            .data::<bevy_reflect::ReflectDeserialize>()
            .is_some());
        // Reflection-driven serialization goes through the crate's serde impl (string form).
        let d: Decimal = "1e100".parse().unwrap();
        let serializer = bevy_reflect::serde::TypedReflectSerializer::new(&d, &registry);
        assert_eq!(serde_json::to_string(&serializer).unwrap(), "\"1e100\"");
    }
}

#[derive(Reflect, Default)]
struct Wallet {
    money: Decimal,
    prestige: u32,
}

#[test]
fn works_as_a_struct_field() {
    use bevy_reflect::structs::Struct;
    let mut wallet = Wallet {
        money: Decimal::from(10),
        prestige: 1,
    };
    let field = wallet.field("money").unwrap();
    assert_eq!(
        field.try_downcast_ref::<Decimal>(),
        Some(&Decimal::from(10))
    );
    let update = Decimal::from(20);
    wallet.field_mut("money").unwrap().apply(&update);
    assert_eq!(wallet.money, update);
}
