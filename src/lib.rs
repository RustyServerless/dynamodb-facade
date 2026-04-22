//! A typed facade over [`aws-sdk-dynamodb`][aws_sdk_dynamodb] with composable
//! expression builders and typestate operation builders.
//!
//! `dynamodb-facade` eliminates the boilerplate of raw DynamoDB calls —
//! manual key maps, expression strings, placeholder tracking, pagination loops,
//! and batch-write chunking — while enforcing correct usage at compile time
//! through Rust's type system.
//!
//! # Key Concepts
//!
//! ## Tables, items, and the `TD` parameter
//!
//! The [`DynamoDBItem<TD>`] trait wires a Rust struct to a
//! [`TableDefinition`], declaring how its fields map to DynamoDB key
//! attributes. The blanket traits [`DynamoDBItemOp<TD>`],
//! [`DynamoDBItemBatchOp<TD>`], and [`DynamoDBItemTransactOp<TD>`] are
//! automatically implemented for every type that implements [`DynamoDBItem`],
//! providing `get`, `put`, `delete`, `update`, `query`, `scan`, `batch_put`,
//! `batch_delete`, `transact_put`, and friends as associated functions.
//!
//! `TD` is deliberately a **generic type parameter**, not an associated type.
//! A single Rust struct can implement `DynamoDBItem` for multiple tables,
//! which is useful when:
//!
//! - **Multiple tables share domain types** — for example, a `User` struct
//!   that exists in both a primary table and an archive table, possibly with
//!   different key mappings.
//! - **Migration logic** — reading items from one table and writing them to
//!   another, for one-shot migrations, compaction, or aggregation across
//!   tables.
//!
//! ## Mono-table (single-table) design
//!
//! The crate has first-class support for the single-table pattern, where all
//! entity types share one DynamoDB table with a composite `PK + SK` key and
//! a type discriminator attribute (e.g. `_TYPE`). This is a natural fit
//! because the trait system already enforces per-entity key mappings, type
//! discriminators, and serialization — but it is not the only layout the
//! crate supports.
//!
//! ## Schema definitions
//!
//! Attributes, tables, and indexes are declared as zero-sized types using the
//! [`attribute_definitions!`], [`table_definitions!`], and
//! [`index_definitions!`] macros. These types serve as compile-time tokens
//! that the library uses to build correct key maps and expression attribute
//! name/value maps without any runtime string manipulation by the caller.
//! They also encode key schema shape into the type system — for instance,
//! attempting to supply a sort key for a table declared with a partition key
//! only is a compile-time error.
//!
//! ## Expression builders
//!
//! [`Condition<'a>`] and [`Update<'a>`] are composable value types that build
//! DynamoDB condition and update expressions. They support the full DynamoDB
//! expression language — comparisons, `begins_with`, `contains`, `between`,
//! `IN`, `size`, `if_not_exists`, `list_append`, set `ADD`/`DELETE` — and
//! compose with `&`, `|`, `!` operators and `.and()` / `.combine()` methods.
//! All placeholder names and values are managed internally; callers never
//! touch `#name` or `:value` strings.
//!
//! ## Typestate operation builders
//!
//! Every operation builder ([`GetItemRequest`], [`PutItemRequest`],
//! [`DeleteItemRequest`], [`UpdateItemRequest`], [`QueryRequest`],
//! [`ScanRequest`]) uses compile-time typestate parameters to enforce correct
//! usage:
//!
//! - **`OutputFormat`** (`Typed` / `Raw`) — whether the terminal method
//!   deserializes into `T` or returns [`Item<TD>`].
//! - **`ReturnValue`** (`ReturnNothing` / `Return<Old>` / `Return<New>`) —
//!   whether put/delete/update return item attributes.
//! - **Expression-set state** (`NoCondition` / `AlreadyHasCondition`, etc.) —
//!   calling `.condition()` or `.filter()` twice is a **compile-time error**.
//!
//! # Quick Start
//!
//! Define the schema, wire a struct, then perform CRUD operations:
//!
//! ```no_run
//! use dynamodb_facade::{
//!     attribute_definitions, table_definitions, index_definitions, dynamodb_item,
//!     Condition, Update, KeyId, DynamoDBItemOp, DynamoDBError, Error,
//!     StringAttribute, NumberAttribute, HasAttribute
//! };
//! use serde::{Deserialize, Serialize};
//!
//! // 1. Declare attribute zero-sized types.
//! attribute_definitions! {
//!     PK { "PK": StringAttribute }
//!     SK { "SK": StringAttribute }
//!     ItemType { "_TYPE": StringAttribute }
//!     Email { "email": StringAttribute }
//! }
//!
//! // 2. Declare the table.
//! table_definitions! {
//!     PlatformTable {
//!         type PartitionKey = PK;
//!         type SortKey = SK;
//!         fn table_name() -> String {
//!             std::env::var("TABLE_NAME").expect("TABLE_NAME must be set")
//!         }
//!     }
//! }
//!
//! // 3. Define an item type and wire it to the table.
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct User {
//!     pub id: String,
//!     pub name: String,
//!     pub email: String,
//! }
//!
//! dynamodb_item! {
//!     #[table = PlatformTable]
//!     User {
//!         #[partition_key]
//!         PK {
//!             fn attribute_id(&self) -> &'id str { &self.id }
//!             fn attribute_value(id) -> String { format!("USER#{id}") }
//!         }
//!         #[sort_key]
//!         SK { const VALUE: &'static str = "PROFILE"; }
//!         ItemType { const VALUE: &'static str = "USER"; }
//!     }
//! }
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct Enrollment {
//!     pub user_id: String,
//!     pub course_id: String,
//!     pub enrolled_at: u64,
//!     pub progress: f64,
//! }
//!
//! dynamodb_item! {
//!     #[table = PlatformTable]
//!     Enrollment {
//!         #[partition_key]
//!         PK {
//!             fn attribute_id(&self) -> <User as HasAttribute<PK>>::Id<'id> {
//!                 &self.user_id
//!             }
//!             fn attribute_value(id) -> <User as HasAttribute<PK>>::Value {
//!                 <User as HasAttribute<PK>>::attribute_value(id)
//!             }
//!         }
//!         #[sort_key]
//!         SK {
//!             fn attribute_id(&self) -> &'id str { &self.course_id }
//!             fn attribute_value(id) -> String { format!("ENROLL#{id}") }
//!         }
//!         ItemType { const VALUE: &'static str = "ENROLLMENT"; }
//!     }
//! }
//!
//! # async fn example(cclient: dynamodb_facade::Client) -> dynamodb_facade::Result<()> {
//! // 4. CRUD — no boilerplate.
//! let user = User {
//!     id: "u-1".to_owned(),
//!     name: "Alice".to_owned(),
//!     email: "alice@example.com".to_owned(),
//! };
//!
//! # let client = cclient.clone();
//! // Put (create or overwrite):
//! user.put(client).await?;
//!
//! # let client = cclient.clone();
//! // Put with create-only guard:
//! user.put(client).not_exists().await?;
//!
//! # let client = cclient.clone();
//! // Get by ID:
//! let loaded /* : Option<User> */ = User::get(client, KeyId::pk("u-1")).await?;
//!
//! # let client = cclient.clone();
//! // Update with condition:
//! User::update_by_id(
//!     client,
//!     KeyId::pk("u-1"),
//!     Update::set("name", "Alicia"),
//! )
//! .exists()
//! .await?;
//!
//! # let client = cclient.clone();
//! // Delete:
//! User::delete_by_id(client, KeyId::pk("u-1")).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Feature Highlights
//!
//! ## Composable conditions
//!
//! ```
//! # use dynamodb_facade::{Condition, DynamoDBItemOp};
//! # use dynamodb_facade::test_fixtures::*;
//! // Attribute-level existence checks:
//! let c = Condition::exists("email") & Condition::not_exists("deleted_at");
//!
//! // Item-level existence (uses the table's PK attribute):
//! let c = User::exists() & Condition::eq("role", "student");
//!
//! // OR / NOT:
//! let c = User::not_exists() | Condition::lt("expiration_timestamp", 9999999999u64);
//! let c = !Condition::eq("status", "archived");
//!
//! // Variadic AND over a collection:
//! let c = Condition::and([
//!     Condition::eq("role", "instructor"),
//!     Condition::size_gt("bio", 0),
//!     Condition::exists("verified_at"),
//! ]);
//! ```
//!
//! ## Composable updates
//!
//! ```
//! # use dynamodb_facade::Update;
//! // Simple set / remove:
//! let u = Update::set("name", "Alice").and(Update::remove("legacy_field"));
//!
//! // Atomic counters:
//! let u = Update::increment("login_count", 1);
//! let u = Update::init_increment("enrollment_count", 0, 1); // if_not_exists + increment
//!
//! // Merge optional updates from an iterator:
//! let new_name: Option<&str> = Some("Alice");
//! let new_role: Option<&str> = None;
//! let u = Update::combine(
//!     [
//!         new_name.map(|n| Update::set("name", n)),
//!         new_role.map(|r| Update::set("role", r)),
//!     ]
//!     .into_iter()
//!     .flatten(),
//! );
//! ```
//!
//! ## Query and scan with automatic pagination
//!
//! ```no_run
//! # use dynamodb_facade::{Condition, KeyCondition, DynamoDBItemOp, DynamoDBItemBatchOp};
//! # use dynamodb_facade::test_fixtures::*;
//! # async fn example(cclient: dynamodb_facade::Client) -> dynamodb_facade::Result<()> {
//! # let client = cclient.clone();
//! // Query all enrollments for a user (auto-paginates):
//! let enrollments: Vec<Enrollment> =
//!     Enrollment::query(client, Enrollment::key_condition("user-1"))
//!         .all()
//!         .await?;
//!
//! # let client = cclient.clone();
//! // Query a GSI:
//! let users: Vec<User> =
//!     User::query_index::<EmailIndex>(client, KeyCondition::pk("alice@example.com"))
//!         .all()
//!         .await?;
//!
//! # let client = cclient.clone();
//! // Scan with a filter (note: from a pure DynamoDB stand point you should never do that):
//! let instructors: Vec<User> = User::scan(client)
//!     .filter(Condition::eq("role", "instructor"))
//!     .all()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Batch writes
//!
//! ```no_run
//! # use dynamodb_facade::{dynamodb_batch_write, DynamoDBItemBatchOp};
//! # use dynamodb_facade::test_fixtures::*;
//! # async fn example(client: dynamodb_facade::Client, enrollments: Vec<Enrollment>) -> dynamodb_facade::Result<()> {
//! // Automatically chunks into 25-item batches, runs in parallel,
//! // and retries unprocessed items:
//! let requests: Vec<_> = enrollments.iter().map(|e| e.batch_put()).collect();
//! dynamodb_batch_write::<PlatformTable>(client, requests).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Transactions
//!
//! ```no_run
//! # use dynamodb_facade::{Condition, Update, KeyId, DynamoDBItemOp, DynamoDBItemTransactOp};
//! # use dynamodb_facade::test_fixtures::*;
//! # async fn example(
//! #     client: dynamodb_facade::Client,
//! #     enrollment: Enrollment,
//! # ) -> dynamodb_facade::Result<()> {
//! // Atomically create an enrollment and increment the user's enrollment count:
//! client
//!     .transact_write_items()
//!     .transact_items(enrollment.transact_put().not_exists().build())
//!     .transact_items(
//!         User::transact_update_by_id(
//!             KeyId::pk("user-1"),
//!             Update::init_increment("enrollment_count", 0, 1),
//!         )
//!         .condition(
//!             User::exists()
//!                 & Condition::lt("enrollment_count", 10u32),
//!         )
//!         .build(),
//!     )
//!     .send()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Logical Module Organization
//!
//! All items are re-exported from the crate root. The internal modules are:
//!
//! - **`schema`** — [`TableDefinition`], [`IndexDefinition`], [`KeySchema`],
//!   [`AttributeDefinition`], [`HasAttribute`], [`HasConstAttribute`], and the
//!   attribute type markers ([`StringAttribute`], [`NumberAttribute`],
//!   [`BinaryAttribute`]).
//! - **`item`** — [`DynamoDBItem<TD>`], [`Item<TD>`], [`Key<TD>`],
//!   [`KeyId`], [`NoId`], [`KeyBuilder`].
//! - **`expressions`** — [`Condition<'a>`], [`Update<'a>`],
//!   [`UpdateSetRhs<'a>`], [`KeyCondition`], [`Projection`], [`Comparison`].
//! - **`operations`** — [`GetItemRequest`], [`PutItemRequest`],
//!   [`DeleteItemRequest`], [`UpdateItemRequest`], [`QueryRequest`],
//!   [`ScanRequest`], [`DynamoDBItemOp`], [`DynamoDBItemBatchOp`],
//!   [`DynamoDBItemTransactOp`], batch helpers ([`dynamodb_batch_write`],
//!   [`batch_put`], [`batch_delete`]), and pagination helpers
//!   ([`dynamodb_execute_query`], [`dynamodb_stream_query`],
//!   [`dynamodb_execute_scan`], [`dynamodb_stream_scan`]).
//! - **`values`** — [`IntoAttributeValue`], [`to_attribute_value`],
//!   [`try_to_attribute_value`], [`AsSet<T>`], [`AsNumber<T>`].
//! - **`error`** — [`Error`], [`Result<T>`].
//! - **`macros`** — [`attribute_definitions!`], [`table_definitions!`],
//!   [`index_definitions!`], [`dynamodb_item!`], [`has_attributes!`],
//!   [`attr_list!`], [`key_schema!`].
//!
//! # Error Handling
//!
//! All fallible operations return [`Result<T>`] (an alias for
//! `core::result::Result<T, `[`Error`]`>`). The [`Error`] enum has five
//! variants:
//!
//! - [`Error::DynamoDB`] — wraps a boxed [`DynamoDBError`] from the AWS SDK.
//!   Use [`Error::as_dynamodb_error`] to downcast and match on specific SDK
//!   error types such as `ConditionalCheckFailedException`.
//! - [`Error::Serde`] — a `serde_dynamo` (de)serialization failure.
//! - [`Error::FailedBatchWrite`] — a batch write that could not complete
//!   after all retry attempts. Contains the unprocessed
//!   [`WriteRequest`](aws_sdk_dynamodb::types::WriteRequest)s.
//! - [`Error::Custom`] — a caller-supplied string message, created via
//!   [`Error::custom`].
//! - [`Error::Other`] — any `Box<dyn Error + Send + Sync>`, created via
//!   [`Error::other`].
//!
//! ```no_run
//! # use dynamodb_facade::{Error, DynamoDBError, DynamoDBItemOp};
//! # use dynamodb_facade::test_fixtures::*;
//! # async fn example(client: dynamodb_facade::Client) -> dynamodb_facade::Result<()> {
//! let user = sample_user();
//!
//! // Override an existing item and retrieve the previous version.
//! // `.exists()` adds a condition that fails if the item is not already present.
//! match user.put(client).exists().return_old().await {
//!     Ok(Some(old)) => { /* found old value */ }
//!     Ok(None) => { unreachable!("condition fail if nothing to return") }
//!     Err(err)
//!         if matches!(
//!             err.as_dynamodb_error(),
//!             Some(DynamoDBError::ConditionalCheckFailedException(_))
//!         ) =>
//!     {
//!         println!("item did not exist yet — nothing was overwritten");
//!     }
//!     Err(err) => return Err(err),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Feature Flags
//!
//! - **`test-fixtures`** — exposes the [`test_fixtures`] module outside of
//!   `cfg(test)` and `cfg(doc)`. Useful for integration test crates that want
//!   to reuse the domain types defined there.
//! - **`integration`** — gates integration tests that require a running
//!   DynamoDB Local instance (via `testcontainers`). Not needed for normal
//!   library use.

// TODO: `#[derive(DynamoDBItem)]` proc macro to eliminate the boilerplate of
//       implementing PkId/SkId/get_pk_from_id/get_sk_from_id/get_key/TYPE.

// TODO: Enrich Error::DynamoDB with operation context (operation name, table
//       name, key) for better diagnostics at each .execute() call site.

mod error;
mod expressions;
mod item;
mod macros;
mod operations;
mod schema;
mod utils;
mod values;

pub use error::*;
pub use expressions::*;
pub use item::*;
pub use operations::*;
pub use schema::*;
pub use values::*;

pub use aws_sdk_dynamodb::{Client, Error as DynamoDBError, types::AttributeValue};

#[cfg(any(test, feature = "test-fixtures", doc))]
pub mod test_fixtures;
