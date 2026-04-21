use std::future::{Future, IntoFuture};
use std::pin::Pin;

use super::*;

use aws_sdk_dynamodb::operation::update_item::builders::UpdateItemFluentBuilder;

/// Builder for a DynamoDB `UpdateItem` request.
///
/// Constructed via [`DynamoDBItemOp::update`] / [`DynamoDBItemOp::update_by_id`]
/// (typed, with a concrete `T`) or [`UpdateItemRequest::new`] (stand-alone,
/// raw output). The builder provides:
///
/// - **Output format** — the result can be deserialized into `T`.
///   Call [`.raw()`][UpdateItemRequest::raw] to receive an untyped [`Item<TD>`]
///   instead (one-way).
/// - **Return value** — by default nothing is returned. Call
///   [`.return_old()`][UpdateItemRequest::return_old],
///   [`.return_new()`][UpdateItemRequest::return_new], or
///   [`.return_none()`][UpdateItemRequest::return_none] to choose whether
///   DynamoDB returns the pre- or post-update item.
///   [`.return_new()`][UpdateItemRequest::return_new] returns `T` /
///   `Item<TD>` directly (DynamoDB's `ALL_NEW` always provides the updated
///   item). [`.return_old()`][UpdateItemRequest::return_old] returns
///   `Option<T>` / `Option<Item<TD>>` because the item may not have existed
///   before the update (DynamoDB's `UpdateItem` is an upsert). Note:
///   [`DynamoDBItemOp::update_by_id`] starts with return-new by default.
/// - **Condition** — optionally add a guard expression via
///   [`.condition()`][UpdateItemRequest::condition],
///   [`.exists()`][UpdateItemRequest::exists], or
///   [`.not_exists()`][UpdateItemRequest::not_exists]. DynamoDB accepts a
///   single condition expression per request, so this can only be called once.
///
/// The builder implements [`IntoFuture`], so it can
/// be `.await`ed directly.
///
/// # Errors
///
/// Returns [`Err`] if the DynamoDB request fails, if a condition check
/// fails, or if deserialization of the returned attributes fails.
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
#[must_use = "builder does nothing until awaited or executed"]
pub struct UpdateItemRequest<
    TD: TableDefinition,
    T = (),
    O: OutputFormat = Raw,
    R: ReturnValue = ReturnNothing,
    C: ConditionState = NoCondition,
> {
    builder: UpdateItemFluentBuilder,
    _marker: PhantomData<(TD, T, O, R, C)>,
}

// -- Common methods (all states) --------------------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, R: ReturnValue, C: ConditionState>
    UpdateItemRequest<TD, T, O, R, C>
{
    /// Consumes the builder and returns the underlying SDK
    /// [`UpdateItemFluentBuilder`].
    ///
    /// Use this escape hatch when you need to set options not exposed by this
    /// facade, or when integrating with code that expects the raw SDK builder.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let sdk_builder = sample_user()
    ///     .update(client, Update::set("role", "instructor"))
    ///     .into_inner();
    /// // configure sdk_builder further, then call .send().await
    /// # Ok(())
    /// # }
    /// ```
    pub fn into_inner(self) -> UpdateItemFluentBuilder {
        self.builder
    }
}

// -- Stand-alone constructor (ReturnNothing, any C, T = (), O = Raw)

impl<TD: TableDefinition> UpdateItemRequest<TD> {
    /// Creates a stand-alone `UpdateItemRequest` with raw output (`T = ()`, `O = Raw`).
    ///
    /// Use this when you already have a [`Key<TD>`] and an [`Update`] expression
    /// and do not need typed deserialization of the returned item. For typed
    /// access, prefer [`DynamoDBItemOp::update`] or
    /// [`DynamoDBItemOp::update_by_id`] instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{UpdateItemRequest, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let key = sample_user_item().into_key_only();
    /// UpdateItemRequest::<PlatformTable>::new(client, key, Update::set("role", "instructor"))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: aws_sdk_dynamodb::Client, key: Key<TD>, update: Update<'_>) -> Self {
        Self::_new(client, key, update)
    }
}

// -- Constructor (any R, any O, any C) ------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, R: ReturnValue, C: ConditionState>
    UpdateItemRequest<TD, T, O, R, C>
{
    /// Creates a new `UpdateItemRequest` targeting the given key and applying `update`.
    pub(super) fn _new(client: aws_sdk_dynamodb::Client, key: Key<TD>, update: Update<'_>) -> Self {
        let table_name = TD::table_name();
        tracing::debug!(table_name, ?key, %update, "UpdateItem");
        Self {
            builder: update.apply(
                client
                    .update_item()
                    .table_name(table_name)
                    .set_key(Some(key.into_inner())),
            ),
            _marker: PhantomData,
        }
    }
}

