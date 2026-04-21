use core::fmt;
use std::marker::PhantomData;

use crate::{AttributeDefinition, CompositeKeySchema, Condition, IntoAttributeValue, KeySchema};

mod sealed_traits {
    pub trait KeyConditionStateSeal {}
}

crate::utils::impl_sealed_marker_types!(
    /// Sealed typestate marker for [`KeyCondition`] build stages.
    ///
    /// This trait is sealed and cannot be implemented outside this crate. The two
    /// implementing types are hidden from public docs:
    ///
    /// - `PkOnly` — only the partition key has been set; sort key methods are
    ///   available for composite-key schemas.
    /// - `WithSk` — a sort key condition has been added; no further SK methods
    ///   are available.
    ///
    /// You may encounter this trait as a bound on [`KeyCondition`]'s type
    /// parameter `S`, but you never need to name the concrete marker types
    /// directly.
    KeyConditionState,
    sealed_traits::KeyConditionStateSeal;
    #[doc(hidden)]
    PkOnly,
    #[doc(hidden)]
    WithSk
);

/// Builder for DynamoDB key condition expressions.
///
/// `KeyCondition` builds the `KeyConditionExpression` used in Query
/// operations. It enforces that:
///
/// 1. A partition key equality condition is always provided (via [`pk`](KeyCondition::pk)).
/// 2. Sort key methods (`sk_eq`, `sk_begins_with`, etc.) are only available
///    for composite-key schemas (tables or indexes with a sort key).
/// 3. At most one sort key condition can be added, as required by the
///    DynamoDB API.
///
/// In practice you rarely name `KS` or `S` directly — they are inferred from
/// the item type's key schema when using the generated `T::key_condition(pk_id)`
/// helper.
///
/// # Examples
///
/// PK-only query (all enrollments for a user):
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{KeyCondition, TableSchema};
///
/// // Targets all items with PK = "USER#user-1"
/// let kc = KeyCondition::<TableSchema<PlatformTable>>::pk("USER#user-1");
/// assert_eq!(format!("{kc}"), r#"PK = S("USER#user-1")"#);
///
/// // Or you may prefer for the same result:
/// use dynamodb_facade::DynamoDBItemOp;
/// let kc = User::key_condition("user-1");
/// assert_eq!(format!("{kc}"), r#"PK = S("USER#user-1")"#);
/// ```
///
/// PK + SK prefix query:
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{KeyCondition, TableSchema};
///
/// let kc = KeyCondition::<TableSchema<PlatformTable>>::pk("USER#user-1")
///     .sk_begins_with("ENROLL#");
/// assert_eq!(format!("{kc}"), r#"(PK = S("USER#user-1") AND begins_with(SK, S("ENROLL#")))"#);
///
/// // Or you may prefer for the same result:
/// use dynamodb_facade::DynamoDBItemOp;
/// let kc = Enrollment::key_condition("user-1").sk_begins_with("ENROLL#");
/// assert_eq!(format!("{kc}"), r#"(PK = S("USER#user-1") AND begins_with(SK, S("ENROLL#")))"#);
/// ```
#[derive(Debug, Clone)]
#[must_use = "key condition does nothing until applied to a request"]
pub struct KeyCondition<'a, KS: KeySchema, S: KeyConditionState = PkOnly>(
    Condition<'a>,
    PhantomData<(KS, S)>,
);

// -- Initial state: only pk_eq available --------------------------------------

impl<'a, KS: KeySchema> KeyCondition<'a, KS> {
    /// Creates a key condition with a partition key equality constraint: `PK = value`.
    ///
    /// This is the required starting point for all key conditions. The partition
    /// key attribute name is taken from the key schema `KS` at compile time.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{KeyCondition, TableSchema};
    ///
    /// let kc = KeyCondition::<TableSchema<PlatformTable>>::pk("USER#user-1");
    /// assert_eq!(format!("{kc}"), r#"PK = S("USER#user-1")"#);
    ///
    /// // Or you may prefer for the same result:
    /// use dynamodb_facade::DynamoDBItemOp;
    /// let kc = User::key_condition("user-1");
    /// assert_eq!(format!("{kc}"), r#"PK = S("USER#user-1")"#);
    /// ```
    pub fn pk(value: impl IntoAttributeValue) -> Self {
        KeyCondition(Condition::eq(KS::PartitionKey::NAME, value), PhantomData)
    }
}
// -- PK state: SK methods available (gated by CompositeKeySchema) -------------

