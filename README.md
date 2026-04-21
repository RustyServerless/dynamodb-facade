<!-- PROJECT SHIELDS -->
[![crates.io](https://img.shields.io/crates/v/dynamodb-facade.svg)](https://crates.io/crates/dynamodb-facade)
[![docs.rs](https://docs.rs/dynamodb-facade/badge.svg)](https://docs.rs/dynamodb-facade/latest/dynamodb_facade)
[![CI](https://github.com/RustyServerless/dynamodb-facade/workflows/CI/badge.svg)](https://github.com/RustyServerless/dynamodb-facade/actions)
[![License](https://img.shields.io/github/license/RustyServerless/dynamodb-facade.svg)](https://github.com/RustyServerless/dynamodb-facade/blob/main/LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.85.0-blue.svg)](https://github.com/RustyServerless/dynamodb-facade/blob/main/Cargo.toml)

# dynamodb-facade

A typed facade over [`aws-sdk-dynamodb`](https://crates.io/crates/aws-sdk-dynamodb) that replaces string-spliced expressions, hand-built key maps, pagination loops, and 25-item batch chunking with composable, compile-time-checked Rust.

> **Pre-1.0.** The API is stabilising but may still change between minor
> versions until `1.0`.

---

## Why this crate

The AWS SDK for DynamoDB is correct and complete, but writing even a simple conditional update means juggling:

- an `expression_attribute_names` map,
- an `expression_attribute_values` map,
- the expression string that references both,
- a manual `HashMap<String, AttributeValue>` key, and
- `serde_dynamo` calls.

`dynamodb-facade` takes that surface and moves as much of it as possible into the type system and internal machinery. **None of the expression wiring is visible in user code**, and several whole categories of bug (duplicate `.condition()`, wrong sort key on a simple-key index, `Return<Old>` deserialisation mismatch) become compile errors.

### Raw SDK vs. facade — a conditional update

Raw `aws-sdk-dynamodb`:

```rust
client.update_item()
    .table_name(table_name())
    .key("PK", AttributeValue::S(format!("USER#{user_id}")))
    .key("SK", AttributeValue::S("USER".to_owned()))
    .update_expression("SET #name = :name")
    .expression_attribute_names("#name", "name")
    .expression_attribute_values(":name", AttributeValue::S(new_name))
    .condition_expression("attribute_exists(PK)")
    .return_values(ReturnValue::AllNew)
    .send()
    .await?
    .attributes
    .map(|attrs| serde_dynamo::from_item(attrs))
    .expect("asked for ALL_NEW")?;
```

Same operation with `dynamodb-facade`:

```rust
User::update_by_id(
    client,
    KeyId::pk(user_id),
    Update::set("name", new_name),
)
.exists()
.await?;
// Returns the updated `User`. Placeholders, key map,
// return-value plumbing, and deserialisation are all handled.
```

The same compression applies across every operation. A 50-line raw batch-write loop with manual 25-item chunking, parallel dispatch, and `UnprocessedItems` retry becomes a single call to `dynamodb_batch_write`. Hand-rolled `ExclusiveStartKey` pagination becomes `.all()` or `.stream()`.

---

<details>
  <summary>Table of Contents</summary>
  <ol>
    <li><a href="#features">Features</a></li>
    <li><a href="#getting-started">Getting Started</a></li>
    <li><a href="#quick-start">Quick Start</a></li>
    <li><a href="#more-examples">More Examples</a></li>
    <li><a href="#faq">FAQ</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#minimum-supported-rust-version">MSRV</a></li>
    <li><a href="#contributing">Contributing</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#acknowledgments">Acknowledgments</a></li>
    <li><a href="#authors">Authors</a></li>
  </ol>
</details>

---

## Features

- **Expression builders, not strings.** `Condition::eq`, `Update::set`, `KeyCondition::pk(...).sk_begins_with(...)` — combined with `&`, `|`, `!`, `.and()`, `.combine()`. The library manages every `#name` / `:value` placeholder for you.
- **Zero-sized schema types.** `attribute_definitions!`, `table_definitions!`, `index_definitions!` generate marker types that encode the table/index key shape. Using for a sort key on a simple-key table/index is a compile error.
- **Typestate operation builders.** `.condition()` twice? Compile error. `.sk_begins_with()` on a PK-only index? Compile error. `.projection()` and attempt using deserialized type? Compile error.
- **Single-table friendly.** Explicit first-class support for the mono-table pattern with PK/SK + type discriminator, including typed and untyped dispatch on scan/query results.
- **Automatic pagination.** `.all()` collects, `.stream()` yields an `impl Stream<Item = Result<T>>`. No `ExclusiveStartKey` bookkeeping.
- **Automatic batch chunking + retry.** `dynamodb_batch_write` splits into 25-item batches, runs them in parallel, and retries `UnprocessedItems` with backoff (up to 5 attempts).
- **Typed transactions.** `transact_put`, `transact_delete`, `transact_update`, `transact_condition` plug straight into the SDK's `transact_write_items()` builder; each `TransactWriteItem` is built with the same condition DSL as a stand-alone operation.
- **Escape hatch preserved.** Every builder exposes `.into_inner()` returning the underlying SDK fluent builder, so nothing the raw SDK can do is locked out.
- **Flexible serialisation.** Items round-trip through `serde_dynamo` by default; `DynamoDBItem` can be hand-implemented when serde is not a good fit.

---

## Getting Started

### Prerequisites

- Rust **1.85.0** or later (edition 2024).
- An AWS account and a DynamoDB table, or DynamoDB Local via Docker for
  development.

### Installation

```sh
cargo add dynamodb-facade
```

Or in `Cargo.toml`:

```toml
[dependencies]
dynamodb-facade = "0.1"
```

---

## Quick Start

Declare the attributes, the table, wire up a struct, perform operations.

```rust
use dynamodb_facade::{
    attribute_definitions, table_definitions, dynamodb_item,
    Condition, Update, KeyId, DynamoDBItemOp, StringAttribute,
};
use serde::{Deserialize, Serialize};

// 1. Attribute tokens (zero-sized types).
attribute_definitions! {
    PK       { "PK": StringAttribute }
    SK       { "SK": StringAttribute }
    ItemType { "_TYPE": StringAttribute }
}

// 2. Table definition.
table_definitions! {
    PlatformTable {
        type PartitionKey = PK;
        type SortKey = SK;
        fn table_name() -> String {
            std::env::var("TABLE_NAME").unwrap_or("my_table".to_owned())
        }
    }
}

// 3. Item type wired to the table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
}

dynamodb_item! {
    #[table = PlatformTable]
    User {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        ItemType { const VALUE: &'static str = "USER"; }
    }
}

// 4. CRUD — no boilerplate.
# async fn example(client: dynamodb_facade::Client) -> dynamodb_facade::Result<()> {
let user = User {
    id: "u-1".to_owned(),
    name: "Alice".to_owned(),
    email: "alice@example.com".to_owned(),
};

// Create or overwrite:
user.put(client.clone()).await?;

// Create-only (fails if the item already exists):
user.put(client.clone()).not_exists().await?;

// Get by id:
let loaded /* : Option<User> */ = User::get(client.clone(), KeyId::pk("u-1")).await?;

// Conditional update, returning the new item:
let updated /* : User */ = User::update_by_id(
    client.clone(),
    KeyId::pk("u-1"),
    Update::set("name", "Alicia"),
)
.exists()
.await?;

// Delete by id, returning the old item:
let deleted /* : Option<User> */ = User::delete_by_id(client, KeyId::pk("u-1")).await?;
# Ok(())
# }
```

---

## More Examples

The following are small, representative slices. For a full tour across a single-table domain (users, courses, enrollments, configs), see [`EXAMPLES.md`](EXAMPLES.md) — 13 sections covering schema design, every CRUD variant, queries on indexes, scans with dispatch, the full condition and update DSL, batch writes, and transactions.

### Composable conditions

```rust
// Attribute-level:
let c = Condition::exists("email") & Condition::not_exists("deleted_at");

// Item-level (uses the table's PK attribute automatically):
let c = User::exists();

// Variadic AND:
let c = Condition::and([
    Condition::eq("status", "draft"),
    Condition::size_gt("content", 0),
    Condition::exists("author_id"),
]);

// OR / NOT:
let c = User::not_exists() | Condition::lt(Expiration::NAME, now_ts);
let c = !Condition::eq("status", "archived");
```

### Composable updates

```rust
// Chain:
let u = Update::set("name", "Alice")
    .and(Update::remove("legacy_field"));

// Atomic counters:
let u = Update::increment("login_count", 1);
let u = Update::init_increment("enrollment_count", 0, 1); // if_not_exists -> init to 0 + increment

// Merge a variable number of optional updates into a single expression:
let u = Update::combine(
    [
        new_name.map(|n| Update::set("name", n)),
        new_email.map(|e| Update::set("email", e)),
        new_role.map(|r| Update::set("role", r)),
    ]
    .into_iter()
    .flatten(),
);
```

### Query with automatic pagination

```rust
// All enrollments for a user — key condition derived from the item type:
let enrollments /* : Vec<Enrollment> */ =
    Enrollment::query(client.clone(), Enrollment::key_condition(user_id))
    .all()
    .await?;

// Query a GSI:
let users_by_email /* : Vec<User> */ =
    User::query_index::<EmailIndex>(
        client.clone(),
        KeyCondition::pk(email_address),
    )
    .all()
    .await?;

// Stream instead of collect:
let mut stream = User::scan(client.clone())
    .filter(Condition::eq("role", "instructor"))
    .stream();
while let Some(user) = stream.try_next().await? { /* ... */ }
```

### Batch writes

```rust
let requests: Vec<_> = enrollments.iter().map(|e| e.batch_put()).collect();
// Chunks into 25-item batches, runs them in parallel,
// and retries UnprocessedItems with backoff.
dynamodb_batch_write::<PlatformTable>(client, requests).await?;
```

### Transactions

```rust
// Atomically create an enrollment and increment the user's enrollment count:
client
    .transact_write_items()
    .transact_items(
        enrollment.transact_put().not_exists().build(),
    )
    .transact_items(
        User::transact_update_by_id(
            KeyId::pk(user_id),
            Update::init_increment("enrollment_count", 0, 1),
        )
        .condition(
            User::exists() &
            Condition::lt("enrollment_count", max_enrollments),
        )
        .build(),
    )
    .send()
    .await?;
```

### Compile-time safety in action

```rust
// This does not compile — EmailIndex has no sort key:
User::index_key_condition::<EmailIndex>(email).sk_begins_with("EMAIL#");

// This does not compile — .condition() twice consumes the NoCondition typestate:
user.put(client)
    .condition(some_cond)
    .condition(other_cond); // error: no method `.condition` on AlreadyHasCondition
```

---

## FAQ

**Why a facade, not a `#[derive(DynamoDBItem)]` proc macro?**
Declarative macros (`dynamodb_item!`, `table_definitions!`, `attribute_definitions!`) cover today's surface with straightforward, readable expansions. A derive-style proc macro may be on the roadmap but is deliberately not the first deliverable: the declarative form keeps compile times low-ish, stays ergonomic for the common cases, and leaves room for the proc macro to reuse the same underlying traits without locking down the design.

**Is single-table design required?**
No. The crate has first-class support for the mono-table pattern (PK + SK with a type discriminator) because that's the author's main use-case, but nothing in the API assumes it. Simple-key tables, multiple tables, and the same struct serialised to different tables (useful for migrations) are all supported.

**Can I drop down to the raw AWS SDK when I need to?**
Yes. Every builder has an `.into_inner()` method returning the underlying `aws_sdk_dynamodb` fluent builder, and the crate re-exports `aws_sdk_dynamodb::{Client, Error as DynamoDBError, types::AttributeValue}` so you do not need to pin the SDK version separately.

**How are conditional-check failures surfaced?**
As `Error::DynamoDB(ConditionalCheckFailedException(_))`. Use `error.as_dynamodb_error()` to downcast and match on specific SDK error types. See the error-handling example in [crate docs](https://docs.rs/dynamodb-facade).

**Does it work on AWS Lambda / inside async runtimes?**
Yes — the crate builds on `tokio` and `futures`, identical to `aws-sdk-dynamodb` itself. It adds no runtime of its own.

---

## Roadmap

Publicly tracked on the issue tracker. The larger items currently planned:

- **API stabilisation toward 1.0** — minor breaking changes are still possible in the `0.x` line while the API settles.

You have a suggestion? Please **do** send an issue my way!

---

## Minimum Supported Rust Version

This crate requires Rust **1.85.0** or later (edition 2024). MSRV changes will be treated as a minor version bump until `1.0`, and as a breaking change after.

---

## Contributing

We welcome bug reports, feature requests, and pull requests. See [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide.

For PRs — in short:

1. Enter a Nix + direnv shell (installs the toolchain and pre-commit hooks automatically), or run `./scripts/install-hooks.sh` manually.
2. Make your change.
3. The pre-commit hook runs the same four checks as CI: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --document-private-items`, and `cargo test --all-features`.
4. Open a PR. If the pre-commit hook passes locally, CI will pass.

CI runs three parallel jobs: **Lint** and **Test** on stable, **MSRV Check** on 1.85.0.

---

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for the full text.

---

## Acknowledgments

- [`aws-sdk-dynamodb`](https://crates.io/crates/aws-sdk-dynamodb) — the official AWS SDK this crate is built on.
- [`serde_dynamo`](https://crates.io/crates/serde_dynamo) — used internally for item (de)serialisation.
- [`thiserror`](https://crates.io/crates/thiserror), [`async-stream`](https://crates.io/crates/async-stream), [`futures`](https://crates.io/crates/futures), and [`tracing`](https://crates.io/crates/tracing) — the usual suspects that make writing ergonomic async libraries in Rust possible.

---

## Authors

- Jérémie RODON ([@JeremieRodon](https://github.com/JeremieRodon)) [![LinkedIn](https://img.shields.io/badge/linkedin-0077B5?style=for-the-badge&logo=linkedin&logoColor=white)](https://linkedin.com/in/JeremieRodon) — [RustyServerless](https://github.com/RustyServerless) [rustysl.com](https://rustysl.com/index.html?from=github-dynamodb-facade)

If you find this crate useful, please star the repository and share your feedback!
