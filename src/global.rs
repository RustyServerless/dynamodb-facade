use std::sync::OnceLock;

static CLIENT: OnceLock<aws_sdk_dynamodb::Client> = OnceLock::new();

/// Returns the process-global DynamoDB client ([`aws_sdk_dynamodb::Client`]).
///
/// Most users never call this directly: it is invoked internally by the
/// default (client-less) operation APIs, such as [`DynamoDBItemOp`] trait
/// methods and the request builders' `new()` constructors. Call it yourself
/// only if you need the underlying client, for example to issue a request
/// that this facade does not expose.
///
/// # Panics
///
/// Panics if [`init_global_client`] has not been called yet.
///
/// [`DynamoDBItemOp`]: super::DynamoDBItemOp
pub fn global_client() -> aws_sdk_dynamodb::Client {
    CLIENT
        .get()
        .expect("Global DynamoDB client must be set early in the program")
        .clone()
}

/// Initializes the process-global DynamoDB client used by the default
/// (client-less) operation APIs.
///
/// Call this exactly once, early during program startup, before any code
/// path that relies on the global client runs — this includes every method
/// on [`DynamoDBItemOp`] and every request builder's `new()` constructor.
///
/// If you need multiple clients, or want to avoid process-global state
/// entirely (e.g. in tests), skip this function and use the methods of
/// [`explicit_client::DynamoDBItemOp`] and request builder's `with_client()`
/// constructors instead.
///
/// # Panics
///
/// Panics if called more than once.
///
/// # Examples
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::User;
/// use dynamodb_facade::{DynamoDBItemOp, KeyId};
///
/// #[tokio::main]
/// async fn main() -> dynamodb_facade::Result<()> {
///     // Create a DynamoDB client
///     let config = aws_config::load_from_env().await;
///     let client = aws_sdk_dynamodb::Client::new(&config);
///
///     // Initialize the global client of the library with it:
///     dynamodb_facade::init_global_client(client);
///
///     // From then on you can use operations (assuming User is a DynamoDBItem)
///     let user /* : Option<User> */ = User::get(KeyId::pk("user-1")).await?;
///     Ok(())
/// }
/// ```
///
/// [`DynamoDBItemOp`]: super::DynamoDBItemOp
/// [`explicit_client::DynamoDBItemOp`]: super::explicit_client::DynamoDBItemOp
pub fn init_global_client(client: aws_sdk_dynamodb::Client) {
    CLIENT
        .set(client)
        .expect("Global DynamoDB client must be set only once")
}