// -- Return-value transitions (preserve O, C) -------------------------------

// From ReturnNothing
impl<TD: TableDefinition, T: DeserializeOwned, O: OutputFormat, C: ConditionState>
    UpdateItemRequest<TD, T, O, ReturnNothing, C>
{
    /// Requests that DynamoDB return the item's attributes before the update.
    ///
    /// When executed, [`execute`][UpdateItemRequest::execute] returns
    /// `Option<T>` (typed) or `Option<Item<TD>>` (raw) containing the
    /// pre-update state, or `None` if no item existed at the target key
    /// prior to the update.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let before /* : Option<User> */ = sample_user()
    ///     .update(client, Update::set("role", "instructor"))
    ///     .exists()
    ///     .return_old()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn return_old(self) -> UpdateItemRequest<TD, T, O, Return<Old>, C> {
        tracing::debug!("UpdateItem return_old");
        UpdateItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }

    /// Requests that DynamoDB return the item's attributes after the update.
    ///
    /// When executed, [`execute`][UpdateItemRequest::execute] returns `T`
    /// (typed) or [`Item<TD>`] (raw) containing the post-update state.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let after /* : User */ = sample_user()
    ///     .update(client, Update::set("role", "instructor"))
    ///     .exists()
    ///     .return_new()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn return_new(self) -> UpdateItemRequest<TD, T, O, Return<New>, C> {
        tracing::debug!("UpdateItem return_new");
        UpdateItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// From ReturnItem<New>
impl<TD: TableDefinition, T: DeserializeOwned, O: OutputFormat, C: ConditionState>
    UpdateItemRequest<TD, T, O, Return<New>, C>
{
    /// Switches from returning the post-update item to returning the
    /// pre-update item.
    ///
    /// The [`execute`][UpdateItemRequest::execute] return type changes
    /// from `T` / `Item<TD>` to `Option<T>` / `Option<Item<TD>>`,
    /// because the old item may not exist if the update created it
    /// (DynamoDB's `UpdateItem` is an upsert).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // update_by_id defaults to Return<New>; switch to Return<Old>
    /// let before /* : Option<User> */ = User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("role", "instructor"),
    /// )
    /// .exists()
    /// .return_old()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn return_old(self) -> UpdateItemRequest<TD, T, O, Return<Old>, C> {
        tracing::debug!("UpdateItem return_old");
        UpdateItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }

    /// Reverts the return-value setting so that nothing is returned.
    ///
    /// After this call, [`execute`][UpdateItemRequest::execute] returns `()`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // update_by_id defaults to Return<New>; opt out
    /// User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("role", "instructor"),
    /// )
    /// .exists()
    /// .return_none()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn return_none(self) -> UpdateItemRequest<TD, T, O, ReturnNothing, C> {
        tracing::debug!("UpdateItem return_none");
        UpdateItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// From ReturnItem<Old>
impl<TD: TableDefinition, T: DeserializeOwned, O: OutputFormat, C: ConditionState>
    UpdateItemRequest<TD, T, O, Return<Old>, C>
{
    /// Switches from returning the pre-update item to returning the
    /// post-update item.
    ///
    /// The [`execute`][UpdateItemRequest::execute] return type changes
    /// from `Option<T>` / `Option<Item<TD>>` to `T` / `Item<TD>`,
    /// because DynamoDB's `ALL_NEW` return mode always includes the full
    /// item after the update.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let after /* : User */ = sample_user()
    ///     .update(client, Update::set("role", "instructor"))
    ///     .exists()
    ///     .return_old()
    ///     .return_new()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn return_new(self) -> UpdateItemRequest<TD, T, O, Return<New>, C> {
        tracing::debug!("UpdateItem return_new");
        UpdateItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }

    /// Reverts the return-value setting so that nothing is returned.
    ///
    /// After this call, [`execute`][UpdateItemRequest::execute] returns `()`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// sample_user()
    ///     .update(client, Update::set("role", "instructor"))
    ///     .exists()
    ///     .return_old()
    ///     .return_none()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn return_none(self) -> UpdateItemRequest<TD, T, O, ReturnNothing, C> {
        tracing::debug!("UpdateItem return_none");
        UpdateItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// -- Condition (NoCondition only) -------------------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, R: ReturnValue>
    UpdateItemRequest<TD, T, O, R, NoCondition>
{
    /// Adds a condition expression that must be satisfied for the update to succeed.
    ///
    /// DynamoDB accepts a single condition expression per request, so this
    /// method can only be called once. If the condition fails at runtime,
    /// DynamoDB returns a `ConditionalCheckFailedException`.
    ///
    /// For the common item exists/not_exists cases, prefer
    /// the [`.exists()`][UpdateItemRequest::exists] and
    /// [`.not_exists()`][UpdateItemRequest::not_exists] shorthands.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId, Update, Condition};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // Update role only if the current role is not "student"
    /// User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("role", "instructor"),
    /// )
    /// .condition(Condition::ne("role", "student"))
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn condition(
        mut self,
        condition: Condition<'_>,
    ) -> UpdateItemRequest<TD, T, O, R, AlreadyHasCondition> {
        tracing::debug!(%condition, "UpdateItem condition");
        self.builder = condition.apply(self.builder);
        UpdateItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

impl<TD: TableDefinition, T: DynamoDBItem<TD>, O: OutputFormat, R: ReturnValue>
    UpdateItemRequest<TD, T, O, R, NoCondition>
{
    /// Adds an `attribute_exists(<PK>)` condition, requiring the item to exist before updating.
    ///
    /// The update fails with `ConditionalCheckFailedException` if the item does not exist.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("name", "Bob"),
    /// )
    /// .exists()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn exists(self) -> UpdateItemRequest<TD, T, O, R, AlreadyHasCondition> {
        self.condition(T::exists())
    }

    /// Adds an `attribute_not_exists(<PK>)` condition, requiring the item to not yet exist.
    ///
    /// Useful for upsert-style operations where you want to initialize an item only
    /// if it does not already exist.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("role", "student"),
    /// )
    /// .not_exists()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn not_exists(self) -> UpdateItemRequest<TD, T, O, R, AlreadyHasCondition> {
        self.condition(T::not_exists())
    }
}

