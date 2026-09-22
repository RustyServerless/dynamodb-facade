use super::*;

use aws_sdk_dynamodb::operation::scan::builders::ScanFluentBuilder;
use futures_core::Stream;
use futures_util::StreamExt;

/// Builder for a DynamoDB `Scan` request.
///
/// Constructed via [`DynamoDBItemOp::scan`] / [`DynamoDBItemOp::scan_index`]
/// (typed, with a concrete `T`) or [`ScanRequest::new`] /
/// [`ScanRequest::index_new`] / [`ScanRequest::with_client`] /
/// [`ScanRequest::index_with_client`] (stand-alone, raw output).
///
/// The builder provides:
///
/// - **Output format** — the result can be deserialized into `T`.
///   Call [`.raw()`][ScanRequest::raw] to receive untyped [`Item<TD>`]
///   values instead (one-way). Calling [`.project()`][ScanRequest::project]
///   also forces raw output.
/// - **Filter** — call [`.filter()`][ScanRequest::filter] to add a
///   server-side filter expression. DynamoDB accepts a single filter
///   expression per request, so this can only be called once.
/// - **Projection** — call [`.project()`][ScanRequest::project] to limit
///   which attributes are returned. This can only be called once.
///
/// Use [`.all()`][ScanRequest::all] to collect all pages into a `Vec`, or
/// [`.stream()`][ScanRequest::stream] for lazy page-by-page iteration.
///
/// **Note:** Scans read every item in the table (or index) and are
/// significantly more expensive than queries. Prefer [`QueryRequest`] when
/// possible.
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
/// use dynamodb_facade::{DynamoDBItemOp, Condition};
///
/// # async fn example() -> dynamodb_facade::Result<()> {
/// // Simple scan
/// let all_users /* : Vec<User> */ = User::scan().all().await?;
///
/// // Scan with a filter
/// let instructors /* : Vec<User> */ = User::scan()
///     .filter(Condition::eq("role", "instructor"))
///     .all()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[must_use = "builder does nothing until executed via .all() or .stream()"]
pub struct ScanRequest<
    TD: TableDefinition,
    T = (),
    O: OutputFormat = Raw,
    F: FilterState = NoFilter,
    P: ProjectionState = NoProjection,
> {
    builder: ScanFluentBuilder,
    _marker: PhantomData<(TD, T, O, F, P)>,
}

// -- Stand-alone constructors (T = (), O = Raw)

#[allow(clippy::new_without_default)]
impl<TD: TableDefinition> ScanRequest<TD> {
    /// Creates a stand-alone `ScanRequest` against the full table with raw output using
    /// the globally defined [`aws_sdk_dynamodb::Client`].
    ///
    /// Output is raw (`T = ()`, `O = Raw`). For typed access, prefer
    /// [`DynamoDBItemOp::scan`] instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::ScanRequest;
    ///
    /// # async fn example() -> dynamodb_facade::Result<()> {
    /// let items = ScanRequest::<PlatformTable>::new().all().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> Self {
        Self::_new(global_client())
    }

    /// Creates a stand-alone `ScanRequest` against the full table with raw output using
    /// the provided [`aws_sdk_dynamodb::Client`].
    ///
    /// Output is raw (`T = ()`, `O = Raw`). For typed access, prefer
    /// [`explicit_client::DynamoDBItemOp::scan`] instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::ScanRequest;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let items = ScanRequest::<PlatformTable>::with_client(client).all().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_client(client: aws_sdk_dynamodb::Client) -> Self {
        Self::_new(client)
    }

    /// Creates a stand-alone `ScanRequest` scoped to a secondary index using
    /// the globally defined [`aws_sdk_dynamodb::Client`].
    ///
    /// Output is raw (`T = ()`, `O = Raw`). For typed access, prefer
    /// [`DynamoDBItemOp::scan_index`] instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::ScanRequest;
    ///
    /// # async fn example() -> dynamodb_facade::Result<()> {
    /// let items = ScanRequest::<PlatformTable>::index_new::<TypeIndex>()
    ///     .all()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn index_new<I: IndexDefinition<TD>>() -> Self {
        Self::_index_new::<I>(global_client())
    }

    /// Creates a stand-alone `ScanRequest` scoped to a secondary index using
    /// the provided [`aws_sdk_dynamodb::Client`].
    ///
    /// Output is raw (`T = ()`, `O = Raw`). For typed access, prefer
    /// [`explicit_client::DynamoDBItemOp::scan_index`] instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::ScanRequest;
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let items = ScanRequest::<PlatformTable>::index_with_client::<TypeIndex>(client)
    ///     .all()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn index_with_client<I: IndexDefinition<TD>>(client: aws_sdk_dynamodb::Client) -> Self {
        Self::_index_new::<I>(client)
    }
}

