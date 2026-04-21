use std::future::{Future, IntoFuture};
use std::pin::Pin;

use super::*;

use aws_sdk_dynamodb::operation::delete_item::builders::DeleteItemFluentBuilder;

/// Builder for a DynamoDB `DeleteItem` request.
///
/// Constructed via [`DynamoDBItemOp::delete`] / [`DynamoDBItemOp::delete_by_id`]
/// (typed, with a concrete `T`) or [`DeleteItemRequest::new`] (stand-alone,
/// raw output). The builder provides:
///
/// - **Output format** — the result can be deserialized into `T`.
///   Call [`.raw()`][DeleteItemRequest::raw] to receive an untyped [`Item<TD>`]
///   instead (one-way).
/// - **Return value** — Call [`.return_old()`][DeleteItemRequest::return_old]
///   to request the deleted item, or [`.return_none()`][DeleteItemRequest::return_none]
///   to return nothing.
/// - **Condition** — optionally add a guard expression via
///   [`.condition()`][DeleteItemRequest::condition], or
///   [`.exists()`][DeleteItemRequest::exists].
///
/// The builder implements [`IntoFuture`], so it can
/// be `.await`ed directly.
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
/// use dynamodb_facade::{DynamoDBItemOp, Condition, KeyId};
///
/// # async fn example(cclient: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
/// let enrollment = sample_enrollment();
///
/// # let client = cclient.clone();
/// // Simple delete
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
///
/// # let client = cclient.clone();
/// // Delete by ID and return the old item
/// let old /* : Option<Enrollment> */ = Enrollment::delete_by_id(
///     client,
///     KeyId::pk("user-1").sk("course-42"),
/// )
/// .await?;
/// # Ok(())
/// # }
/// ```
#[must_use = "builder does nothing until awaited or executed"]
pub struct DeleteItemRequest<
    TD: TableDefinition,
    T = (),
    O: OutputFormat = Raw,
    R: ReturnValue = ReturnNothing,
    C: ConditionState = NoCondition,
> {
    builder: DeleteItemFluentBuilder,
    _marker: PhantomData<(TD, T, O, R, C)>,
}

// -- Common methods (all states) --------------------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, R: ReturnValue, C: ConditionState>
    DeleteItemRequest<TD, T, O, R, C>
{
    /// Consumes the builder and returns the underlying SDK
    /// [`DeleteItemFluentBuilder`].
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
    /// let sdk_builder = sample_enrollment().delete(client).into_inner();
    /// // configure sdk_builder further, then call .send().await
    /// # Ok(())
    /// # }
    /// ```
    pub fn into_inner(self) -> DeleteItemFluentBuilder {
        self.builder
    }
}

// -- Stand-alone constructor (ReturnNothing, NoCondition, T = (), O = Raw)

impl<TD: TableDefinition> DeleteItemRequest<TD> {
    /// Creates a stand-alone `DeleteItemRequest` with raw output (`T = ()`, `O = Raw`).
    ///
    /// Use this when you already have a [`Key<TD>`] and do not need typed
    /// deserialization of the deleted value. For typed access, prefer
    /// [`DynamoDBItemOp::delete`] or [`DynamoDBItemOp::delete_by_id`] instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DeleteItemRequest;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let key = sample_user_item().into_key_only();
    /// DeleteItemRequest::<PlatformTable>::new(client, key).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: aws_sdk_dynamodb::Client, key: Key<TD>) -> Self {
        Self::_new(client, key)
    }
}

// -- Constructor (any R, any O, any C) ------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, R: ReturnValue, C: ConditionState>
    DeleteItemRequest<TD, T, O, R, C>
{
    /// Creates a new `DeleteItemRequest` targeting the given key.
    pub(super) fn _new(client: aws_sdk_dynamodb::Client, key: Key<TD>) -> Self {
        let table_name = TD::table_name();
        tracing::debug!(table_name, ?key, "DeleteItem");
        Self {
            builder: client
                .delete_item()
                .table_name(table_name)
                .set_key(Some(key.into_inner())),
            _marker: PhantomData,
        }
    }
}

