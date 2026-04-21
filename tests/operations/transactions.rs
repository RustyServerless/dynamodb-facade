// =============================================================================
// Scenario: Transactions
// =============================================================================
//
// Goal: Exercise DynamoDB TransactWriteItems — happy path, rollback on
// condition failure, and an atomic swap operation.
//
// Schema: composite PK+SK table (name published via TXN_TABLE_NAME OnceLock).
//   - TxnUser:       PK = "USER#{id}",       SK = "USER",          counter: u32
//   - TxnEnrollment: PK = "USER#{user_id}",  SK = "ENROLL#{course_id}"
//
// Steps:
//   1.  Seed a TxnUser with counter = 0 via plain put.
//   2.  Happy transaction: atomically put a new enrollment (not_exists) AND
//       increment the user's counter (exists + counter < 10).
//       Verify enrollment exists and counter == 1.
//   3.  Condition-failure rollback: attempt the same transaction with a
//       condition that will fail (counter < 0).  Assert TransactionCanceledException.
//       Verify state is unchanged (enrollment NOT added, counter still 1).
//   4.  Swap: atomically delete the existing enrollment AND put a new one
//       under a different course_id.  Verify old key is gone, new key exists.
// =============================================================================

use std::sync::OnceLock;

use dynamodb_facade::{
    Condition, DynamoDBError, DynamoDBItemOp, DynamoDBItemTransactOp, KeyId, Result, Update,
    attribute_definitions, dynamodb_item, table_definitions,
};
use serde::{Deserialize, Serialize};

