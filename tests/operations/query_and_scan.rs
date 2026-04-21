// =============================================================================
// Scenario: Query and Scan
// =============================================================================
//
// Goal: Exercise query and scan operations over a realistic multi-type dataset.
//
// Schema: composite PK+SK table (name published via QS_TABLE_NAME OnceLock) with two GSIs:
//   - iType  on _TYPE  (HASH) — query all items of a given type
//   - iEmail on email  (HASH) — look up a user by email
//
// Two item types:
//   User       — PK = "USER#{id}",       SK = "USER",          _TYPE = "USER"
//   Enrollment — PK = "USER#{user_id}",  SK = "ENROLL#{course_id}", _TYPE = "ENROLLMENT"
//
// Steps:
//   1.  Batch put 5 users + 25 enrollments (30 items total, 2 batch chunks)
//   2.  Query user's enrollments by PK + SK begins_with → 5 items
//   3.  Query GSI iType for all users → 5 items
//   4.  Query GSI iEmail by email → 1 user
//   5.  Scan with filter (role == "instructor") → count matches seeded instructors
//   6.  Scan raw (whole table) → 30 raw items
// =============================================================================

use std::sync::OnceLock;

use dynamodb_facade::{
    Condition, DynamoDBItemBatchOp, DynamoDBItemOp, Item, KeyCondition, Result,
    attribute_definitions, dynamodb_batch_write, dynamodb_item, index_definitions,
    table_definitions,
};
use serde::{Deserialize, Serialize};

/// Holds the per-test table name.  Initialized once at the start of the test
/// before any DynamoDB call, and read by `QsTable::table_name()` on every
/// operation.  Using `OnceLock` avoids the `unsafe` mutable-environment
/// manipulation that would be required with `std::env::set_var`.
static QS_TABLE_NAME: OnceLock<String> = OnceLock::new();

// ---------------------------------------------------------------------------
// Local schema — unique to this scenario (QS_TABLE_NAME OnceLock)
// ---------------------------------------------------------------------------

attribute_definitions! {
    QsPK    { "PK":    dynamodb_facade::StringAttribute }
    QsSK    { "SK":    dynamodb_facade::StringAttribute }
    QsType  { "_TYPE": dynamodb_facade::StringAttribute }
    QsEmail { "email": dynamodb_facade::StringAttribute }
}

table_definitions! {
    QsTable {
        type PartitionKey = QsPK;
        type SortKey = QsSK;
        fn table_name() -> String {
            QS_TABLE_NAME
                .get()
                .expect("QS_TABLE_NAME must be set by the test before any DynamoDB call")
                .clone()
        }
    }
}

index_definitions! {
    #[table = QsTable]
    QsTypeIndex {
        type PartitionKey = QsType;
        fn index_name() -> String { "iType".to_owned() }
    }

    #[table = QsTable]
    QsEmailIndex {
        type PartitionKey = QsEmail;
        fn index_name() -> String { "iEmail".to_owned() }
    }
}

// ---------------------------------------------------------------------------
// User item
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
}

