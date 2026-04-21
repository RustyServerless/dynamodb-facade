use aws_sdk_dynamodb::types::WriteRequest;
use thiserror::Error;

/// A specialized [`Result`](core::result::Result) type for this crate.
///
/// All fallible operations in `dynamodb-facade` return this type.
///
/// # Examples
///
/// ```
/// use dynamodb_facade::{Error, Result};
///
/// fn validate_role(role: &str) -> Result<()> {
///     if role.is_empty() {
///         return Err(Error::custom("role must not be empty"));
///     }
///     Ok(())
/// }
///
/// assert!(validate_role("student").is_ok());
/// assert!(validate_role("").is_err());
/// ```
pub type Result<T> = core::result::Result<T, Error>;

/// The error type for all `dynamodb-facade` operations.
///
/// Wraps the various failure modes that can occur when interacting with
/// DynamoDB: SDK-level transport and service errors, serialization failures,
/// and application-defined errors.
///
/// # Variants
///
/// - [`Error::DynamoDB`] — an error originating from the AWS SDK, such as a
///   `ConditionalCheckFailedException`, a throttling error, or a network
///   failure. Use [`Error::as_dynamodb_error`] to inspect the underlying
///   [`aws_sdk_dynamodb::Error`].
/// - [`Error::Serde`] — a (de)serialization failure produced by
///   [`serde_dynamo`] when converting between Rust types and DynamoDB items.
/// - [`Error::Other`] — any other boxed [`core::error::Error`]. Useful for
///   wrapping domain errors via [`Error::other`].
/// - [`Error::FailedBatchWrite`] — a batch write that could not complete
///   after all retry attempts. Contains the unprocessed [`WriteRequest`]s.
/// - [`Error::Custom`] — a plain string error message. Useful for quick
///   ad-hoc errors via [`Error::custom`].
///
/// # Examples
///
/// Matching on error variants:
///
/// ```
/// use dynamodb_facade::Error;
///
/// fn handle(err: Error) {
///     match err {
///         Error::DynamoDB(_)          => eprintln!("AWS SDK error"),
///         Error::Serde(_)             => eprintln!("serialization error"),
///         Error::FailedBatchWrite(r)  => eprintln!("{} items unprocessed", r.len()),
///         Error::Other(_)             => eprintln!("other error"),
///         Error::Custom(msg)          => eprintln!("custom error: {msg}"),
///     }
/// }
/// ```
#[derive(Debug, Error)]
pub enum Error {
    /// An error returned by the AWS DynamoDB SDK.
    ///
    /// This variant is produced automatically via the [`From`] impls for
    /// [`aws_sdk_dynamodb::error::SdkError`] and [`aws_sdk_dynamodb::Error`].
    /// Use [`Error::as_dynamodb_error`] to borrow the inner error for
    /// pattern-matching on specific service errors such as
    /// `ConditionalCheckFailedException`.
    #[error(transparent)]
    DynamoDB(Box<aws_sdk_dynamodb::Error>),

