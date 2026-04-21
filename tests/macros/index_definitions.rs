// Tests for the `index_definitions!` macro.
//
// Verifies that:
//  - The macro generates an `IndexDefinition<TableType>` impl with the
//    declared `KeySchema`.
//  - `index_name()` returns the declared value.
//  - The `#[table = ...]` attribute correctly binds the index to its table.
//
// Note: `index_definitions!` wraps the generated key schema in a
// `const _: () = { ... }` block, so the concrete schema type is opaque.
// We access it only through the `IndexDefinition::KeySchema` associated-type
// projection, which is sufficient to verify all trait bounds.

use dynamodb_facade::{
    AttributeDefinition, CompositeKeySchema, IndexDefinition, KeySchema, StringAttribute,
};

dynamodb_facade::attribute_definitions! {
    IdxPk { "IDX_PK": StringAttribute }
    IdxSk { "IDX_SK": StringAttribute }
    TabPk { "TAB_PK": StringAttribute }
}

dynamodb_facade::table_definitions! {
    TestTable {
        type PartitionKey = TabPk;
        fn table_name() -> String { "tab".to_owned() }
    }
}

dynamodb_facade::index_definitions! {
    #[table = TestTable]
    TestSimpleIndex {
        type PartitionKey = IdxPk;
        fn index_name() -> String { "my-simple-index".to_owned() }
    }

    #[table = TestTable]
    TestCompositeIndex {
        type PartitionKey = IdxPk;
        type SortKey = IdxSk;
        fn index_name() -> String { "my-composite-index".to_owned() }
    }
}

#[test]
fn test_index_definitions_wires_index_to_table() {
    assert_eq!(
        <TestSimpleIndex as IndexDefinition<TestTable>>::index_name(),
        "my-simple-index"
    );
    assert_eq!(
        <TestCompositeIndex as IndexDefinition<TestTable>>::index_name(),
        "my-composite-index"
    );

    type ISimpleKS = <TestSimpleIndex as IndexDefinition<TestTable>>::KeySchema;
    assert_eq!(<ISimpleKS as KeySchema>::PartitionKey::NAME, "IDX_PK");
    type ICompositeKS = <TestCompositeIndex as IndexDefinition<TestTable>>::KeySchema;
    assert_eq!(<ICompositeKS as KeySchema>::PartitionKey::NAME, "IDX_PK");
    assert_eq!(
        <ICompositeKS as CompositeKeySchema>::SortKey::NAME,
        "IDX_SK"
    );
}
