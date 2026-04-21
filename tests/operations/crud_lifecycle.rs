// =============================================================================
// Scenario: CRUD Lifecycle
// =============================================================================
//
// Goal: Exercise the full CRUD lifecycle of a single item type, covering every
// major builder variant in one sequential test.
//
// Schema: composite PK+SK table (name published via CRUD_TABLE_NAME OnceLock).
//   - User: PK = "USER#{id}", SK = "USER", _TYPE = "USER", email (marker only)
//
// Steps (in order):
//   1.  Initial state — get a non-existent item → None
//   2.  Put with not_exists() — create-only succeeds
//   3.  Put with not_exists() again — fails with ConditionalCheckFailedException
//   4.  Get typed — returns Some(User) with correct fields
//   5.  Get raw — returns Some(Item<CrudTable>), inspect _TYPE and PK
//   6.  Get projected — only PK, SK, name returned; role and email absent
//   7.  Update with exists() + return_new() — name changed, new item returned
//   8.  Update with failing condition — ConditionalCheckFailedException; item unchanged
//   9.  Compound update (set + remove) — role removed from item
//  10.  Update with return_old() — old item returned before the update
//  11.  Delete with exists() — succeeds
//  12.  Delete with exists() on missing key — ConditionalCheckFailedException
//  13.  Delete without condition on missing key — succeeds (DynamoDB no-op)
//
// Each step is separated by a comment header and explains WHY the assertion
// matters.
// =============================================================================

use std::sync::OnceLock;

use dynamodb_facade::{
    Condition, DynamoDBError, DynamoDBItemOp, Item, KeyId, Projection, Result, Update,
    attribute_definitions, dynamodb_item, table_definitions,
};
use serde::{Deserialize, Serialize};

/// Holds the per-test table name.  Initialized once at the start of the test
/// before any DynamoDB call, and read by `CrudTable::table_name()` on every
/// operation.  Using `OnceLock` avoids the `unsafe` mutable-environment
/// manipulation that would be required with `std::env::set_var`.
static CRUD_TABLE_NAME: OnceLock<String> = OnceLock::new();

// ---------------------------------------------------------------------------
// Local schema — unique to this scenario file
// ---------------------------------------------------------------------------
//
// We define our own attribute/table/item types rather than reusing
// test_fixtures::PlatformTable, because PlatformTable reads TABLE_NAME from an
// env var that would clash with other parallel tests.  Each scenario uses its
// own private OnceLock (CRUD_TABLE_NAME here) so parallel execution is safe.

attribute_definitions! {
    CrudPK { "PK": dynamodb_facade::StringAttribute }
    CrudSK { "SK": dynamodb_facade::StringAttribute }
    CrudItemType { "_TYPE": dynamodb_facade::StringAttribute }
    CrudEmail { "email": dynamodb_facade::StringAttribute }
}

table_definitions! {
    CrudTable {
        type PartitionKey = CrudPK;
        type SortKey = CrudSK;
        fn table_name() -> String {
            // Set by the test before any DynamoDB call.
            CRUD_TABLE_NAME
                .get()
                .expect("CRUD_TABLE_NAME must be set by the test before any DynamoDB call")
                .clone()
        }
    }
}

// User item stored in CrudTable.
// `role` is Option<String> so we can test removing it via Update::remove.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: Option<String>,
}

