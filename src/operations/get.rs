use std::future::{Future, IntoFuture};
use std::pin::Pin;

use super::*;

use aws_sdk_dynamodb::operation::get_item::builders::GetItemFluentBuilder;

/// Builder for a DynamoDB `GetItem` request.
///
/// Constructed via [`DynamoDBItemOp::get`] (typed, with a concrete `T`) or
/// [`GetItemRequest::new`] (stand-alone, raw output). The builder provides:
///
/// - **Output format** — the result can be deserialized into `T`.
///   Call [`.raw()`][GetItemRequest::raw] to receive an untyped [`Item<TD>`]
///   instead (one-way).
/// - **Projection** — call [`.project()`][GetItemRequest::project] to limit
///   which attributes are returned. This can only be called once and
///   automatically switches to raw output, since the projected result may
///   not contain all fields required for deserialization.
///
/// The builder implements [`IntoFuture`], so it can
/// be `.await`ed directly without calling `.execute()` explicitly.
///
/// # Errors
///
/// Returns [`Err`] if the DynamoDB request fails or if deserialization of
/// the returned attributes fails.
///
/// # Examples
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{DynamoDBItemOp, KeyId};
///
/// # async fn example(cclient: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
/// # let client = cclient.clone();
/// // Simple get
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
#[must_use = "builder does nothing until awaited or executed"]
pub struct GetItemRequest<
    TD: TableDefinition,
    T = (),
    O: OutputFormat = Raw,
    P: ProjectionState = NoProjection,
> {
    builder: GetItemFluentBuilder,
    _marker: PhantomData<(TD, T, O, P)>,
}

// -- Stand-alone constructor (T = (), O = Raw)

impl<TD: TableDefinition> GetItemRequest<TD> {
    /// Creates a stand-alone `GetItemRequest` with raw output (`T = ()`, `O = Raw`).
    ///
    /// Use this when you do not have a concrete item type and want to work with
    /// the raw [`Item<TD>`] map directly. For typed access, prefer
    /// [`DynamoDBItemOp::get`] instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{GetItemRequest, Key};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// # let key: Key<PlatformTable> = sample_user_item().into_key_only();
    /// let raw_item = GetItemRequest::<PlatformTable>::new(client, key).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: aws_sdk_dynamodb::Client, key: Key<TD>) -> Self {
        Self::_new(client, key)
    }
}

// -- Common methods (all states) --------------------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, P: ProjectionState> GetItemRequest<TD, T, O, P> {
    /// Creates a new `GetItemRequest` targeting the given key.
    pub(super) fn _new(client: aws_sdk_dynamodb::Client, key: Key<TD>) -> Self {
        let table_name = TD::table_name();
        tracing::debug!(table_name, ?key, "GetItem");
        Self {
            builder: client
                .get_item()
                .table_name(table_name)
                .set_key(Some(key.into_inner())),
            _marker: PhantomData,
        }
    }

    /// Enables strongly consistent reads for this request.
    ///
    /// By default DynamoDB uses eventually consistent reads. Enabling consistent
    /// reads guarantees the most up-to-date data but consumes twice the read
    /// capacity units and is not supported on Global Secondary Indexes.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let user /* : Option<User> */ = User::get(client, KeyId::pk("user-1"))
    ///     .consistent_read()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn consistent_read(mut self) -> Self {
        tracing::debug!("GetItem consistent_read");
        self.builder = self.builder.consistent_read(true);
        self
    }

    /// Consumes the builder and returns the underlying SDK
    /// [`GetItemFluentBuilder`].
    ///
    /// Use this escape hatch when you need to set options not exposed by this
    /// facade, such as `expression_attribute_names` for a manual projection, or
    /// when integrating with code that expects the raw SDK builder.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let sdk_builder = User::get(client, KeyId::pk("user-1")).into_inner();
    /// // configure sdk_builder further, then call .send().await
    /// # Ok(())
    /// # }
    /// ```
    pub fn into_inner(self) -> GetItemFluentBuilder {
        self.builder
    }
}

// -- Projection (NoProjection only) -----------------------------------------

