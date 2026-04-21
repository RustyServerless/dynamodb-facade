use crate::{HasAttribute, IntoAttributeValue};

use super::*;

/// A type-safe wrapper for a DynamoDB key belonging to a specific table.
///
/// `Key<TD>` holds only the key attributes (PK, and SK for composite-key tables)
/// for the table defined by the [`TableDefinition`] `TD`.
///
/// A `Key` can only be obtain from a type implementing [`KeyBuilder`] —
/// typically any [`DynamoDBItem`] — or by extracting it from an [`Item`].
///
/// # Examples
///
/// Building a key from a [`KeyBuilder`]:
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{DynamoDBItem, Key, KeyBuilder};
///
/// fn platform_key<KB: KeyBuilder<PlatformTable>>(key_builder: &KB) -> Key<PlatformTable> {
///     key_builder.get_key()
/// }
///
/// // User implements KeyBuilder<PlatformTable> because it is a DynamoDBItem<PlatformTable>
/// let user = sample_user();
/// let key = platform_key(&user);
///
/// let raw = key.into_inner();
/// assert_eq!(
///     raw["PK"].as_s().unwrap(),
///     "USER#user-1"
/// );
/// ```
///
/// Extracting a key from an item:
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::DynamoDBItem;
///
/// let item = sample_user().to_item();
/// let key = item.into_key_only();
/// let raw = key.into_inner();
///
/// assert!(raw.contains_key("PK"));
/// assert!(raw.contains_key("SK"));
/// assert!(!raw.contains_key("name")); // payload stripped
/// ```
pub struct Key<TD: TableDefinition>(HashMap<String, AttributeValue>, PhantomData<TD>);
impl<TD: TableDefinition> fmt::Debug for Key<TD> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
impl<TD: TableDefinition> Key<TD> {
    /// Consumes the key and returns the underlying raw attribute map.
    ///
    /// The map contains only the key attributes (PK and SK for composite-key
    /// tables). Use this when you need to pass the key to raw SDK builders.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItem, Key, KeyBuilder};
    ///
    /// let key: Key<PlatformTable> = sample_user().get_key();
    /// let raw = key.into_inner();
    /// assert_eq!(raw["PK"].as_s().unwrap(), "USER#user-1");
    /// assert_eq!(raw["SK"].as_s().unwrap(), "USER");
    /// ```
    pub fn into_inner(self) -> HashMap<String, AttributeValue> {
        self.0
    }
}

/// Builds DynamoDB keys from type-safe key IDs.
///
/// This trait is automatically implemented for every type that implements
/// [`DynamoDBItem`]. It provides three methods:
///
/// - [`get_key_from_id`](KeyBuilder::get_key_from_id) — construct a [`Key<TD>`]
///   from a [`KeyId`] without an instance of the type
/// - [`get_key_id`](KeyBuilder::get_key_id) — extract the logical [`KeyId`] from
///   an existing instance
/// - [`get_key`](KeyBuilder::get_key) — convenience: extract the [`KeyId`] from
///   `self` and immediately build the [`Key<TD>`]
///
/// The associated type `KeyId<'id>` is a [`KeyId<PkId, SkId>`] whose concrete
/// `PkId` and `SkId` types are determined by the item's [`HasAttribute`] impls.
///
/// # Examples
///
/// Building a key from an existing instance:
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{Key, KeyBuilder};
///
/// let user = sample_user();
/// let key: Key<PlatformTable> = user.get_key();
///
/// let raw = key.into_inner();
/// assert_eq!(raw["PK"].as_s().unwrap(), "USER#user-1");
/// assert_eq!(raw["SK"].as_s().unwrap(), "USER");
/// ```
///
/// Building a key from a [`KeyId`] without an instance:
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{Key, KeyBuilder, KeyId};
///
/// let key: Key<PlatformTable> = User::get_key_from_id(KeyId::pk("user-42"));
///
/// let raw = key.into_inner();
/// assert_eq!(raw["PK"].as_s().unwrap(), "USER#user-42");
/// assert_eq!(raw["SK"].as_s().unwrap(), "USER");
/// ```
pub trait KeyBuilder<TD: TableDefinition> {
    /// The logical key identifier type for this item, typically a [`KeyId<PkId, SkId>`]
    /// whose components are derived from the item's [`HasAttribute`] implementations.
    type KeyId<'id>;

