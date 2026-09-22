use super::*;

use aws_sdk_dynamodb::types::{
    WriteRequest,
    builders::{DeleteRequestBuilder, PutRequestBuilder},
};

/// Entry points for building DynamoDB `BatchWriteItem` write requests.
///
/// This trait is **blanket-implemented** for every type that implements
/// [`DynamoDBItemOp<TD>`]. You never implement it manually.
///
/// Each method returns a [`WriteRequest`] that can be collected into a `Vec`
/// and passed to [`dynamodb_batch_write`] for execution. Batch writes are
/// limited to put and delete operations — batch updates are not supported
/// by the DynamoDB API.
///
/// # Examples
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{DynamoDBItemBatchOp, dynamodb_batch_write, KeyId};
///
/// # async fn example(
/// #     enrollments: Vec<Enrollment>,
/// # ) -> dynamodb_facade::Result<()> {
/// // Batch put a collection of enrollments
/// let requests: Vec<_> = enrollments.iter().map(|e| e.batch_put()).collect();
/// dynamodb_batch_write::<PlatformTable>(requests).await?;
/// # Ok(())
/// # }
/// ```
pub trait DynamoDBItemBatchOp<TD: TableDefinition>: DynamoDBItemOp<TD> {
    /// Creates a `PutRequest` [`WriteRequest`] for this item.
    ///
    /// Serializes `self` into a DynamoDB item map and wraps it in a
    /// `WriteRequest::PutRequest`. Pass the result to
    /// [`dynamodb_batch_write`] for execution.
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
    /// use dynamodb_facade::{DynamoDBItemBatchOp, dynamodb_batch_write};
    ///
    /// # async fn example(
    /// #     enrollments: Vec<Enrollment>,
    /// # ) -> dynamodb_facade::Result<()> {
    /// // enrollments: Vec<Enrollment>
    /// let requests: Vec<_> = enrollments.iter().map(|e| e.batch_put()).collect();
    /// dynamodb_batch_write::<PlatformTable>(requests).await?;
    /// # Ok(())
    /// # }
    /// ```
    fn batch_put(&self) -> WriteRequest
    where
        Self: Serialize,
    {
        batch_put(self.to_item())
    }

    /// Creates a `DeleteRequest` [`WriteRequest`] for this item's key.
    ///
    /// Extracts the key from `self` and wraps it in a
    /// `WriteRequest::DeleteRequest`. Pass the result to
    /// [`dynamodb_batch_write`] for execution.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemBatchOp, dynamodb_batch_write};
    ///
    /// # async fn example(
    /// #     enrollments: Vec<Enrollment>,
    /// # ) -> dynamodb_facade::Result<()> {
    /// // enrollments: Vec<Enrollment>
    /// let requests: Vec<_> = enrollments.iter().map(|e| e.batch_delete()).collect();
    /// dynamodb_batch_write::<PlatformTable>(requests).await?;
    /// # Ok(())
    /// # }
    /// ```
    fn batch_delete(&self) -> WriteRequest {
        batch_delete(self.get_key())
    }

    /// Creates a `DeleteRequest` [`WriteRequest`] from a key ID, without loading the item.
    ///
    /// Builds the key from `key_id` using the type's `HasAttribute` impl and
    /// wraps it in a `WriteRequest::DeleteRequest`. Use this when you have the
    /// key components but not the full item.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemBatchOp, dynamodb_batch_write, KeyId};
    ///
    /// # async fn example(
    /// #     user_ids: Vec<String>,
    /// # ) -> dynamodb_facade::Result<()> {
    /// // user_ids: Vec<String>
    /// let requests: Vec<_> = user_ids
    ///     .iter()
    ///     .map(|id| User::batch_delete_by_id(KeyId::pk(id)))
    ///     .collect();
    /// dynamodb_batch_write::<PlatformTable>(requests).await?;
    /// # Ok(())
    /// # }
    /// ```
    fn batch_delete_by_id(key_id: Self::KeyId<'_>) -> WriteRequest {
        batch_delete(Self::get_key_from_id(key_id))
    }
}
impl<TD: TableDefinition, DBI: DynamoDBItemOp<TD>> DynamoDBItemBatchOp<TD> for DBI {}

