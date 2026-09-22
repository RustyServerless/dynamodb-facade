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
//! ## Client setup: the process-global client
//!
//! The client-less entry points used throughout this crate — `user.put()`,
//! `User::get(...)`, and the rest of [`DynamoDBItemOp`] — use a process-global
//! [`aws_sdk_dynamodb::Client`] that must be initialized exactly once, early
//! in program startup, via [`init_global_client`]:
//!
//! ```no_run
//! # use dynamodb_facade::test_fixtures::*;
//! use dynamodb_facade::{init_global_client, DynamoDBItemOp, KeyId};
//!
//! # async fn setup(client: dynamodb_facade::Client) -> dynamodb_facade::Result<()> {
//! // Once, early in `main`, before any operation runs:
//! init_global_client(client);
//!
//! // From then on, every operation uses that client implicitly:
//! let user /* : Option<User> */ = User::get(KeyId::pk("user-1")).await?;
//! # Ok(())
//! # }
//! ```
//!
//! Calling [`global_client`] (directly, or transitively through
//! [`DynamoDBItemOp`]) before [`init_global_client`] has been run will panic.
//!
//! If a single implicit global client does not fit your use case, use the
//! [`explicit_client::DynamoDBItemOp`] trait instead. It provides the exact same
//! interface as [`DynamoDBItemOp`], but all the I/O methods take an additional
//! `client: aws_sdk_dynamodb::Client` first argument. You can use it to pass the exact
//! client that suits you for every individual interaction with DynamoDB.
//!
//! # Quick Start
//!
//! Define the schema, wire a struct, then perform CRUD operations:
//!
//! ```no_run
//! use dynamodb_facade::{
//!     attribute_definitions, table_definitions, dynamodb_item,
//!     StringAttribute, NumberAttribute, HasAttribute
//! };
//!
//! // 1. Declare attributes.
//! attribute_definitions! {
//!     // The PK attribute,
//!     // named "PK" in the DynamoDB table
//!     // of DynamoDB type "String"
//!     PK { "PK": StringAttribute }
//!     // The SK attribute,
//!     // named "SK" in the DynamoDB table
//!     // of DynamoDB type "String"
//!     SK { "SK": StringAttribute }
//!     // The ItemType attribute,
//!     // named "_TYPE" in the DynamoDB table
//!     // of DynamoDB type "String"
//!     ItemType { "_TYPE": StringAttribute }
//!     // The Email attribute,
//!     // named "email" in the DynamoDB table
//!     // of DynamoDB type "String"
//!     Email { "email": StringAttribute }
//! }
//!
//! // 2. Declare the table.
//! table_definitions! {
//!     // The table definition is named PlatformTable
//!     PlatformTable {
//!         // Uses the attribute definition "PK" as its partition key
//!         type PartitionKey = PK;
//!         // Uses the attribute definition "SK" as its sort key
//!         type SortKey = SK;
//!         // The name is retrieved from an environment variable
//!         fn table_name() -> String {
//!             std::env::var("TABLE_NAME").expect("TABLE_NAME must be set")
//!         }
//!     }
//! }
//!
//! // 3. Define an item type and wire it to the table.
//! use serde::{Deserialize, Serialize};
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct User {
//!     pub id: String,
//!     pub name: String,
//!     pub email: String,
//! }
//!
//! dynamodb_item! {
//!     // Wire "User" to the table definition PlatformTable
//!     #[table = PlatformTable]
//!     User {
//!         // Partition Key is the "PK" attribute definition
//!         // It should use the `id` field of User
//!         // And compute the value as "USER#{id}"
//!         #[partition_key]
//!         PK {
//!             fn attribute_id(&self) -> &'id str { &self.id }
//!             fn attribute_value(id) -> String { format!("USER#{id}") }
//!         }
//!         // Sort Key is the "SK" attribute definition
//!         // It has the constant "PROFILE" value in our example
//!         #[sort_key]
//!         SK { const VALUE: &'static str = "PROFILE"; }
//!         // A user should always have the "ItemType" attribute
//!         // with constant value "USER"
//!         ItemType { const VALUE: &'static str = "USER"; }
//!
//!         // No need to declare how to process "email" explicitly
//!         // It will simply be serialized like all the attributes of the struct
//!     }
//! }
//!
//! // 4. Initialize the global client in main().
//! #[tokio::main]
//! async fn main() -> dynamodb_facade::Result<()> {
//!     // Create a DynamoDB client
//!     let config = aws_config::load_from_env().await;
//!     let client = aws_sdk_dynamodb::Client::new(&config);
//!
//!     // Initialize the global client of the library with it:
//!     dynamodb_facade::init_global_client(client);
//!
//!     // Use the lib operations
//!     examples().await
//! }
//!
//! // 5. Example CRUD operations — no boilerplate.
//! async fn examples() -> dynamodb_facade::Result<()> {
//!     use dynamodb_facade::{DynamoDBItemOp, KeyId, Update};
//!     let user = User {
//!         id: "u-1".to_owned(),
//!         name: "Alice".to_owned(),
//!         email: "alice@example.com".to_owned(),
//!     };
//!
//!     // Put (create or overwrite):
//!     user.put().await?;
//!
//!     // Put with create-only guard:
//!     user.put().not_exists().await?; // Would fail due to previous put operation
//!
//!     // Get by ID:
//!     let loaded_user /* : Option<User> */ = User::get(KeyId::pk("u-1")).await?;
//!     let loaded_user = loaded_user.expect("u-1 should exist in the table");
//!
//!     // Update with condition:
//!     loaded_user.update(Update::set("name", "Alicia"))
//!         .exists()
//!         .await?;
//!
//!     // Delete:
//!     loaded_user.delete().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Feature Highlights
//!
//! ## Types that map to items sharing the same Partition Key
//!
//! ```no_run
//! # use dynamodb_facade::test_fixtures::*;
//! use dynamodb_facade::{
//!     dynamodb_item,
//!     StringAttribute, NumberAttribute, HasAttribute
//! };
//! use serde::{Deserialize, Serialize};
//!
//! // Say a user can enroll to multiple courses
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct Enrollment {
//!     pub user_id: String,
//!     pub course_id: String,
//!     pub enrolled_at: u64,
//!     pub progress: f64,
//! }
//!
//! dynamodb_item! {
//!     // Wire "Enrollment" to the table definition PlatformTable
//!     #[table = PlatformTable]
//!     Enrollment {
//!         // Partition Key is the "PK" attribute definition
//!         // It should use the `user_id` field of Enrollment
//!         // And compute the PK value exactly the same as the User type
//!         // That way we are sure the computation of the Partition Key
//!         // for User and Enrollment cannot diverge
//!         #[partition_key]
//!         PK {
//!             fn attribute_id(&self) -> <User as HasAttribute<PK>>::Id<'id> {
//!                 &self.user_id
//!             }
//!             fn attribute_value(id) -> <User as HasAttribute<PK>>::Value {
//!                 <User as HasAttribute<PK>>::attribute_value(id)
//!             }
//!         }
//!         // Sort Key is the "SK" attribute definition
//!         // It should use the `course_id` field of Enrollment
//!         // And compute the value as "ENROLL#{id}"
//!         #[sort_key]
//!         SK {
//!             fn attribute_id(&self) -> &'id str { &self.course_id }
//!             fn attribute_value(id) -> String { format!("ENROLL#{id}") }
//!         }
//!         // An enrolment should always have the "ItemType" attribute
//!         // with constant value "ENROLLMENT"
//!         ItemType { const VALUE: &'static str = "ENROLLMENT"; }
//!     }
//! }
//! ```
//!
//! ## Index declaration and usage
//!
//! ```no_run
//! # use dynamodb_facade::test_fixtures::*;
//! use dynamodb_facade::{
//!     index_definitions,
//!     StringAttribute, NumberAttribute, HasAttribute
//! };
//!
//! index_definitions! {
//!     // GSI on item type to query all items of a given type.
//!     #[table = PlatformTable]
//!     TypeIndex {
//!         // The GSI uses the ItemType attribute definition
//!         // as its Partition Key
//!         type PartitionKey = ItemType;
//!         // It is named "iType"
//!         fn index_name() -> String { "iType".to_owned() }
//!     }
//!
//!     // GSI on email — look up any user by email address.
//!     #[table = PlatformTable]
//!     EmailIndex {
//!         // The GSI uses the Email attribute definition
//!         // as its Partition Key
//!         type PartitionKey = Email;
//!         // It is named "iEmail"
//!         fn index_name() -> String { "iEmail".to_owned() }
//!     }
//!
//!     // Note that a LSI would be declared exactly the same way,
//!     // simply using the same Partition Key as the table. There is
//!     // in fact no operational difference between GSI and LSI from
//!     // this library standpoint.
//! }
//!
//! use dynamodb_facade::DynamoDBItemOp;
//! async fn query_all_users() -> Vec<User> {
//!     // Because:
//!     // 1. User has been wired to the ItemType attribute definition
//!     // 2. ItemType is a constant attribute
//!     // 3. TypeIndex uses ItemType as its partition key
//!     // we can just use the query_all_index method and it will
//!     // magically return all the Users of the table
//!     User::query_all_index::<TypeIndex>().all().await.unwrap_or_default()
//! }
//!
//! use dynamodb_facade::{Stream, Result};
//! async fn query_all_enrolments() -> impl Stream<Item = Result<Vec<Enrollment>>> {
//!     // Same here, except we expect a large amount of items
//!     // So we chose to return a page streamer instead of all the items at once
//!     Enrollment::query_all_index::<TypeIndex>().stream()
//! }
//!
//! use dynamodb_facade::KeyCondition;
//! async fn get_user_by_email(email: String) -> Option<User> {
//!     // Query the EmailIndex index using a KeyCondition on the email attribute
//!     // Return the last User found, if any
//!     User::query_index::<EmailIndex>(KeyCondition::pk(email))
//!         .all()
//!         .await
//!         .unwrap_or_default()
//!         .pop()
//! }
//! ```
//!
//! ## Composable conditions
//!
//! You can easily create conditions for your updates, puts and deletes.
//! Conditions can be combined using `&` and `|` logic operators.
//!
//! ```
//! # use dynamodb_facade::{Condition, DynamoDBItemOp, KeyId};
//! # use dynamodb_facade::test_fixtures::User;
//! # async fn examples() -> dynamodb_facade::Result<()> {
//! // Attribute "email" exists AND "deleted_at" does not:
//! let c = Condition::exists("email") & Condition::not_exists("deleted_at");
//!
//! // Item-level existence (verifies the item's PK attribute exists):
//! // the User should exist AND have an attribute role == student
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
//! // resulting conditions can be used with conditional operations
//! User::delete_by_id(KeyId::pk("u-1")).condition(c).await?;
//! # Ok(())
//! # }
//! ```
//!
//!
//! ## Composable updates
//!
//! Very much like conditions, you can easily create and combine updates.
//!
//! ```
//! # use dynamodb_facade::{Update, DynamoDBItemOp, KeyId};
//! # use dynamodb_facade::test_fixtures::User;
//! # async fn examples() -> dynamodb_facade::Result<()> {
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
//! User::update_by_id(KeyId::pk("u-1"), u).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Query and scan with automatic pagination
//!
//! ```no_run
//! # use dynamodb_facade::{Condition, KeyCondition, DynamoDBItemOp, DynamoDBItemBatchOp};
//! # use dynamodb_facade::test_fixtures::*;
//! # async fn example() -> dynamodb_facade::Result<()> {
//! // Query all enrollments for a user (auto-paginates):
//! let enrollments: Vec<Enrollment> =
//!     Enrollment::query(Enrollment::key_condition("user-1"))
//!         .all()
//!         .await?;
//!
//! // Query a GSI:
//! let users: Vec<User> =
//!     User::query_index::<EmailIndex>(KeyCondition::pk("alice@example.com"))
//!         .all()
//!         .await?;
//!
//! // Scan with a filter (note: from a pure DynamoDB standpoint you should never do that
//! // as it will still consume a lot of RCUs and take time, prefer querying an index):
//! let instructors: Vec<User> = User::scan()
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
//! # async fn example(enrollments: Vec<Enrollment>) -> dynamodb_facade::Result<()> {
//! // Automatically chunks into 25-item batches, runs in parallel,
//! // and retries unprocessed items:
//! let requests: Vec<_> = enrollments.iter().map(|e| e.batch_put()).collect();
//! dynamodb_batch_write::<PlatformTable>(requests).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Transactions
//!
//! ```no_run
//! # use dynamodb_facade::{Condition, Update, KeyId, DynamoDBItemOp, DynamoDBItemTransactOp};
//! # use dynamodb_facade::test_fixtures::*;
//! use dynamodb_facade::global_client;
//! # async fn example(
//! #     enrollment: Enrollment,
//! # ) -> dynamodb_facade::Result<()> {
//! // Atomically create an enrollment and increment the user's enrollment count:
//! global_client()
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
//! - **`global`** — [`init_global_client`], [`global_client`] — set and get
//!   the process-global client used by the default (client-less) operation
//!   APIs.
//! - **`operations::explicit_client`** — [`explicit_client`] — client-passing
//!   mirror of the operation entry points, for callers that need explicit
//!   control over which client is used.
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
//! - [`Error::Other`] — any boxed `core::error::Error`, created via
//!   [`Error::other`].
//!
//! ```no_run
//! # use dynamodb_facade::{Error, DynamoDBError, DynamoDBItemOp};
//! # use dynamodb_facade::test_fixtures::*;
//! # async fn example() -> dynamodb_facade::Result<()> {
//! let user = sample_user();
//!
//! // Override an existing item and retrieve the previous version.
//! // `.exists()` adds a condition that fails if the item is not already present.
//! match user.put().exists().return_old().await {
//!     Ok(Some(old)) => { /* found old value */ }
//!     Ok(None) => { unreachable!("condition fails if there is nothing to return") }
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
//! - **`dyndb-local-integration`** — gates integration tests that require a running
//!   DynamoDB Local instance (via `testcontainers`). Not needed for normal
//!   library use.

// TODO: A prelude, once I have enough feedback to know what should be in it

// TODO: Enrich Error::DynamoDB with operation context (operation name, table
//       name, key) for better diagnostics at each .execute() call site.

mod error;
mod expressions;
mod global;
mod item;
mod macros;
mod operations;
mod schema;
mod utils;
mod values;

pub use error::*;
pub use expressions::*;
pub use global::*;
pub use item::*;
pub use operations::*;
pub use schema::*;
pub use values::*;

pub use aws_sdk_dynamodb;
pub use aws_sdk_dynamodb::{Client, Error as DynamoDBError, types::AttributeValue};
pub use futures_core::Stream;

#[cfg(any(test, feature = "test-fixtures", doc))]
pub mod test_fixtures;