dynamodb_item! {
    #[table = CrudTable]
    User {
        #[partition_key]
        CrudPK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        CrudSK { const VALUE: &'static str = "USER"; }
        CrudItemType { const VALUE: &'static str = "USER"; }
        #[marker_only]
        CrudEmail {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: assert a Result is a ConditionalCheckFailedException
// ---------------------------------------------------------------------------

fn assert_conditional_check_failed<T: std::fmt::Debug>(result: Result<T>) {
    let err = result.expect_err("expected ConditionalCheckFailedException");
    assert!(
        matches!(
            err.as_dynamodb_error(),
            Some(DynamoDBError::ConditionalCheckFailedException(_))
        ),
        "expected ConditionalCheckFailedException, got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// The test
// ---------------------------------------------------------------------------

#[tokio::test]
async fn crud_lifecycle() -> Result<()> {
    // Spin up the shared DynamoDB Local container and create a fresh table.
    let ctx = crate::common::TestContext::new("crud").await;

    // Publish the table name to the OnceLock before any DynamoDB call so that
    // `CrudTable::table_name()` can read it.  Each scenario uses its own
    // OnceLock static so parallel tests cannot interfere with each other.
    CRUD_TABLE_NAME
        .set(ctx.table_name.clone())
        .expect("CRUD_TABLE_NAME must only be set once per test binary");

    let client = ctx.client.clone();

    // ---- 1. Initial state ----
    // Verify the table is empty: getting a non-existent key must return None.
    let result: Option<User> = User::get(client.clone(), KeyId::pk("u-1")).await?;
    assert!(
        result.is_none(),
        "table should be empty at test start; got: {result:?}"
    );

    // ---- 2. Put with not_exists() — create-only, first time ----
    // This is the canonical "insert if absent" pattern.  It must succeed
    // because the item does not yet exist.
    let alice = User {
        id: "u-1".to_owned(),
        name: "Alice".to_owned(),
        email: "alice@example.com".to_owned(),
        role: Some("student".to_owned()),
    };
    alice.put(client.clone()).not_exists().await?;

    // ---- 3. Put with not_exists() again — must fail ----
    // The item now exists, so the attribute_not_exists(PK) condition fails.
    // DynamoDB returns ConditionalCheckFailedException.
    let duplicate_result = alice.put(client.clone()).not_exists().await;
    assert_conditional_check_failed(duplicate_result);

    // ---- 4. Get typed ----
    // Retrieve the item and verify every field round-trips correctly.
    let loaded: Option<User> = User::get(client.clone(), KeyId::pk("u-1")).await?;
    let loaded = loaded.expect("item should exist after put");
    assert_eq!(loaded.id, alice.id, "id should round-trip");
    assert_eq!(loaded.name, alice.name, "name should round-trip");
    assert_eq!(loaded.email, alice.email, "email should round-trip");
    assert_eq!(loaded.role, alice.role, "role should round-trip");
    assert_eq!(loaded.id, "u-1", "id should round-trip");
    assert_eq!(loaded.name, "Alice", "name should round-trip");
    assert_eq!(loaded.email, "alice@example.com", "email should round-trip");
    assert_eq!(
        loaded.role,
        Some("student".to_owned()),
        "role should round-trip"
    );

    // ---- 5. Get raw ----
    // Switch to raw output to inspect the underlying DynamoDB attribute map.
    // This verifies that the key encoding (PK = "USER#u-1") and the type
    // discriminator (_TYPE = "USER") are written correctly.
    let raw: Option<Item<CrudTable>> = User::get(client.clone(), KeyId::pk("u-1")).raw().await?;
    let raw = raw.expect("raw item should exist");
    assert_eq!(raw.pk(), "USER#u-1", "PK should be encoded as USER#<id>");
    assert_eq!(raw.sk(), "USER", "SK should be the constant 'USER'");
    assert_eq!(
        raw.attribute::<CrudItemType>(),
        Some("USER"),
        "_TYPE discriminator should be 'USER'"
    );

    // ---- 6. Get projected ----
    // Request only the "name" attribute.  The result must contain PK, SK, and
    // name, but NOT role or email — verifying that the projection expression
    // is applied server-side.
    let projected: Option<Item<CrudTable>> = User::get(client.clone(), KeyId::pk("u-1"))
        .project(Projection::<CrudTable>::new(["name"]))
        .await?;
    let projected = projected.expect("projected item should exist");
    assert!(
        projected.contains_key("name"),
        "projected item should contain 'name'"
    );
    assert!(
        !projected.contains_key("role"),
        "projected item must NOT contain 'role' (not in projection)"
    );
    assert!(
        !projected.contains_key("email"),
        "projected item must NOT contain 'email' (not in projection)"
    );

    // ---- 7. Update with exists() + return_new() ----
    // Update the name to "Alicia" and request the post-update item.
    // The exists() guard ensures the update only applies to existing items.
    let updated: User = User::update_by_id(
        client.clone(),
        KeyId::pk("u-1"),
        Update::set("name", "Alicia"),
    )
    .exists()
    .await?;
    assert_eq!(
        updated.name, "Alicia",
        "return_new should reflect the updated name"
    );
    assert_eq!(
        updated.role,
        Some("student".to_owned()),
        "role should be unchanged after name update"
    );

    // ---- 8. Update with failing condition ----
    // Attempt to update the name while asserting role == "admin".  Since the
    // role is "student", the condition fails.  The item must remain unchanged.
    let bad_update = User::update_by_id(
        client.clone(),
        KeyId::pk("u-1"),
        Update::set("name", "Mallory"),
    )
    .condition(Condition::eq("role", "admin"))
    .await;
    assert_conditional_check_failed(bad_update);

    // Verify the item was NOT modified by the failed update.
    let after_failed: User = User::get(client.clone(), KeyId::pk("u-1"))
        .await?
        .expect("item should still exist");
    assert_eq!(
        after_failed.name, "Alicia",
        "name must still be 'Alicia' after failed conditional update"
    );

    // ---- 9. Compound update: set + remove ----
    // Set name to "Bob" and simultaneously remove the role attribute.
    // After this, role should be absent (None when deserialized).
    User::update_by_id(
        client.clone(),
        KeyId::pk("u-1"),
        Update::set("name", "Bob").and(Update::remove("role")),
    )
    .exists()
    .return_none()
    .await?;

    let after_compound: User = User::get(client.clone(), KeyId::pk("u-1"))
        .await?
        .expect("item should still exist");
    assert_eq!(after_compound.name, "Bob", "name should be 'Bob' after set");
    assert!(
        after_compound.role.is_none(),
        "role should be absent after remove"
    );

    // ---- 10. Update with return_old() ----
    // Update name to "Charlie" and request the pre-update item.
    // The returned old item should still have name = "Bob".
    let old: Option<User> = User::update_by_id(
        client.clone(),
        KeyId::pk("u-1"),
        Update::set("name", "Charlie"),
    )
    .exists()
    .return_old()
    .await?;
    let old = old.expect("return_old should return the pre-update item");
    assert_eq!(
        old.name, "Bob",
        "return_old should reflect the name before the update"
    );

    // Confirm the item now has name = "Charlie".
    let current: User = User::get(client.clone(), KeyId::pk("u-1"))
        .await?
        .expect("item should still exist");
    assert_eq!(
        current.name, "Charlie",
        "name should be 'Charlie' after update"
    );

    // ---- 11. Delete with exists() — succeeds ----
    // The item exists, so the attribute_exists(PK) condition passes.
    User::delete_by_id(client.clone(), KeyId::pk("u-1"))
        .exists()
        .return_none()
        .await?;

    // Confirm the item is gone.
    let gone: Option<User> = User::get(client.clone(), KeyId::pk("u-1")).await?;
    assert!(gone.is_none(), "item should be gone after delete");

    // ---- 12. Delete with exists() on missing key — must fail ----
    // The item no longer exists, so the condition fails.
    let delete_missing = User::delete_by_id(client.clone(), KeyId::pk("u-1"))
        .exists()
        .return_none()
        .await;
    assert_conditional_check_failed(delete_missing);

    // ---- 13. Delete without condition on missing key — DynamoDB no-op ----
    // An unconditional delete on a non-existent key is a silent no-op in
    // DynamoDB — it must NOT return an error.
    User::delete_by_id(client.clone(), KeyId::pk("u-1"))
        .return_none()
        .await?;

    // Cleanup: delete the table.
    ctx.cleanup().await;
    Ok(())
}