impl<TD: TableDefinition, T, O: OutputFormat> GetItemRequest<TD, T, O, NoProjection> {
    /// Applies a projection expression, limiting the attributes returned.
    ///
    /// This method can only be called once. It forces the output to raw
    /// [`Item<TD>`] because projected results may not contain all fields
    /// required for deserialization into `T`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{AttributeDefinition, DynamoDBItemOp, KeyId, Projection};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // Fetch only the "name" and "email" attributes
    /// let partial /* : Option<Item<PlatformTable>> */ =
    ///     User::get(client, KeyId::pk("user-1"))
    ///         .project(Projection::new(["name", Email::NAME]))
    ///         .await?;
    /// // partial: contains only "PK", "SK", "name" and "email"
    /// # Ok(())
    /// # }
    /// ```
    pub fn project(
        mut self,
        projection: Projection<'_, TD>,
    ) -> GetItemRequest<TD, T, Raw, AlreadyHasProjection> {
        tracing::debug!(%projection, "GetItem project");
        self.builder = projection.apply_projection(self.builder);
        GetItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// -- Output format transition (preserve P) ----------------------------------

impl<TD: TableDefinition, T, P: ProjectionState> GetItemRequest<TD, T, Typed, P> {
    /// Switches the output format from `Typed` to `Raw`.
    ///
    /// After calling `.raw()`, [`execute`][GetItemRequest::execute] returns
    /// `Option<Item<TD>>` instead of `Option<T>`. This transition is one-way —
    /// you cannot switch back to `Typed`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let raw_item = User::get(client, KeyId::pk("user-1"))
    ///     .raw()
    ///     .await?;
    /// // raw_item: Option<Item<PlatformTable>>
    /// if let Some(item) = raw_item {
    ///     let name = item.get("name");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn raw(self) -> GetItemRequest<TD, T, Raw, P> {
        GetItemRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// -- Terminal: Typed (any P) ------------------------------------------------

impl<TD: TableDefinition, T: DynamoDBItem<TD> + DeserializeOwned, P: ProjectionState>
    GetItemRequest<TD, T, Typed, P>
{
    /// Sends the `GetItem` request and returns the item deserialized as `T`.
    ///
    /// Returns `Ok(None)` if no item exists for the given key.
    ///
    /// This method is also available implicitly via `.await` because
    /// [`GetItemRequest`] implements [`IntoFuture`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the DynamoDB request fails or if deserialization of
    /// the returned attributes fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let user /* : Option<User> */ = User::get(client, KeyId::pk("user-1"))
    ///     .consistent_read()
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "get_execute")]
    pub fn execute(self) -> impl Future<Output = Result<Option<T>>> + Send + 'static {
        let builder = self.builder;
        async move {
            builder
                .send()
                .await?
                .item
                .map(Item::from_dynamodb_response)
                .map(T::try_from_item)
                .transpose()
        }
    }
}

impl<TD: TableDefinition, T: DynamoDBItem<TD> + DeserializeOwned, P: ProjectionState> IntoFuture
    for GetItemRequest<TD, T, Typed, P>
{
    type Output = Result<Option<T>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

// -- Terminal: Raw (any P) --------------------------------------------------

impl<TD: TableDefinition, T, P: ProjectionState> GetItemRequest<TD, T, Raw, P> {
    /// Sends the `GetItem` request and returns the raw item map.
    ///
    /// Returns `Ok(None)` if no item exists for the given key.
    ///
    /// This method is also available implicitly via `.await` because
    /// [`GetItemRequest`] implements [`IntoFuture`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the DynamoDB request fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, KeyId};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let raw = User::get(client, KeyId::pk("user-1"))
    ///     .raw()
    ///     .execute()
    ///     .await?;
    /// // raw: Option<Item<PlatformTable>>
    /// if let Some(item) = raw {
    ///     let name = item.get("name");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "get_execute_raw")]
    pub fn execute(self) -> impl Future<Output = Result<Option<Item<TD>>>> + Send + 'static {
        let builder = self.builder;
        async move { Ok(builder.send().await?.item.map(Item::from_dynamodb_response)) }
    }
}

impl<TD: TableDefinition, T, P: ProjectionState> IntoFuture for GetItemRequest<TD, T, Raw, P> {
    type Output = Result<Option<Item<TD>>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}
