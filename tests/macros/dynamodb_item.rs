// Tests for the `dynamodb_item!` macro.
//
// Verifies that:
//  - The macro generates a `DynamoDBItem<TD>` impl whose `to_item()`
//    produces the expected attribute map.
//  - Const-valued keys (via `const VALUE: ...`) emit a fixed attribute.
//  - Variable-valued keys (via `fn attribute_id / attribute_value`) read
//    from self and emit the transformed value.
//  - `#[partition_key]` / `#[sort_key]` are honored and the resulting
//    `HasAttribute` impls are wired through.
//  - `HasConstAttribute` is implemented for constant-value attributes.
//  - `HasAttribute::attribute_id` returns the raw id field (no formatting).

use std::collections::HashMap;

use dynamodb_facade::{
    AttributeValue, DynamoDBItem, HasAttribute, HasConstAttribute, NoId, StringAttribute,
};
use serde::{Deserialize, Serialize};

dynamodb_facade::attribute_definitions! {
    DiPk { "PK": StringAttribute }
    DiSk { "SK": StringAttribute }
    DiType { "_TYPE": StringAttribute }
}

dynamodb_facade::table_definitions! {
    DiTable {
        type PartitionKey = DiPk;
        type SortKey = DiSk;
        fn table_name() -> String { "di-table".to_owned() }
    }
}

// ---------------------------------------------------------------------------
// Singleton — both PK and SK are constants.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    max: u32,
}

dynamodb_facade::dynamodb_item! {
    #[table = DiTable]
    Config {
        #[partition_key]
        DiPk { const VALUE: &'static str = "CONFIG"; }
        #[sort_key]
        DiSk { const VALUE: &'static str = "CONFIG"; }
        DiType { const VALUE: &'static str = "CONFIG"; }
    }
}

// ---------------------------------------------------------------------------
// Variable PK + const SK.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entity {
    id: String,
    name: String,
}

dynamodb_facade::dynamodb_item! {
    #[table = DiTable]
    Entity {
        #[partition_key]
        DiPk {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("E#{id}") }
        }
        #[sort_key]
        DiSk { const VALUE: &'static str = "ENTITY"; }
        DiType { const VALUE: &'static str = "ENTITY"; }
    }
}

// ---------------------------------------------------------------------------
// Helper: extract a String attribute value from a raw map.
// ---------------------------------------------------------------------------

fn expect_s<'a>(map: &'a HashMap<String, AttributeValue>, key: &str) -> &'a str {
    match map.get(key) {
        Some(AttributeValue::S(s)) => s.as_str(),
        other => panic!("expected S for {key}, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_dynamodb_item_const_singleton() {
    let item = Config { max: 42 }.to_item();
    // Deref Item<DiTable> → HashMap to pass to expect_s.
    let raw: &HashMap<String, AttributeValue> = &item;

    // PK/SK/_TYPE present with their constant values.
    assert_eq!(expect_s(raw, "PK"), "CONFIG");
    assert_eq!(expect_s(raw, "SK"), "CONFIG");
    assert_eq!(expect_s(raw, "_TYPE"), "CONFIG");

    // The `max` field also present via serde as a Number attribute.
    match raw.get("max") {
        Some(AttributeValue::N(n)) => assert_eq!(n, "42"),
        other => panic!("expected N for max, got {other:?}"),
    }

    // HasConstAttribute impls are wired — VALUE consts are accessible.
    assert_eq!(<Config as HasConstAttribute<DiPk>>::VALUE, "CONFIG");
    assert_eq!(<Config as HasConstAttribute<DiSk>>::VALUE, "CONFIG");
    assert_eq!(<Config as HasConstAttribute<DiType>>::VALUE, "CONFIG");

    // Blanket HasAttribute impl from HasConstAttribute: attribute_value(NoId) == VALUE.
    assert_eq!(
        <Config as HasAttribute<DiPk>>::attribute_value(NoId),
        "CONFIG"
    );
}

#[test]
fn test_dynamodb_item_variable_pk() {
    let e = Entity {
        id: "abc".to_owned(),
        name: "hello".to_owned(),
    };
    let item = e.to_item();
    let raw: &HashMap<String, AttributeValue> = &item;

    // PK is formatted from the id field.
    assert_eq!(expect_s(raw, "PK"), "E#abc");
    // SK and _TYPE are constants.
    assert_eq!(expect_s(raw, "SK"), "ENTITY");
    assert_eq!(expect_s(raw, "_TYPE"), "ENTITY");
    // The `name` field is serialized by serde.
    assert_eq!(expect_s(raw, "name"), "hello");

    // HasAttribute impl is wired — `attribute_id` returns the raw id (no prefix).
    assert_eq!(<Entity as HasAttribute<DiPk>>::attribute_id(&e), "abc");

    // `attribute` (convenience method) returns the formatted value.
    assert_eq!(<Entity as HasAttribute<DiPk>>::attribute(&e), "E#abc");
}
