# AGENTS.md — dynamodb-facade

Rust library: typed facade over `aws-sdk-dynamodb` with expression builders,
typestate operation builders, and batch/transaction support. Single-table
(mono-table) friendly.

- **Edition:** 2024 — **MSRV:** 1.85.0 (`Cargo.toml` `rust-version`)
- **Dev toolchain:** 1.93 (`nix/rust-toolchain.toml`)
- Pure Cargo; no JS/TS tooling. No `rustfmt.toml` / `clippy.toml`.

## Build / Lint / Test

The canonical checks (same as CI and pre-commit) are:

```sh
cargo fmt --check \
&& cargo clippy --all-targets --all-features -- -D warnings \
&& RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --document-private-items \
&& cargo test --all-features
```

All clippy + doc warnings are errors (`-D warnings`). Clippy's
`incompatible_msrv` lint catches APIs introduced after 1.85.0 — run clippy
on **stable**, not MSRV, or the lint is silent.

Single test / module:
```sh
cargo test --all-features <test_name>
cargo test --all-features expressions::utils::tests
```

### Feature flags — important

- `--all-features` enables `integration` **and** `test-fixtures`.
- `integration` — gates everything in `tests/operations.rs` behind
  `#![cfg(feature = "integration")]`. Requires **Docker** (testcontainers
  spins DynamoDB Local). Without Docker these tests don't compile into the
  binary.
- `test-fixtures` — exposes `dynamodb_facade::test_fixtures` (shared domain
  types: `PlatformTable`, `User`, `Enrollment`, `TypeIndex`, `EmailIndex`, ...)
  outside of `cfg(test)` / `cfg(doc)`. Integration tests and many doc examples
  depend on it.
- `.hooks/pre-commit` runs `cargo test --all-features` when Docker is up,
  else falls back to `cargo test --features test-fixtures` (NOT bare
  `cargo test` — bare would fail several doc examples).

### Test layout

Three integration-test binaries under `tests/`:
- `macros.rs` — macro expansion tests, no features needed.
- `try_build.rs` — trybuild compile-pass/compile-fail tests in `tests/try_build/`.
- `operations.rs` — end-to-end CRUD/query/batch/transactions against
  DynamoDB Local. Feature-gated on `integration`. All tests share one
  container via `LazyLock` + per-test random table names (see
  `tests/common/mod.rs`).

CI runs three parallel jobs: `lint`, `test` (both on stable, `--all-features`),
`msrv` (1.85.0, `cargo check` + `cargo test --all-features`). Release
publishes to crates.io on `v*` tags.

## Module Layout

`src/` has four files + six module directories:

- `lib.rs` — barrel (`mod x; pub use x::*;`) and crate-level `//!` docs.
  Re-exports `aws_sdk_dynamodb::{Client, Error as DynamoDBError, types::AttributeValue}`.
- `error.rs`, `utils.rs`, `macros.rs`, `test_fixtures.rs`
- `schema/` — `TableDefinition`, `IndexDefinition`, `KeySchema`, attribute
  marker types (`StringAttribute` / `NumberAttribute` / `BinaryAttribute`).
- `item/` — `DynamoDBItem`, `Item<TD>`, `Key<TD>`, `KeyId`, `NoId`.
- `values/` — `IntoAttributeValue`, `AsSet<T>`, `AsNumber<T>`, typed conversions.
- `expressions/` — `Condition`, `Update`, `KeyCondition`, `Projection`,
  builder traits (see below).
- `operations/` — per-verb request builders + pagination + batch + transactions
  + typestate markers.

### Trait hierarchy (blanket-impl chain)

```
DynamoDBItem<TD>
    └─ DynamoDBItemOp<TD>            (get/put/delete/update/query/scan)
           ├─ DynamoDBItemBatchOp<TD>    (batch_put / batch_delete)
           └─ DynamoDBItemTransactOp<TD> (transact_put / _delete / _update / _condition)
```

Implement **only** `DynamoDBItem` (typically via `dynamodb_item!` macro);
the rest follow automatically. `TD` is a generic parameter, not an associated
type — a single struct may implement `DynamoDBItem` for multiple tables
(useful for migrations between tables).

### Expression builder trait split (`expressions/builders.rs`)

- `ExpressionAttrNames` (sealed, base) — implemented for **all** SDK fluent
  builders, including `GetItemFluentBuilder` which has no
  `expression_attribute_values`.
- `ExpressionAttrBuilder: ExpressionAttrNames` — adds values. Implemented for
  every builder **except Get**.
- `ConditionableBuilder` / `FilterableBuilder` / `KeyConditionableBuilder` /
  `UpdatableBuilder` extend `ExpressionAttrBuilder`.
- `ProjectionableBuilder` extends `ExpressionAttrNames` only (projections
  never use value placeholders).

## Patterns and Conventions

### Imports — three groups, blank-line separated

