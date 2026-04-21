// Should fail: calling `.sk_eq()` on a `KeyCondition` whose table has a
// simple key schema (partition key only, no sort key).
//
// The `sk_*` methods are defined on `impl<'a, KS: CompositeKeySchema> KeyCondition<'a, KS>`.
// When `KS = TableSchema<SimpleTable>` and `SimpleTable` has no `SortKey`,
// `TableSchema<SimpleTable>` implements `SimpleKeySchema` but NOT
// `CompositeKeySchema`, so the `sk_eq` method does not exist for this type.
//
// The compiler rejects the call with "no method named `sk_eq`".

use dynamodb_facade::{KeyCondition, StringAttribute, TableSchema};

dynamodb_facade::attribute_definitions! {
    SPk { "SPK": StringAttribute }
}

dynamodb_facade::table_definitions! {
    SimpleTable {
        type PartitionKey = SPk;
        fn table_name() -> String { "simple".to_owned() }
    }
}

fn main() {
    let _ = KeyCondition::<TableSchema<SimpleTable>>::pk("abc").sk_eq("oops"); // <-- compile error here
}
