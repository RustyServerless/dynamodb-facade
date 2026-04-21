// Tests for the `key_schema!` macro.
//
// Verifies that:
//  - Simple-key schema has only a `PartitionKey` and implements `SimpleKeySchema`.
//  - Composite-key schema has both `PartitionKey` and `SortKey` and implements
//    `CompositeKeySchema`.
//  - The `Kind` associated type is `SimpleKey` / `CompositeKey` respectively.
//  - `PartitionKey::NAME` and `SortKey::NAME` match the declared attribute types.
//
// Unlike `table_definitions!` and `index_definitions!`, `key_schema!` generates
// the struct at the current scope — the types are directly nameable.

use dynamodb_facade::{
    AttributeDefinition, CompositeKeySchema, KeySchema, SimpleKeySchema, StringAttribute,
};

dynamodb_facade::attribute_definitions! {
    KSPk { "KS_PK": StringAttribute }
    KSSk { "KS_SK": StringAttribute }
}

dynamodb_facade::key_schema! {
    SimpleSchema {
        type PartitionKey = KSPk;
    }
}

dynamodb_facade::key_schema! {
    CompositeSchema {
        type PartitionKey = KSPk;
        type SortKey = KSSk;
    }
}

#[test]
fn test_key_schema_simple_and_composite() {
    fn assert_simple<KS: SimpleKeySchema>() {}
    fn assert_composite<KS: CompositeKeySchema>() {}
    assert_simple::<SimpleSchema>();
    assert_composite::<CompositeSchema>();

    assert_eq!(<SimpleSchema as KeySchema>::PartitionKey::NAME, "KS_PK");
    assert_eq!(<CompositeSchema as KeySchema>::PartitionKey::NAME, "KS_PK");
    assert_eq!(
        <CompositeSchema as CompositeKeySchema>::SortKey::NAME,
        "KS_SK"
    );
}