// -- Output format transition (preserve R, C) -------------------------------

impl<TD: TableDefinition, T, R: ReturnValue, C: ConditionState>
    UpdateItemRequest<TD, T, Typed, R, C>
{
    /// Switches the output format from `Typed` to `Raw`.
    ///
    /// After calling `.raw()`, [`execute`][UpdateItemRequest::execute] returns
    /// [`Item<TD>`] instead of `T` when a return value is requested.
    /// This transition is one-way.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let raw_new = User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("role", "instructor"),
    /// )
    /// .exists()
    /// .raw()
    /// .await?;
    /// // raw_new: Item<PlatformTable>
    /// # Ok(())
    /// # }
    /// ```
    pub fn raw(self) -> UpdateItemRequest<TD, T, Raw, R, C> {
        UpdateItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// -- Terminal: ReturnNothing (any O, any C) ---------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, C: ConditionState>
    UpdateItemRequest<TD, T, O, ReturnNothing, C>
{
    /// Sends the `UpdateItem` request, returning nothing on success.
    ///
    /// This method is also available implicitly via `.await`.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the DynamoDB request fails or if a condition
    /// expression is set and the check fails
    /// (`ConditionalCheckFailedException`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("name", "Bob"),
    /// )
    /// .exists()
    /// .return_none()
    /// .execute()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "update_execute")]
    pub fn execute(self) -> impl Future<Output = Result<()>> + Send + 'static {
        let builder = self.builder;
        async move {
            builder.return_values(SDKReturnValue::None).send().await?;
            Ok(())
        }
    }
}

impl<TD: TableDefinition, T, O: OutputFormat, C: ConditionState> IntoFuture
    for UpdateItemRequest<TD, T, O, ReturnNothing, C>
{
    type Output = Result<()>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

// -- Terminal: ReturnItem<Old> + Typed (any C) -------------------------------

impl<TD: TableDefinition, T: DynamoDBItem<TD> + DeserializeOwned, C: ConditionState>
    UpdateItemRequest<TD, T, Typed, Return<Old>, C>
{
    /// Sends the `UpdateItem` request and returns the pre-update item
    /// deserialized as `Option<T>`.
    ///
    /// Returns `Some(T)` containing the item's state **before** the update
    /// was applied, or `None` if no item existed at the target key prior to
    /// the update (DynamoDB's `UpdateItem` is an upsert — it creates the
    /// item if absent).
    ///
    /// This method is also available implicitly via `.await`.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the DynamoDB request fails, if a condition check
    /// fails, or if deserialization of the returned attributes fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let before /* : Option<User> */ = User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("role", "instructor"),
    /// )
    /// .exists()
    /// .return_old()
    /// .execute()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "update_execute_old")]
    pub fn execute(self) -> impl Future<Output = Result<Option<T>>> + Send + 'static {
        let builder = self.builder;
        async move {
            let out = builder.return_values(Old::return_value()).send().await?;

            out.attributes
                .map(Item::from_dynamodb_response)
                .map(T::try_from_item)
                .transpose()
        }
    }
}

impl<TD: TableDefinition, T: DynamoDBItem<TD> + DeserializeOwned, C: ConditionState> IntoFuture
    for UpdateItemRequest<TD, T, Typed, Return<Old>, C>
{
    type Output = Result<Option<T>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

// -- Terminal: ReturnItem<New> + Typed (any C) -------------------------------

impl<TD: TableDefinition, T: DynamoDBItem<TD> + DeserializeOwned, C: ConditionState>
    UpdateItemRequest<TD, T, Typed, Return<New>, C>
{
    /// Sends the `UpdateItem` request and returns the post-update item
    /// deserialized as `T`.
    ///
    /// Because DynamoDB's `ALL_NEW` return mode always includes the full
    /// item after the update, this method returns `T` directly (not
    /// `Option<T>`).
    ///
    /// This method is also available implicitly via `.await`.
    ///
    /// # Panics
    ///
    /// Panics if DynamoDB does not return attributes in the response. This
    /// should not happen when `ALL_NEW` is requested, but could indicate a
    /// bug in the SDK or an unexpected API change.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the DynamoDB request fails, if a condition check
    /// fails, or if deserialization of the returned attributes fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let updated /* : User */ = User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("role", "instructor"),
    /// )
    /// .exists()
    /// .execute()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "update_execute_new")]
    pub fn execute(self) -> impl Future<Output = Result<T>> + Send + 'static {
        let builder = self.builder;
        async move {
            let out = builder.return_values(New::return_value()).send().await?;

            out.attributes
                .map(Item::from_dynamodb_response)
                .map(T::try_from_item)
                .expect("asked to return something")
        }
    }
}

impl<TD: TableDefinition, T: DynamoDBItem<TD> + DeserializeOwned, C: ConditionState> IntoFuture
    for UpdateItemRequest<TD, T, Typed, Return<New>, C>
{
    type Output = Result<T>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

// -- Terminal: ReturnItem<Old> + Raw (any C) ---------------------------------

impl<TD: TableDefinition, T, C: ConditionState> UpdateItemRequest<TD, T, Raw, Return<Old>, C> {
    /// Sends the `UpdateItem` request and returns the pre-update raw item
    /// map as `Option<Item<TD>>`.
    ///
    /// Returns `Some(Item<TD>)` containing the item's state **before** the
    /// update was applied, or `None` if no item existed at the target key
    /// prior to the update (DynamoDB's `UpdateItem` is an upsert — it
    /// creates the item if absent).
    ///
    /// This method is also available implicitly via `.await`.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the DynamoDB request fails or if a condition check
    /// fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let raw /* : Option<dynamodb_facade::Item<PlatformTable>> */ = User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("role", "instructor"),
    /// )
    /// .exists()
    /// .return_old()
    /// .raw()
    /// .execute()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "update_execute_old_raw")]
    pub fn execute(self) -> impl Future<Output = Result<Option<Item<TD>>>> + Send + 'static {
        let builder = self.builder;
        async move {
            let out = builder.return_values(Old::return_value()).send().await?;

            Ok(out.attributes.map(Item::from_dynamodb_response))
        }
    }
}

impl<TD: TableDefinition, T, C: ConditionState> IntoFuture
    for UpdateItemRequest<TD, T, Raw, Return<Old>, C>
{
    type Output = Result<Option<Item<TD>>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

// -- Terminal: ReturnItem<New> + Raw (any C) ---------------------------------

impl<TD: TableDefinition, T, C: ConditionState> UpdateItemRequest<TD, T, Raw, Return<New>, C> {
    /// Sends the `UpdateItem` request and returns the post-update raw item
    /// map as `Item<TD>`.
    ///
    /// Because DynamoDB's `ALL_NEW` return mode always includes the full
    /// item after the update, this method returns `Item<TD>` directly (not
    /// `Option<Item<TD>>`).
    ///
    /// This method is also available implicitly via `.await`.
    ///
    /// # Panics
    ///
    /// Panics if DynamoDB does not return attributes in the response. This
    /// should not happen when `ALL_NEW` is requested, but could indicate a
    /// bug in the SDK or an unexpected API change.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the DynamoDB request fails or if a condition check
    /// fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let raw /* : dynamodb_facade::Item<PlatformTable> */ = User::update_by_id(
    ///     client,
    ///     KeyId::pk("user-1"),
    ///     Update::set("role", "instructor"),
    /// )
    /// .exists()
    /// .raw()
    /// .execute()
    /// .await?;
    /// assert!(raw.get("role").is_some());
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "update_execute_new_raw")]
    pub fn execute(self) -> impl Future<Output = Result<Item<TD>>> + Send + 'static {
        let builder = self.builder;
        async move {
            let out = builder.return_values(New::return_value()).send().await?;

            Ok(out
                .attributes
                .map(Item::from_dynamodb_response)
                .expect("asked to return something"))
        }
    }
}

impl<TD: TableDefinition, T, C: ConditionState> IntoFuture
    for UpdateItemRequest<TD, T, Raw, Return<New>, C>
{
    type Output = Result<Item<TD>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}