// -- Return-value transitions (preserve O, C) -------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, C: ConditionState>
    DeleteItemRequest<TD, T, O, ReturnNothing, C>
{
    /// Requests that DynamoDB return the item's attributes before deletion.
    ///
    /// When executed, [`execute`][DeleteItemRequest::execute] returns
    /// `Option<T>` (typed) or `Option<Item<TD>>` (raw) — `None` if no item
    /// existed at that key.
    ///
    /// Use [`.return_none()`][DeleteItemRequest::return_none] to revert.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let old /* : Option<Enrollment> */ = sample_enrollment()
    ///     .delete(client)
    ///     .return_old()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn return_old(self) -> DeleteItemRequest<TD, T, O, Return<Old>, C> {
        tracing::debug!("DeleteItem return_old");
        DeleteItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

impl<TD: TableDefinition, T, O: OutputFormat, C: ConditionState>
    DeleteItemRequest<TD, T, O, Return<Old>, C>
{
    /// Reverts the return-value setting so that nothing is returned.
    ///
    /// After this call, [`execute`][DeleteItemRequest::execute] returns `()`
    /// instead of the deleted item.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // delete_by_id defaults to Return<Old>; opt out with return_none
    /// Enrollment::delete_by_id(client, KeyId::pk("user-1").sk("course-42"))
    ///     .return_none()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn return_none(self) -> DeleteItemRequest<TD, T, O, ReturnNothing, C> {
        tracing::debug!("DeleteItem return_none");
        DeleteItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// -- Condition (NoCondition only) -------------------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, R: ReturnValue>
    DeleteItemRequest<TD, T, O, R, NoCondition>
{
    /// Adds a condition expression that must be satisfied for the delete to succeed.
    ///
    /// DynamoDB accepts a single condition expression per request, so this
    /// method can only be called once. If the condition fails at runtime,
    /// DynamoDB returns a `ConditionalCheckFailedException`.
    ///
    /// For the common item exists case, prefer the
    /// [`.exists()`][DeleteItemRequest::exists] shorthands.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Condition};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // Delete only if the enrollment has not been completed
    /// sample_enrollment()
    ///     .delete(client)
    ///     .condition(Enrollment::exists() & Condition::not_exists("completed_at"))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn condition(
        mut self,
        condition: Condition<'_>,
    ) -> DeleteItemRequest<TD, T, O, R, AlreadyHasCondition> {
        tracing::debug!(%condition, "DeleteItem condition");
        self.builder = condition.apply(self.builder);
        DeleteItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}
impl<TD: TableDefinition, T: DynamoDBItem<TD>, O: OutputFormat, R: ReturnValue>
    DeleteItemRequest<TD, T, O, R, NoCondition>
{
    /// Adds an `attribute_exists(<PK>)` condition, requiring the item to exist before deletion.
    ///
    /// The delete fails with `ConditionalCheckFailedException` if the item does not exist.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// sample_enrollment().delete(client).exists().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn exists(self) -> DeleteItemRequest<TD, T, O, R, AlreadyHasCondition> {
        self.condition(T::exists())
    }
}

// -- Output format transition (preserve R, C) -------------------------------

impl<TD: TableDefinition, T, R: ReturnValue, C: ConditionState>
    DeleteItemRequest<TD, T, Typed, R, C>
{
    /// Switches the output format from `Typed` to `Raw`.
    ///
    /// After calling `.raw()`, [`execute`][DeleteItemRequest::execute] returns
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
    /// let old_raw /* : Option<Item<PlatformTable>> */ =
    ///     sample_enrollment()
    ///         .delete(client)
    ///         .return_old()
    ///         .raw()
    ///         .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn raw(self) -> DeleteItemRequest<TD, T, Raw, R, C> {
        DeleteItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// -- Terminal: ReturnNothing (any O, any C) ---------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, C: ConditionState>
    DeleteItemRequest<TD, T, O, ReturnNothing, C>
{
    /// Sends the `DeleteItem` request, returning nothing on success.
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
    /// sample_enrollment().delete(client).exists().execute().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "delete_execute")]
    pub fn execute(self) -> impl Future<Output = Result<()>> + Send + 'static {
        let builder = self.builder;
        async move {
            builder.return_values(SDKReturnValue::None).send().await?;
            Ok(())
        }
    }
}

impl<TD: TableDefinition, T, O: OutputFormat, C: ConditionState> IntoFuture
    for DeleteItemRequest<TD, T, O, ReturnNothing, C>
{
    type Output = Result<()>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

// -- Terminal: ReturnItem<Old> + Typed (any C) ------------------------------

impl<TD: TableDefinition, T: DynamoDBItem<TD> + DeserializeOwned, C: ConditionState>
    DeleteItemRequest<TD, T, Typed, Return<Old>, C>
{
    /// Sends the `DeleteItem` request and returns the deleted item deserialized as `T`.
    ///
    /// Returns `Ok(None)` if no item existed at the key.
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
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let old /* : Option<Enrollment> */ = Enrollment::delete_by_id(
    ///     client,
    ///     KeyId::pk("user-1").sk("course-42"),
    /// )
    /// .execute()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "delete_execute_old")]
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
    for DeleteItemRequest<TD, T, Typed, Return<Old>, C>
{
    type Output = Result<Option<T>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

// -- Terminal: ReturnItem<Old> + Raw (any C) --------------------------------

impl<TD: TableDefinition, T, C: ConditionState> DeleteItemRequest<TD, T, Raw, Return<Old>, C> {
    /// Sends the `DeleteItem` request and returns the deleted raw item map.
    ///
    /// Returns `Ok(None)` if no item existed at the key.
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
    /// let old_raw = sample_enrollment()
    ///     .delete(client)
    ///     .return_old()
    ///     .raw()
    ///     .execute()
    ///     .await?;
    /// // old_raw: Option<Item<PlatformTable>>
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "delete_execute_old_raw")]
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
    for DeleteItemRequest<TD, T, Raw, Return<Old>, C>
{
    type Output = Result<Option<Item<TD>>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}
