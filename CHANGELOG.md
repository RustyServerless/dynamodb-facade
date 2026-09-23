# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

[Unreleased]: https://github.com/RustyServerless/dynamodb-facade/compare/v0.2.0...main

## [0.2.0]

### Added

- **Breaking**: `IntoAttributeValue` and `IntoStringAttributeValue` implementations for `Rc<str>`, `Arc<str>` and `Box<str>`.
- The `global` module with the `init_global_client` and `global_client` functions allowing to define and use a globally defined and accessible `aws_sdk_dynamodb::Client`
- A new version of the `DynamoDBItemOp` trait with I/O operations that no longer take the `client` parameter but instead rely on the `global_client`.
- A new version of the `dynamodb_batch_write` function that no longer takes the `client` parameter but instead relies on the `global_client`.
- The old behavior still exists and has been put in a new `explicit_client` public module.

### Changed

- Bumped MSRV to 1.88 and re-fixing try_build tests outputs
- Renamed the `integration` feature to `dyndb-local-integration` to clarify its purpose
- **Breaking**: `DynamoDBItemOp` methods no longer take the `client` parameter but instead rely on the `global_client`. Use `dynamodb_facade::explicit_client::DynamoDBItemOp` instead of `dynamodb_facade::DynamoDBItemOp` to keep the old behavior.
- **Breaking**: `dynamodb_batch_write` no longer takes the `client` parameter but instead relies on the `global_client`. Use `dynamodb_facade::explicit_client::dynamodb_batch_write` instead of `dynamodb_facade::dynamodb_batch_write` to keep the old behavior.
- **Breaking**: Renamed the `QueryRequest` and `ScanRequest` index constructors from `new_index` to `index_new`.
- **Breaking**: All the operations `*Request` (`GetRequest`, `PutRequest`, `QueryRequest`, ...) builders `new()` constructors are no longer expecting a `client` parameter. The new `with_client` constructor has been added with the old `new` signature.
- Making clippy more aggressive and adding missing doc

[0.2.0]: https://github.com/RustyServerless/dynamodb-facade/compare/v0.1.1...v0.2.0

## [0.1.1] - 2026-04-27

### Added

- `Projection::keys_only()` constructor for building a key-only projection (PK, plus SK on composite-key tables) without supplying any extra attribute names.
- Re-export the entire `aws_sdk_dynamodb` crate so downstream code can reach any SDK item (e.g. `aws_sdk_dynamodb::operation::*`, `aws_sdk_dynamodb::types::*`) without adding a separate dependency and risking a version mismatch.

### Changed

- Removed over-restrictive `DeserializeOwned` bounds from request constructors and return-value transitions. Hand-implemented `DynamoDBItem` types that aren't `DeserializeOwned` can now build and configure requests then ask for `.raw()` output ; the bound is still enforced at the "Typed" terminal step where deserialization actually happens.

### Fixed

- Crate-level documentation in `lib.rs` listed only four `Error` variants; updated to enumerate all five (`DynamoDB`, `Serde`, `FailedBatchWrite`, `Other`, `Custom`).
- Corrected an incorrect code snippet in `EXAMPLES.md`.

[0.1.1]: https://github.com/RustyServerless/dynamodb-facade/compare/v0.1.0...v0.1.1

## [0.1.0] - 2026-04-21

