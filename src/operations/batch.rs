use super::*;

use aws_sdk_dynamodb::types::{
    WriteRequest,
    builders::{DeleteRequestBuilder, PutRequestBuilder},
};
use tracing::Instrument;

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
/// #     client: aws_sdk_dynamodb::Client,
/// #     enrollments: Vec<Enrollment>,
/// # ) -> dynamodb_facade::Result<()> {
/// // Batch put a collection of enrollments
/// let requests: Vec<_> = enrollments.iter().map(|e| e.batch_put()).collect();
/// dynamodb_batch_write::<PlatformTable>(client, requests).await?;
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
    /// #     client: aws_sdk_dynamodb::Client,
    /// #     enrollments: Vec<Enrollment>,
    /// # ) -> dynamodb_facade::Result<()> {
    /// // enrollments: Vec<Enrollment>
    /// let requests: Vec<_> = enrollments.iter().map(|e| e.batch_put()).collect();
    /// dynamodb_batch_write::<PlatformTable>(client, requests).await?;
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
    /// #     client: aws_sdk_dynamodb::Client,
    /// #     enrollments: Vec<Enrollment>,
    /// # ) -> dynamodb_facade::Result<()> {
    /// // enrollments: Vec<Enrollment>
    /// let requests: Vec<_> = enrollments.iter().map(|e| e.batch_delete()).collect();
    /// dynamodb_batch_write::<PlatformTable>(client, requests).await?;
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
    /// #     client: aws_sdk_dynamodb::Client,
    /// #     user_ids: Vec<String>,
    /// # ) -> dynamodb_facade::Result<()> {
    /// // user_ids: Vec<String>
    /// let requests: Vec<_> = user_ids
    ///     .iter()
    ///     .map(|id| User::batch_delete_by_id(KeyId::pk(id)))
    ///     .collect();
    /// dynamodb_batch_write::<PlatformTable>(client, requests).await?;
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
/// # async fn example(
/// #     client: aws_sdk_dynamodb::Client,
/// # ) -> dynamodb_facade::Result<()> {
/// let item /* : Item<PlatformTable> */ = sample_user_item();
/// let request = batch_put(item);
/// dynamodb_batch_write::<PlatformTable>(client, vec![request]).await?;
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
/// # async fn example(
/// #     client: aws_sdk_dynamodb::Client,
/// # ) -> dynamodb_facade::Result<()> {
/// let key = sample_user_item().into_key_only();
/// let request = batch_delete(key);
/// dynamodb_batch_write::<PlatformTable>(client, vec![request]).await?;
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

/// Executes a batch of `WriteRequest`s against a DynamoDB table.
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
/// #     client: aws_sdk_dynamodb::Client,
/// #     enrollments: Vec<Enrollment>,
/// # ) -> dynamodb_facade::Result<()> {
/// // Batch put a large collection — chunking and retries are handled automatically
/// let requests: Vec<_> = enrollments.iter().map(|e| e.batch_put()).collect();
/// dynamodb_batch_write::<PlatformTable>(client, requests).await?;
/// # Ok(())
/// # }
/// ```
#[tracing::instrument(level = "debug", skip(client))]
pub async fn dynamodb_batch_write<TD: TableDefinition>(
    client: aws_sdk_dynamodb::Client,
    mut batch_write_requests: Vec<WriteRequest>,
) -> Result<()> {
    const MAX_RETRY: usize = 3;

    let table_name = TD::table_name();
    // Process the Batch(es) in massively parallel fashion
    // Because Rust.
    tracing::debug!("putting {} items...", batch_write_requests.len());
    let mut retry = 0;
    while !batch_write_requests.is_empty() && retry < MAX_RETRY {
        retry += 1;
        tracing::debug!("Try #{retry}/{MAX_RETRY}");
        let handles = batch_write_requests
            .chunks(25)
            .enumerate()
            .map(|(index, chunk)| {
                let chunk = chunk.to_vec();
                let cclient = client.clone();
                let ctable_name = table_name.clone();
                tokio::spawn(
                    async move {
                        tracing::debug!("Sending BatchWriteItem for chunk #{index}...");
                        let result = cclient
                            .batch_write_item()
                            .set_request_items(Some([(ctable_name, chunk)].into()))
                            .send()
                            .await;
                        tracing::debug!("BatchWriteItem finished for chunk #{index}");
                        result
                    }
                    .instrument(tracing::info_span!("batch_write_chunk", %index, try=retry)),
                )
            })
            .collect::<Vec<_>>();
        let mut unprocess_vec = Vec::default();

        for h in handles {
            let batch_output = h.await.expect("batch write task panicked")?;
            if let Some(unproccessed) = batch_output.unprocessed_items {
                if !unproccessed.is_empty() {
                    unprocess_vec.extend(unproccessed.into_iter().flat_map(|e| e.1));
                }
            }
        }

        batch_write_requests = unprocess_vec;

        tracing::debug!("{} items were unprocessed", batch_write_requests.len());
    }

    if batch_write_requests.is_empty() {
        Ok(())
    } else {
        Err(crate::Error::FailedBatchWrite(batch_write_requests))
    }
}
