mod key;
mod key_id;

use core::{fmt, marker::PhantomData, ops::Deref};
use std::collections::HashMap;

use serde::{Serialize, de::DeserializeOwned};

use super::{
    AttributeDefinition, AttributeList, AttributeValue, AttributeValueRef, CompositeKey,
    CompositeKeySchema, HasTableKeyAttributes, KeySchema, KeySchemaKind, Result, SimpleKey,
    SimpleKeySchema, TableDefinition,
};

pub use key::{Key, KeyBuilder};
pub use key_id::*;

/// Core trait for types stored in a DynamoDB table.
///
/// Implementing this trait connects a Rust type to a specific [`TableDefinition`],
/// enabling typed CRUD operations, expression building, and (de)serialization. Most of
/// the time you implement it via the [`dynamodb_item!`](crate::dynamodb_item) macro
/// rather than by hand.
///
/// # Why `TD` is a generic parameter
///
/// `TD` is a **type parameter** rather than an associated type so that a single
/// type can implement this trait for multiple tables. This supports scenarios
/// where the same domain type must live in more than one table — for example,
/// shared types across a primary and an archive table, or migration logic that
/// reads from one table and writes to another with different key mappings.
///
/// # Methods
///
/// The three associated methods cover the full round-trip between a Rust value and
/// a DynamoDB item:
///
/// - [`to_item`](DynamoDBItem::to_item) — serialize `self` into an [`Item<TD>`]
/// - [`try_from_item`](DynamoDBItem::try_from_item) — fallibly deserialize an [`Item<TD>`]
/// - [`from_item`](DynamoDBItem::from_item) — infallibly deserialize (panics on mismatch)
///
/// # Associated Types
///
/// `AdditionalAttributes` lists the non-key attributes that are written by
/// [`Item::minimal_from`] (e.g. type discriminators such as `_TYPE`). Usualy,
/// additional attributes are there to ensure the resulting DynamoDB item is included
/// in some Local or Global Secondary Indexes.
///
/// Note that the table Key attributes are always included automatically regardless of
/// the content of `AdditionalAttributes`.
///
/// # Blanket Implementations
///
/// Implementing [`DynamoDBItem<TD>`] is the **gateway to the entire typed operation
/// API**. Three additional traits are automatically provided via blanket
/// implementations:
///
/// - [`DynamoDBItemOp<TD>`](crate::DynamoDBItemOp) — single-item CRUD and collection operations:
///   `get`, `put`, `delete`, `update`, `query`, `scan`
/// - [`DynamoDBItemBatchOp<TD>`](crate::DynamoDBItemBatchOp) — batch write requests:
///   `batch_put`, `batch_delete`
/// - [`DynamoDBItemTransactOp<TD>`](crate::DynamoDBItemTransactOp) — transactional requests:
///   `transact_put`, `transact_delete`, `transact_update`, `transact_condition`
///
/// # Examples
///
/// Serializing a user to a DynamoDB item and back:
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use serde::{Deserialize, Serialize};
/// use dynamodb_facade::{dynamodb_item, DynamoDBItem, DynamoDBItemOp, KeyId};
///
/// #[derive(Serialize, Deserialize)]
/// pub struct User {
///     pub id: String,
///     pub name: String,
///     pub email: String,
///     pub role: String,
/// }
///
/// dynamodb_item! {
///     #[table = PlatformTable]
///     User {
///         #[partition_key]
///         PK {
///             fn attribute_id(&self) -> &'id str { &self.id }
///             fn attribute_value(id) -> String { format!("USER#{id}") }
///         }
///         #[sort_key]
///         SK { const VALUE: &'static str = "USER"; }
///     }
/// }
///
/// fn _assert_dynamodb_item<DBI: DynamoDBItem<PlatformTable> + DynamoDBItemOp<PlatformTable>>() {}
/// _assert_dynamodb_item::<User>();
///
/// # async fn example(cclient: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
/// let user = User {
///     id: "user-1".to_owned(),
///     name: "Alice".to_owned(),
///     email: "alice@example.com".to_owned(),
///     role: "student".to_owned(),
/// };
///
/// # let client = cclient.clone();
/// // Put the User in DynamoDB if it does not already exist
/// user.put(client).not_exists().await?;
///
/// # let client = cclient.clone();
/// // Retrieve an existing user
/// let existing /* : Option<User> */ = User::get(client, KeyId::pk("user-2")).await?;
///
/// # let client = cclient.clone();
/// // Delete the user, if it exist
/// if let Some(u) = existing {
///     u.delete(client).exists().await?;
/// }
/// # Ok(())
/// # }
/// ```
pub trait DynamoDBItem<TD: TableDefinition>:
    Sized + HasTableKeyAttributes<TD> + KeyBuilder<TD>
{
    type AdditionalAttributes: AttributeList<TD, Self>;

    /// Serializes `self` into an [`Item<TD>`].
    ///
    /// Combines the key attributes and
    /// [`AdditionalAttributes`](DynamoDBItem::AdditionalAttributes) produced by
    /// [`Item::minimal_from`] with the full `serde_dynamo` representation of
    /// `self`.
    ///
    /// # Panics
    ///
    /// Panics if [`serde_dynamo::to_item`] fails to serialize `self` — e.g.
    /// non-string map keys, or a [`Serialize`] impl that returns an error.
    /// Users are responsible for providing a [`Serialize`] implementation
    /// compatible with DynamoDB's attribute-value model; plain
    /// `#[derive(Serialize)]` on structs with string / number / bool / Vec /
    /// Option / nested-struct fields is always fine.
    fn to_item(&self) -> Item<TD>
    where
        Self: Serialize,
    {
        let minimal_item = Item::minimal_from(self);
        let item: HashMap<_, _> = serde_dynamo::to_item(self).expect("valid serialization");
        minimal_item.with_attributes(item)
    }

    /// Fallibly deserializes an [`Item<TD>`] into `Self`.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if [`serde_dynamo::from_item`] fails — e.g. a required
    /// attribute is missing, has an unexpected type, or cannot be decoded into
    /// the target field.
    fn try_from_item(item: Item<TD>) -> Result<Self>
    where
        Self: DeserializeOwned,
    {
        Ok(serde_dynamo::from_item(item.into_inner())?)
    }

    /// Infallibly deserializes an [`Item<TD>`] into `Self`.
    ///
    /// # Panics
    ///
    /// Panics if [`try_from_item`](DynamoDBItem::try_from_item) returns an
    /// error. Use [`try_from_item`](DynamoDBItem::try_from_item) when the
    /// item may not match the schema.
    fn from_item(item: Item<TD>) -> Self
    where
        Self: DeserializeOwned,
    {
        Self::try_from_item(item).expect("valid schema")
    }
}
/// A type-safe wrapper around a DynamoDB item for a specific table.
///
/// `Item<TD>` is a [`HashMap<String, AttributeValue>`] branded with the
/// [`TableDefinition`] `TD`. The branding enforces at compile time that items
/// from different tables are never mixed up, and it unlocks typed attribute
/// accessors such as [`pk`](Item::pk), [`sk`](Item::sk), and
/// [`attribute`](Item::attribute).
///
/// An `Item<TD>` is guaranteed to contain the table's key attributes (PK, and SK
/// for composite-key tables). This invariant is upheld by all constructors.
///
/// `Item<TD>` implements [`Deref`] to `HashMap<String, AttributeValue>`, so you
/// can call `.get("field")` and other map methods directly.
///
/// # Examples
///
/// Building an item and inspecting its attributes:
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::DynamoDBItem;
///
/// let user /* : User */ = sample_user();
/// let item /* : Item<PlatformTable> */ = user.to_item();
///
/// // Typed key accessors — always present.
/// assert_eq!(item.pk(), "USER#user-1");
/// assert_eq!(item.sk(), "USER");
///
/// // Optional typed attribute access (present for the User type).
/// let item_type: Option<&str> = item.attribute::<ItemType>();
/// assert_eq!(item_type, Some("USER"));
/// ```
///
/// Consuming the item into its raw map:
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::DynamoDBItem;
///
/// let item /* : Item<PlatformTable> */ = sample_user().to_item();
/// let raw /* : HashMap<String, AttributeValue> */ = item.into_inner();
/// assert!(raw.contains_key("PK"));
/// ```
///
/// # Equality and hashing
///
/// `Item<TD>` intentionally does not implement [`PartialEq`], [`Eq`], or
/// [`Hash`](core::hash::Hash). The backing type is a
/// [`HashMap<String, AttributeValue>`], and while attribute-value byte-equality
/// could be derived, it does not match DynamoDB's semantic equality (number
/// string normalization, set element ordering, etc.). Compare items by
/// deserializing to `T` first.
#[derive(Clone)]
pub struct Item<TD: TableDefinition>(HashMap<String, AttributeValue>, PhantomData<TD>);
impl<TD: TableDefinition> fmt::Debug for Item<TD> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

