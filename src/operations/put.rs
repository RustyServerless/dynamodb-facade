use std::future::{Future, IntoFuture};
use std::pin::Pin;

use super::*;

use aws_sdk_dynamodb::operation::put_item::builders::PutItemFluentBuilder;

/// Builder for a DynamoDB `PutItem` request.
///
/// Constructed via [`DynamoDBItemOp::put`] (typed, with a concrete `T`) or
/// [`PutItemRequest::new`] (stand-alone, raw output). The builder provides:
///
/// - **Output format** — the result can be deserialized into `T`.
///   Call [`.raw()`][PutItemRequest::raw] to receive an untyped [`Item<TD>`]
///   instead (one-way).
/// - **Return value** — by default nothing is returned. Call
///   [`.return_old()`][PutItemRequest::return_old] to request the previous
///   item, or [`.return_none()`][PutItemRequest::return_none] to revert.
/// - **Condition** — optionally add a guard expression via
///   [`.condition()`][PutItemRequest::condition],
///   [`.exists()`][PutItemRequest::exists], or
///   [`.not_exists()`][PutItemRequest::not_exists]. DynamoDB accepts a
///   single condition expression per request, so this can only be called once.
///
/// The builder implements [`IntoFuture`], so it can
/// be `.await`ed directly.
///
/// # Errors
///
/// Returns [`Err`] if the DynamoDB request fails, if a condition expression
/// is set and the condition check fails
/// (`ConditionalCheckFailedException`), or if serialization of `self` fails.
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
/// // Simple put
/// user.put(client).await?;
///
/// # let client = cclient.clone();
/// // Create-only: fails if item already exists
/// user.put(client).not_exists().await?;
///
/// # let client = cclient.clone();
/// // Custom condition
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
#[must_use = "builder does nothing until awaited or executed"]
pub struct PutItemRequest<
    TD: TableDefinition,
    T = (),
    O: OutputFormat = Raw,
    R: ReturnValue = ReturnNothing,
    C: ConditionState = NoCondition,
> {
    builder: PutItemFluentBuilder,
    _marker: PhantomData<(TD, T, O, R, C)>,
}

// -- Common methods (all states) --------------------------------------------

impl<TD: TableDefinition, T, R: ReturnValue, O: OutputFormat, C: ConditionState>
    PutItemRequest<TD, T, O, R, C>
{
    /// Consumes the builder and returns the underlying SDK
    /// [`PutItemFluentBuilder`].
    ///
    /// Use this escape hatch when you need to set options not exposed by this
    /// facade, or when integrating with code that expects the raw SDK builder.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let sdk_builder = sample_user().put(client).into_inner();
    /// // configure sdk_builder further, then call .send().await
    /// # Ok(())
    /// # }
    /// ```
    pub fn into_inner(self) -> PutItemFluentBuilder {
        self.builder
    }
}

// -- Stand-alone constructor (ReturnNothing, NoCondition, T = (), O = Raw)

impl<TD: TableDefinition> PutItemRequest<TD, (), Raw> {
    /// Creates a stand-alone `PutItemRequest` with raw output (`T = ()`, `O = Raw`).
    ///
    /// Use this when you already have an [`Item<TD>`] and do not need typed
    /// deserialization of the old value. For typed access, prefer
    /// [`DynamoDBItemOp::put`] instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::PutItemRequest;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let item = sample_user_item();
    /// PutItemRequest::<PlatformTable>::new(client, item).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: aws_sdk_dynamodb::Client, item: Item<TD>) -> Self {
        Self::_new(client, item)
    }
}

// -- Constructor (any R, any O, any C) ------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, R: ReturnValue, C: ConditionState>
    PutItemRequest<TD, T, O, R, C>
{
    /// Creates a new `PutItemRequest` with the given item.
    pub(super) fn _new(client: aws_sdk_dynamodb::Client, item: Item<TD>) -> Self {
        let table_name = TD::table_name();
        tracing::debug!(table_name, "PutItem");
        Self {
            builder: client
                .put_item()
                .table_name(table_name)
                .set_item(Some(item.into_inner())),
            _marker: PhantomData,
        }
    }
}

// -- Return-value transitions (preserve O, C) -------------------------------

