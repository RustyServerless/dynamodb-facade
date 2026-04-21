# dynamodb-facade — Usage Examples

This document showcases every major feature of `dynamodb-facade` through an
**Online Learning Platform** domain: users, courses, enrollments, assignments,
and platform configuration — all stored in a single DynamoDB table (mono-table
pattern).

Where relevant, the equivalent raw `aws-sdk-dynamodb` code is shown for
comparison.

---

## Table of Contents

- [1. Schema Definitions](#1-schema-definitions)
  - [1.1 Attribute Definitions](#11-attribute-definitions)
  - [1.2 Table Definition (Composite Key)](#12-table-definition-composite-key)
  - [1.3 Table Definition (Simple Key)](#13-table-definition-simple-key)
  - [1.4 Index Definitions](#14-index-definitions)
- [2. Item Definitions](#2-item-definitions)
  - [2.1 Singleton Item (Constant PK + SK)](#21-singleton-item-constant-pk--sk)
  - [2.2 Variable PK, Constant SK](#22-variable-pk-constant-sk)
  - [2.3 Variable PK + Variable SK (Hierarchical)](#23-variable-pk--variable-sk-hierarchical)
  - [2.4 Delegation to Another Type's Keys](#24-delegation-to-another-types-keys)
  - [2.5 Manual DynamoDBItem Implementation](#25-manual-dynamodbitem-implementation)
  - [2.6 Custom IntoAttributeValue](#26-custom-intoattributevalue)
- [3. CRUD Operations](#3-crud-operations)
  - [3.1 Get — Typed](#31-get--typed)
  - [3.2 Get — Raw (Projection)](#32-get--raw-projection)
  - [3.3 Put — Unconditional](#33-put--unconditional)
  - [3.4 Put — not_exists (Create-Only)](#34-put--not_exists-create-only)
  - [3.5 Put — Custom Condition](#35-put--custom-condition)
  - [3.6 Delete — By ID with exists](#36-delete--by-id-with-exists)
  - [3.7 Delete — Instance (Unconditional)](#37-delete--instance-unconditional)
  - [3.8 Update — set + exists](#38-update--set--exists)
  - [3.9 Update — Compound (set + remove)](#39-update--compound-set--remove)
  - [3.10 Update — combine (Optional Fields)](#310-update--combine-optional-fields)
  - [3.11 Update — Atomic Counters](#311-update--atomic-counters)
  - [3.12 Update — return_new / return_none](#312-update--return_new--return_none)
  - [3.13 Update — Custom Condition](#313-update--custom-condition)
- [4. Query Operations](#4-query-operations)
  - [4.1 Query — Partition Key Only](#41-query--partition-key-only)
  - [4.2 Query — SK begins_with](#42-query--sk-begins_with)
  - [4.3 Query — All (Constant PK)](#43-query--all-constant-pk)
  - [4.4 Query — Index](#44-query--index)
  - [4.5 Query — Index (All Items of a Type)](#45-query--index-all-items-of-a-type)
- [5. Scan Operations](#5-scan-operations)
  - [5.1 Scan — Typed with Filter](#51-scan--typed-with-filter)
  - [5.2 Scan — Raw (Untyped Dispatch)](#52-scan--raw-untyped-dispatch)
- [6. Condition Expressions](#6-condition-expressions)
  - [6.1 Equality / Inequality / Comparison](#61-equality--inequality--comparison)
  - [6.2 exists / not_exists](#62-exists--not_exists)
  - [6.3 begins_with / contains](#63-begins_with--contains)
  - [6.4 between / is_in](#64-between--is_in)
  - [6.5 size_cmp](#65-size_cmp)
  - [6.6 Boolean Operators (AND / OR / NOT)](#66-boolean-operators-and--or--not)
  - [6.7 Variadic and / or](#67-variadic-and--or)
- [7. Update Expressions](#7-update-expressions)
  - [7.1 set / remove](#71-set--remove)
  - [7.2 increment / decrement](#72-increment--decrement)
  - [7.3 init_increment / init_decrement](#73-init_increment--init_decrement)
  - [7.4 set_custom with UpdateSetRhs](#74-set_custom-with-updatesetrhs)
  - [7.5 list_append / list_prepend](#75-list_append--list_prepend)
  - [7.6 add / delete (Sets)](#76-add--delete-sets)
  - [7.7 and / combine / try_combine](#77-and--combine--try_combine)
- [8. Key Conditions](#8-key-conditions)
- [9. Batch Operations](#9-batch-operations)
  - [9.1 Batch Put](#91-batch-put)
  - [9.2 Batch Delete](#92-batch-delete)
  - [9.3 Batch Mixed (Put + Delete)](#93-batch-mixed-put--delete)
- [10. Transactions](#10-transactions)
  - [10.1 Transact Put + Update](#101-transact-put--update)
  - [10.2 Transact Delete + Update](#102-transact-delete--update)
  - [10.3 Transact Delete + Put (Swap)](#103-transact-delete--put-swap)
  - [10.4 Transact Condition Check](#104-transact-condition-check)
- [11. Error Handling](#11-error-handling)
- [12. Item Inspection APIs](#12-item-inspection-apis)
- [13. Typestate Builder Transitions](#13-typestate-builder-transitions)

---

## 1. Schema Definitions

### 1.1 Attribute Definitions

Attribute definitions declare DynamoDB attribute names and their types (S, N, B).
They are used as type-level identifiers throughout the library.

```rust
use dynamodb_facade::{attribute_definitions, StringAttribute, NumberAttribute};

attribute_definitions! {
    /// Partition key for the platform mono-table.
    PK { "PK": StringAttribute }

    /// Sort key for the platform mono-table.
    SK { "SK": StringAttribute }

    /// Item type discriminator (single-table design).
    ItemType { "_TYPE": StringAttribute }

    /// TTL attribute for expiring items.
    Expiration { "expiration_timestamp": NumberAttribute }

    /// Email attribute, used as a Index partition key.
    Email { "email": StringAttribute }

    /// Searchable unique identifier, used as a Index partition key.
    SearchId { "id": StringAttribute }
}
```

Each invocation generates a zero-sized `pub struct` that implements `AttributeDefinition`,
providing `const NAME: &str` and `type Type`.

### 1.2 Table Definition (Composite Key)

```rust
use dynamodb_facade::table_definitions;

table_definitions! {
    /// The platform mono-table with composite key (PK + SK).
    PlatformTable {
        type PartitionKey = PK;
        type SortKey = SK;
        fn table_name() -> String {
            std::env::var("TABLE_NAME")
                .expect("TABLE_NAME env var must be set")
        }
    }
}
```

Generates a `pub struct PlatformTable` implementing `TableDefinition` with
`CompositeKeySchema`.

### 1.3 Table Definition (Simple Key)

```rust
table_definitions! {
    /// A simple-key table (PK only, no sort key).
    SimpleTable {
        type PartitionKey = PK;
        fn table_name() -> String {
            std::env::var("SIMPLE_TABLE_NAME")
                .expect("SIMPLE_TABLE_NAME env var must be set")
        }
    }
}
```

Generates `SimpleKeySchema` — no sort key methods available at compile time.

### 1.4 Index Definitions

```rust
use dynamodb_facade::index_definitions;

// table_definitions! and index_definitions! can have multiple definitions in one call-site
index_definitions! {
    /////////
    // Index with partition key only (simple key).
    /////////
    
    /// Index on item type — query all items of a given type.
    #[table = PlatformTable]
    TypeIndex {
        type PartitionKey = ItemType;
        fn index_name() -> String { "iType".to_owned() }
    }

    /////////
    // Index with partition key only.
    /////////
    
    /// Index on email — look up any item by email.
    #[table = PlatformTable]
    EmailIndex {
        type PartitionKey = Email;
        fn index_name() -> String { "iEmail".to_owned() }
    }

    /////////
    // Index with composite key.
    /////////
    
    /// Index on searchable ID + type — composite key index.
    #[table = PlatformTable]
    IdTypeIndex {
        type PartitionKey = SearchId;
        type SortKey = ItemType;
        fn index_name() -> String { "iIdType".to_owned() }
    }
}
```

---

## 2. Item Definitions

### 2.1 Singleton Item (Constant PK + SK)

For items that exist as a single row (e.g. platform configuration):

```rust
use dynamodb_facade::{dynamodb_item, KeyId, NoId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub max_enrollments: u32,
    pub maintenance_mode: bool,
}

dynamodb_item! {
    #[table = PlatformTable]
    PlatformConfig {
        #[partition_key]
        PK { const VALUE: &'static str = "PLATFORM_CONFIG"; }
        #[sort_key]
        SK { const VALUE: &'static str = "PLATFORM_CONFIG"; }
        ItemType { const VALUE: &'static str = "PLATFORM_CONFIG"; }
    }
}
```

Both PK and SK are compile-time constants. Access via `KeyId::NONE`:

```rust
let config = PlatformConfig::get(client, KeyId::NONE).await?;
```

### 2.2 Variable PK, Constant SK

The most common pattern — entity keyed by a unique ID:

```rust
use uuid::Uuid;
type ID = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: ID,
    pub name: String,
    pub email: String,
    pub role: String,
}

dynamodb_item! {
    #[table = PlatformTable]
    User {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> ID { self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        ItemType { const VALUE: &'static str = "USER"; }
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
    }
}
```

`attribute_id` extracts the logical ID from the struct. `attribute_value` builds
the DynamoDB attribute string from that ID. Access via `KeyId::pk(user_id)`.

`#[marker_only]` signals attributes that are already part of the type serialization
and prevents the macro from adding the attribute to the 
`DynamoDBItem::AdditionalAttributes` associated type. It stills implement `HasAttribute`
on the type, signaling it is part of an index (in this example, `User` becomes part of 
`EmailIndex` from the type-system perpective).

### 2.3 Variable PK + Variable SK (Hierarchical)

For child entities stored under a parent's partition:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enrollment {
    pub user_id: ID,
    pub course_id: ID,
    pub enrolled_at: u64,
    pub progress: f64,
}

dynamodb_item! {
    #[table = PlatformTable]
    Enrollment {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> ID { self.user_id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        SK {
            fn attribute_id(&self) -> ID { self.course_id }
            fn attribute_value(id) -> String { format!("ENROLL#{id}") }
        }
        ItemType { const VALUE: &'static str = "ENROLLMENT"; }
    }
}
```

Both PK and SK are dynamic. Access via `KeyId::pk(user_id).sk(course_id)`.
Query all enrollments for a user via `Enrollment::key_condition(user_id)`.

### 2.4 Delegation to Another Type's Keys

When a wrapper type shares key layout with an existing type:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct UserWithSecret<'a> {
    #[serde(flatten)]
    user: &'a User,
    secret: String,
}

dynamodb_item! {
    #[table = PlatformTable]
    UserWithSecret<'_> {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> <User as HasAttribute<PK>>::Id<'id> {
                <User as HasAttribute<PK>>::attribute_id(self.user)
            }
            fn attribute_value(id) -> <User as HasAttribute<PK>>::Value {
                <User as HasAttribute<PK>>::attribute_value(id)
            }
        }
        #[sort_key]
        SK { const VALUE: &'static str = <User as HasConstAttribute<SK>>::VALUE; }
        ItemType { const VALUE: &'static str = <User as HasConstAttribute<ItemType>>::VALUE; }
    }
}
```

Uses fully-qualified associated types to delegate key generation to `User`.

### 2.5 Manual DynamoDBItem Implementation

When serde round-trip is insufficient (e.g. enum stored as a single attribute):

```rust
use dynamodb_facade::{
    attr_list, has_attributes, DynamoDBItem, Error, IntoAttributeValue, Item,
};

#[derive(Debug, Clone, PartialEq)]
pub enum CourseStatus {
    Draft,
    Published,
    Archived,
}

impl DynamoDBItem<PlatformTable> for CourseStatus {
    type AdditionalAttributes = attr_list!(ItemType);

    fn to_item(&self) -> Item<PlatformTable>
    where
        Self: Serialize,
    {
        let minimal_item = Item::minimal_from(self);
        minimal_item
            .with_attributes([("status".to_owned(), self.to_string().into_attribute_value())])
    }

    fn try_from_item(item: Item<PlatformTable>) -> dynamodb_facade::Result<Self> {
        item.get("status")
            .ok_or_else(|| Error::custom("Missing 'status' attribute"))
            .and_then(|av| {
                av.as_s()
                    .map_err(|e| Error::custom(format!("Invalid schema: {e:?}")))
            })
            .and_then(|s| s.parse().map_err(Error::other))
    }
}

has_attributes! {
    CourseStatus {
        PK { const VALUE: &'static str = "COURSE_STATUS"; }
        SK { const VALUE: &'static str = "COURSE_STATUS"; }
        ItemType { const VALUE: &'static str = "COURSE_STATUS"; }
    }
}
```

`has_attributes!` replaces manual `HasConstAttribute` / `HasAttribute` impls.

### 2.6 Custom IntoAttributeValue

For domain newtypes that should map to DynamoDB attribute values:

```rust
use dynamodb_facade::{IntoAttributeValue, AttributeValue};

pub struct EmailAddress(String);

impl IntoAttributeValue for EmailAddress {
    fn into_attribute_value(self) -> AttributeValue {
        self.0.into_attribute_value()
    }
}

pub struct Credits(u32);

impl IntoAttributeValue for Credits {
    fn into_attribute_value(self) -> AttributeValue {
        self.0.into_attribute_value()
    }
}
```

These can then be used directly in expressions: `Update::set("email", email_address)`.

---

## 3. CRUD Operations

### 3.1 Get — Typed

Returns a deserialized `Option<T>`:

```rust
// Singleton (const PK + SK)
let config = PlatformConfig::get(client, KeyId::NONE).await?;

// By ID (variable PK, const SK)
let user = User::get(client, KeyId::pk(user_id)).await?;

// By composite key (variable PK + SK)
let enrollment = Enrollment::get(client, KeyId::pk(user_id).sk(course_id)).await?;
```

**Raw SDK equivalent:**
```rust
// 16 lines of boilerplate per get:
let resp = client.get_item()
    .table_name(table_name())
    .key("PK", AttributeValue::S(format!("USER#{user_id}")))
    .key("SK", AttributeValue::S("USER".to_owned()))
    .send()
    .await?;
let user = resp.item.map(|item| serde_dynamo::from_item(item).unwrap());
```

### 3.2 Get — Raw (Projection)

When you need untyped access or a subset of attributes:

```rust
let raw_item = User::get(client, KeyId::pk(user_id))
    .raw()
    .await?;
// raw_item: Option<Item<PlatformTable>>
```

### 3.3 Put — Unconditional

Overwrites any existing item:

```rust
config.put(client).await?;
```

### 3.4 Put — not_exists (Create-Only)

Fails with `ConditionalCheckFailedException` if item already exists:

```rust
new_user.put(client).not_exists().await?;
```

**Raw SDK equivalent:**
```rust
client.put_item()
    .table_name(table_name())
    .set_item(Some(serde_dynamo::to_item(&new_user).unwrap()))
    .condition_expression("attribute_not_exists(PK)")
    .return_values(ReturnValue::None)
    .send()
    .await?;
```

### 3.5 Put — Custom Condition

Overwrite only if a specific condition holds (e.g. not exist or expired TTL):

```rust
course_status
    .put(client)
    .condition(
        CourseStatus::not_exists()
            | Condition::lt(Expiration::NAME, now_timestamp),
    )
    .await?;
```

### 3.6 Delete — By ID with exists

Returns the old item, fails if item doesn't exist:

```rust
let old_enrollment = Enrollment::delete_by_id(
    client,
    KeyId::pk(user_id).sk(course_id),
)
    .exists()
    .await?
    .expect("exists() guarantees Some");
```

`delete_by_id` defaults to `Return<Old>` — returns `Option<T>`.
`.exists()` adds `attribute_exists(PK)` condition.

### 3.7 Delete — Instance (Unconditional)

Delete from an already-loaded item:

```rust
enrollment.delete(client).await?;
```

### 3.8 Update — set + exists

```rust
User::update_by_id(
    client,
    KeyId::pk(user_id),
    Update::set("name", new_name),
)
    .exists()
    .await?;
// Returns the updated User (default: Return<New>)
```

**Raw SDK equivalent:**
```rust
client.update_item()
    .table_name(table_name())
    .key("PK", AttributeValue::S(format!("USER#{user_id}")))
    .key("SK", AttributeValue::S("USER".to_owned()))
    .update_expression("SET #name = :name")
    .expression_attribute_names("#name", "name")
    .expression_attribute_values(":name", AttributeValue::S(new_name))
    .condition_expression("attribute_exists(PK)")
    .return_values(ReturnValue::AllNew)
    .send()
    .await?
    .attributes
    .map(|attrs| serde_dynamo::from_item(attrs).unwrap())
    .expect("asked for ALL_NEW");
```

### 3.9 Update — Compound (set + remove)

Chain multiple update actions with `.and()`:

```rust
user.update(
    client,
    Update::set("email", new_email)
        .and(Update::set("email_verified", true)),
)
    .exists()
    .await?;
```

Conditional set/remove based on domain logic:

```rust
let update = if let Some(bio) = new_bio {
    Update::set("bio", bio)
} else {
    Update::remove("bio")
};
User::update_by_id(client, KeyId::pk(user_id), update)
    .exists()
    .await?;
```

### 3.10 Update — combine (Optional Fields)

Merge an iterator of optional updates into one expression:

```rust
let update = Update::combine(
    [
        new_name.map(|n| Update::set("name", n)),
        new_email.map(|e| Update::set("email", e)),
        new_role.map(|r| Update::set("role", r)),
    ]
    .into_iter()
    .flatten(),
);

User::update_by_id(client, KeyId::pk(user_id), update)
    .exists()
    .await?;
```

### 3.11 Update — Atomic Counters

```rust
// Increment: SET clicks = clicks + 1
User::update_by_id(client, KeyId::pk(user_id), Update::increment("login_count", 1))
    .exists()
    .await?;

// Decrement: SET credits = credits - 5
User::update_by_id(client, KeyId::pk(user_id), Update::decrement("credits", 5))
    .exists()
    .await?;

// Init + increment: SET enrollments = if_not_exists(enrollments, 0) + 1
User::update_by_id(client, KeyId::pk(user_id), Update::init_increment("enrollments", 0, 1))
    .exists()
    .await?;
```

**Raw SDK equivalent for init_increment:**
```rust
// .update_expression("SET #enrollments = if_not_exists(#enrollments, :zero) + :one")
// .expression_attribute_names("#enrollments", "enrollments")
// .expression_attribute_values(":zero", AttributeValue::N("0".to_owned()))
// .expression_attribute_values(":one", AttributeValue::N("1".to_owned()))
```

### 3.12 Update — return_new / return_none

Control what DynamoDB returns after mutation:

```rust
// Return the updated item (default for update_by_id):
let updated_user: User = User::update_by_id(client, KeyId::pk(user_id), update)
    .exists()
    .await?;

// Return nothing (skip deserialization cost):
User::update_by_id(client, KeyId::pk(user_id), update)
    .exists()
    .return_none()
    .await?;  // Result<()>

// Instance update returning the new item:
let updated: User = user.update(client, update)
    .condition(some_condition)
    .return_new()
    .await?;
```

### 3.13 Update — Custom Condition

```rust
// Update only if status matches:
User::update_by_id(client, KeyId::pk(user_id), Update::set("role", "instructor"))
    .condition(Condition::eq("role", "student"))
    .await?;

// Optimistic concurrency — check balance before deducting:
User::update_by_id(
    client,
    KeyId::pk(user_id),
    Update::set("balance", new_balance),
)
    .condition(
        User::exists()
            & Condition::eq("balance", old_balance),
    )
    .await?;
```

---

## 4. Query Operations

### 4.1 Query — Partition Key Only

Fetch all items under a partition (e.g. all enrollments for a user):

```rust
let enrollments /* : Vec<Enrollment> */ =
    Enrollment::query(client, Enrollment::key_condition(user_id))
        .all()
        .await?;
```

`key_condition(pk_id)` generates the appropriate `KeyCondition` from the type's
`HasAttribute<PK>` impl.

### 4.2 Query — SK begins_with

Fetch a subset of items by sort key prefix:

```rust
let assignments /* : Vec<Assignment> */ =
    Assignment::query(
        client,
        Assignment::key_condition(course_id)
            .sk_begins_with("ASSIGN#"),
    )
        .all()
        .await?;
```

### 4.3 Query — All (Constant PK)

For types with a constant partition key (singletons or collections with fixed PK):

```rust
let all_configs /* : Vec<PlatformConfig> */ =
    PlatformConfig::query_all(client).all().await?;
```

`query_all` is available when the type has `HasConstAttribute<PK>`.

### 4.4 Query — Index

Query a Global Secondary Index with a key condition:

```rust
use dynamodb_facade::KeyCondition;

let users_by_email /* : Vec<User> */ =
    User::query_index::<EmailIndex>(
        client,
        KeyCondition::pk(email_address),
    )
        .all()
        .await?;

// Composite Index — PK + SK condition:
let user_by_id_and_type /* : Vec<User> */ =
    User::query_index::<IdTypeIndex>(
        client,
        KeyCondition::pk(search_id).sk_eq("USER"),
    )
        .all()
        .await?;
```

**Raw-level index query (when entity type doesn't match):**
```rust
use dynamodb_facade::QueryRequest;

let results /* : Vec<Item<PlatformTable>> */ =
    QueryRequest::new_index::<EmailIndex>(
        client,
        KeyCondition::pk(email.to_string()),
    )
        .all()
        .await?;
```

### 4.5 Query — Index (All Items of a Type)

Query all items of a given type via the TypeIndex:

```rust
let all_users /* : Vec<User> */ =
    User::query_all_index::<TypeIndex>(client).all().await?;
```
`query_all_index` is available when the type has `HasConstAttribute` for the PK of the queried index (`HasConstAttribute<ItemType>` in this example).

---

## 5. Scan Operations

### 5.1 Scan — Typed with Filter

```rust
let active_users /* : Vec<User> */ = User::scan(client)
    .filter(Condition::eq("role", "instructor"))
    .all()
    .await?;
```

**Raw SDK equivalent:**
```rust
// Manual pagination loop with:
// .filter_expression("#role = :role")
// .expression_attribute_names("#role", "role")
// .expression_attribute_values(":role", AttributeValue::S("instructor".to_owned()))
// ... plus ExclusiveStartKey handling across pages
```

### 5.2 Scan — Raw (Untyped Dispatch)

Scan all items and dispatch by type discriminator:

```rust
use dynamodb_facade::ScanRequest;

let items /* : Vec<Item<PlatformTable>> */ = ScanRequest::<PlatformTable>::new(client)
    .all()
    .await?;

for item in items {
    match item.attribute::<ItemType>() {
        Some("USER") => {
            let user = User::from_item(item);
            // ...
        }
        Some("ENROLLMENT") => {
            let enrollment = Enrollment::from_item(item);
            // ...
        }
        _ => { /* skip unknown types */ }
    }
}
```

Scan with a PK prefix filter:

```rust
let user_items /* : Vec<Item<PlatformTable>> */ = ScanRequest::<PlatformTable>::new(client)
    .filter(Condition::begins_with(PK::NAME, "USER#"))
    .all()
    .await?;
```

---

## 6. Condition Expressions

### 6.1 Equality / Inequality / Comparison

```rust
Condition::eq("role", "admin")          // role = :val
Condition::ne("status", "archived")     // status <> :val
Condition::lt("progress", 0.5)          // progress < :val
Condition::le("credits", 100)           // credits <= :val
Condition::gt("score", 80)              // score > :val
Condition::ge("enrolled_at", cutoff_ts) // enrolled_at >= :val
```

### 6.2 exists / not_exists

```rust
// Attribute-level:
Condition::exists("email")              // attribute_exists(email)
Condition::not_exists("deleted_at")     // attribute_not_exists(deleted_at)

// Item-level (PK existence based on the TableDefinition's PK):
User::exists()                          // attribute_exists(PK)
User::not_exists()                      // attribute_not_exists(PK)
```

### 6.3 begins_with / contains

```rust
Condition::begins_with(PK::NAME, "USER#")   // begins_with(PK, :prefix)
Condition::contains("tags", "rust")          // contains(tags, :val)
```

### 6.4 between / is_in

```rust
Condition::between("score", 60, 100)    // score BETWEEN :low AND :high
Condition::is_in("role", ["admin", "instructor"])  // role IN (:v1, :v2)
```

### 6.5 size_cmp

Compare the size of an attribute (string length, list length, etc.):

```rust
Condition::size_gt("tags", 0)       // size(tags) > 0
Condition::size_ge("email", 5)      // size(email) >= 5
Condition::size_lt("content", 1000) // size(content) < 1000
```

### 6.6 Boolean Operators (AND / OR / NOT)

```rust
// AND via &
let cond = User::exists() & Condition::eq("role", "student");

// OR via |
let cond = User::not_exists() | Condition::lt(Expiration::NAME, now_ts);

// NOT via !
let cond = !Condition::eq("status", "archived");

// Complex composition:
let cond = User::exists()
    & (Condition::not_exists("email")
        | (Condition::exists("email") & Condition::not_exists("email_verified")));
```

### 6.7 Variadic and / or

Combine a collection of conditions:

```rust
let cond = Condition::and([
    Condition::eq("status", "draft"),
    Condition::size_cmp("content", Comparison::Gt, 0),
    Condition::size_cmp("title", Comparison::Gt, 0),
    Condition::exists("author_id"),
]);
```

---

## 7. Update Expressions

### 7.1 set / remove

```rust
Update::set("name", "Alice")           // SET #name = :val
Update::set("score", 95.5)             // SET #score = :val
Update::set("verified", true)           // SET #verified = :val
Update::remove("temporary_field")       // REMOVE #temporary_field
Update::remove("tags[2]")               // REMOVE #tags[2]
```

### 7.2 increment / decrement

```rust
Update::increment("view_count", 1)      // SET #view_count = #view_count + :val
Update::decrement("credits", 10)        // SET #credits = #credits - :val
```

### 7.3 init_increment / init_decrement

Safely initialize-and-increment (no prior value required):

```rust
// SET #count = if_not_exists(#count, :zero) + :one
Update::init_increment("enrollment_count", 0, 1)

// SET #balance = if_not_exists(#balance, :initial) - :amount
Update::init_decrement("balance", 1000, 50)
```

### 7.4 set_custom with UpdateSetRhs

Advanced SET expressions:

```rust
use dynamodb_facade::UpdateSetRhs;

// SET old_score = if_not_exists(score, 0)
Update::set_custom("old_score", UpdateSetRhs::if_not_exists("score", 0))

// SET score = other_score + bonus
Update::set_custom("score", UpdateSetRhs::attr("other_score") + UpdateSetRhs::value(10))

// SET score = base_score - penalty
Update::set_custom("score", UpdateSetRhs::attr("base_score") - UpdateSetRhs::value(5))
```

### 7.5 list_append / list_prepend

```rust
Update::list_append("tags", to_attribute_value(&["new_tag"]))
Update::list_prepend("notifications", to_attribute_value(&[notification]))
```

### 7.6 add / delete (Sets)

For DynamoDB Set types (SS, NS, BS):

```rust
Update::add("tag_set", AsSet(vec!["rust".to_owned()]).into_attribute_value())
Update::delete("tag_set", AsSet(vec!["old_tag".to_owned()]).into_attribute_value())
```

### 7.7 and / combine / try_combine

```rust
// Chain two updates:
let update = Update::set("name", "Alice")
    .and(Update::set("email", "alice@example.com"));

// Combine from iterator (panics if empty):
let update = Update::combine([
    name.map(|n| Update::set("name", n)),
    email.map(|e| Update::set("email", e)),
    should_clear_bio.then(|| Update::remove("bio")),
].into_iter().flatten());

// Safe version — returns None if iterator is empty:
let update: Option<Update<'_>> = Update::try_combine([
    name.map(|n| Update::set("name", n)),
].into_iter().flatten());
```

---

## 8. Key Conditions

Key conditions restrict query results based on partition key and (optionally)
sort key:

```rust
use dynamodb_facade::KeyCondition;

// PK only (for composite-key tables/indexes, returns all SK values):
KeyCondition::pk(user_id_string)

// PK + SK exact match:
KeyCondition::pk(user_id_string).sk_eq("USER")

// PK + SK prefix:
KeyCondition::pk(user_id_string).sk_begins_with("ENROLL#")

// PK + SK range:
KeyCondition::pk(user_id_string).sk_between("ASSIGN#A", "ASSIGN#Z")

// PK + SK comparison:
KeyCondition::pk(user_id_string).sk_gt("ENROLL#2024-01-01")
KeyCondition::pk(user_id_string).sk_le("ASSIGN#999")

// Generated from item type (uses HasAttribute<PK> to build the PK value):
Enrollment::key_condition(user_id)
Enrollment::key_condition(user_id).sk_begins_with("ENROLL#2024")

// Works for indexes too:
User::index_key_condition::<EmailIndex>(user_email)
```

Typestate prevents calling SK methods on simple-key schemas at compile time.

```rust
// Will fail to compile because EmailIndex have no SortKey:
User::index_key_condition::<EmailIndex>(user_email).sk_begins_with("EMAIL#")
```
---

## 9. Batch Operations

### 9.1 Batch Put

```rust
use dynamodb_facade::{DynamoDBItemBatchOp, dynamodb_batch_write};

let requests: Vec<_> = new_enrollments.iter()
    .map(|e| e.batch_put())
    .collect();

dynamodb_batch_write::<PlatformTable>(client, requests).await?;
```

`dynamodb_batch_write` automatically chunks into 25-item batches, runs in
parallel, and retries unprocessed items (up to 5 times with backoff).

### 9.2 Batch Delete

```rust
let requests: Vec<_> = enrollments.iter()
    .map(|e| e.batch_delete())
    .collect();

dynamodb_batch_write::<PlatformTable>(client, requests).await?;
```

Or by ID without loading the item:

```rust
let requests: Vec<_> = enrollment_keys.iter()
    .map(|key_id| Enrollment::batch_delete_by_id(key_id))
    .collect();

dynamodb_batch_write::<PlatformTable>(client, requests).await?;
```

### 9.3 Batch Mixed (Put + Delete)

```rust
use dynamodb_facade::batch_delete;

let requests: Vec<_> = items.into_iter()
    .filter_map(|item| match item.attribute::<ItemType>() {
        Some("USER") => {
            let mut user = User::from_item(item);
            user.login_count = 0;
            Some(user.batch_put())
        }
        Some("ENROLLMENT") => Some(batch_delete(item.into_key_only())),
        _ => None,
    })
    .collect();

dynamodb_batch_write::<PlatformTable>(client, requests).await?;
```

**Raw SDK equivalent (batch write with 25-item chunking + retry):**
```rust
// ~50 lines: manual WriteRequest::builder().put_request(...) / .delete_request(...),
// chunks(25), tokio::spawn per chunk, UnprocessedItems retry loop
```

---

## 10. Transactions

Transactions use the native `aws_sdk_dynamodb` `transact_write_items()` builder,
but facade types generate each `TransactWriteItem` with type-safe conditions.

### 10.1 Transact Put + Update

Create an enrollment and atomically increment user's enrollment count:

```rust
use dynamodb_facade::DynamoDBItemTransactOp;

client
    .transact_write_items()
    .transact_items(
        enrollment.transact_put()
            .not_exists()
            .build(),
    )
    .transact_items(
        User::transact_update_by_id(
            KeyId::pk(user_id),
            Update::init_increment("enrollment_count", 0, 1),
        )
            .condition(
                User::exists()
                    & (Condition::not_exists("enrollment_count")
                        | Condition::lt("enrollment_count", max_enrollments)),
            )
            .build(),
    )
    .send()
    .await?;
```

### 10.2 Transact Delete + Update

Remove an enrollment and decrement user's count:

```rust
client
    .transact_write_items()
    .transact_items(
        Enrollment::transact_delete_by_id(KeyId::pk(user_id).sk(course_id))
            .exists()
            .build(),
    )
    .transact_items(
        User::transact_update_by_id(
            KeyId::pk(user_id),
            Update::decrement("enrollment_count", 1),
        )
            .condition(Condition::exists("enrollment_count"))
            .build(),
    )
    .send()
    .await?;
```

### 10.3 Transact Delete + Put (Swap)

Atomically replace one enrollment with another:

```rust
client
    .transact_write_items()
    .transact_items(
        old_enrollment.transact_delete()
            .condition(
                Enrollment::exists()
                    & Condition::not_exists("completed_at"),
            )
            .build(),
    )
    .transact_items(
        new_enrollment.transact_put()
            .not_exists()
            .build(),
    )
    .send()
    .await?;
```

### 10.4 Transact Condition Check

Include a pure condition check (no mutation) in a transaction:

```rust
client
    .transact_write_items()
    .transact_items(
        user.transact_condition(
            User::exists() & Condition::eq("role", "admin"),
        )
        .build(),
    )
    .transact_items(
        dangerous_operation.transact_put()
            .not_exists()
            .build(),
    )
    .send()
    .await?;
```

### Complex Transaction: Multi-Update with Optimistic Concurrency

```rust
let transaction = client.transact_write_items();

let transaction = transaction.transact_items(
    user.transact_update(Update::combine([
        Update::set("balance", new_balance),
        Update::increment("purchase_count", 1),
        Update::remove("pending_order"),
    ]))
        .condition(
            Condition::eq("secret", secret)
                & Condition::eq("balance", old_balance)
                & Condition::exists("pending_order"),
        )
        .build(),
);

let transaction = transaction.transact_items(
    course.transact_update(Update::increment("enrollment_count", 1))
        .exists()
        .build(),
);

transaction.send().await?;
```

---

## 11. Error Handling

The facade provides a unified `Error` enum:

```rust
use dynamodb_facade::{DynamoDBError, Error};

match result {
    Ok(user) => { /* success */ }
    Err(error) => {
        // Downcast to underlying SDK error:
        if let Some(DynamoDBError::ConditionalCheckFailedException(_)) =
            error.as_dynamodb_error()
        {
            // Handle conflict (e.g. item already exists)
        }

        // Match on facade error variants:
        match error {
            Error::DynamoDB(boxed_err) => { /* AWS SDK error */ }
            Error::Serde(serde_err) => { /* (de)serialization failure */ }
            Error::Custom(msg) => { /* custom string error */ }
            Error::Other(boxed_err) => { /* any other error */ }
        }
    }
}
```

Create custom errors:

```rust
Error::custom("Invalid enrollment state")
Error::other(some_std_error)
```

---

## 12. Item Inspection APIs

`Item<TD>` provides type-safe attribute access and key manipulation.
It is designed to *always be* a valid item for the table schema:

```rust
let item: Item<PlatformTable> = /* from any request `.raw()` */;

// Type-safe attribute extraction (returns typed reference):
let item_type: Option<&str> = item.attribute::<ItemType>();
let pk_value: &str = item.pk();          // always present
let sk_value: &str = item.sk();          // only available for CompositeKeySchema, and always present in that case

// Convert to typed struct:
let user = User::from_item(item);        // panics on schema mismatch
let user = User::try_from_item(item)?;   // returns Result

// Key manipulation:
let (key, remaining_attrs) = item.extract_key();
let key_only: Key<PlatformTable> = item.into_key_only();
let reconstructed = Item::from_key_and_attributes(key, extra_attrs);

// Custom item construction:
let minimal = Item::minimal_from(&my_item);  // only key + additional attributes defined by the DynamoDBItem trait
let enriched = minimal.with_attributes([
    ("extra_field".to_owned(), some_value.into_attribute_value()),
]);

// Raw read-only access (Deref to HashMap<String, AttributeValue>):
let raw_av = item.get("some_field");
```

---

## 13. Typestate Builder Transitions

Every operation builder uses compile-time typestates to prevent misuse. Here is
a visual summary:

**PutItemRequest:**
```
put(client)                           → PutItemRequest<Typed, ReturnNothing, NoCondition>
  .not_exists() / .condition(cond)    → ...<..., AlreadyHasCondition>   (one-shot)
  .return_old()                       → ...<..., Return<Old>, ...>
  .return_none()                      → ...<..., ReturnNothing, ...>    (from Return<Old>)
  .raw()                              → ...<Raw, ...>                   (one-way)
  .await / .execute()                 → terminal
```

**GetItemRequest:**
```
get(client, key_id)                   → GetItemRequest<Typed, NoProjection>
  .raw()                              → ...<Raw, ...>
  .project(projection)                → ...<Raw, AlreadyHasProjection>  (forces Raw)
  .consistent_read()                  → self
  .await / .execute()                 → terminal
```

**UpdateItemRequest:**
```
update_by_id(client, key_id, update)  → UpdateItemRequest<Typed, Return<New>, NoCondition>
  .exists() / .condition(cond)        → ...<..., AlreadyHasCondition>
  .return_old()                       → ...<..., Return<Old>, ...>
  .return_new()                       → ...<..., Return<New>, ...>
  .return_none()                      → ...<..., ReturnNothing, ...>
  .raw()                              → ...<Raw, ...>
  .await / .execute()                 → terminal
```

**QueryRequest / ScanRequest:**
```
query(client, key_cond)               → QueryRequest<Typed, NoFilter, NoProjection>
  .filter(cond)                       → ...<..., AlreadyHasFilter, ...>
  .project(projection)                → ...<Raw, ..., AlreadyHasProjection>
  .limit(n) / .scan_index_forward(b)  → self
  .all()                              → Vec<T> or Vec<Item<TD>>
  .stream()                           → impl Stream<Item=Result<T>> or ...Item<TD>>
```

**TransactWriteItem builders:**
```
transact_put() / transact_delete() / transact_update()
  .condition(cond) / .exists() / .not_exists()  → ...<AlreadyHasCondition>
  .build()                                       → TransactWriteItem
```

Key invariant: calling `.condition()` twice, or `.filter()` twice, is a
**compile-time error** — the typestate transitions consume the `NoCondition` /
`NoFilter` marker and produce `AlreadyHasCondition` / `AlreadyHasFilter`, which
does not have the `.condition()` / `.filter()` method.

---

## Summary — Why dynamodb-facade?

| Concern | Raw `aws-sdk-dynamodb` | `dynamodb-facade` |
|---|---|---|
| **Key construction** | Manual `HashMap<String, AV>`, format strings | `KeyId::pk(id).sk(id)`, type-checked |
| **Expressions** | Raw strings (`"SET #n = :v"`), separate name/value maps | `Update::set("n", v)`, auto-managed placeholders |
| **Conditions** | String concatenation, manual `:placeholder` tracking | `Condition::eq(...)`, `&` / `\|` operators |
| **Serialization** | Manual `serde_dynamo` calls, `AttributeValue::S(...)` | Automatic via `DynamoDBItem` trait |
| **Pagination** | Hand-written `ExclusiveStartKey` loops | `.all()` auto-paginates, `.stream()` for lazy |
| **Batch writes** | Manual 25-item chunking, retry loop | `dynamodb_batch_write()` handles everything |
| **Transactions** | Raw `TransactWriteItem::builder()` | `.transact_put().condition(...).build()` |
| **Type safety** | Runtime errors on wrong key/expression | Compile-time typestate enforcement |
| **Duplicate calls** | Silent runtime bugs | `.condition()` twice = compile error |
| **Single-table** | No built-in support for type dispatch | `item.attribute::<ItemType>()` + `T::from_item()` |
