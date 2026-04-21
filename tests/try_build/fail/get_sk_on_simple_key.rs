// Should fail: calling `.sk_eq()` on a `KeyCondition` whose table has a
// simple key schema (partition key only, no sort key).
//
// The `sk_*` methods are defined on `impl<'a, KS: CompositeKeySchema> KeyCondition<'a, KS>`.
// When `KS = TableSchema<SimpleTable>` and `SimpleTable` has no `SortKey`,
// `TableSchema<SimpleTable>` implements `SimpleKeySchema` but NOT
// `CompositeKeySchema`, so the `sk_eq` method does not exist for this type.
//
// The compiler rejects the call with "no method named `sk_eq`".

use dynamodb_facade::test_fixtures::*;
use dynamodb_facade::{DynamoDBItemOp, KeyId, Projection};

fn test(client: dynamodb_facade::Client) {
    let _ = User::get(client, KeyId::pk("u-1").sk("oops")); // <-- compile error here
}

fn main() {}
