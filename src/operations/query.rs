use super::*;

use aws_sdk_dynamodb::operation::query::builders::QueryFluentBuilder;

/// Builder for a DynamoDB `Query` request.
///
/// Constructed via [`DynamoDBItemOp::query`] / [`DynamoDBItemOp::query_index`]
/// (typed, with a concrete `T`) or [`QueryRequest::new`] /
/// [`QueryRequest::new_index`] (stand-alone, raw output). The builder provides:
///
/// - **Output format** — the result can be deserialized into `T`.
///   Call [`.raw()`][QueryRequest::raw] to receive untyped [`Item<TD>`]
///   values instead (one-way). Calling [`.project()`][QueryRequest::project]
///   also forces raw output.
/// - **Filter** — call [`.filter()`][QueryRequest::filter] to add a
///   server-side filter expression. DynamoDB accepts a single filter
///   expression per request, so this can only be called once.
/// - **Projection** — call [`.project()`][QueryRequest::project] to limit
///   which attributes are returned. This can only be called once.
///
/// Use [`.all()`][QueryRequest::all] to collect all pages into a `Vec`, or
/// [`.stream()`][QueryRequest::stream] for lazy page-by-page iteration.
///
/// # Errors
///
/// Returns [`Err`] if any DynamoDB page request fails or if deserialization
/// of any returned item fails.
///
/// # Examples
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{DynamoDBItemOp, Condition, KeyCondition};
///
/// # async fn example(cclient: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
/// # let client = cclient.clone();
/// // Simple query
/// let enrollments /* : Vec<Enrollment> */ =
///     Enrollment::query(client, Enrollment::key_condition("user-1"))
///         .all()
///         .await?;
///
/// # let client = cclient.clone();
/// // Query with a filter
/// let advanced /* : Vec<Enrollment> */ =
///     Enrollment::query(client, Enrollment::key_condition("user-1"))
///         .filter(Condition::gt("progress", 0.5))
///         .all()
///         .await?;
///
/// # let client = cclient.clone();
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
#[must_use = "builder does nothing until executed via .all() or .stream()"]
pub struct QueryRequest<
    TD: TableDefinition,
    T = (),
    O: OutputFormat = Raw,
    F: FilterState = NoFilter,
    P: ProjectionState = NoProjection,
> {
    builder: QueryFluentBuilder,
    _marker: PhantomData<(TD, T, O, F, P)>,
}

// -- Stand-alone constructors (T = (), O = Raw)

impl<TD: TableDefinition> QueryRequest<TD> {
    /// Creates a stand-alone `QueryRequest` against the table's primary key schema.
    ///
    /// Output is raw (`T = ()`, `O = Raw`). For typed access, prefer
    /// [`DynamoDBItemOp::query`] instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{QueryRequest, KeyCondition};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let items = QueryRequest::<PlatformTable>::new(
    ///     client,
    ///     KeyCondition::pk("USER#user-1".to_owned()),
    /// )
    /// .all()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        client: aws_sdk_dynamodb::Client,
        key_condition: KeyCondition<'_, TD::KeySchema, impl KeyConditionState>,
    ) -> Self {
        Self::_new(client, key_condition)
    }

    /// Creates a stand-alone `QueryRequest` against a secondary index.
    ///
    /// Output is raw (`T = ()`, `O = Raw`). For typed access, prefer
    /// [`DynamoDBItemOp::query_index`] instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{QueryRequest, KeyCondition};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let items = QueryRequest::<PlatformTable>::new_index::<EmailIndex>(
    ///     client,
    ///     KeyCondition::pk("alice@example.com".to_owned()),
    /// )
    /// .all()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_index<I: IndexDefinition<TD>>(
        client: aws_sdk_dynamodb::Client,
        key_condition: KeyCondition<'_, I::KeySchema, impl KeyConditionState>,
    ) -> Self {
        Self::_new_index::<I>(client, key_condition)
    }
}

