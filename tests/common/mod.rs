// Shared DynamoDB Local container lifecycle for integration tests.
// Single container shared across all tests via LazyLock.
// Per-test isolation via random temporary tables.
use std::sync::LazyLock;

use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::config::Credentials;
use aws_sdk_dynamodb::types::{
    AttributeDefinition as SdkAttrDef, BillingMode, GlobalSecondaryIndex, KeySchemaElement,
    KeyType, Projection, ProjectionType, ScalarAttributeType,
};
use testcontainers::ContainerAsync;
use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::dynamodb_local::DynamoDb;
use tokio::sync::OnceCell;

// -- Shared container singleton ---------------------------------------------------

/// Global container handle + endpoint URL, initialized once per test binary.
static DYNAMODB_CONTAINER: OnceCell<(ContainerAsync<DynamoDb>, String)> = OnceCell::const_new();

async fn init_container() -> &'static (ContainerAsync<DynamoDb>, String) {
    DYNAMODB_CONTAINER
        .get_or_init(|| async {
            let container = DynamoDb::default()
                .start()
                .await
                .expect("DynamoDB Local container should start");
            let host = container
                .get_host()
                .await
                .expect("container host should be available");
            let port = container
                .get_host_port_ipv4(8000.tcp())
                .await
                .expect("container port should be mapped");
            let endpoint = format!("http://{host}:{port}");
            (container, endpoint)
        })
        .await
}

// -- Client builder ---------------------------------------------------------------

/// Returns an SDK client connected to the shared DynamoDB Local container.
pub async fn test_client() -> Client {
    let (_container, endpoint) = init_container().await;
    build_client(endpoint).await
}

async fn build_client(endpoint: &str) -> Client {
    let creds = Credentials::new("fakeKey", "fakeSecret", None, None, "test");
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region("us-east-1")
        .endpoint_url(endpoint)
        .credentials_provider(creds)
        .load()
        .await;
    Client::new(&config)
}

// -- Per-test table helpers -------------------------------------------------------

static TABLE_COUNTER: LazyLock<std::sync::atomic::AtomicU64> =
    LazyLock::new(|| std::sync::atomic::AtomicU64::new(0));

/// Generates a unique table name for test isolation.
pub fn random_table_name(prefix: &str) -> String {
    let id = TABLE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_millis();
    format!("{prefix}_{ts}_{id}")
}

// -- Table creation helpers -------------------------------------------------------

/// Creates a composite PK+SK table with two GSIs:
///   - `iType`  on `_TYPE` (HASH)  — for type-dispatched queries
///   - `iEmail` on `email` (HASH)  — for email lookups
///
/// Uses on-demand billing (PayPerRequest) to avoid capacity parameters.
/// Blocks until the table reaches ACTIVE status before returning.
pub async fn create_composite_table(client: &Client, table_name: &str) {
    let _ = client
        .create_table()
        .table_name(table_name)
        .billing_mode(BillingMode::PayPerRequest)
        // Primary key schema: PK (HASH) + SK (RANGE)
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("PK")
                .key_type(KeyType::Hash)
                .build()
                .expect("PK key schema element"),
        )
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("SK")
                .key_type(KeyType::Range)
                .build()
                .expect("SK key schema element"),
        )
        // Attribute definitions for all keyed attributes (table + GSI keys)
        .attribute_definitions(
            SdkAttrDef::builder()
                .attribute_name("PK")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .expect("PK attr def"),
        )
        .attribute_definitions(
            SdkAttrDef::builder()
                .attribute_name("SK")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .expect("SK attr def"),
        )
        .attribute_definitions(
            SdkAttrDef::builder()
                .attribute_name("_TYPE")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .expect("_TYPE attr def"),
        )
        .attribute_definitions(
            SdkAttrDef::builder()
                .attribute_name("email")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .expect("email attr def"),
        )
        // GSI: iType — query all items of a given type
        .global_secondary_indexes(
            GlobalSecondaryIndex::builder()
                .index_name("iType")
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name("_TYPE")
                        .key_type(KeyType::Hash)
                        .build()
                        .expect("iType key schema"),
                )
                .projection(
                    Projection::builder()
                        .projection_type(ProjectionType::All)
                        .build(),
                )
                .build()
                .expect("iType GSI"),
        )
        // GSI: iEmail — look up any item by email address
        .global_secondary_indexes(
            GlobalSecondaryIndex::builder()
                .index_name("iEmail")
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name("email")
                        .key_type(KeyType::Hash)
                        .build()
                        .expect("iEmail key schema"),
                )
                .projection(
                    Projection::builder()
                        .projection_type(ProjectionType::All)
                        .build(),
                )
                .build()
                .expect("iEmail GSI"),
        )
        .send()
        .await
        .expect("create_table should succeed");

    // Poll until the table is ACTIVE — DynamoDB Local is fast but not instant.
    loop {
        let desc = client
            .describe_table()
            .table_name(table_name)
            .send()
            .await
            .expect("describe_table should succeed");
        let status = desc
            .table()
            .expect("table descriptor present")
            .table_status()
            .cloned()
            .expect("table status present");
        if matches!(status, aws_sdk_dynamodb::types::TableStatus::Active) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

/// Deletes a table, ignoring any errors (e.g. table already gone).
pub async fn delete_table(client: &Client, table_name: &str) {
    let _ = client.delete_table().table_name(table_name).send().await;
}

// -- TestContext ------------------------------------------------------------------

/// Per-test context: a live DynamoDB client and a freshly-created table.
///
/// Create with `TestContext::new(prefix).await` at the start of each test.
/// Call `ctx.cleanup().await` at the end to delete the table.
///
/// We deliberately do NOT implement `Drop` for cleanup because `Drop` cannot
/// be async — the caller must call `.cleanup().await` explicitly.
pub struct TestContext {
    pub client: Client,
    pub table_name: String,
}

impl TestContext {
    /// Creates a new test context:
    /// 1. Builds a client connected to the shared DynamoDB Local container.
    /// 2. Generates a unique table name via `random_table_name`.
    /// 3. Creates the composite table and waits until it is ACTIVE.
    pub async fn new(prefix: &str) -> Self {
        let client = test_client().await;
        let table_name = random_table_name(prefix);
        create_composite_table(&client, &table_name).await;
        Self { client, table_name }
    }

    /// Deletes the table created by this context.
    ///
    /// Must be called explicitly at the end of each test — there is no
    /// automatic cleanup on drop.
    pub async fn cleanup(self) {
        delete_table(&self.client, &self.table_name).await;
    }
}
