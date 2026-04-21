// Should fail: calling `.project()` twice on a get builder.
//
// The first `.project()` call transitions the builder's `P` typestate
// parameter from `NoProjection` to `AlreadyHasProjection`. The second call
// requires `P = NoProjection` (the method is only defined on
// `GetItemRequest<TD, T, O, NoProjection>`), so the compiler rejects it with
// a "no method named `project`" or trait-bound error.
//
// The error surface is on the second `.project()` call.

use dynamodb_facade::test_fixtures::*;
use dynamodb_facade::{DynamoDBItemOp, KeyId, Projection};

fn test(client: dynamodb_facade::Client) {
    let _ = User::get(client, KeyId::pk("u-1"))
        .project(Projection::<PlatformTable>::new(["name"]))
        .project(Projection::<PlatformTable>::new(["email"])); // <-- compile error here
}

fn main() {}