/// Creates a `PutRequest` [`WriteRequest`] from a raw [`Item`].
///
/// Low-level counterpart to [`DynamoDBItemBatchOp::batch_put`]. Use this
/// when you already have an [`Item<TD>`].
///
/// # Examples
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{batch_put, dynamodb_batch_write};
///
/// # async fn example() -> dynamodb_facade::Result<()> {
/// let item /* : Item<PlatformTable> */ = sample_user_item();
/// let request = batch_put(item);
/// dynamodb_batch_write::<PlatformTable>(vec![request]).await?;
/// # Ok(())
/// # }
/// ```
#[tracing::instrument(level = "debug")]
pub fn batch_put(item: Item<impl TableDefinition>) -> WriteRequest {
    WriteRequest::builder()
        .put_request(
            PutRequestBuilder::default()
                .set_item(Some(item.into_inner()))
                .build()
                .expect("item is set"),
        )
        .build()
}

/// Creates a `DeleteRequest` [`WriteRequest`] from a raw [`Key`].
///
/// Low-level counterpart to [`DynamoDBItemBatchOp::batch_delete`]. Use this
/// when you already have a [`Key<TD>`].
///
/// # Examples
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{batch_delete, dynamodb_batch_write};
///
/// # async fn example() -> dynamodb_facade::Result<()> {
/// let key = sample_user_item().into_key_only();
/// let request = batch_delete(key);
/// dynamodb_batch_write::<PlatformTable>(vec![request]).await?;
/// # Ok(())
/// # }
/// ```
#[tracing::instrument(level = "debug")]
pub fn batch_delete(key: Key<impl TableDefinition>) -> WriteRequest {
    WriteRequest::builder()
        .delete_request(
            DeleteRequestBuilder::default()
                .set_key(Some(key.into_inner()))
                .build()
                .expect("key is set"),
        )
        .build()
}

/// Executes a batch of `WriteRequest`s against a DynamoDB table using the [`global_client`].
///
/// Handles all the complexity of the DynamoDB batch write API:
///
/// - **Chunking** — automatically splits the input into chunks of 25 items
///   (the DynamoDB maximum per `BatchWriteItem` call).
/// - **Parallelism** — each chunk is sent concurrently via
///   [`tokio::spawn`].
/// - **Retry** — any unprocessed items returned by DynamoDB are retried up
///   to 3 times total. If items remain unprocessed after all attempts, the
///   function returns [`Error::FailedBatchWrite`](crate::Error::FailedBatchWrite)
///   containing the unprocessed [`WriteRequest`]s.
///
/// Build `WriteRequest` values using [`DynamoDBItemBatchOp::batch_put`],
/// [`DynamoDBItemBatchOp::batch_delete`], [`batch_put`], or [`batch_delete`].
///
/// # Errors
///
/// - Returns [`Error::FailedBatchWrite`](crate::Error::FailedBatchWrite) if
///   items remain unprocessed after 3 retry attempts.
/// - Returns [`Error::DynamoDB`](crate::Error::DynamoDB) if any individual
///   `BatchWriteItem` SDK call fails with a non-retryable error.
///
/// # Examples
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{DynamoDBItemBatchOp, dynamodb_batch_write};
///
/// # async fn example(
/// #     enrollments: Vec<Enrollment>,
/// # ) -> dynamodb_facade::Result<()> {
/// // Batch put a large collection — chunking and retries are handled automatically
/// let requests: Vec<_> = enrollments.iter().map(|e| e.batch_put()).collect();
/// dynamodb_batch_write::<PlatformTable>(requests).await?;
/// # Ok(())
/// # }
/// ```
pub fn dynamodb_batch_write<TD: TableDefinition>(
    batch_write_requests: Vec<WriteRequest>,
) -> impl Future<Output = Result<()>> {
    explicit_client::dynamodb_batch_write::<TD>(global_client(), batch_write_requests)
}
