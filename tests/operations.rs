// Integration tests for DynamoDB operations.
// Gated on the "integration" feature — requires Docker for DynamoDB Local.
//
// The four scenario modules each exercise a distinct end-to-end workflow:
//   - crud_lifecycle     : full CRUD lifecycle with every builder variant
//   - query_and_scan     : query/scan over a realistic multi-type dataset
//   - batch_and_pagination: batch writes, auto-pagination, and streaming
//   - transactions       : atomic transactions, rollback, and swap
#![cfg(feature = "integration")]

mod common;

mod operations {
    mod batch_and_pagination;
    mod crud_lifecycle;
    mod query_and_scan;
    mod transactions;
}
