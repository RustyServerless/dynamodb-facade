use std::pin::pin;

use super::*;

use async_stream::try_stream;
use aws_sdk_dynamodb::operation::{
    query::builders::QueryFluentBuilder, scan::builders::ScanFluentBuilder,
};
pub(super) use futures_core::Stream;
pub(super) use futures_util::StreamExt;

/// Executes a DynamoDB `Scan` request, collecting all pages into a `Vec`.
///
/// Automatically follows `LastEvaluatedKey` pagination tokens until all items
/// have been retrieved. This is the low-level function used by
/// [`ScanRequest::all`][crate::ScanRequest::all]. Prefer using
/// [`DynamoDBItemOp::scan`] or [`ScanRequest`] directly
/// unless you are working with a raw [`ScanFluentBuilder`].
///
/// # Errors
///
/// Returns [`Err`] if any DynamoDB page request fails.
///
/// # Examples
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::dynamodb_execute_scan;
///
/// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
/// let builder = client.scan().table_name("platform");
/// let items = dynamodb_execute_scan::<PlatformTable>(builder).await?;
/// # Ok(())
/// # }
/// ```
#[tracing::instrument(level = "debug", skip(builder))]
pub async fn dynamodb_execute_scan<TD: TableDefinition>(
    builder: ScanFluentBuilder,
) -> Result<Vec<Item<TD>>> {
    let mut stream = pin!(dynamodb_stream_scan::<TD>(builder));
    let mut items = Vec::new();
    while let Some(item) = stream.next().await {
        items.extend(item?);
    }
    Ok(items)
}

/// Creates a lazy async [`Stream`] of scan results with automatic pagination.
///
/// Each element yielded by the stream is a `Vec<Item<TD>>` representing one
/// page of results. Pages are fetched on demand as the stream is consumed.
/// Each page's `LastEvaluatedKey` is used as the `ExclusiveStartKey` for the
/// next request. This is the low-level function used by
/// [`ScanRequest::stream`][crate::ScanRequest::stream]. Prefer using
/// [`DynamoDBItemOp::scan`] or [`ScanRequest`] directly
/// unless you are working with a raw [`ScanFluentBuilder`].
///
/// # Examples
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::dynamodb_stream_scan;
/// use futures_util::StreamExt;
/// use std::pin::pin;
///
/// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
/// let builder = client.scan().table_name("platform");
/// let stream = dynamodb_stream_scan::<PlatformTable>(builder);
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
pub fn dynamodb_stream_scan<TD: TableDefinition>(
    builder: ScanFluentBuilder,
) -> impl Stream<Item = Result<Vec<Item<TD>>>> {
    try_stream! {
        let res = builder.clone().send().await?;
        yield res.items.unwrap_or_default()
            .into_iter()
            .map(Item::from_dynamodb_response)
            .collect();
        let mut lek = res.last_evaluated_key;
        while lek.is_some() {
            let res = builder.clone().set_exclusive_start_key(lek).send().await?;
            lek = res.last_evaluated_key;
            yield res.items.unwrap_or_default()
                .into_iter()
                .map(Item::from_dynamodb_response)
                .collect();
        }
    }
}

/// Executes a DynamoDB `Query` request, collecting all pages into a `Vec`.
///
/// Automatically follows `LastEvaluatedKey` pagination tokens until all
/// matching items have been retrieved. This is the low-level function used by
/// [`QueryRequest::all`][crate::QueryRequest::all]. Prefer using
/// [`DynamoDBItemOp::query`] or [`QueryRequest`] directly
/// unless you are working with a raw [`QueryFluentBuilder`].
///
/// # Errors
///
/// Returns [`Err`] if any DynamoDB page request fails.
///
/// # Examples
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::dynamodb_execute_query;
///
/// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
/// let builder = client
///     .query()
///     .table_name("platform")
///     .key_condition_expression("PK = :pk")
///     .expression_attribute_values(
///         ":pk",
///         aws_sdk_dynamodb::types::AttributeValue::S("USER#user-1".into()),
///     );
/// let items = dynamodb_execute_query::<PlatformTable>(builder).await?;
/// # Ok(())
/// # }
/// ```
#[tracing::instrument(level = "debug", skip(builder))]
pub async fn dynamodb_execute_query<TD: TableDefinition>(
    builder: QueryFluentBuilder,
) -> Result<Vec<Item<TD>>> {
    let mut stream = pin!(dynamodb_stream_query::<TD>(builder));
    let mut items = Vec::new();
    while let Some(item) = stream.next().await {
        items.extend(item?);
    }
    Ok(items)
}

/// Creates a lazy async [`Stream`] of query results with automatic pagination.
///
/// Each element yielded by the stream is a `Vec<Item<TD>>` representing one
/// page of results. Pages are fetched on demand as the stream is consumed.
/// Each page's `LastEvaluatedKey` is used as the `ExclusiveStartKey` for the
/// next request. This is the low-level function used by
/// [`QueryRequest::stream`][crate::QueryRequest::stream]. Prefer using
/// [`DynamoDBItemOp::query`] or [`QueryRequest`] directly
/// unless you are working with a raw [`QueryFluentBuilder`].
///
/// # Examples
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::dynamodb_stream_query;
/// use futures_util::StreamExt;
/// use std::pin::pin;
///
/// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
/// let builder = client
///     .query()
///     .table_name("platform")
///     .key_condition_expression("PK = :pk")
///     .expression_attribute_values(
///         ":pk",
///         aws_sdk_dynamodb::types::AttributeValue::S("USER#user-1".into()),
///     );
/// let stream = dynamodb_stream_query::<PlatformTable>(builder);
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
pub fn dynamodb_stream_query<TD: TableDefinition>(
    builder: QueryFluentBuilder,
) -> impl Stream<Item = Result<Vec<Item<TD>>>> {
    try_stream! {
        let res = builder.clone().send().await?;
        yield res.items.unwrap_or_default()
            .into_iter()
            .map(Item::from_dynamodb_response)
            .collect();
        let mut lek = res.last_evaluated_key;
        while lek.is_some() {
            let res = builder.clone().set_exclusive_start_key(lek).send().await?;
            lek = res.last_evaluated_key;
            yield res.items.unwrap_or_default()
                .into_iter()
                .map(Item::from_dynamodb_response)
                .collect();
        }
    }
}