// -- Common methods (all states) --------------------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, F: FilterState, P: ProjectionState>
    QueryRequest<TD, T, O, F, P>
{
    pub(super) fn _new(
        client: aws_sdk_dynamodb::Client,
        key_condition: KeyCondition<'_, TD::KeySchema, impl KeyConditionState>,
    ) -> Self {
        let table_name = TD::table_name();
        tracing::debug!(table_name, %key_condition, "Query");
        Self {
            builder: key_condition.apply_key_condition(client.query().table_name(table_name)),
            _marker: PhantomData,
        }
    }

    pub(super) fn _new_index<I: IndexDefinition<TD>>(
        client: aws_sdk_dynamodb::Client,
        key_condition: KeyCondition<'_, I::KeySchema, impl KeyConditionState>,
    ) -> Self {
        let table_name = TD::table_name();
        let index_name = I::index_name();
        tracing::debug!(table_name, index_name, %key_condition, "Query (index)");
        Self {
            builder: key_condition
                .apply_key_condition(client.query().table_name(table_name).index_name(index_name)),
            _marker: PhantomData,
        }
    }

    /// Enables strongly consistent reads for this query.
    ///
    /// By default DynamoDB uses eventually consistent reads. Enabling consistent
    /// reads guarantees the most up-to-date data but consumes twice the read
    /// capacity units and is not supported on Global Secondary Indexes.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let enrollments /* : Vec<Enrollment> */ =
    ///     Enrollment::query(client, Enrollment::key_condition("user-1"))
    ///         .consistent_read()
    ///         .all()
    ///         .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn consistent_read(mut self) -> Self {
        tracing::debug!("Query consistent_read");
        self.builder = self.builder.consistent_read(true);
        self
    }

    /// Sets the maximum number of items to evaluate per page.
    ///
    /// Note that DynamoDB evaluates up to `limit` items before applying any
    /// filter expression, so the number of items returned may be less than
    /// `limit` when a filter is active. Pagination continues automatically
    /// when using [`.all()`][QueryRequest::all] or
    /// [`.stream()`][QueryRequest::stream].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // Evaluate at most 10 items per page
    /// let enrollments /* : Vec<Enrollment> */ =
    ///     Enrollment::query(client, Enrollment::key_condition("user-1"))
    ///         .limit(10)
    ///         .all()
    ///         .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn limit(mut self, limit: i32) -> Self {
        tracing::debug!(limit, "Query limit");
        self.builder = self.builder.limit(limit);
        self
    }

    /// Change the sort order of results by sort key by setting
    /// `scan_index_forward = false`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // Return enrollments in reverse sort-key order
    /// let enrollments /* : Vec<Enrollment> */ =
    ///     Enrollment::query(client, Enrollment::key_condition("user-1"))
    ///         .reverse()
    ///         .all()
    ///         .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn reverse(mut self) -> Self {
        tracing::debug!("Query scan_index_forward = false");
        self.builder = self.builder.scan_index_forward(false);
        self
    }

    /// Consumes the builder and returns the underlying SDK [`QueryFluentBuilder`].
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
    /// let sdk_builder =
    ///     Enrollment::query(client, Enrollment::key_condition("user-1")).into_inner();
    /// // configure sdk_builder further, then call .send().await
    /// # Ok(())
    /// # }
    /// ```
    pub fn into_inner(self) -> QueryFluentBuilder {
        self.builder
    }
}