1. `std` / `core`
2. External crates (`aws_sdk_dynamodb`, `serde`, `serde_dynamo`, `thiserror`, `tracing`)
3. Intra-crate — **always** `use super::...`, never `crate::...`

**Only** accepted use of `crate::` path: macro invocations like
`crate::utils::impl_sealed_marker_types!(...)` (used in `schema/mod.rs`,
`schema/attributes.rs`, `expressions/key_conditions.rs`) — macro paths
require absolute crate-rooted resolution.

### Parameter type preferences

- String-like → `impl Into<Cow<'a, str>>` (zero-copy)
- DynamoDB values → `impl IntoAttributeValue`
- Table / index names → `impl Into<String>`

### Newtype + sealed-inner

Public expression types wrap a **private** inner enum:
`Condition<'a>(ConditionInner<'a>)`, `Update<'a>(UpdateInner<'a>)`, etc.
Inner types stay `pub(super)` / private — never `pub`.

### Sealed trait naming

- Standard: `mod sealed_traits { pub trait FooSeal {} }` — used in
  `builders.rs`, `key_conditions.rs`, `schema/mod.rs`,
  `schema/attributes.rs`, `schema/attribute_list.rs`, `values/typed.rs`.
- **Exception:** `operations/type_state.rs` uses `mod state_traits` with
  **no `*Seal` suffix** (hand-written marker structs + trait impls per
  typestate dimension, not the macro).

### Visibility

- `pub(crate)` — internal traits (`ApplyCondition`, `ApplyUpdate`,
  `ApplyFilter`, `ApplyKeyCondition`, `ExpressionAttrBuilder`, ...).
- `pub(super)` — helpers scoped to a module subtree.
- **Never** leak `*Inner` enums or `Built*` types to `pub`.

### Error handling (`error.rs`)

`Error` variants: `DynamoDB(Box<aws_sdk_dynamodb::Error>)`, `Serde`,
`FailedBatchWrite(Vec<WriteRequest>)`, `Other(Box<dyn Error + Send>)`,
`Custom(String)`. `Result<T>` alias provided.

- `From<SdkError<T,R>>` and `From<aws_sdk_dynamodb::Error>` → `Error::DynamoDB`.
- `Error::as_dynamodb_error()` downcasts for matching specific SDK errors
  (e.g. `ConditionalCheckFailedException`).
- `Error::custom(msg)` / `Error::other(err)` constructors.
- **Never** use bare `.unwrap()` — always `.expect("why this cannot fail")`.
- `panic!()` for violated structural invariants only (e.g. missing PK/SK keys).

### Typestate operation builders — three orthogonal dimensions

All marker types live in `operations/type_state.rs`:

1. **`OutputFormat`** — `Typed` ↔ `Raw`. `.raw()` (one-way) and
   `.project()` (any → Raw) transition. Raw terminals return `Item<TD>`
   instead of deserializing into `T`.
2. **`ReturnValue`** — `ReturnNothing` ↔ `Return<Old>` ↔ `Return<New>`.
   `.return_old()` / `.return_new()` / `.return_none()` transition.
3. **Expression-set state** — `NoCondition`/`AlreadyHasCondition`,
   `NoFilter`/`AlreadyHasFilter`, `NoProjection`/`AlreadyHasProjection`.
   Calling `.condition()` / `.filter()` / `.project()` twice is a
   **compile error**.

All three dimensions are fully orthogonal: every transition preserves the
other typestate parameters.

Type-parameter order (matters for `Type::<TD>` turbofish):
- put / delete / update: `<TD, T, O, R, C>`
- get: `<TD, T, O, P>`
- query / scan: `<TD, T, O, F, P>`

Each builder structure:
1. `pub fn new(...)` — stand-alone constructor (`T = ()`, `O = Raw`).
2. `impl<...> Builder<...>` — shared methods (`into_inner()`).
3. Per-state `impl` blocks — transitioning methods consume `self`.
4. `.execute()` for single-item; `.all()` / `.stream()` for query/scan.
5. `.into_inner()` — escape hatch returning the raw SDK fluent builder.

`IntoFuture` impls extract the SDK builder **before** the `async move` so
`PhantomData<(TD, T, ...)>` is never captured in the returned future.

### Documentation

- `///` on every public item. `//!` module docs only in `lib.rs` and
  `test_fixtures.rs`.
- Section-separator comments: `// -- Section Name ---...`
- Include `# Errors` / `# Panics` on doc comments where they apply.

### Tests

- Unit tests: inline `#[cfg(test)] mod tests { use super::*; ... }`.
- Naming: `test_<function_name>_<scenario>`.
- `assert_eq!` / `assert!` only — no external test frameworks, no mocking.
- Integration tests (Docker) live in `tests/operations/`, share one
  DynamoDB Local container via `LazyLock` in `tests/common/mod.rs`.
- Compile-fail tests in `tests/try_build/fail/*.rs`, compile-pass in
  `tests/try_build/pass/*.rs` (trybuild).