    /// Constructs a [`Key<TD>`] from a [`KeyId`] without requiring an instance of the
    /// implementing type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{Key, KeyBuilder, KeyId};
    ///
    /// let key: Key<PlatformTable> = User::get_key_from_id(KeyId::pk("user-42"));
    ///
    /// let raw = key.into_inner();
    /// assert_eq!(raw["PK"].as_s().unwrap(), "USER#user-42");
    /// assert_eq!(raw["SK"].as_s().unwrap(), "USER");
    /// ```
    fn get_key_from_id(key_id: Self::KeyId<'_>) -> Key<TD>;

    /// Extracts the logical [`KeyId`] from an existing instance of the implementing type.
    ///
    /// The returned [`KeyId`] borrows from `self` and can be passed to
    /// [`get_key_from_id`][KeyBuilder::get_key_from_id] to produce a [`Key<TD>`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{KeyBuilder, KeyId, NoId};
    ///
    /// let user = sample_user();
    /// let key_id: KeyId<&str, NoId> = <User as KeyBuilder<PlatformTable>>::get_key_id(&user);
    /// ```
    fn get_key_id(&self) -> Self::KeyId<'_>;

    /// Convenience method that builds a [`Key<TD>`] directly from `self`.
    ///
    /// Equivalent to calling [`get_key_id`][KeyBuilder::get_key_id] followed by
    /// [`get_key_from_id`][KeyBuilder::get_key_from_id]. Prefer this method when you
    /// have an instance of the item and simply need its DynamoDB primary key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{Key, KeyBuilder};
    ///
    /// let user = sample_user();
    /// let key: Key<PlatformTable> = user.get_key();
    ///
    /// let raw = key.into_inner();
    /// assert_eq!(raw["PK"].as_s().unwrap(), "USER#user-1");
    /// assert_eq!(raw["SK"].as_s().unwrap(), "USER");
    /// ```
    fn get_key(&self) -> Key<TD> {
        Self::get_key_from_id(self.get_key_id())
    }
}

mod key_builder_helper {
    //! Blanket [`KeyBuilder`] impl and the internal [`KeyBuilderHelper`] trait,
    //! dispatched by key schema kind.
    use super::*;

    impl<TD: TableDefinition, T> KeyBuilder<TD> for T
    where
        T: KeyBuilderHelper<TD, <TD::KeySchema as KeySchema>::Kind>,
    {
        type KeyId<'id> = KeyId<
            <Self as KeyBuilderHelper<TD, <TD::KeySchema as KeySchema>::Kind>>::PkId<'id>,
            <Self as KeyBuilderHelper<TD, <TD::KeySchema as KeySchema>::Kind>>::SkId<'id>,
        >;

        fn get_key_from_id(key_id: Self::KeyId<'_>) -> Key<TD> {
            Self::get_key_from_id_helper(key_id)
        }

        fn get_key_id(&self) -> Self::KeyId<'_> {
            self.get_key_id_helper()
        }
    }

    /// Internal helper that builds keys dispatched by [`KeySchemaKind`].
    pub trait KeyBuilderHelper<TD: TableDefinition, KSK: KeySchemaKind> {
        type PkId<'pk>;
        type SkId<'sk>;
        fn get_key_from_id_helper(key_id: KeyId<Self::PkId<'_>, Self::SkId<'_>>) -> Key<TD>;
        fn get_key_id_helper(&self) -> KeyId<Self::PkId<'_>, Self::SkId<'_>>;
    }

    // -- Simple: PK only ------------------------------------------------------

    impl<TD: TableDefinition, T: DynamoDBItem<TD>> KeyBuilderHelper<TD, SimpleKey> for T
    where
        TD::KeySchema: SimpleKeySchema,
        T: HasTableKeyAttributes<TD>,
        T: HasAttribute<PartitionKeyDefinition<TD>>,
    {
        type PkId<'pk> = <Self as HasAttribute<PartitionKeyDefinition<TD>>>::Id<'pk>;
        type SkId<'sk> = NoId;
        fn get_key_from_id_helper(key_id: KeyId<Self::PkId<'_>, Self::SkId<'_>>) -> Key<TD> {
            let pk_value =
                <Self as HasAttribute<PartitionKeyDefinition<TD>>>::attribute_value(key_id.pk);
            Key(
                HashMap::from([(
                    PartitionKeyDefinition::<TD>::NAME.to_owned(),
                    pk_value.into_attribute_value(),
                )]),
                PhantomData,
            )
        }

        fn get_key_id_helper(&self) -> KeyId<Self::PkId<'_>, Self::SkId<'_>> {
            let pk_id = T::attribute_id(self);
            KeyId::pk(pk_id)
        }
    }

    // -- Composite: PK + SK ---------------------------------------------------