First public release on crates.io. `dynamodb-facade` is a typed facade over [`aws-sdk-dynamodb`](https://crates.io/crates/aws-sdk-dynamodb) that replaces string-spliced expressions, hand-built key maps, pagination loops, and 25-item batch chunking with composable, compile-time-checked Rust.

### Added

- **Expression builders, not strings.** `Condition::eq`, `Condition::exists`, `Condition::begins_with`, `Condition::between`, `Condition::is_in`, `Condition::size_cmp`, variadic `Condition::and` / `Condition::or`, combined with `&`, `|`, `!`. `Update::set`, `Update::remove`, `Update::increment`, `Update::decrement`, `Update::init_increment`, `Update::init_decrement`, `Update::set_custom` (with `UpdateSetRhs`), `Update::list_append`, `Update::list_prepend`, `Update::add`, `Update::delete`, `Update::and`, `Update::combine`, `Update::try_combine`. `KeyCondition::pk(...)` with `.sk_eq`, `.sk_begins_with`, `.sk_between`, `.sk_lt`/`.sk_le`/`.sk_gt`/ `.sk_ge`. Every `#name` / `:value` placeholder is managed internally.
- **Zero-sized schema types.** `attribute_definitions!`, `table_definitions!` and `index_definitions!` declarative macros generate marker types encoding the table / index key shape. Simple-key and composite-key tables and indexes are both supported. Using a sort-key method on a simple-key schema is a compile error.
- **Typed items via `dynamodb_item!`.** Wires a user struct to a table with constant or variable PK / SK, a type discriminator attribute, marker-only attributes (e.g. GSI partition keys that serde already serialises), and delegation of key construction to another type. `has_attributes!` macro for hand-rolled `DynamoDBItem` implementations.
- **Typestate operation builders.** `get`, `put`, `delete`, `update`, `query`, `scan` and their `_by_id` / instance variants. Three orthogonal typestate dimensions: output format (`Typed` ↔ `Raw`), return value (`ReturnNothing` ↔ `Return<Old>` ↔ `Return<New>`), and expression-set state (`NoCondition` / `NoFilter` / `NoProjection` ↔ `AlreadyHas*`). Calling `.condition()`, `.filter()` or `.project()` twice is a compile error. `.exists()` / `.not_exists()` shorthands derived from the item's PK. `.raw()` escape hatch to an untyped `Item<TD>`.
- **Single-table friendly.** First-class support for the mono-table pattern (PK + SK + type discriminator). `Item<TD>::attribute::<ItemType>()` for runtime dispatch on scan/query results, `T::from_item` / `T::try_from_item` for typed reconstruction, `Item::into_key_only` / `Item::minimal_from` / `Item::with_attributes` / `Item::extract_key` / `Item::from_key_and_attributes` for key-level manipulation.
- **Automatic pagination.** `.all()` collects all pages into a `Vec`, `.stream()` yields an `impl Stream<Item = Result<T>>` (or `Item<TD>` in Raw mode). No `ExclusiveStartKey` bookkeeping. Works on both query and scan. `query_all` / `query_all_index` for types with a constant partition key.
- **Index queries.** `T::query_index::<Idx>(client, key_cond)` and `T::index_key_condition::<Idx>(id)` for typed index queries, `QueryRequest::new_index::<Idx>(...)` for raw-level index queries when the entity type does not match the index's primary attribute.
- **Automatic batch chunking and retry.** `dynamodb_batch_write` splits `Vec<WriteRequest>` into 25-item batches, runs them in parallel, and retries `UnprocessedItems` with backoff (up to 5 attempts). `T::batch_put`, `T::batch_delete`, `T::batch_delete_by_id`, and the free `batch_delete` helper produce the individual requests. Mixed put + delete batches supported.
- **Typed transactions.** `T::transact_put`, `T::transact_delete`, `T::transact_update` / `transact_update_by_id`, `T::transact_condition` and `T::transact_delete_by_id` build a `TransactWriteItem` with the same condition DSL as stand-alone operations, then `.build()` plugs into the SDK's `transact_write_items()` fluent builder.
- **Flexible serialisation.** Items round-trip through `serde_dynamo` by default; `DynamoDBItem<TD>` can be hand-implemented when serde is not a good fit (e.g. enum stored as a single attribute). The `TD` table parameter is generic, so a single struct may implement `DynamoDBItem` for multiple tables (useful for migrations).
- **Typed values.** `IntoAttributeValue` trait for domain newtypes, with implementations for the standard scalar types, `AsSet<T>` for DynamoDB set types (SS/NS/BS), `AsNumber<T>` for numeric-flavoured attributes.
- **Unified error type.** `Error` enum with `DynamoDB`, `Serde`, `FailedBatchWrite`, `Other`, `Custom` variants and a `Result<T>` alias. `Error::as_dynamodb_error()` downcasts for matching specific SDK errors (e.g. `ConditionalCheckFailedException`). `Error::custom` / `Error::other` constructors.
- **Escape hatch preserved.** Every builder exposes `.into_inner()` returning the underlying `aws_sdk_dynamodb` fluent builder. The crate re-exports `aws_sdk_dynamodb::{Client, Error as DynamoDBError, types::AttributeValue}` so downstream code does not need to pin the SDK version separately.
- **MSRV 1.85.0, edition 2024.** `tokio` / `futures`-based async, identical runtime assumptions to `aws-sdk-dynamodb` itself.
- **Feature flags.** `test-fixtures` exposes the shared `PlatformTable` / `User` / `Enrollment` / `TypeIndex` / `EmailIndex` domain types used in doc examples. `integration` gates the Docker-backed end-to-end test suite against DynamoDB Local.
- **Documentation.** Full API documentation on [docs.rs], a [`README.md`](README.md) with a side-by-side raw-SDK-vs-facade comparison, and [`EXAMPLES.md`](EXAMPLES.md) — a 13-section tour across a single-table domain covering schema design, every CRUD variant, index queries, scans with dispatch, the full condition and update DSL, batch writes, and transactions.

[docs.rs]: https://docs.rs/dynamodb-facade
[0.1.0]: https://github.com/RustyServerless/dynamodb-facade/compare/v0.0.0...v0.1.0

## [0.0.0] - 2026-02-20

### Added

- Crate.io placeholder

[0.0.0]: https://github.com/RustyServerless/dynamodb-facade/releases/tag/v0.0.0