dynamodb_item! {
    #[table = QsTable]
    User {
        #[partition_key]
        QsPK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        QsSK { const VALUE: &'static str = "USER"; }
        QsType { const VALUE: &'static str = "USER"; }
        #[marker_only]
        QsEmail {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
    }
}

// ---------------------------------------------------------------------------
// Enrollment item
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enrollment {
    pub user_id: String,
    pub course_id: String,
    pub enrolled_at: u64,
    pub progress: f64,
}

dynamodb_item! {
    #[table = QsTable]
    Enrollment {
        #[partition_key]
        QsPK {
            fn attribute_id(&self) -> &'id str { &self.user_id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        QsSK {
            fn attribute_id(&self) -> &'id str { &self.course_id }
            fn attribute_value(id) -> String { format!("ENROLL#{id}") }
        }
        QsType { const VALUE: &'static str = "ENROLLMENT"; }
    }
}

// ---------------------------------------------------------------------------
// The test
// ---------------------------------------------------------------------------

#[tokio::test]
async fn query_and_scan() -> Result<()> {
    let ctx = crate::common::TestContext::new("qs").await;

    // Publish the table name to the OnceLock before any DynamoDB call so that
    // `QsTable::table_name()` can read it.  Each scenario uses its own
    // OnceLock static so parallel tests cannot interfere with each other.
    QS_TABLE_NAME
        .set(ctx.table_name.clone())
        .expect("QS_TABLE_NAME must only be set once per test binary");

    let client = ctx.client.clone();

    // ---- 1. Batch put 5 users + 25 enrollments ----
    //
    // We seed a realistic dataset: 5 users (2 instructors, 3 students) and
    // 5 enrollments per user (25 total).  Total = 30 items, which requires
    // 2 batch-write chunks (25 + 5).  This exercises the chunking logic in
    // dynamodb_batch_write.

    // Build users: user-0 and user-1 are instructors; user-2..4 are students.
    let users: Vec<User> = (0..5)
        .map(|i| User {
            id: format!("user-{i}"),
            name: format!("User {i}"),
            email: format!("user-{i}@example.com"),
            role: if i < 2 {
                "instructor".to_owned()
            } else {
                "student".to_owned()
            },
        })
        .collect();

    // Build enrollments: each user is enrolled in courses 0..5.
    let enrollments: Vec<Enrollment> = users
        .iter()
        .flat_map(|u| {
            (0..5).map(move |c| Enrollment {
                user_id: u.id.clone(),
                course_id: format!("course-{c}"),
                enrolled_at: 1_700_000_000 + c as u64,
                progress: c as f64 * 0.2,
            })
        })
        .collect();

    // Collect all write requests and batch-write them.
    let mut requests: Vec<_> = users.iter().map(|u| u.batch_put()).collect();
    requests.extend(enrollments.iter().map(|e| e.batch_put()));
    // 30 items → 2 chunks (25 + 5)
    dynamodb_batch_write::<QsTable>(client.clone(), requests).await?;

    // ---- 2. Query user's enrollments (PK + SK begins_with) ----
    //
    // Query all items under PK = "USER#user-0" whose SK starts with "ENROLL#".
    // This should return exactly the 5 enrollments for user-0, not the user
    // item itself (SK = "USER" does not match the begins_with filter).
    let enrollments_for_user0: Vec<Enrollment> = Enrollment::query(
        client.clone(),
        Enrollment::key_condition("user-0").sk_begins_with("ENROLL#"),
    )
    .all()
    .await?;

    assert_eq!(
        enrollments_for_user0.len(),
        5,
        "should return exactly 5 enrollments for user-0"
    );
    for e in &enrollments_for_user0 {
        assert_eq!(
            e.user_id, "user-0",
            "all returned enrollments should belong to user-0"
        );
    }

    // ---- 3. Query GSI iType for all users ----
    //
    // The iType GSI has _TYPE as its partition key.  Querying with
    // KeyCondition::pk("USER") returns all items whose _TYPE == "USER".
    // There are 5 users, so we expect exactly 5 results.
    let all_users: Vec<User> =
        User::query_index::<QsTypeIndex>(client.clone(), KeyCondition::pk("USER"))
            .all()
            .await?;

    assert_eq!(
        all_users.len(),
        5,
        "iType GSI query for 'USER' should return all 5 users"
    );
    // ---- 3bis. Query GSI iType for all users ----
    //
    // The iType GSI has _TYPE as its partition key. Querying with
    // query_all_index should returns all items whose _TYPE == "USER".
    // There are 5 users, so we expect exactly 5 results.
    let all_users: Vec<User> = User::query_all_index::<QsTypeIndex>(client.clone())
        .all()
        .await?;

    assert_eq!(
        all_users.len(),
        5,
        "iType GSI query for 'USER' should return all 5 users"
    );

    // ---- 4. Query GSI iEmail by email ----
    //
    // The iEmail GSI has email as its partition key.  Querying with a specific
    // email address should return exactly 1 user.
    let by_email: Vec<User> =
        User::query_index::<QsEmailIndex>(client.clone(), KeyCondition::pk("user-2@example.com"))
            .all()
            .await?;

    assert_eq!(
        by_email.len(),
        1,
        "iEmail GSI query should return exactly 1 user for a unique email"
    );
    assert_eq!(
        by_email[0].id, "user-2",
        "the returned user should be user-2"
    );

    // ---- 5. Scan with filter (role == "instructor") ----
    //
    // User::scan scans the entire table and attempts to deserialize every item
    // as a User.  We add a filter so only items with role == "instructor" are
    // returned.  However, because the table also contains Enrollment items
    // (which lack a "role" field), we use raw scan + filter to avoid
    // deserialization errors on non-User items.
    //
    // We scan the iType GSI (which contains only User items when filtered by
    // _TYPE = "USER") to avoid mixed-type deserialization issues.
    let instructors: Vec<User> = User::scan_index::<QsTypeIndex>(client.clone())
        .filter(Condition::eq("role", "instructor"))
        .all()
        .await?;

    // We seeded 2 instructors (user-0 and user-1).
    assert_eq!(
        instructors.len(),
        2,
        "scan with role==instructor filter should return exactly 2 users"
    );
    for u in &instructors {
        assert_eq!(
            u.role, "instructor",
            "all returned users should have role 'instructor'"
        );
    }

    // ---- 6. Scan raw (whole table) ----
    //
    // A raw scan of the entire table returns all 30 items (5 users + 25
    // enrollments) as untyped Item<QsTable> maps.  We use .raw() to avoid
    // deserialization failures on mixed item types.
    let all_raw: Vec<Item<QsTable>> = User::scan(client.clone()).raw().all().await?;

    assert_eq!(
        all_raw.len(),
        30,
        "raw scan of the whole table should return all 30 items"
    );

    ctx.cleanup().await;
    Ok(())
}
