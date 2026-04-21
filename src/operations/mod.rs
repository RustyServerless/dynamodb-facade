mod batch;
mod delete;
mod get;
mod pagination;
mod put;
mod query;
mod scan;
mod transactions;
mod type_state;
mod update;

pub use batch::*;
pub use delete::*;
pub use get::*;
pub use pagination::*;
pub use put::*;
pub use query::*;
pub use scan::*;
pub use transactions::*;
pub use type_state::*;
pub use update::*;

use serde::{Serialize, de::DeserializeOwned};
use std::marker::PhantomData;

use super::{
    ApplyCondition, ApplyFilter, ApplyKeyCondition, ApplyProjection, ApplyUpdate,
    AttributeDefinition, Condition, DynamoDBItem, HasAttribute, HasConstAttribute,
    HasIndexKeyAttributes, IndexDefinition, Item, Key, KeyCondition, KeyConditionState, KeySchema,
    PartitionKeyDefinition, Projection, Result, TableDefinition, Update,
};

// ---------------------------------------------------------------------------
// DynamoDBItemOp trait — typed operation entry points
// ---------------------------------------------------------------------------

/// Primary entry point for typed single-item and collection CRUD operations.
///
/// This trait is **blanket-implemented** for every type that implements
/// [`DynamoDBItem<TD>`]. You never implement it manually — implement
/// `DynamoDBItem` (via the `dynamodb_item!` macro) and all methods here become
/// available automatically.
///
/// Every method returns a builder with compile-time safety guarantees that
/// mirror DynamoDB API constraints. For example, calling `.condition()` twice
/// is a compile error (DynamoDB accepts one condition expression per request),
/// and `.project()` automatically switches to raw output since projected
/// results may be incomplete for deserialization.
///
/// # Operation overview
///
/// | Method | DynamoDB operation | Default return |
/// |---|---|---|
/// | [`get`][DynamoDBItemOp::get] | `GetItem` | `Option<T>` |
/// | [`put`][DynamoDBItemOp::put] | `PutItem` | `()` |
/// | [`delete`][DynamoDBItemOp::delete] | `DeleteItem` | `()` |
/// | [`delete_by_id`][DynamoDBItemOp::delete_by_id] | `DeleteItem` | `Option<T>` (old) |
/// | [`update`][DynamoDBItemOp::update] | `UpdateItem` | `()` |
/// | [`update_by_id`][DynamoDBItemOp::update_by_id] | `UpdateItem` | `T` (new) |
/// | [`scan`][DynamoDBItemOp::scan] | `Scan` | `Vec<T>` |
/// | [`scan_index`][DynamoDBItemOp::scan_index] | `Scan` (LSI/GSI) | `Vec<T>` |
/// | [`query`][DynamoDBItemOp::query] | `Query` | `Vec<T>` |
/// | [`query_all`][DynamoDBItemOp::query_all] | `Query` (const PK) | `Vec<T>` |
/// | [`query_index`][DynamoDBItemOp::query_index] | `Query` (LSI/GSI) | `Vec<T>` |
/// | [`query_all_index`][DynamoDBItemOp::query_all_index] | `Query` (LSI/GSI, const PK) | `Vec<T>` |
///
/// # Examples
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{DynamoDBItemOp, KeyId, Update, Condition};
///
/// # async fn example(cclient: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
/// # let client = cclient.clone();
/// // Get a user by ID
/// let user /* : Option<User> */ = User::get(client, KeyId::pk("user-1")).await?;
///
/// # let client = cclient.clone();
/// // Get with consistent read
/// let user /* : Option<User> */ = User::get(client, KeyId::pk("user-1"))
///     .consistent_read()
///     .await?;
///
/// # let client = cclient.clone();
/// // Put a new user (unconditional)
/// sample_user().put(client).await?;
///
/// # let client = cclient.clone();
/// // Put a new user (create-only)
/// sample_user().put(client).not_exists().await?;
///
/// # let client = cclient.clone();
/// // Put with a custom condition
/// sample_user()
///     .put(client)
///     .condition(User::not_exists() | Condition::lt("expiration_timestamp", 1_700_000_000))
///     .await?;
///
/// # let client = cclient.clone();
/// // Put and return the old item
/// let old /* : Option<User> */ = sample_user().put(client).return_old().await?;
///
/// # let client = cclient.clone();
/// // Delete an enrollment (unconditional)
/// sample_enrollment().delete(client).await?;
///
/// # let client = cclient.clone();
/// // Delete only if the item exists
/// sample_enrollment().delete(client).exists().await?;
///
/// # let client = cclient.clone();
/// // Delete with a custom condition
/// sample_enrollment()
///     .delete(client)
///     .condition(Enrollment::exists() & Condition::not_exists("completed_at"))
///     .await?;
///
/// # let client = cclient.clone();
/// // Delete by ID and return the old item
/// let old /* : Option<Enrollment> */ = Enrollment::delete_by_id(
///     client,
///     KeyId::pk("user-1").sk("course-42"),
/// )
/// .exists()
/// .await?;
///
/// # let client = cclient.clone();
/// // Update a user's role (fire-and-forget)
/// sample_user()
///     .update(client, Update::set("role", "instructor"))
///     .exists()
///     .await?;
///
/// # let client = cclient.clone();
/// // Update by ID and return the updated item (default for update_by_id)
/// let updated /* : User */ = User::update_by_id(
///     client,
///     KeyId::pk("user-1"),
///     Update::set("name", "Bob"),
/// )
/// .exists()
/// .await?;
///
/// # let client = cclient.clone();
/// // Update with a custom condition
/// let updated /* : User */ = User::update_by_id(
///     client,
///     KeyId::pk("user-1"),
///     Update::set("role", "instructor"),
/// )
/// .condition(Condition::eq("role", "student"))
/// .await?;
///
/// # let client = cclient.clone();
/// // Update without returning the item
/// User::update_by_id(
///     client,
///     KeyId::pk("user-1"),
///     Update::set("name", "Bob"),
/// )
/// .exists()
/// .return_none()
/// .await?;
///
/// # let client = cclient.clone();
/// // Query all enrollments for a user
/// let enrollments /* : Vec<Enrollment> */ =
///     Enrollment::query(client, Enrollment::key_condition("user-1"))
///         .all()
///         .await?;
///
/// # let client = cclient.clone();
/// // Scan all users with a filter
/// let instructors /* : Vec<User> */ = User::scan(client)
///     .filter(Condition::eq("role", "instructor"))
///     .all()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub trait DynamoDBItemOp<TD: TableDefinition>: DynamoDBItem<TD> {
    /// Returns a [`GetItemRequest`] builder in `Typed` output mode for
    /// fetching a single item by key.
    ///
    /// The returned builder can be `.await`ed directly (returns
    /// `Option<T>`), or further configured with
    /// [`.raw()`][GetItemRequest::raw],
    /// [`.project()`][GetItemRequest::project], or
    /// [`.consistent_read()`][GetItemRequest::consistent_read].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId};
    ///
    /// # async fn example(cclient: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// # let client = cclient.clone();
    /// // Simple get by ID
    /// let user /* : Option<User> */ = User::get(client, KeyId::pk("user-1")).await?;
    ///
    /// # let client = cclient.clone();
    /// // Consistent read
    /// let user /* : Option<User> */ = User::get(client, KeyId::pk("user-1"))
    ///     .consistent_read()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    fn get(
        client: aws_sdk_dynamodb::Client,
        key_id: Self::KeyId<'_>,
    ) -> GetItemRequest<TD, Self, Typed>
    where
        Self: DeserializeOwned,
    {
        GetItemRequest::_new(client, Self::get_key_from_id(key_id))
    }

    /// Returns a [`PutItemRequest`] builder in `Typed` output mode with
    /// `ReturnNothing` and no condition.
    ///
    /// The returned builder can be `.await`ed directly, or further configured
    /// with [`.not_exists()`][PutItemRequest::not_exists],
    /// [`.condition()`][PutItemRequest::condition],
    /// [`.return_old()`][PutItemRequest::return_old], or
    /// [`.raw()`][PutItemRequest::raw].
    ///
    /// # Panics
    ///
    /// Panics if serializing `self` via [`DynamoDBItem::to_item`] fails. See
    /// [`DynamoDBItem::to_item`] for the conditions under which this can
    /// happen — it is the caller's responsibility to provide a compatible
    /// [`Serialize`] implementation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Condition};
    ///
    /// # async fn example(cclient: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let user = sample_user();
    ///
    /// # let client = cclient.clone();
    /// // Unconditional put (overwrites any existing item)
    /// user.put(client).await?;
    ///
    /// # let client = cclient.clone();
    /// // Create-only: fails if item already exists
    /// user.put(client).not_exists().await?;
    ///
    /// # let client = cclient.clone();
    /// // Custom condition: create-only OR expired TTL
    /// user.put(client)
    ///     .condition(User::not_exists() | Condition::lt("expiration_timestamp", 1_700_000_000))
    ///     .await?;
    ///
    /// # let client = cclient.clone();
    /// // Put and return the old item
    /// let old /* : Option<User> */ = user.put(client).return_old().await?;
    /// # Ok(())
    /// # }
    /// ```
    fn put(&self, client: aws_sdk_dynamodb::Client) -> PutItemRequest<TD, Self, Typed>
    where
        Self: Serialize,
    {
        PutItemRequest::_new(client, self.to_item())
    }

    /// Returns a [`DeleteItemRequest`] builder in `Typed` output mode with
    /// `ReturnNothing` and no condition.
    ///
    /// The returned builder can be `.await`ed directly, or further configured
    /// with [`.exists()`][DeleteItemRequest::exists],
    /// [`.condition()`][DeleteItemRequest::condition],
    /// [`.return_old()`][DeleteItemRequest::return_old], or
    /// [`.raw()`][DeleteItemRequest::raw].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Condition};
    ///
    /// # async fn example(cclient: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let enrollment = sample_enrollment();
    ///
    /// # let client = cclient.clone();
    /// // Unconditional delete
    /// enrollment.delete(client).await?;
    ///
    /// # let client = cclient.clone();
    /// // Delete only if the item exists
    /// enrollment.delete(client).exists().await?;
    ///
    /// # let client = cclient.clone();
    /// // Delete with a custom condition
    /// enrollment
    ///     .delete(client)
    ///     .condition(Enrollment::exists() & Condition::not_exists("completed_at"))
    ///     .await?;
    ///
    /// # let client = cclient.clone();
    /// // Delete and return the old item
    /// let old /* : Option<Enrollment> */ = enrollment.delete(client).return_old().await?;
    /// # Ok(())
    /// # }
    /// ```
    fn delete(&self, client: aws_sdk_dynamodb::Client) -> DeleteItemRequest<TD, Self, Typed> {
        DeleteItemRequest::_new(client, self.get_key())
    }

    /// Returns a [`DeleteItemRequest`] builder in `Typed` output mode with
    /// `Return<Old>` and no condition.
    ///
    /// Unlike [`delete`][DynamoDBItemOp::delete], this method accepts a
    /// `KeyId` instead of a loaded instance, and defaults to
    /// `Return<Old>` — the deleted item is returned as `Option<T>`.
    ///
    /// The returned builder can be further configured with
    /// [`.exists()`][DeleteItemRequest::exists],
    /// [`.condition()`][DeleteItemRequest::condition],
    /// [`.return_none()`][DeleteItemRequest::return_none], or
    /// [`.raw()`][DeleteItemRequest::raw].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Condition, KeyId};
    ///
    /// # async fn example(cclient: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// # let client = cclient.clone();
    /// // Simple delete by ID (returns the old item by default)
    /// let old /* : Option<Enrollment> */ = Enrollment::delete_by_id(
    ///     client,
    ///     KeyId::pk("user-1").sk("course-42"),
    /// )
    /// .await?;
    ///
    /// # let client = cclient.clone();
    /// // Delete only if the item exists
    /// let old /* : Option<Enrollment> */ = Enrollment::delete_by_id(
    ///     client,
    ///     KeyId::pk("user-1").sk("course-42"),
    /// )
    /// .exists()
    /// .await?;
    ///
    /// # let client = cclient.clone();
    /// // Delete with a custom condition
    /// let old /* : Option<Enrollment> */ = Enrollment::delete_by_id(
    ///     client,
    ///     KeyId::pk("user-1").sk("course-42"),
    /// )
    /// .condition(Enrollment::exists() & Condition::not_exists("completed_at"))
    /// .await?;
    ///
    /// # let client = cclient.clone();
    /// // Delete without returning the old item
    /// Enrollment::delete_by_id(
    ///     client,
    ///     KeyId::pk("user-1").sk("course-42"),
    /// )
    /// .return_none()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    fn delete_by_id(
        client: aws_sdk_dynamodb::Client,
        key_id: Self::KeyId<'_>,
    ) -> DeleteItemRequest<TD, Self, Typed, Return<Old>>
    where
        Self: DeserializeOwned,
    {
        DeleteItemRequest::_new(client, Self::get_key_from_id(key_id)).return_old()
    }

    /// Returns an [`UpdateItemRequest`] builder in `Typed` output mode with
    /// `ReturnNothing` and no condition.
    ///
    /// The returned builder can be `.await`ed directly, or further configured
    /// with [`.exists()`][UpdateItemRequest::exists],
    /// [`.condition()`][UpdateItemRequest::condition],
    /// [`.return_new()`][UpdateItemRequest::return_new],
    /// [`.return_old()`][UpdateItemRequest::return_old], or
    /// [`.raw()`][UpdateItemRequest::raw].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Condition, Update};
    ///
    /// # async fn example(cclient: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let user = sample_user();
    ///
    /// # let client = cclient.clone();
    /// // Simple update (fire-and-forget)
    /// user.update(client, Update::set("role", "instructor")).await?;
    ///
    /// # let client = cclient.clone();
    /// // Update guarded by existence
    /// user.update(client, Update::set("role", "instructor"))
    ///     .exists()
    ///     .await?;
    ///
    /// # let client = cclient.clone();
    /// // Update with a custom condition
    /// user.update(client, Update::set("role", "instructor"))
    ///     .condition(Condition::eq("role", "student"))
    ///     .await?;
    ///
    /// # let client = cclient.clone();
    /// // Update if exist and return the new item
    /// let updated /* : User */ = user
    ///     .update(client, Update::set("name", "Alice B."))
    ///     .exists()
    ///     .return_new()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    fn update(
        &self,
        client: aws_sdk_dynamodb::Client,
        update: Update<'_>,
    ) -> UpdateItemRequest<TD, Self, Typed> {
        UpdateItemRequest::_new(client, self.get_key(), update)
    }

    /// Returns an [`UpdateItemRequest`] builder in `Typed` output mode with
    /// `Return<New>` and no condition.
    ///
    /// Unlike [`update`][DynamoDBItemOp::update], this method accepts a
    /// `KeyId` instead of a loaded instance, and defaults to `Return<New>` —
    /// the updated item is returned as `T`.
    ///
    /// The returned builder can be further configured with
    /// [`.exists()`][UpdateItemRequest::exists],
    /// [`.condition()`][UpdateItemRequest::condition],
    /// [`.return_none()`][UpdateItemRequest::return_none],
    /// [`.return_old()`][UpdateItemRequest::return_old], or
    /// [`.raw()`][UpdateItemRequest::raw].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Condition, KeyId, Update};
    ///
    /// # async fn example(cclient: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// # let client = cclient.clone();
    /// // Simple update by ID (returns the updated item by default)
    /// let updated /* : User */ = User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("role", "instructor"),
    /// )
    /// .await?;
    ///
    /// # let client = cclient.clone();
    /// // Update guarded by existence
    /// let updated /* : User */ = User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("role", "instructor"),
    /// )
    /// .exists()
    /// .await?;
    ///
    /// # let client = cclient.clone();
    /// // Update with a custom condition
    /// let updated /* : User */ = User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("role", "instructor"),
    /// )
    /// .condition(Condition::eq("role", "student"))
    /// .await?;
    ///
    /// # let client = cclient.clone();
    /// // Update without returning the item
    /// User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("name", "Bob"),
    /// )
    /// .exists()
    /// .return_none()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    fn update_by_id(
        client: aws_sdk_dynamodb::Client,
        key_id: Self::KeyId<'_>,
        update: Update<'_>,
    ) -> UpdateItemRequest<TD, Self, Typed, Return<New>>
    where
        Self: DeserializeOwned,
    {
        UpdateItemRequest::_new(client, Self::get_key_from_id(key_id), update)
    }

    /// Returns a [`ScanRequest`] builder in `Typed` output mode for scanning
    /// the entire table.
    ///
    /// The returned builder can be executed with
    /// [`.all()`][ScanRequest::all] or [`.stream()`][ScanRequest::stream],
    /// and further configured with [`.filter()`][ScanRequest::filter],
    /// [`.project()`][ScanRequest::project], [`.limit()`][ScanRequest::limit],
    /// or [`.raw()`][ScanRequest::raw].
    ///
    /// Prefer [`query`][DynamoDBItemOp::query] when possible — scans read every
    /// item in the table and are significantly more expensive.
    ///
    /// Also, note that this method will fail in most cases because a DynamoDB table
    /// rarely contains only items of the same type. Use
    /// [`scan_index`][DynamoDBItemOp::scan_index] to scan an index that may contain
    /// only a specific type of item, or [`.raw()`][ScanRequest::raw] to prevent item
    /// deserialization.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Condition};
    ///
    /// # async fn example(cclient: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// # let client = cclient.clone();
    /// // Scan the table and attempts projecting all items as users
    /// let all_users /* : Vec<User> */ = User::scan(client).all().await?;
    ///
    /// # let client = cclient.clone();
    /// // Scan with a filter (prefer using query on an appropriate index)
    /// let instructors /* : Vec<User> */ = User::scan(client)
    ///     .filter(Condition::eq("role", "instructor"))
    ///     .all()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    fn scan(client: aws_sdk_dynamodb::Client) -> ScanRequest<TD, Self, Typed>
    where
        Self: DeserializeOwned,
    {
        ScanRequest::_new(client)
    }

    /// Returns a [`ScanRequest`] builder in `Typed` output mode for scanning
    /// a secondary index (GSI or LSI).
    ///
    /// `I` must be an [`IndexDefinition`] for `TD`, and `Self` must implement
    /// [`HasAttribute<A>`] for evey keys of the IndexDefinition to confirm the
    /// type participates in that index.
    ///
    /// Also, note that this method will fail is the index does not contains only items
    /// of the expected type. Use [`.raw()`][ScanRequest::raw] to prevent item
    /// deserialization.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Condition};
    ///
    /// # async fn example(cclient: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// # let client = cclient.clone();
    /// // Scan an index and attempts projecting all items as enrollments
    /// let all_enrollments /* : Vec<Enrollment> */ =
    ///     Enrollment::scan_index::<TypeIndex>(client)
    ///         .all()
    ///         .await?;
    ///
    /// # let client = cclient.clone();
    /// // Scan an index with a filter (prefer using query on an appropriate index)
    /// let recent /* : Vec<Enrollment> */ =
    ///     Enrollment::scan_index::<TypeIndex>(client)
    ///         .filter(Condition::gt("enrolled_at", 1_700_000_000))
    ///         .all()
    ///         .await?;
    /// # Ok(())
    /// # }
    /// ```
    fn scan_index<I: IndexDefinition<TD>>(
        client: aws_sdk_dynamodb::Client,
    ) -> ScanRequest<TD, Self, Typed>
    where
        Self: DeserializeOwned + HasIndexKeyAttributes<TD, I>,
    {
        ScanRequest::_new_index::<I>(client)
    }

    /// Returns a [`QueryRequest`] builder in `Typed` output mode for querying
    /// the table with the given key condition.
    ///
    /// The returned builder can be executed with
    /// [`.all()`][QueryRequest::all] or [`.stream()`][QueryRequest::stream],
    /// and further configured with [`.filter()`][QueryRequest::filter],
    /// [`.project()`][QueryRequest::project], [`.limit()`][QueryRequest::limit],
    /// [`.reverse()`][QueryRequest::reverse], or
    /// [`.raw()`][QueryRequest::raw].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Condition, KeyCondition};
    ///
    /// # async fn example(cclient: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// # let client = cclient.clone();
    /// // Query all enrollments for a specific user
    /// let enrollments /* : Vec<Enrollment> */ = Enrollment::query(
    ///     client,
    ///     Enrollment::key_condition("user-1").sk_begins_with("ENROLL#"),
    /// )
    /// .all()
    /// .await?;
    ///
    /// # let client = cclient.clone();
    /// // Query with a filter
    /// let advanced /* : Vec<Enrollment> */ =
    ///     Enrollment::query(client, Enrollment::key_condition("user-1"))
    ///         .filter(Condition::gt("progress", 0.5))
    ///         .all()
    ///         .await?;
    /// # Ok(())
    /// # }
    /// ```
    fn query(
        client: aws_sdk_dynamodb::Client,
        key_condition: KeyCondition<'_, TD::KeySchema, impl KeyConditionState>,
    ) -> QueryRequest<TD, Self, Typed>
    where
        Self: DeserializeOwned,
    {
        QueryRequest::_new(client, key_condition)
    }

    /// Returns a [`QueryRequest`] builder in `Typed` output mode, using the
    /// type's constant partition key value as the key condition.
    ///
    /// Available only when `Self` has a compile-time constant value for the
    /// table's partition key (i.e. implements
    /// `HasConstAttribute<TD::KeySchema::PartitionKey>`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // Query all items stored under the constant PlatformConfig PK
    /// let configs /* : Vec<PlatformConfig> */ = PlatformConfig::query_all(client)
    ///     .all()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    fn query_all(client: aws_sdk_dynamodb::Client) -> QueryRequest<TD, Self, Typed>
    where
        Self: DeserializeOwned + HasConstAttribute<<TD::KeySchema as KeySchema>::PartitionKey>,
    {
        Self::query(client, KeyCondition::pk(Self::VALUE))
    }

    /// Returns a [`QueryRequest`] builder in `Typed` output mode for querying
    /// a secondary index (GSI or LSI) with the given key condition.
    ///
    /// `I` must be an [`IndexDefinition`] for `TD`, and `Self` must implement
    /// [`HasAttribute<A>`] for evey keys of the IndexDefinition. The key
    /// condition is typed to the index's key schema, preventing mismatched
    /// attribute usage at compile time.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyCondition};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // Query a secondary index
    /// let users /* : Vec<User> */ = User::query_index::<EmailIndex>(
    ///     client,
    ///     KeyCondition::pk("alice@example.com".to_owned()),
    /// )
    /// .all()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    fn query_index<I: IndexDefinition<TD>>(
        client: aws_sdk_dynamodb::Client,
        key_condition: KeyCondition<'_, I::KeySchema, impl KeyConditionState>,
    ) -> QueryRequest<TD, Self, Typed>
    where
        Self: DeserializeOwned + HasIndexKeyAttributes<TD, I>,
    {
        QueryRequest::_new_index::<I>(client, key_condition)
    }

    /// Returns a [`QueryRequest`] builder in `Typed` output mode for querying
    /// a secondary index (GSI or LSI) using the type's constant PK value for
    /// that index.
    ///
    /// Available only when `Self` has a compile-time constant value for the
    /// index's partition key (i.e. implements
    /// `HasConstAttribute<I::KeySchema::PartitionKey>`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // Query all users via the TypeIndex (constant ItemType = "USER")
    /// let all_users /* : Vec<User> */ = User::query_all_index::<TypeIndex>(client)
    ///     .all()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    fn query_all_index<I: IndexDefinition<TD>>(
        client: aws_sdk_dynamodb::Client,
    ) -> QueryRequest<TD, Self, Typed>
    where
        Self: DeserializeOwned
            + HasIndexKeyAttributes<TD, I>
            + HasConstAttribute<<I::KeySchema as KeySchema>::PartitionKey>,
    {
        Self::query_index::<I>(client, KeyCondition::pk(Self::VALUE))
    }

    // -- Condition helpers ----------------------------------------------------

    /// Returns a [`Condition`] that checks whether an item exists.
    ///
    /// Generates `attribute_exists(<PK>)` using the table's partition key
    /// attribute name. Useful as a guard on put, delete, and update operations,
    /// or as a component in compound conditions.
    ///
    /// See also `exists` shorthand methods on the individual request builders.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Condition};
    ///
    /// // Use as a standalone condition
    /// let cond = User::exists();
    ///
    /// // Combine with another condition using `&`
    /// let cond = User::exists() & Condition::eq("role", "admin");
    /// ```
    fn exists() -> Condition<'static> {
        Condition::exists(<TD::KeySchema as KeySchema>::PartitionKey::NAME)
    }

    /// Returns a [`Condition`] that checks whether an item does not exist.
    ///
    /// Generates `attribute_not_exists(<PK>)` using the table's partition key
    /// attribute name. Commonly used with [`put`][DynamoDBItemOp::put] to
    /// implement create-only semantics.
    ///
    /// See also `not_exists` shorthand methods on the individual request builders.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Condition};
    ///
    /// // Use as a standalone condition
    /// let cond = User::not_exists();
    ///
    /// // Combine: create-only OR expired TTL
    /// let cond = User::not_exists() | Condition::lt("expiration_timestamp", 1_700_000_000);
    /// ```
    fn not_exists() -> Condition<'static> {
        Condition::not_exists(<TD::KeySchema as KeySchema>::PartitionKey::NAME)
    }

    // -- Key Condition helpers ------------------------------------------------

    /// Builds a [`KeyCondition`] for the table's partition key from a typed ID.
    ///
    /// Uses the type's [`HasAttribute`] implementation to convert `pk_id` into
    /// the DynamoDB attribute value for the partition key. The resulting
    /// condition can be extended with sort-key constraints before being passed
    /// to [`query`][DynamoDBItemOp::query] methods.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // All enrollments for a user
    /// let kc = Enrollment::key_condition("user-1").sk_begins_with("ENROLL#");
    /// let enrollments /* : Vec<Enrollment> */ = Enrollment::query(client, kc)
    ///     .all()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    fn key_condition(
        pk_id: <Self as HasAttribute<PartitionKeyDefinition<TD>>>::Id<'_>,
    ) -> KeyCondition<'static, TD::KeySchema> {
        KeyCondition::pk(<Self as HasAttribute<PartitionKeyDefinition<TD>>>::attribute_value(pk_id))
    }

    /// Builds a [`KeyCondition`] for a secondary index's partition key from a typed ID.
    ///
    /// Uses the type's [`HasAttribute`] implementation for the index's
    /// partition key attribute to convert `pk_id` into the appropriate
    /// DynamoDB value. The resulting condition is typed to the index's key
    /// schema, so sort-key methods are only available when the index has a
    /// sort key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// // Build a key condition for the EmailIndex
    /// let kc = User::index_key_condition::<EmailIndex>("alice@example.com");
    /// let _ = kc;
    /// ```
    fn index_key_condition<I: IndexDefinition<TD>>(
        pk_id: <Self as HasAttribute<<I::KeySchema as KeySchema>::PartitionKey>>::Id<'_>,
    ) -> KeyCondition<'static, I::KeySchema>
    where
        Self: HasAttribute<<I::KeySchema as KeySchema>::PartitionKey>,
    {
        KeyCondition::pk(<Self as HasAttribute<
            <I::KeySchema as KeySchema>::PartitionKey,
        >>::attribute_value(pk_id))
    }
}

impl<TD: TableDefinition, DBI: DynamoDBItem<TD>> DynamoDBItemOp<TD> for DBI {}