    impl<TD: TableDefinition, T: DynamoDBItem<TD>> KeyBuilderHelper<TD, CompositeKey> for T
    where
        TD::KeySchema: CompositeKeySchema,
        T: HasTableKeyAttributes<TD>,
        T: HasAttribute<PartitionKeyDefinition<TD>>,
        T: HasAttribute<SortKeyDefinition<TD>>,
    {
        type PkId<'pk> = <Self as HasAttribute<PartitionKeyDefinition<TD>>>::Id<'pk>;
        type SkId<'sk> = <Self as HasAttribute<SortKeyDefinition<TD>>>::Id<'sk>;

        fn get_key_from_id_helper(key_id: KeyId<Self::PkId<'_>, Self::SkId<'_>>) -> Key<TD> {
            let pk_value =
                <Self as HasAttribute<PartitionKeyDefinition<TD>>>::attribute_value(key_id.pk);
            let sk_value =
                <Self as HasAttribute<SortKeyDefinition<TD>>>::attribute_value(key_id.sk);
            Key(
                HashMap::from([
                    (
                        PartitionKeyDefinition::<TD>::NAME.to_owned(),
                        pk_value.into_attribute_value(),
                    ),
                    (
                        SortKeyDefinition::<TD>::NAME.to_owned(),
                        sk_value.into_attribute_value(),
                    ),
                ]),
                PhantomData,
            )
        }

        fn get_key_id_helper(&self) -> KeyId<Self::PkId<'_>, Self::SkId<'_>> {
            let pk_id = <Self as HasAttribute<PartitionKeyDefinition<TD>>>::attribute_id(self);
            let sk_id = <Self as HasAttribute<SortKeyDefinition<TD>>>::attribute_id(self);
            KeyId::pk(pk_id).sk(sk_id)
        }
    }
}

impl<TD: TableDefinition> From<Key<TD>> for Item<TD> {
    fn from(value: Key<TD>) -> Self {
        Item(value.0, PhantomData)
    }
}

impl<TD: TableDefinition> Item<TD>
where
    Self: KeyItemExtractor<TD, <TD::KeySchema as KeySchema>::Kind>,
{
    /// Splits the item into its key and the remaining non-key attributes.
    ///
    /// Returns a tuple of `(Key<TD>, HashMap<String, AttributeValue>)` where
    /// the key contains only the PK (and SK for composite-key tables) and the
    /// map contains every other attribute that was in the item.
    ///
    /// This is meant to allow the direct manipulation of the attribute map
    /// while enforcing the invariant that an Item always contains valid key
    /// attributes. Use it in conjunction with [`Item::from_key_and_attributes`]
    /// to accomplish that.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItem;
    ///
    /// let item = sample_user().to_item();
    /// let (key, rest) = item.extract_key();
    ///
    /// // Key contains only PK + SK.
    /// let raw_key = key.into_inner();
    /// assert!(raw_key.contains_key("PK"));
    /// assert!(raw_key.contains_key("SK"));
    ///
    /// // Remaining map has everything else.
    /// assert!(rest.contains_key("name"));
    /// assert!(!rest.contains_key("PK"));
    /// ```
    pub fn extract_key(self) -> (Key<TD>, HashMap<String, AttributeValue>) {
        KeyItemExtractor::extract_key(self)
    }

    /// Consumes the item and returns only its key, discarding all other attributes.
    ///
    /// This is a convenience wrapper around [`extract_key`](Item::extract_key)
    /// that drops the remaining attribute map.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItem;
    ///
    /// let item = sample_user().to_item();
    /// let key = item.into_key_only();
    ///
    /// let raw = key.into_inner();
    /// assert!(raw.contains_key("PK"));
    /// assert!(raw.contains_key("SK"));
    /// assert!(!raw.contains_key("name"));
    /// ```
    pub fn into_key_only(self) -> Key<TD> {
        self.extract_key().0
    }
}

/// Splits an [`Item<TD>`] into its [`Key<TD>`] and the remaining attribute map.
pub trait KeyItemExtractor<TD: TableDefinition, KSK: KeySchemaKind> {
    fn extract_key(self) -> (Key<TD>, HashMap<String, AttributeValue>);
}

/// Moves a single key attribute from `from_item` into `to_key`.
fn transfer_keyschema_helper<TD: TableDefinition>(
    from_item: &mut Item<TD>,
    to_key: &mut Key<TD>,
    k: &str,
) {
    let v = from_item
        .0
        .remove(k)
        .expect("Item is guaranteed to contain the KeySchema");
    to_key.0.insert(k.to_owned(), v);
}
impl<TD: TableDefinition> KeyItemExtractor<TD, SimpleKey> for Item<TD>
where
    TD::KeySchema: SimpleKeySchema,
{
    fn extract_key(mut self) -> (Key<TD>, HashMap<String, AttributeValue>) {
        let mut key = Key(HashMap::with_capacity(1), PhantomData);
        transfer_keyschema_helper(
            &mut self,
            &mut key,
            <TD::KeySchema as KeySchema>::PartitionKey::NAME,
        );
        (key, self.0)
    }
}
impl<TD: TableDefinition> KeyItemExtractor<TD, CompositeKey> for Item<TD>
where
    TD::KeySchema: CompositeKeySchema,
{
    fn extract_key(mut self) -> (Key<TD>, HashMap<String, AttributeValue>) {
        let mut key = Key(HashMap::with_capacity(2), PhantomData);
        transfer_keyschema_helper(
            &mut self,
            &mut key,
            <TD::KeySchema as KeySchema>::PartitionKey::NAME,
        );
        transfer_keyschema_helper(
            &mut self,
            &mut key,
            <TD::KeySchema as CompositeKeySchema>::SortKey::NAME,
        );
        (key, self.0)
    }
}