// -- Filter (NoFilter only) -------------------------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, P: ProjectionState>
    QueryRequest<TD, T, O, NoFilter, P>
{
    /// Adds a filter expression applied after the key condition.
    ///
    /// DynamoDB accepts a single filter expression per request, so this method
    /// can only be called once. The filter is evaluated server-side after items
    /// are read but before they are returned, so it does not reduce read
    /// capacity consumption.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Condition};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // Query enrollments with progress above 50%
    /// let advanced /* : Vec<Enrollment> */ =
    ///     Enrollment::query(client, Enrollment::key_condition("user-1"))
    ///         .filter(Condition::gt("progress", 0.5))
    ///         .all()
    ///         .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn filter(mut self, filter: Condition<'_>) -> QueryRequest<TD, T, O, AlreadyHasFilter, P> {
        tracing::debug!(%filter, "Query filter");
        self.builder = filter.apply_filter(self.builder);
        QueryRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// -- Projection (NoProjection only) -----------------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, F: FilterState>
    QueryRequest<TD, T, O, F, NoProjection>
{
    /// Applies a projection expression, limiting the attributes returned per item.
    ///
    /// This method can only be called once. It forces the output to raw
    /// [`Item<TD>`] because projected results may not contain all fields
    /// required for deserialization into `T`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, Projection};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // Fetch only the "progress" attribute for each enrollment
    /// let partial /* : Vec<Item<PlatformTable>> */ =
    ///     Enrollment::query(client, Enrollment::key_condition("user-1"))
    ///         .project(Projection::new(["progress"]))
    ///         .all()
    ///         .await?;
    /// // partial: contains only "PK", "SK" and "progress"
    /// # Ok(())
    /// # }
    /// ```
    pub fn project(
        mut self,
        projection: Projection<'_, TD>,
    ) -> QueryRequest<TD, T, Raw, F, AlreadyHasProjection> {
        tracing::debug!(%projection, "Query project");
        self.builder = projection.apply_projection(self.builder);
        QueryRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// -- Output format transition (preserve F, P) -------------------------------

impl<TD: TableDefinition, T, F: FilterState, P: ProjectionState> QueryRequest<TD, T, Typed, F, P> {
    /// Switches the output format from `Typed` to `Raw`.
    ///
    /// After calling `.raw()`, [`.all()`][QueryRequest::all] returns
    /// `Vec<Item<TD>>` and [`.stream()`][QueryRequest::stream] yields
    /// `Result<Vec<Item<TD>>>` (pages of raw items) instead of the typed
    /// equivalents. This transition is one-way.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let raw_items /* : Vec<Item<PlatformTable>> */ =
    ///     Enrollment::query(client, Enrollment::key_condition("user-1"))
    ///         .raw()
    ///         .all()
    ///         .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn raw(self) -> QueryRequest<TD, T, Raw, F, P> {
        QueryRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// -- Terminal: Typed (any F, any P) -----------------------------------------

impl<
    TD: TableDefinition,
    T: DynamoDBItem<TD> + DeserializeOwned,
    F: FilterState,
    P: ProjectionState,
> QueryRequest<TD, T, Typed, F, P>
{
    /// Executes the query, collecting all pages and returning items deserialized as `T`.
    ///
    /// Automatically follows pagination tokens until all matching items have
    /// been retrieved. For large result sets, prefer
    /// [`.stream()`][QueryRequest::stream] to avoid loading everything into
    /// memory at once.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if any DynamoDB page request fails or if deserialization
    /// of any item fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let enrollments /* : Vec<Enrollment> */ =
    ///     Enrollment::query(client, Enrollment::key_condition("user-1"))
    ///         .all()
    ///         .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "query_all")]
    pub async fn all(self) -> Result<Vec<T>> {
        dynamodb_execute_query(self.builder)
            .await?
            .into_iter()
            .map(T::try_from_item)
            .collect()
    }

    /// Executes the query as a lazy async stream, yielding one page at a time.
    ///
    /// Each element yielded by the stream is a `Vec<T>` representing one page
    /// of results deserialized as `T`. Pages are fetched on demand as the
    /// stream is consumed. Use this for large result sets where loading
    /// everything into memory at once is undesirable.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    /// use futures_util::StreamExt;
    /// use std::pin::pin;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let stream = Enrollment::query(client, Enrollment::key_condition("user-1"))
    ///     .stream();
    /// // Must pin the stream
    /// let mut stream = pin!(stream);
    ///
    /// while let Some(result) = stream.next().await {
    ///     let page /* : Vec<Enrollment> */ = result?;
    ///     for enrollment in page {
    ///         let _ = enrollment;
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn stream(self) -> impl Stream<Item = Result<Vec<T>>> {
        dynamodb_stream_query::<TD>(self.builder).map(|result| {
            result.and_then(|items| items.into_iter().map(T::try_from_item).collect())
        })
    }
}

// -- Terminal: Raw (any F, any P) -------------------------------------------

impl<TD: TableDefinition, T, F: FilterState, P: ProjectionState> QueryRequest<TD, T, Raw, F, P> {
    /// Executes the query, collecting all pages and returning raw item maps.
    ///
    /// Automatically follows pagination tokens until all matching items have
    /// been retrieved.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if any DynamoDB page request fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, QueryRequest, KeyCondition};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let raw_items = QueryRequest::<PlatformTable>::new(
    ///     client,
    ///     KeyCondition::pk("USER#user-1".to_owned()),
    /// )
    /// .all()
    /// .await?;
    /// // raw_items: Vec<Item<PlatformTable>>
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "query_all_raw")]
    pub async fn all(self) -> Result<Vec<Item<TD>>> {
        dynamodb_execute_query(self.builder).await
    }

    /// Executes the query as a lazy async stream, yielding one page of raw item maps at a time.
    ///
    /// Each element yielded by the stream is a `Vec<Item<TD>>` representing one
    /// page of results. Pages are fetched on demand as the stream is consumed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{QueryRequest, KeyCondition};
    /// use futures_util::StreamExt;
    /// use std::pin::pin;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let stream = QueryRequest::<PlatformTable>::new(
    ///     client,
    ///     KeyCondition::pk("USER#user-1".to_owned()),
    /// )
    /// .stream();
    /// // Must pin the stream
    /// let mut stream = pin!(stream);
    ///
    /// while let Some(result) = stream.next().await {
    ///     let page /* : Vec<Item<PlatformTable>> */ = result?;
    ///     for item in page {
    ///         let _ = item;
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn stream(self) -> impl Stream<Item = Result<Vec<Item<TD>>>> {
        dynamodb_stream_query(self.builder)
    }
}
