// =============================================================================
// Scenario: Batch Writes and Pagination
// =============================================================================
//
// Goal: Exercise batch-write chunking and query pagination (both .all() and
// .stream()) over a large dataset.
//
// Schema: composite PK+SK table (name published via BP_TABLE_NAME OnceLock).
//   - Doc: PK = "DOC" (constant), SK = "SEQ#{seq:08}" (zero-padded for sort),
//          _TYPE = "DOC"
//
// Steps:
//   1.  Batch put 60 items — 3 chunks of 25+25+10 (exercises chunking)
//   2.  Query all with .all() (auto-paginate) → 60 docs
//   3.  Query with .limit(10).all() → still 60 docs (auto-pagination drains pages)
//   4.  Stream pages with .limit(10).stream() → total items across pages = 60
//   5.  Batch delete first 30 items by key
//   6.  Query again → 30 remaining docs
// =============================================================================

use std::pin::pin;
use std::sync::OnceLock;

use dynamodb_facade::{
    DynamoDBItemBatchOp, DynamoDBItemOp, Result, attribute_definitions, dynamodb_batch_write,
    dynamodb_item, table_definitions,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

/// Holds the per-test table name.  Initialized once at the start of the test
/// before any DynamoDB call, and read by `BpTable::table_name()` on every
/// operation.  Using `OnceLock` avoids the `unsafe` mutable-environment
/// manipulation that would be required with `std::env::set_var`.
static BP_TABLE_NAME: OnceLock<String> = OnceLock::new();

// ---------------------------------------------------------------------------
// Local schema — unique to this scenario (BP_TABLE_NAME OnceLock)
// ---------------------------------------------------------------------------

attribute_definitions! {
    BpPK   { "PK":    dynamodb_facade::StringAttribute }
    BpSK   { "SK":    dynamodb_facade::StringAttribute }
    BpType { "_TYPE": dynamodb_facade::StringAttribute }
}

table_definitions! {
    BpTable {
        type PartitionKey = BpPK;
        type SortKey = BpSK;
        fn table_name() -> String {
            BP_TABLE_NAME
                .get()
                .expect("BP_TABLE_NAME must be set by the test before any DynamoDB call")
                .clone()
        }
    }
}

// ---------------------------------------------------------------------------
// Doc item — constant PK "DOC", sortable SK "SEQ#{seq:08}"
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Doc {
    pub seq: u32,
    pub data: String,
}

dynamodb_item! {
    #[table = BpTable]
    Doc {
        #[partition_key]
        BpPK { const VALUE: &'static str = "DOC"; }
        #[sort_key]
        BpSK {
            fn attribute_id(&self) -> &'id u32 { &self.seq }
            fn attribute_value(id) -> String { format!("SEQ#{id:08}") }
        }
        BpType { const VALUE: &'static str = "DOC"; }
    }
}

// ---------------------------------------------------------------------------
// The test
// ---------------------------------------------------------------------------

#[tokio::test]
async fn batch_and_pagination() -> Result<()> {
    let ctx = crate::common::TestContext::new("bp").await;

    // Publish the table name to the OnceLock before any DynamoDB call so that
    // `BpTable::table_name()` can read it.  Each scenario uses its own
    // OnceLock static so parallel tests cannot interfere with each other.
    BP_TABLE_NAME
        .set(ctx.table_name.clone())
        .expect("BP_TABLE_NAME must only be set once per test binary");

    let client = ctx.client.clone();

    // ---- 1. Batch put 60 items ----
    //
    // 60 items → 3 batch-write chunks (25 + 25 + 10).
    // dynamodb_batch_write handles the chunking and parallel dispatch
    // automatically.  We verify the count in step 2.
    let docs: Vec<Doc> = (0..60u32)
        .map(|i| Doc {
            seq: i,
            data: format!("payload-{i}"),
        })
        .collect();

    let requests: Vec<_> = docs.iter().map(|d| d.batch_put()).collect();
    dynamodb_batch_write::<BpTable>(client.clone(), requests).await?;

    // ---- 2. Query all with .all() (auto-paginate) ----
    //
    // Doc has a constant PK ("DOC"), so we use query_all() which builds the
    // key condition from the type's HasConstAttribute<BpPK> impl.
    // .all() follows LastEvaluatedKey automatically until all items are
    // returned, regardless of how many internal SDK calls are needed.
    let all_docs: Vec<Doc> = Doc::query_all(client.clone()).all().await?;

    assert_eq!(
        all_docs.len(),
        60,
        ".all() should return all 60 docs via auto-pagination"
    );

    // ---- 3. Query with .limit(10).all() ----
    //
    // Setting .limit(10) tells DynamoDB to evaluate at most 10 items per
    // internal SDK call.  However, .all() still drains all pages, so the
    // total count must still be 60.  This verifies that the pagination loop
    // correctly follows LastEvaluatedKey even when a per-page limit is set.
    let limited_all: Vec<Doc> = Doc::query_all(client.clone()).limit(10).all().await?;

    assert_eq!(
        limited_all.len(),
        60,
        ".limit(10).all() should still return all 60 docs via auto-pagination"
    );

    // ---- 4. Stream pages with .limit(10).stream() ----
    //
    // .stream() yields one Vec<Doc> per DynamoDB page.  With limit=10 and 60
    // items, we expect 6 pages of 10 items each.  We collect all pages and
    // verify the total item count equals 60.
    let stream = Doc::query_all(client.clone()).limit(10).stream();
    let mut stream = pin!(stream);

    let mut total_from_stream = 0usize;
    let mut page_count = 0usize;
    while let Some(page_result) = stream.next().await {
        let page: Vec<Doc> = page_result?;
        total_from_stream += page.len();
        page_count += 1;
    }

    assert_eq!(
        total_from_stream, 60,
        "total items across all streamed pages should be 60"
    );
    // With limit=10 and 60 items we expect at least 6 pages.
    assert!(
        page_count >= 6,
        "should have at least 6 pages of 10 items each"
    );

    // ---- 5. Batch delete first 30 items ----
    //
    // Build delete requests for the first 30 docs (seq 0..29) using
    // batch_delete(), which extracts the key from the loaded item.
    let delete_requests: Vec<_> = docs[0..30].iter().map(|d| d.batch_delete()).collect();
    dynamodb_batch_write::<BpTable>(client.clone(), delete_requests).await?;

    // ---- 6. Query again — 30 remaining ----
    //
    // After deleting the first 30 docs (seq 0..29), only seq 30..59 remain.
    let remaining: Vec<Doc> = Doc::query_all(client.clone()).all().await?;

    assert_eq!(
        remaining.len(),
        30,
        "30 docs should remain after batch-deleting the first 30"
    );
    // The remaining docs should all have seq >= 30.
    for d in &remaining {
        assert!(
            d.seq >= 30,
            "remaining docs should have seq >= 30, got seq={}",
            d.seq
        );
    }

    ctx.cleanup().await;
    Ok(())
}
