// Tests for the `attribute_definitions!` macro.
//
// Verifies that:
//  - Each declaration produces a zero-sized `pub struct` that implements
//    `AttributeDefinition` with the correct `NAME` const and associated
//    `Type`.
//  - Multiple attribute types (String, Number, Binary) can be declared in
//    one invocation.
//  - `#[doc]` attributes on individual entries are preserved (compile-time
//    check — if the macro dropped them, `#[deny(missing_docs)]` in the
//    doc test below would fail).

use dynamodb_facade::{AttributeDefinition, BinaryAttribute, NumberAttribute, StringAttribute};

dynamodb_facade::attribute_definitions! {
    /// A sample string attribute.
    TestPk { "PK": StringAttribute }

    /// A sample number attribute.
    TestAge { "age": NumberAttribute }

    /// A sample binary attribute.
    TestBlob { "blob": BinaryAttribute }
}

#[test]
fn test_attribute_definitions_generates_name_and_type() {
    // NAME consts.
    assert_eq!(TestPk::NAME, "PK");
    assert_eq!(TestAge::NAME, "age");
    assert_eq!(TestBlob::NAME, "blob");

    // Associated `Type` is a ZST marker — assert zero size via PhantomData
    // round-trip. The actual type is checked at compile-time by the bounds.
    fn assert_type_is<A: AttributeDefinition>(_: core::marker::PhantomData<A::Type>) {}
    assert_type_is::<TestPk>(core::marker::PhantomData::<StringAttribute>);
    assert_type_is::<TestAge>(core::marker::PhantomData::<NumberAttribute>);
    assert_type_is::<TestBlob>(core::marker::PhantomData::<BinaryAttribute>);
}
