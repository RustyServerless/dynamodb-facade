// Should fail: calling `.condition()` twice on a put builder.
//
// The first `.condition()` call transitions the builder's `C` typestate
// parameter from `NoCondition` to `AlreadyHasCondition`. The second call
// requires `C = NoCondition` (the method is only defined on
// `PutItemRequest<TD, T, O, R, NoCondition>`), so the compiler rejects it
// with a "no method named `condition`" or trait-bound error.
//
// The error surface is on the second `.condition()` call.

use dynamodb_facade::test_fixtures::*;
use dynamodb_facade::{Condition, DynamoDBItemOp};

fn test(client: dynamodb_facade::Client) {
    let _ = sample_user()
        .delete(client)
        .condition(Condition::eq("role", "student"))
        .condition(Condition::eq("role", "instructor")); // <-- compile error here
}

fn main() {}