// -- Common methods (all states) --------------------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, F: FilterState, P: ProjectionState>
    ScanRequest<TD, T, O, F, P>
{
    pub(super) fn _new(client: aws_sdk_dynamodb::Client) -> Self {
        let table_name = TD::table_name();
        tracing::debug!(table_name, "Scan");
        Self {
            builder: client.scan().table_name(table_name),
            _marker: PhantomData,
        }
    }

    pub(super) fn _index_new<I: IndexDefinition<TD>>(client: aws_sdk_dynamodb::Client) -> Self {
        let table_name = TD::table_name();
        let index_name = I::index_name();
        tracing::debug!(table_name, index_name, "Scan (index)");
        Self {
            builder: client.scan().table_name(table_name).index_name(index_name),
            _marker: PhantomData,
        }
    }

    /// Enables strongly consistent reads for this scan.
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
    /// # async fn example() -> dynamodb_facade::Result<()> {
    /// let users /* : Vec<User> */ = User::scan().consistent_read().all().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn consistent_read(mut self) -> Self {
        tracing::debug!("Scan consistent_read");
        self.builder = self.builder.consistent_read(true);
        self
    }

    /// Sets the maximum number of items to evaluate per page.
    ///
    /// Note that DynamoDB evaluates up to `limit` items before applying any
    /// filter expression, so the number of items returned may be less than
    /// `limit` when a filter is active. Pagination continues automatically
    /// when using [`.all()`][ScanRequest::all] or
    /// [`.stream()`][ScanRequest::stream].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example() -> dynamodb_facade::Result<()> {
    /// let users /* : Vec<User> */ = User::scan().limit(100).all().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn limit(mut self, limit: i32) -> Self {
        tracing::debug!(limit, "Scan limit");
        self.builder = self.builder.limit(limit);
        self
    }

    /// Consumes the builder and returns the underlying SDK [`ScanFluentBuilder`].
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
    /// # async fn example() -> dynamodb_facade::Result<()> {
    /// let sdk_builder = User::scan().into_inner();
    /// // configure sdk_builder further, then call .send().await
    /// # Ok(())
    /// # }
    /// ```
    pub fn into_inner(self) -> ScanFluentBuilder {
        self.builder
    }
}