impl<'a, KS: CompositeKeySchema> KeyCondition<'a, KS> {
    /// Adds a sort key equality constraint: `SK = value`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    /// let kc = Enrollment::key_condition("user-1").sk_eq("ENROLL#course-42");
    /// assert_eq!(format!("{kc}"), r#"(PK = S("USER#user-1") AND SK = S("ENROLL#course-42"))"#);
    /// ```
    pub fn sk_eq(self, value: impl IntoAttributeValue) -> KeyCondition<'a, KS, WithSk> {
        KeyCondition(
            self.0 & Condition::eq(KS::SortKey::NAME, value),
            PhantomData,
        )
    }

    /// Adds a sort key less-than constraint: `SK < value`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// let kc = Enrollment::key_condition("user-1").sk_lt("ENROLL#z");
    /// assert_eq!(format!("{kc}"), r#"(PK = S("USER#user-1") AND SK < S("ENROLL#z"))"#);
    /// ```
    pub fn sk_lt(self, value: impl IntoAttributeValue) -> KeyCondition<'a, KS, WithSk> {
        KeyCondition(
            self.0 & Condition::lt(KS::SortKey::NAME, value),
            PhantomData,
        )
    }

    /// Adds a sort key less-than-or-equal constraint: `SK <= value`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// let kc = Enrollment::key_condition("user-1").sk_le("ENROLL#z");
    /// assert_eq!(format!("{kc}"), r#"(PK = S("USER#user-1") AND SK <= S("ENROLL#z"))"#);
    /// ```
    pub fn sk_le(self, value: impl IntoAttributeValue) -> KeyCondition<'a, KS, WithSk> {
        KeyCondition(
            self.0 & Condition::le(KS::SortKey::NAME, value),
            PhantomData,
        )
    }

    /// Adds a sort key greater-than constraint: `SK > value`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// let kc = Enrollment::key_condition("user-1").sk_gt("ENROLL#2024");
    /// assert_eq!(format!("{kc}"), r#"(PK = S("USER#user-1") AND SK > S("ENROLL#2024"))"#);
    /// ```
    pub fn sk_gt(self, value: impl IntoAttributeValue) -> KeyCondition<'a, KS, WithSk> {
        KeyCondition(
            self.0 & Condition::gt(KS::SortKey::NAME, value),
            PhantomData,
        )
    }

    /// Adds a sort key greater-than-or-equal constraint: `SK >= value`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// let kc = Enrollment::key_condition("user-1").sk_ge("ENROLL#2024");
    /// assert_eq!(format!("{kc}"), r#"(PK = S("USER#user-1") AND SK >= S("ENROLL#2024"))"#);
    /// ```
    pub fn sk_ge(self, value: impl IntoAttributeValue) -> KeyCondition<'a, KS, WithSk> {
        KeyCondition(
            self.0 & Condition::ge(KS::SortKey::NAME, value),
            PhantomData,
        )
    }

    /// Adds a sort key range constraint: `SK BETWEEN low AND high` (inclusive).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// let kc = Enrollment::key_condition("user-1").sk_between("ENROLL#2024-01", "ENROLL#2024-12");
    /// assert_eq!(
    ///     format!("{kc}"),
    ///     r#"(PK = S("USER#user-1") AND SK BETWEEN S("ENROLL#2024-01") AND S("ENROLL#2024-12"))"#,
    /// );
    /// ```
    pub fn sk_between(
        self,
        low: impl IntoAttributeValue,
        high: impl IntoAttributeValue,
    ) -> KeyCondition<'a, KS, WithSk> {
        KeyCondition(
            self.0 & Condition::between(KS::SortKey::NAME, low, high),
            PhantomData,
        )
    }

    /// Adds a sort key prefix constraint: `begins_with(SK, prefix)`.
    ///
    /// This is the most common sort key condition for hierarchical single-table
    /// designs, where sort keys are prefixed by entity type (e.g. `"ENROLL#"`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemOp;
    ///
    /// let kc = Enrollment::key_condition("user-1").sk_begins_with("ENROLL#2025");
    /// assert_eq!(
    ///     format!("{kc}"),
    ///     r#"(PK = S("USER#user-1") AND begins_with(SK, S("ENROLL#2025")))"#,
    /// );
    /// ```
    pub fn sk_begins_with(self, prefix: impl IntoAttributeValue) -> KeyCondition<'a, KS, WithSk> {
        KeyCondition(
            self.0 & Condition::begins_with(KS::SortKey::NAME, prefix),
            PhantomData,
        )
    }
}

// -- Display ------------------------------------------------------------------

impl<KS: KeySchema, S: KeyConditionState> fmt::Display for KeyCondition<'_, KS, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// -- ApplyKeyCondition impls --------------------------------------------------

use super::{ApplyKeyCondition, KeyConditionableBuilder};

impl<B: KeyConditionableBuilder, KS: KeySchema, S: KeyConditionState> ApplyKeyCondition<B>
    for KeyCondition<'_, KS, S>
{
    fn apply_key_condition(self, builder: B) -> B {
        self.0.apply_key_condition(builder)
    }
}