/// Holds the per-test table name.  Initialized once at the start of the test
/// before any DynamoDB call, and read by `TxnTable::table_name()` on every
/// operation.  Using `OnceLock` avoids the `unsafe` mutable-environment
/// manipulation that would be required with `std::env::set_var`.
static TXN_TABLE_NAME: OnceLock<String> = OnceLock::new();
// ---------------------------------------------------------------------------
// Local schema — unique to this scenario (TXN_TABLE_NAME OnceLock)
// ---------------------------------------------------------------------------
attribute_definitions! {
    TxnPK { "PK": dynamodb_facade::StringAttribute }
    TxnSK { "SK": dynamodb_facade::StringAttribute }
}
table_definitions! {
    TxnTable {
        type PartitionKey = TxnPK;
        type SortKey = TxnSK;
        fn table_name() -> String {
            TXN_TABLE_NAME
                .get()
                .expect("TXN_TABLE_NAME must be set by the test before any DynamoDB call")
                .clone()
        }
    }
}
// ---------------------------------------------------------------------------
// TxnUser item — tracks an enrollment counter
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxnUser {
    pub id: String,
    pub counter: u32,
}
dynamodb_item! {
    #[table = TxnTable]
    TxnUser {
        #[partition_key]
        TxnPK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        TxnSK { const VALUE: &'static str = "USER"; }
    }
}
// ---------------------------------------------------------------------------
// TxnEnrollment item
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxnEnrollment {
    pub user_id: String,
    pub course_id: String,
}
dynamodb_item! {
    #[table = TxnTable]
    TxnEnrollment {
        #[partition_key]
        TxnPK {
            fn attribute_id(&self) -> &'id str { &self.user_id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        TxnSK {
            fn attribute_id(&self) -> &'id str { &self.course_id }
            fn attribute_value(id) -> String { format!("ENROLL#{id}") }
        }
    }
}
// ---------------------------------------------------------------------------
// Helper: assert a Result is a TransactionCanceledException
// ---------------------------------------------------------------------------
fn assert_transaction_cancelled<T: std::fmt::Debug>(result: Result<T>) {
    let err = result.expect_err("expected TransactionCanceledException");
    assert!(
        matches!(
            err.as_dynamodb_error(),
            Some(DynamoDBError::TransactionCanceledException(_))
        ),
        "expected TransactionCanceledException, got: {err:?}"
    );
}
// ---------------------------------------------------------------------------
// The test
// ---------------------------------------------------------------------------
#[tokio::test]
async fn transactions() -> Result<()> {
    let ctx = crate::common::TestContext::new("txn").await;
    // Publish the table name to the OnceLock before any DynamoDB call so that
    // `TxnTable::table_name()` can read it.  Each scenario uses its own
    // OnceLock static so parallel tests cannot interfere with each other.
    TXN_TABLE_NAME
        .set(ctx.table_name.clone())
        .expect("TXN_TABLE_NAME must only be set once per test binary");
    let client = ctx.client.clone();
    // ---- 1. Seed user with counter = 0 ----
    //
    // We need a pre-existing user so the transactional update can use
    // exists() as a guard.  A plain unconditional put is sufficient here.
    let user = TxnUser {
        id: "txn-user-1".to_owned(),
        counter: 0,
    };
    user.put(client.clone()).await?;
    // Verify the seed was written correctly.
    let seeded: TxnUser = TxnUser::get(client.clone(), KeyId::pk("txn-user-1"))
        .await?
        .expect("seeded user should exist");
    assert_eq!(seeded.counter, 0, "initial counter should be 0");
    // ---- 2. Happy transaction ----
    //
    // Atomically:
    //   (a) Put a new enrollment with not_exists() — fails if already present.
    //   (b) Increment the user's counter using init_increment (if_not_exists + add),
    //       guarded by exists() AND counter < 10.
    //
    // Both operations must succeed together or neither applies.
    let enrollment_a = TxnEnrollment {
        user_id: "txn-user-1".to_owned(),
        course_id: "course-A".to_owned(),
    };
    client
        .transact_write_items()
        .transact_items(enrollment_a.transact_put().not_exists().build())
        .transact_items(
            TxnUser::transact_update_by_id(
                KeyId::pk("txn-user-1"),
                Update::init_increment("counter", 0, 1),
            )
            .condition(TxnUser::exists() & Condition::lt("counter", 10))
            .build(),
        )
        .send()
        .await?;
    // Verify enrollment was created.
    let enroll_check: Option<TxnEnrollment> =
        TxnEnrollment::get(client.clone(), KeyId::pk("txn-user-1").sk("course-A")).await?;
    assert!(
        enroll_check.is_some(),
        "enrollment for course-A should exist after happy transaction"
    );
    // Verify counter was incremented.
    let after_happy: TxnUser = TxnUser::get(client.clone(), KeyId::pk("txn-user-1"))
        .await?
        .expect("user should still exist");
    assert_eq!(
        after_happy.counter, 1,
        "counter should be 1 after happy transaction"
    );
    // ---- 3. Condition-failure rollback ----
    //
    // Attempt the same transaction but with a condition that will fail:
    // counter < 0 is false (counter is 1).  The entire transaction must be
    // rolled back — neither the enrollment nor the counter update is applied.
    let enrollment_b = TxnEnrollment {
        user_id: "txn-user-1".to_owned(),
        course_id: "course-B".to_owned(),
    };
    let rollback_result = client
        .transact_write_items()
        .transact_items(enrollment_b.transact_put().not_exists().build())
        .transact_items(
            TxnUser::transact_update_by_id(
                KeyId::pk("txn-user-1"),
                Update::init_increment("counter", 0, 1),
            )
            // counter < 0 is false → this condition fails → whole txn cancelled
            .condition(TxnUser::exists() & Condition::lt("counter", 0))
            .build(),
        )
        .send()
        .await;
    assert_transaction_cancelled(rollback_result.map_err(Into::into));
    // Verify course-B enrollment was NOT created (rollback worked).
    let no_enroll_b: Option<TxnEnrollment> =
        TxnEnrollment::get(client.clone(), KeyId::pk("txn-user-1").sk("course-B")).await?;
    assert!(
        no_enroll_b.is_none(),
        "course-B enrollment must NOT exist after rollback"
    );
    // Verify counter is still 1 (not incremented by the rolled-back txn).
    let after_rollback: TxnUser = TxnUser::get(client.clone(), KeyId::pk("txn-user-1"))
        .await?
        .expect("user should still exist");
    assert_eq!(
        after_rollback.counter, 1,
        "counter must still be 1 after rollback"
    );
    // ---- 4. Swap ----
    //
    // Atomically delete the existing enrollment (course-A) and put a new one
    // (course-C) in a single transaction.  This is the classic "move" pattern.
    // After the transaction: course-A is gone, course-C exists.
    let enrollment_c = TxnEnrollment {
        user_id: "txn-user-1".to_owned(),
        course_id: "course-C".to_owned(),
    };
    client
        .transact_write_items()
        // Delete course-A — must exist (guard against double-delete races).
        .transact_items(
            TxnEnrollment::transact_delete_by_id(KeyId::pk("txn-user-1").sk("course-A"))
                .exists()
                .build(),
        )
        // Put course-C — must not already exist.
        .transact_items(enrollment_c.transact_put().not_exists().build())
        .send()
        .await?;
    // Verify course-A is gone.
    let old_gone: Option<TxnEnrollment> =
        TxnEnrollment::get(client.clone(), KeyId::pk("txn-user-1").sk("course-A")).await?;
    assert!(
        old_gone.is_none(),
        "course-A enrollment should be gone after swap"
    );
    // Verify course-C now exists.
    let new_exists: Option<TxnEnrollment> =
        TxnEnrollment::get(client.clone(), KeyId::pk("txn-user-1").sk("course-C")).await?;
    assert!(
        new_exists.is_some(),
        "course-C enrollment should exist after swap"
    );
    ctx.cleanup().await;
    Ok(())
}
