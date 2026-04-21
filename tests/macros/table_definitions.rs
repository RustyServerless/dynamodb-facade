// Tests for the `table_definitions!` macro.
//
// Verifies that:
//  - Composite-key tables expose `CompositeKeySchema` with correct
//    `PartitionKey` and `SortKey` associated types.
//  - Simple-key tables expose `SimpleKeySchema` with only `PartitionKey`.
//  - `table_name()` returns the function body evaluated at call time.
//
// Note: `table_definitions!` wraps the generated key schema in a
// `const _: () = { ... }` block, so the concrete schema type is opaque.
// We access it only through the `TableDefinition::KeySchema` associated-type
// projection, which is sufficient to verify all trait bounds.

use dynamodb_facade::{
    AttributeDefinition, CompositeKeySchema, KeySchema, SimpleKeySchema, StringAttribute,
    TableDefinition,
};

dynamodb_facade::attribute_definitions! {
    TPk { "TPK": StringAttribute }
    TSk { "TSK": StringAttribute }
}

dynamodb_facade::table_definitions! {
    // Composite key.
    CompositeTestTable {
        type PartitionKey = TPk;
        type SortKey = TSk;
        fn table_name() -> String { "composite".to_owned() }
    }

    // Simple key.
    SimpleTestTable {
        type PartitionKey = TPk;
        fn table_name() -> String { "simple".to_owned() }
    }
}

#[test]
fn test_table_definitions_composite_and_simple_wiring() {
    // `table_name()` returns the declared string.
    assert_eq!(CompositeTestTable::table_name(), "composite");
    assert_eq!(SimpleTestTable::table_name(), "simple");

    // Key schema types wired correctly.
    // For composite: PartitionKey and SortKey.
    type CompositeKS = <CompositeTestTable as TableDefinition>::KeySchema;
    fn assert_composite<KS: CompositeKeySchema>() {}
    assert_composite::<CompositeKS>();
    assert_eq!(<CompositeKS as KeySchema>::PartitionKey::NAME, "TPK");
    assert_eq!(<CompositeKS as CompositeKeySchema>::SortKey::NAME, "TSK");

    // For simple: only PartitionKey.
    type SimpleKS = <SimpleTestTable as TableDefinition>::KeySchema;
    fn assert_simple<KS: SimpleKeySchema>() {}
    assert_simple::<SimpleKS>();
    assert_eq!(<SimpleKS as KeySchema>::PartitionKey::NAME, "TPK");
}
