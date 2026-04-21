// Integration tests for declarative macros.
// Not feature-gated — no DynamoDB needed, validates macro expansion at compile time.

mod macros {
    mod attribute_definitions;
    mod dynamodb_item;
    mod index_definitions;
    mod key_schema;
    mod table_definitions;
}