/// Shorthand for the partition key [`AttributeDefinition`] of table `TD`.
pub(crate) type PartitionKeyDefinition<TD> =
    <<TD as TableDefinition>::KeySchema as KeySchema>::PartitionKey;
/// The [`AttributeType`](crate::AttributeType) of the partition key for table `TD`.
type PartitionKeyType<TD> = <PartitionKeyDefinition<TD> as AttributeDefinition>::Type;
/// Shorthand for the sort key [`AttributeDefinition`] of composite-key table `TD`.
type SortKeyDefinition<TD> = <<TD as TableDefinition>::KeySchema as CompositeKeySchema>::SortKey;
/// The [`AttributeType`](crate::AttributeType) of the sort key for table `TD`.
type SortKeyType<TD> = <SortKeyDefinition<TD> as AttributeDefinition>::Type;

impl<TD: TableDefinition> Item<TD> {
    /// Returns the partition key value of this item.
    ///
    /// The return type is a typed reference determined by the table's partition
    /// key attribute definition (e.g. `&str` for a `StringAttribute` PK).
    ///
    /// # Panics
    ///
    /// Panics if the partition key attribute is absent. This should never happen
    /// for items produced by this crate's constructors, which always include the
    /// key attributes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItem;
    ///
    /// let item /* : Item<PlatformTable> */ = sample_user().to_item();
    /// assert_eq!(item.pk(), "USER#user-1");
    /// ```
    pub fn pk(&self) -> <PartitionKeyType<TD> as AttributeValueRef>::Ref<'_> {
        self.attribute::<PartitionKeyDefinition<TD>>()
            .expect("PK is always present")
    }

    /// Returns a typed reference to the named attribute, or `None` if absent.
    ///
    /// The attribute is identified by the type parameter `A`, which must
    /// implement [`AttributeDefinition`]. The return type is a typed reference
    /// whose concrete type is determined by `A::Type` (e.g. `&str` for
    /// `StringAttribute`, `&str` for `NumberAttribute`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItem;
    ///
    /// let item /* : Item<PlatformTable> */ = sample_user().to_item();
    ///
    /// // Present attribute:
    /// let item_type: Option<&str> = item.attribute::<ItemType>();
    /// assert_eq!(item_type, Some("USER"));
    ///
    /// // Absent attribute returns None:
    /// let expiration: Option<&str> = item.attribute::<Expiration>();
    /// assert!(expiration.is_none());
    /// ```
    pub fn attribute<A: AttributeDefinition>(
        &self,
    ) -> Option<<A::Type as AttributeValueRef>::Ref<'_>> {
        self.0
            .get(A::NAME)
            .map(<A::Type as AttributeValueRef>::attribute_value_ref)
    }

    /// Consumes the item and returns the underlying raw attribute map.
    ///
    /// Use this when you need to pass the item to code that works with the raw
    /// `aws-sdk-dynamodb` types.
    ///
    /// Beware that you will not be able to re-construct the original [`Item<TD>`].
    /// See [`Item::extract_key`] and [`Item::from_key_and_attributes`] if you want
    /// to manipulate the underlying `HashMap` and then re-create it afterward.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItem;
    ///
    /// let raw /* : HashMap<String, AttributeValue> */ = sample_user().to_item().into_inner();
    /// assert!(raw.contains_key("PK"));
    /// assert!(raw.contains_key("SK"));
    /// ```
    pub fn into_inner(self) -> HashMap<String, AttributeValue> {
        self.0
    }

    /// Wraps a raw attribute map returned by the SDK into a typed `Item<TD>`.
    pub(crate) fn from_dynamodb_response(item: HashMap<String, AttributeValue>) -> Self {
        Self(item, PhantomData)
    }

    /// Creates a minimal [`Item<TD>`] from a [`DynamoDBItem`] value.
    ///
    /// The resulting item contains only the key attributes (PK and SK for
    /// composite-key tables) and the type's
    /// [`AdditionalAttributes`](DynamoDBItem::AdditionalAttributes) (e.g. type
    /// discriminators). It does **not** include the serialized underlying type.
    ///
    /// This is mainly used when you need to implement [`DynamoDBItem::to_item`]
    /// manually.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItem, Item};
    ///
    /// let user = sample_user();
    /// let minimal = Item::minimal_from(&user);
    ///
    /// // Key attributes are present.
    /// assert_eq!(minimal.pk(), "USER#user-1");
    /// assert_eq!(minimal.sk(), "USER");
    ///
    /// // The type discriminator (AdditionalAttributes) is also present.
    /// assert_eq!(minimal.attribute::<ItemType>(), Some("USER"));
    ///
    /// // But non-key payload fields are absent.
    /// assert!(!minimal.contains_key("name"));
    /// ```
    pub fn minimal_from<DBI: DynamoDBItem<TD>>(dynamodb_item: &DBI) -> Self {
        let key = dynamodb_item.get_key();
        let additional_attributes = DBI::AdditionalAttributes::get_attributes(dynamodb_item);
        Item::from_key_and_attributes(key, additional_attributes)
    }

    /// Merges additional attributes into this item.
    ///
    /// In case of an attribute name conflict, attributes already present on
    /// the item take precedence. In other words, this method cannot overwrite
    /// existing item attributes.
    ///
    /// This is the mechanism used by [`DynamoDBItem::to_item`]'s default
    /// implementation to combine the full serde payload with the typed key
    /// attributes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItem, Item, IntoAttributeValue};
    ///
    /// let user = sample_user();
    /// let minimal = Item::minimal_from(&user);
    ///
    /// // Merge in extra attributes.
    /// let enriched = minimal.with_attributes([
    ///     ("extra".to_owned(), "hello".into_attribute_value()),
    /// ]);
    ///
    /// assert_eq!(enriched.pk(), "USER#user-1");
    /// assert!(enriched.contains_key("extra"));
    /// ```
    pub fn with_attributes(self, attributes: impl Into<HashMap<String, AttributeValue>>) -> Self {
        let mut item = attributes.into();
        item.extend(self.0);
        Self(item, PhantomData)
    }

    /// Constructs an item from a [`Key<TD>`] and an additional attribute map.
    ///
    /// The key attributes always take precedence: if `attributes` contains an
    /// entry with the same name as a key attribute, the key attribute wins.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItem, KeyBuilder, Item, IntoAttributeValue};
    ///
    /// let key = sample_user().get_key();
    /// let item: Item<PlatformTable> = Item::from_key_and_attributes(key, [
    ///     ("role".to_owned(), "instructor".into_attribute_value()),
    /// ]);
    ///
    /// assert_eq!(item.pk(), "USER#user-1");
    /// assert!(item.contains_key("role"));
    /// ```
    pub fn from_key_and_attributes(
        key: Key<TD>,
        attributes: impl Into<HashMap<String, AttributeValue>>,
    ) -> Self {
        let mut item = attributes.into();
        item.extend(key.into_inner());
        Self(item, PhantomData)
    }
}
impl<TD: TableDefinition> Item<TD>
where
    TD::KeySchema: CompositeKeySchema,
{
    /// Returns the sort key value of this item.
    ///
    /// Only available when the table uses a [`CompositeKeySchema`] (PK + SK).
    /// The return type is a typed reference determined by the table's sort key
    /// attribute definition (e.g. `&str` for a `StringAttribute` SK).
    ///
    /// # Panics
    ///
    /// Panics if the sort key attribute is absent. This should never happen for
    /// items produced by this crate's constructors, which always include the key
    /// attributes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItem;
    ///
    /// let item = sample_user().to_item();
    /// assert_eq!(item.sk(), "USER");
    ///
    /// let enrollment_item = sample_enrollment().to_item();
    /// assert_eq!(enrollment_item.sk(), "ENROLL#course-42");
    /// ```
    pub fn sk(&self) -> <SortKeyType<TD> as AttributeValueRef>::Ref<'_> {
        self.attribute::<SortKeyDefinition<TD>>()
            .expect("SK is always present")
    }
}

