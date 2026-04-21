//! Shared domain types for doc examples and integration tests.
//!
//! This module defines an **Online Learning Platform** stored in a single
//! DynamoDB table (mono-table pattern). It provides concrete attribute
//! definitions, table/index definitions, and item types that are referenced
//! throughout the crate's documentation examples.
//!
//! # Domain overview
//!
//! All entities live in `PlatformTable` (composite key `PK` + `SK`):
//!
//! | Entity | PK | SK |
//! |---|---|---|
//! | [`PlatformConfig`] | `"PLATFORM_CONFIG"` | `"PLATFORM_CONFIG"` |
//! | [`User`] | `"USER#<uuid>"` | `"USER"` |
//! | [`Enrollment`] | `"USER#<uuid>"` | `"ENROLL#<uuid>"` |
//!
//! GSIs:
//! - [`TypeIndex`] — partition key `_TYPE`; query all items of a given type
//! - [`EmailIndex`] — partition key `email`; look up any item by email

use serde::{Deserialize, Serialize};

use super::{Item, KeyId, NumberAttribute, StringAttribute};

// ---------------------------------------------------------------------------
// Attribute definitions
// ---------------------------------------------------------------------------

crate::attribute_definitions! {
    /// Partition key for the platform mono-table.
    PK { "PK": StringAttribute }

    /// Sort key for the platform mono-table.
    SK { "SK": StringAttribute }

    /// Item type discriminator (single-table design).
    ItemType { "_TYPE": StringAttribute }

    /// TTL attribute for expiring items.
    Expiration { "expiration_timestamp": NumberAttribute }

    /// Email attribute, used as a GSI partition key.
    Email { "email": StringAttribute }
}

// ---------------------------------------------------------------------------
// Table definition
// ---------------------------------------------------------------------------

crate::table_definitions! {
    /// The platform mono-table with composite key (PK + SK).
    PlatformTable {
        type PartitionKey = PK;
        type SortKey = SK;
        fn table_name() -> String {
            std::env::var("TABLE_NAME").unwrap_or_else(|_| "platform".to_owned())
        }
    }
}

// ---------------------------------------------------------------------------
// Index definitions
// ---------------------------------------------------------------------------

crate::index_definitions! {
    /// GSI on item type — query all items of a given type.
    #[table = PlatformTable]
    TypeIndex {
        type PartitionKey = ItemType;
        fn index_name() -> String { "iType".to_owned() }
    }

    /// GSI on email — look up any item by email address.
    #[table = PlatformTable]
    EmailIndex {
        type PartitionKey = Email;
        fn index_name() -> String { "iEmail".to_owned() }
    }
}

// ---------------------------------------------------------------------------
// Item types
// ---------------------------------------------------------------------------

/// Platform-wide configuration (singleton item).
///
/// Stored at `PK = "PLATFORM_CONFIG"`, `SK = "PLATFORM_CONFIG"`.
/// Accessed via `PlatformConfig::get(client, KeyId::NONE)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub max_enrollments: u32,
    pub maintenance_mode: bool,
}

crate::dynamodb_item! {
    #[table = PlatformTable]
    PlatformConfig {
        #[partition_key]
        PK { const VALUE: &'static str = "PLATFORM_CONFIG"; }
        #[sort_key]
        SK { const VALUE: &'static str = "PLATFORM_CONFIG"; }
        ItemType { const VALUE: &'static str = "PLATFORM_CONFIG"; }
    }
}

/// A registered user on the platform.
///
/// Stored at `PK = "USER#<id>"`, `SK = "USER"`.
/// Accessed via `User::get(client, KeyId::pk(user_id))`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
}

crate::dynamodb_item! {
    #[table = PlatformTable]
    User {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        ItemType { const VALUE: &'static str = "USER"; }
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
    }
}

/// A user's enrollment in a course.
///
/// Stored at `PK = "USER#<user_id>"`, `SK = "ENROLL#<course_id>"`.
/// Accessed via `Enrollment::get(client, KeyId::pk(user_id).sk(course_id))`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enrollment {
    pub user_id: String,
    pub course_id: String,
    pub enrolled_at: u64,
    pub progress: f64,
}

crate::dynamodb_item! {
    #[table = PlatformTable]
    Enrollment {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.user_id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        SK {
            fn attribute_id(&self) -> &'id str { &self.course_id }
            fn attribute_value(id) -> String { format!("ENROLL#{id}") }
        }
        ItemType { const VALUE: &'static str = "ENROLLMENT"; }
    }
}

// ---------------------------------------------------------------------------
// Helper constructors for doc examples
// ---------------------------------------------------------------------------

/// Returns a sample [`PlatformConfig`] for use in doc examples.
pub fn sample_config() -> PlatformConfig {
    PlatformConfig {
        max_enrollments: 10,
        maintenance_mode: false,
    }
}

/// Returns a sample [`User`] for use in doc examples.
pub fn sample_user() -> User {
    User {
        id: "user-1".to_owned(),
        name: "Alice".to_owned(),
        email: "alice@example.com".to_owned(),
        role: "student".to_owned(),
    }
}

/// Returns a sample [`Enrollment`] for use in doc examples.
pub fn sample_enrollment() -> Enrollment {
    Enrollment {
        user_id: "user-1".to_owned(),
        course_id: "course-42".to_owned(),
        enrolled_at: 1_700_000_000,
        progress: 0.0,
    }
}

/// Returns a [`KeyId`] for the sample user.
pub fn sample_user_key_id() -> KeyId<&'static str, super::NoId> {
    KeyId::pk("user-1")
}

/// Returns a [`KeyId`] for the sample enrollment.
pub fn sample_enrollment_key_id() -> KeyId<&'static str, &'static str> {
    KeyId::pk("user-1").sk("course-42")
}

/// Returns a sample [`Item<PlatformTable>`] representing the sample user.
pub fn sample_user_item() -> Item<PlatformTable> {
    use super::DynamoDBItem;
    sample_user().to_item()
}