    /// A (de)serialization error from [`serde_dynamo`].
    ///
    /// Produced when converting a Rust struct to or from a DynamoDB item map
    /// fails — for example, when a required attribute is missing or has an
    /// unexpected type.
    #[error(transparent)]
    Serde(#[from] serde_dynamo::Error),

    /// A batch write that did not complete after all retry attempts.
    ///
    /// Returned by [`dynamodb_batch_write`](crate::dynamodb_batch_write) when
    /// some [`WriteRequest`]s remain unprocessed after the maximum number of
    /// retries. The contained vector holds the requests that were never
    /// acknowledged by DynamoDB, allowing the caller to inspect or retry them.
    #[error("BatchWriteItem failure: {len} items", len = .0.len())]
    FailedBatchWrite(Vec<WriteRequest>),

    /// Any other boxed error.
    ///
    /// Use [`Error::other`] to wrap an arbitrary [`core::error::Error`] value
    /// into this variant.
    #[error(transparent)]
    Other(#[from] Box<dyn core::error::Error + Send>),

    /// A plain string error message.
    ///
    /// Use [`Error::custom`] to construct this variant.
    #[error("Custom Error: {0}")]
    Custom(String),
}

impl Error {
    /// Creates an [`Error::Custom`] from any value that converts into a
    /// [`String`].
    ///
    /// This is a convenience constructor for quick ad-hoc errors without
    /// needing to define a dedicated error type.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Error;
    ///
    /// let err = Error::custom("enrollment limit reached");
    /// assert!(matches!(err, Error::Custom(_)));
    /// assert_eq!(err.to_string(), "Custom Error: enrollment limit reached");
    /// ```
    pub fn custom(message: impl Into<String>) -> Self {
        Self::Custom(message.into())
    }

    /// Creates an [`Error::Other`] by boxing any [`core::error::Error`] value.
    ///
    /// Use this to wrap domain-specific or standard-library errors when
    /// implementing fallible methods from this crate — for example, a
    /// [`FromStr`](std::str::FromStr) parse error inside a manual
    /// [`DynamoDBItem::try_from_item`](crate::DynamoDBItem::try_from_item)
    /// implementation.
    ///
    /// # Examples
    ///
    /// Wrapping a [`ParseIntError`](std::num::ParseIntError) when deserializing
    /// a DynamoDB string attribute into a numeric field:
    ///
    /// ```
    /// use dynamodb_facade::Error;
    ///
    /// fn parse_credits(raw: &str) -> dynamodb_facade::Result<u32> {
    ///     raw.parse::<u32>().map_err(Error::other)
    /// }
    ///
    /// assert!(parse_credits("42").is_ok());
    /// assert!(matches!(parse_credits("not-a-number"), Err(Error::Other(_))));
    /// ```
    pub fn other(error: impl core::error::Error + Send + Sync + 'static) -> Self {
        Self::Other(Box::new(error))
    }

    /// Returns a reference to the inner [`aws_sdk_dynamodb::Error`] if this
    /// error is the [`Error::DynamoDB`] variant, or [`None`] otherwise.
    ///
    /// Use this to inspect or pattern-match on specific DynamoDB service
    /// errors (e.g. `ConditionalCheckFailedException`, `ResourceNotFoundException`)
    /// without unwrapping the full error chain.
    ///
    /// # Examples
    ///
    /// Distinguishing a "not found" condition failure from other errors when
    /// deleting an enrollment that must already exist:
    ///
    /// ```no_run
    /// # use dynamodb_facade::{DynamoDBItemOp, DynamoDBError, KeyId};
    /// # use dynamodb_facade::test_fixtures::*;
    /// # async fn example(
    /// #     client: dynamodb_facade::Client,
    /// #     user_id: &str,
    /// #     course_id: &str,
    /// # ) -> Result<Enrollment, String> {
    /// match Enrollment::delete_by_id(client, KeyId::pk(user_id).sk(course_id))
    ///     .exists()
    ///     .await
    /// {
    ///     Ok(enrollment) => Ok(enrollment.expect("exists guard guarantees a return value")),
    ///     Err(e) if matches!(
    ///         e.as_dynamodb_error(),
    ///         Some(DynamoDBError::ConditionalCheckFailedException(_))
    ///     ) => Err(format!("enrollment for user {user_id} / course {course_id} not found")),
    ///     Err(e) => Err(format!("unexpected error: {e}")),
    /// }
    /// # }
    /// ```
    pub fn as_dynamodb_error(&self) -> Option<&aws_sdk_dynamodb::Error> {
        match self {
            Self::DynamoDB(e) => Some(e),
            _ => None,
        }
    }
}

/// Converts an [`aws_sdk_dynamodb::error::SdkError`] into [`Error::DynamoDB`].
///
/// This impl is provided for all `SdkError<T, R>` where the SDK can convert
/// the operation-specific error into the generic [`aws_sdk_dynamodb::Error`].
/// It allows the `?` operator to be used directly on SDK call results.
impl<T, R> From<aws_sdk_dynamodb::error::SdkError<T, R>> for Error
where
    aws_sdk_dynamodb::Error: From<aws_sdk_dynamodb::error::SdkError<T, R>>,
{
    fn from(value: aws_sdk_dynamodb::error::SdkError<T, R>) -> Self {
        Self::DynamoDB(Box::new(value.into()))
    }
}

/// Converts an [`aws_sdk_dynamodb::Error`] into [`Error::DynamoDB`].
///
/// Boxes the SDK error and wraps it in the [`Error::DynamoDB`] variant.
impl From<aws_sdk_dynamodb::Error> for Error {
    fn from(value: aws_sdk_dynamodb::Error) -> Self {
        Self::DynamoDB(Box::new(value))
    }
}
