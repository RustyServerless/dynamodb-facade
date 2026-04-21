// Should fail: calling `.filter()` twice on a scan builder.
//
// The first `.filter()` call transitions the builder's `F` typestate
// parameter from `NoFilter` to `AlreadyHasFilter`. The second call requires
// `F = NoFilter` (the method is only defined on
// `ScanRequest<TD, T, O, NoFilter, P>`), so the compiler rejects it with a
// "no method named `filter`" or trait-bound error.
//
// The error surface is on the second `.filter()` call.

use dynamodb_facade::test_fixtures::*;
use dynamodb_facade::{Condition, DynamoDBItemOp};
fn test(client: dynamodb_facade::Client) {
    let _ = User::query(client, User::key_condition("user-1"))
        .filter(Condition::eq("role", "student"))
        .filter(Condition::eq("role", "instructor")); // <-- compile error here
}
fn main() {}