// -- Filter (NoFilter only) -------------------------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, P: ProjectionState>
    ScanRequest<TD, T, O, NoFilter, P>
{
    /// Adds a filter expression applied after items are read from the table.
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
    /// # async fn example() -> dynamodb_facade::Result<()> {
    /// let instructors /* : Vec<User> */ = User::scan()
    ///     .filter(Condition::eq("role", "instructor"))
    ///     .all()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn filter(mut self, filter: Condition<'_>) -> ScanRequest<TD, T, O, AlreadyHasFilter, P> {
        tracing::debug!(%filter, "Scan filter");
        self.builder = filter.apply_filter(self.builder);
        ScanRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// -- Projection (NoProjection only) -----------------------------------------

impl<TD: TableDefinition, T, O: OutputFormat, F: FilterState>
    ScanRequest<TD, T, O, F, NoProjection>
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
    /// # async fn example() -> dynamodb_facade::Result<()> {
    /// // Fetch only the "name" attribute for each user
    /// let partial = User::scan()
    ///     .project(Projection::new(["name"]))
    ///     .all()
    ///     .await?;
    /// // partial: Vec<Item<PlatformTable>>
    /// // with only "PK", "SK" and "name"
    /// # Ok(())
    /// # }
    /// ```
    pub fn project(
        mut self,
        projection: Projection<'_, TD>,
    ) -> ScanRequest<TD, T, Raw, F, AlreadyHasProjection> {
        tracing::debug!(%projection, "Scan project");
        self.builder = projection.apply_projection(self.builder);
        ScanRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

// -- Output format transition (preserve F, P) -------------------------------

impl<TD: TableDefinition, T, F: FilterState, P: ProjectionState> ScanRequest<TD, T, Typed, F, P> {
    /// Switches the output format from `Typed` to `Raw`.
    ///
    /// After calling `.raw()`, [`.all()`][ScanRequest::all] returns
    /// `Vec<Item<TD>>` and [`.stream()`][ScanRequest::stream] yields
    /// `Result<Vec<Item<TD>>>` (pages of raw items) instead of the typed
    /// equivalents. This transition is one-way.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example() -> dynamodb_facade::Result<()> {
    /// let raw_items = User::scan().raw().all().await?;
    /// // raw_items: Vec<Item<PlatformTable>>
    /// # Ok(())
    /// # }
    /// ```
    pub fn raw(self) -> ScanRequest<TD, T, Raw, F, P> {
        ScanRequest {
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
> ScanRequest<TD, T, Typed, F, P>
{
    /// Executes the scan, collecting all pages and returning items deserialized as `T`.
    ///
    /// Automatically follows pagination tokens until all matching items have
    /// been retrieved. For large tables, prefer
    /// [`.stream()`][ScanRequest::stream] to avoid loading everything into
    /// memory at once.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if any DynamoDB page request fails or if deserialization
    /// of any item fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// # async fn example() -> dynamodb_facade::Result<()> {
    /// let all_users /* : Vec<User> */ = User::scan().all().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "scan_all")]
    pub async fn all(self) -> Result<Vec<T>> {
        dynamodb_execute_scan(self.builder)
            .await?
            .into_iter()
            .map(T::try_from_item)
            .collect()
    }

    /// Executes the scan as a lazy async stream, yielding one page at a time.
    ///
    /// Each element yielded by the stream is a `Vec<T>` representing one page
    /// of results deserialized as `T`. Pages are fetched on demand as the
    /// stream is consumed. Use this for large tables where loading everything
    /// into memory at once is undesirable.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    /// use futures_util::StreamExt;
    /// use std::pin::pin;
    ///
    /// # async fn example() -> dynamodb_facade::Result<()> {
    /// let stream = User::scan().stream();
    /// // Must pin the stream
    /// let mut stream = pin!(stream);
    ///
    /// while let Some(result) = stream.next().await {
    ///     let page /* : Vec<User> */ = result?;
    ///     for user in page {
    ///         let _ = user;
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn stream(self) -> impl Stream<Item = Result<Vec<T>>> {
        dynamodb_stream_scan::<TD>(self.builder).map(|result| {
            result.and_then(|items| items.into_iter().map(T::try_from_item).collect())
        })
    }
}

// -- Terminal: Raw (any F, any P) -------------------------------------------

impl<TD: TableDefinition, T, F: FilterState, P: ProjectionState> ScanRequest<TD, T, Raw, F, P> {
    /// Executes the scan, collecting all pages and returning raw item maps.
    ///
    /// Automatically follows pagination tokens until all items have been
    /// retrieved.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if any DynamoDB page request fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::ScanRequest;
    ///
    /// # async fn example() -> dynamodb_facade::Result<()> {
    /// let items = ScanRequest::<PlatformTable>::new().all().await?;
    /// // items: Vec<Item<PlatformTable>>
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", skip(self), name = "scan_all_raw")]
    pub async fn all(self) -> Result<Vec<Item<TD>>> {
        dynamodb_execute_scan(self.builder).await
    }

    /// Executes the scan as a lazy async stream, yielding one page of raw item maps at a time.
    ///
    /// Each element yielded by the stream is a `Vec<Item<TD>>` representing one
    /// page of results. Pages are fetched on demand as the stream is consumed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::ScanRequest;
    /// use futures_util::StreamExt;
    /// use std::pin::pin;
    ///
    /// # async fn example() -> dynamodb_facade::Result<()> {
    /// let stream = ScanRequest::<PlatformTable>::new().stream();
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
        dynamodb_stream_scan(self.builder)
    }
}