impl<TD: TableDefinition> Deref for Item<TD> {
    type Target = HashMap<String, AttributeValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<TD: TableDefinition> IntoIterator for Item<TD> {
    type Item = <HashMap<String, AttributeValue> as IntoIterator>::Item;
    type IntoIter = <HashMap<String, AttributeValue> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use aws_sdk_dynamodb::types::AttributeValue;

    use super::super::test_fixtures::*;
    use super::*;

    // ---------------------------------------------------------------------------
    // test_item_minimal_from_user
    // ---------------------------------------------------------------------------

    #[test]
    fn test_item_minimal_from_user() {
        let user = sample_user();
        let item = Item::<PlatformTable>::minimal_from(&user);

        // Key attributes are present with correct values.
        assert_eq!(
            item.get("PK"),
            Some(&AttributeValue::S("USER#user-1".to_owned()))
        );
        assert_eq!(item.get("SK"), Some(&AttributeValue::S("USER".to_owned())));
        // ItemType is in AdditionalAttributes (not marker_only).
        assert_eq!(
            item.get("_TYPE"),
            Some(&AttributeValue::S("USER".to_owned()))
        );
        // Email is #[marker_only] — NOT in AdditionalAttributes, so absent from minimal_from.
        assert!(!item.contains_key("email"));
        // Exactly PK + SK + _TYPE = 3 attributes.
        assert_eq!(item.len(), 3);
    }

    // ---------------------------------------------------------------------------
    // test_item_with_attributes_key_takes_precedence
    // ---------------------------------------------------------------------------

    #[test]
    fn test_item_with_attributes_key_takes_precedence() {
        let item = sample_user().to_item();
        let extra = HashMap::from([
            (
                "PK".to_owned(),
                AttributeValue::S("SHOULD_NOT_WIN".to_owned()),
            ),
            ("custom".to_owned(), AttributeValue::S("added".to_owned())),
        ]);

        let enriched = item.with_attributes(extra);

        // Key attribute wins over the conflicting extra value.
        assert_eq!(
            enriched.get("PK"),
            Some(&AttributeValue::S("USER#user-1".to_owned()))
        );
        // New attribute is present.
        assert_eq!(
            enriched.get("custom"),
            Some(&AttributeValue::S("added".to_owned()))
        );
        // Existing non-conflicting attributes are unchanged.
        assert_eq!(
            enriched.get("SK"),
            Some(&AttributeValue::S("USER".to_owned()))
        );
        assert_eq!(
            enriched.get("_TYPE"),
            Some(&AttributeValue::S("USER".to_owned()))
        );
    }

    // ---------------------------------------------------------------------------
    // test_item_from_key_and_attributes_key_takes_precedence
    // ---------------------------------------------------------------------------

    #[test]
    fn test_item_from_key_and_attributes_key_takes_precedence() {
        let key: Key<PlatformTable> = sample_user().get_key();
        let extra = HashMap::from([
            ("PK".to_owned(), AttributeValue::S("WRONG".to_owned())),
            ("other".to_owned(), AttributeValue::S("kept".to_owned())),
        ]);

        let item = Item::from_key_and_attributes(key, extra);

        // Key attribute wins over the conflicting extra value.
        assert_eq!(
            item.get("PK"),
            Some(&AttributeValue::S("USER#user-1".to_owned()))
        );
        // SK from key is present.
        assert_eq!(item.get("SK"), Some(&AttributeValue::S("USER".to_owned())));
        // Non-conflicting extra attribute is preserved.
        assert_eq!(
            item.get("other"),
            Some(&AttributeValue::S("kept".to_owned()))
        );
    }

    // ---------------------------------------------------------------------------
    // test_item_extract_key_composite
    // ---------------------------------------------------------------------------

    #[test]
    fn test_item_extract_key_composite() {
        let item = sample_enrollment().to_item();
        let (key, rest) = item.extract_key();

        let raw_key = key.into_inner();

        // Key contains PK and SK with correct values.
        assert_eq!(
            raw_key.get("PK"),
            Some(&AttributeValue::S("USER#user-1".to_owned()))
        );
        assert_eq!(
            raw_key.get("SK"),
            Some(&AttributeValue::S("ENROLL#course-42".to_owned()))
        );
        // Key contains exactly PK + SK.
        assert_eq!(raw_key.len(), 2);

        // Remaining map does NOT contain key attributes.
        assert!(!rest.contains_key("PK"));
        assert!(!rest.contains_key("SK"));

        // Remaining map contains the payload fields.
        assert!(rest.contains_key("user_id"));
        assert!(rest.contains_key("course_id"));
        assert!(rest.contains_key("enrolled_at"));
        assert!(rest.contains_key("progress"));
        // _TYPE is in AdditionalAttributes for Enrollment.
        assert!(rest.contains_key("_TYPE"));
    }

    // ---------------------------------------------------------------------------
    // test_item_extract_key_simple — local simple-key table
    // ---------------------------------------------------------------------------

    crate::attribute_definitions! {
        SimplePK { "SPK": crate::StringAttribute }
    }
    crate::table_definitions! {
        SimpleTable {
            type PartitionKey = SimplePK;
            fn table_name() -> String { "simple".to_owned() }
        }
    }
    #[derive(serde::Deserialize, serde::Serialize)]
    struct SimpleItem {
        id: String,
        value: String,
    }
    crate::dynamodb_item! {
        #[table = SimpleTable]
        SimpleItem {
            #[partition_key]
            SimplePK {
                fn attribute_id(&self) -> &'id str { &self.id }
                fn attribute_value(id) -> String { format!("ID#{id}") }
            }
        }
    }

    #[test]
    fn test_item_extract_key_simple() {
        let si = SimpleItem {
            id: "x".to_owned(),
            value: "v".to_owned(),
        };
        let item = si.to_item();
        let (key, rest) = item.extract_key();

        let raw_key = key.into_inner();

        // Key contains only SPK.
        assert_eq!(raw_key.len(), 1);
        assert_eq!(
            raw_key.get("SPK"),
            Some(&AttributeValue::S("ID#x".to_owned()))
        );

        // Remaining map does NOT contain the key attribute.
        assert!(!rest.contains_key("SPK"));
        // Payload field is present.
        assert!(rest.contains_key("value"));
    }

    // ---------------------------------------------------------------------------
    // test_dynamodb_item_to_item_and_try_from_item_roundtrip
    // ---------------------------------------------------------------------------

    #[test]
    fn test_dynamodb_item_to_item_and_try_from_item_roundtrip() {
        let original = sample_user();
        let item = original.to_item();
        let restored = User::try_from_item(item).unwrap();

        assert_eq!(restored.id, original.id);
        assert_eq!(restored.name, original.name);
        assert_eq!(restored.email, original.email);
        assert_eq!(restored.role, original.role);
    }
}