impl<TD: TableDefinition, T: DeserializeOwned, O: OutputFormat, C: ConditionState>
    PutItemRequest<TD, T, O, ReturnNothing, C>
{
    /// Requests that DynamoDB return the item's previous attributes after the put.
    ///
    /// When executed, [`execute`][PutItemRequest::execute] returns
    /// `Option<T>` (typed) or `Option<Item<TD>>` (raw) — `None` if no item
    /// previously existed at that key.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let user = sample_user();
    /// let old /* : Option<User> */ = user.put(client).return_old().await?;
    /// // old is None if this was the first put, Some(prev_user) otherwise
    /// # Ok(())
    /// # }
    /// ```
    pub fn return_old(self) -> PutItemRequest<TD, T, O, Return<Old>, C> {
        tracing::debug!("PutItem return_old");
        PutItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

impl<TD: TableDefinition, T: DeserializeOwned, O: OutputFormat, C: ConditionState>
    PutItemRequest<TD, T, O, Return<Old>, C>
{
    /// Reverts the return-value setting so that nothing is returned.
    ///
    /// After this call, [`execute`][PutItemRequest::execute] returns `()`
    /// instead of the old item.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let user = sample_user();
    /// // Start with return_old, then decide we don't need the old value
    /// user.put(client).return_old().return_none().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn return_none(self) -> PutItemRequest<TD, T, O, ReturnNothing, C> {
        tracing::debug!("PutItem return_none");
        PutItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// -- Condition (NoCondition only) -------------------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, R: ReturnValue>
    PutItemRequest<TD, T, O, R, NoCondition>
{
    /// Adds a condition expression that must be satisfied for the put to succeed.
    ///
    /// DynamoDB accepts a single condition expression per request, so this
    /// method can only be called once. If the condition fails at runtime,
    /// DynamoDB returns a `ConditionalCheckFailedException`.
    ///
    /// For the common item exists/not_exists cases, prefer
    /// the [`.exists()`][PutItemRequest::exists] and
    /// [`.not_exists()`][PutItemRequest::not_exists] shorthands.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Condition};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let user = sample_user();
    /// // Put only if the item does not exist OR its TTL has expired
    /// user.put(client)
    ///     .condition(User::not_exists() | Condition::lt("expiration_timestamp", 1_700_000_000))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn condition(
        mut self,
        condition: Condition<'_>,
    ) -> PutItemRequest<TD, T, O, R, AlreadyHasCondition> {
        tracing::debug!(%condition, "PutItem condition");
        self.builder = condition.apply(self.builder);
        PutItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

impl<TD: TableDefinition, T: DynamoDBItem<TD>, O: OutputFormat, R: ReturnValue>
    PutItemRequest<TD, T, O, R, NoCondition>
{
    /// Adds an `attribute_exists(<PK>)` condition, requiring the item to already exist.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // Overwrite only if the item already exists
    /// sample_user().put(client).exists().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn exists(self) -> PutItemRequest<TD, T, O, R, AlreadyHasCondition> {
        self.condition(T::exists())
    }

    /// Adds an `attribute_not_exists(<PK>)` condition, requiring the item to not yet exist.
    ///
    /// Use this to implement create-only (insert-if-absent) semantics.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // Create-only: fails if user already exists
    /// sample_user().put(client).not_exists().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn not_exists(self) -> PutItemRequest<TD, T, O, R, AlreadyHasCondition> {
        self.condition(T::not_exists())
    }
}

// -- Output format transition (preserve R, C) -------------------------------

impl<TD: TableDefinition, T, R: ReturnValue, C: ConditionState> PutItemRequest<TD, T, Typed, R, C> {
    /// Switches the output format from `Typed` to `Raw`.
    ///
    /// After calling `.raw()`, [`execute`][PutItemRequest::execute] returns
    /// `Option<Item<TD>>` instead of `Option<T>` when `Return<Old>` is active.
    /// This transition is one-way.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let old_raw = sample_user()
    ///     .put(client)
    ///     .return_old()
    ///     .raw()
    ///     .await?;
    /// // old_raw: Option<Item<PlatformTable>>
    /// # Ok(())
    /// # }
    /// ```
    pub fn raw(self) -> PutItemRequest<TD, T, Raw, R, C> {
        PutItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// -- Terminal: ReturnNothing (any O, any C) ---------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, C: ConditionState>
    PutItemRequest<TD, T, O, ReturnNothing, C>
{
    /// Sends the `PutItem` request, returning nothing on success.
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
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// sample_user().put(client).not_exists().execute().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "put_execute")]
    pub fn execute(self) -> impl Future<Output = Result<()>> + Send + 'static {
        let builder = self.builder;
        async move {
            builder.return_values(SDKReturnValue::None).send().await?;
            Ok(())
        }
    }
}

impl<TD: TableDefinition, T, O: OutputFormat, C: ConditionState> IntoFuture
    for PutItemRequest<TD, T, O, ReturnNothing, C>
{
    type Output = Result<()>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

// -- Terminal: ReturnItem<Old> + Typed (any C) ------------------------------

impl<TD: TableDefinition, T: DynamoDBItem<TD> + DeserializeOwned, C: ConditionState>
    PutItemRequest<TD, T, Typed, Return<Old>, C>
{
    /// Sends the `PutItem` request and returns the previous item deserialized as `T`.
    ///
    /// Returns `Ok(None)` if no item previously existed at the key.
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
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let old /* : Option<User> */ = sample_user().put(client).return_old().execute().await?;
    /// // old is None on first put, Some(previous_user) on subsequent puts
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "put_execute_old")]
    pub fn execute(self) -> impl Future<Output = Result<Option<T>>> + Send + 'static {
        let builder = self.builder;
        async move {
            builder
                .return_values(SDKReturnValue::AllOld)
                .send()
                .await?
                .attributes
                .map(Item::from_dynamodb_response)
                .map(T::try_from_item)
                .transpose()
        }
    }
}

impl<TD: TableDefinition, T: DynamoDBItem<TD> + DeserializeOwned, C: ConditionState> IntoFuture
    for PutItemRequest<TD, T, Typed, Return<Old>, C>
{
    type Output = Result<Option<T>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

// -- Terminal: ReturnItem<Old> + Raw (any C) --------------------------------

impl<TD: TableDefinition, T, C: ConditionState> PutItemRequest<TD, T, Raw, Return<Old>, C> {
    /// Sends the `PutItem` request and returns the previous raw item map.
    ///
    /// Returns `Ok(None)` if no item previously existed at the key.
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
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let old_raw = sample_user()
    ///     .put(client)
    ///     .return_old()
    ///     .raw()
    ///     .execute()
    ///     .await?;
    /// // old_raw: Option<Item<PlatformTable>>
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "put_execute_old_raw")]
    pub fn execute(self) -> impl Future<Output = Result<Option<Item<TD>>>> + Send + 'static {
        let builder = self.builder;
        async move {
            Ok(builder
                .return_values(SDKReturnValue::AllOld)
                .send()
                .await
                .map(|out| out.attributes.map(Item::from_dynamodb_response))?)
        }
    }
}

impl<TD: TableDefinition, T, C: ConditionState> IntoFuture
    for PutItemRequest<TD, T, Raw, Return<Old>, C>
{
    type Output = Result<Option<Item<TD>>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}
