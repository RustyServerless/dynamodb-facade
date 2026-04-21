use crate::{IntoTypedAttributeValue, NoId};

use super::AttributeValueRef;

pub(super) mod sealed_traits {
    /// Seals [`AttributeType`](super::AttributeType) so only `StringAttribute`, `NumberAttribute`, and `BinaryAttribute` can implement it.
    pub trait AttributeTypeSeal {}
}

crate::utils::impl_sealed_marker_types!(
    /// Sealed marker trait for DynamoDB attribute types.
    ///
    /// Implemented only by [`StringAttribute`], [`NumberAttribute`], and
    /// [`BinaryAttribute`]. This trait is sealed and cannot be implemented
    /// outside of this crate. It is used as a bound on
    /// [`AttributeDefinition::Type`] to restrict attribute definitions to the
    /// three scalar DynamoDB types authorized for key schemas (S, N, B).
    AttributeType,
    sealed_traits::AttributeTypeSeal;
    /// Marker type for DynamoDB String (`S`) attributes.
    ///
    /// Use this as the `Type` in an [`attribute_definitions!`](crate::attribute_definitions)
    /// block to declare that an attribute stores a string value. Rust types
    /// that implement [`IntoTypedAttributeValue<StringAttribute>`](crate::IntoTypedAttributeValue)
    /// include [`String`], [`&str`], `&String`, and [`Cow<'_, str>`](std::borrow::Cow).
    ///
    /// The type system enforces this constraint at compile time: passing a
    /// value of the wrong type (e.g. a `u32` or `Vec<u8>`) to a
    /// `StringAttribute` attribute will not compile.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::{attribute_definitions, has_attributes, StringAttribute};
    ///
    /// attribute_definitions! {
    ///     Label { "label": StringAttribute }
    /// }
    ///
    /// struct MyItem;
    ///
    /// // ✓ &'static str implements IntoTypedAttributeValue<StringAttribute>
    /// has_attributes! {
    ///     MyItem {
    ///         Label { const VALUE: &'static str = "hello"; }
    ///     }
    /// }
    ///
    /// struct OtherItem;
    /// // ✓ String also works
    /// has_attributes! {
    ///     OtherItem {
    ///         Label { fn attribute_value(id) -> String { "world".to_owned() } }
    ///     }
    /// }
    ///
    /// // These would NOT compile — wrong attribute type:
    /// // has_attributes! { MyItem { Label { const VALUE: u32 = 42; } } }
    /// // has_attributes! { MyItem { Label { fn attribute_value(id) -> Vec<u8> { vec![] } } } }
    /// ```
    StringAttribute,
    /// Marker type for DynamoDB Number (`N`) attributes.
    ///
    /// Use this as the `Type` in an [`attribute_definitions!`](crate::attribute_definitions)
    /// block to declare that an attribute stores a numeric value. Rust types
    /// that implement [`IntoTypedAttributeValue<NumberAttribute>`](crate::IntoTypedAttributeValue)
    /// include all integer and floating-point primitives, as well as
    /// [`AsNumber<T>`](crate::AsNumber) (for pre-formatted number strings).
    ///
    /// The type system enforces this constraint at compile time: passing a
    /// value of the wrong type (e.g. a `&str` or `Vec<u8>`) to a
    /// `NumberAttribute` attribute will not compile.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::{attribute_definitions, has_attributes, NumberAttribute};
    ///
    /// attribute_definitions! {
    ///     Score { "score": NumberAttribute }
    /// }
    ///
    /// struct MyItem;
    /// // ✓ u32 implements IntoTypedAttributeValue<NumberAttribute>
    /// has_attributes! {
    ///     MyItem {
    ///         Score { fn attribute_value(id) -> u32 { 100 } }
    ///     }
    /// }
    ///
    /// struct OtherItem;
    /// // ✓ i64 also works
    /// has_attributes! {
    ///     OtherItem {
    ///         Score { fn attribute_value(id) -> i64 { -5 } }
    ///     }
    /// }
    ///
    /// // These would NOT compile — wrong attribute type:
    /// // has_attributes! { MyItem { Score { const VALUE: &'static str = "hi"; } } }
    /// // has_attributes! { MyItem { Score { fn attribute_value(id) -> Vec<u8> { vec![] } } } }
    /// ```
    NumberAttribute,
    /// Marker type for DynamoDB Binary (`B`) attributes.
    ///
    /// Use this as the `Type` in an [`attribute_definitions!`](crate::attribute_definitions)
    /// block to declare that an attribute stores binary data. Rust types that
    /// implement [`IntoTypedAttributeValue<BinaryAttribute>`](crate::IntoTypedAttributeValue)
    /// include [`Vec<u8>`] and [`&[u8]`].
    ///
    /// The type system enforces this constraint at compile time: passing a
    /// value of the wrong type (e.g. a `&str` or `u32`) to a
    /// `BinaryAttribute` attribute will not compile.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::{attribute_definitions, has_attributes, BinaryAttribute};
    ///
    /// attribute_definitions! {
    ///     Thumbnail { "thumbnail": BinaryAttribute }
    /// }
    ///
    /// struct MyItem;
    ///
    /// // ✓ Vec<u8> implements IntoTypedAttributeValue<BinaryAttribute>
    /// has_attributes! {
    ///     MyItem {
    ///         Thumbnail {
    ///             fn attribute_value(id) -> Vec<u8> { vec![0x89, 0x50, 0x4e, 0x47] }
    ///         }
    ///     }
    /// }
    ///
    /// // These would NOT compile — wrong attribute type:
    /// // has_attributes! { MyItem { Thumbnail { const VALUE: &'static str = "img"; } } }
    /// // has_attributes! { MyItem { Thumbnail { fn attribute_value(id) -> u32 { 0 } } } }
    /// ```
    BinaryAttribute
);

/// Defines the name and type of a single DynamoDB attribute at the type level.
///
/// Implementations are generated by
/// [`attribute_definitions!`](crate::attribute_definitions). Each implementing
/// type is a zero-sized struct that carries:
///
/// - `NAME` — the DynamoDB attribute name as a `&'static str`.
/// - `Type` — one of [`StringAttribute`], [`NumberAttribute`], or
///   [`BinaryAttribute`], indicating the DynamoDB scalar type.
///
/// These types serve as type-safe identifiers that connect attribute names and
/// DynamoDB types to your key schemas, item definitions, and query builders.
///
/// # Examples
///
/// ```
/// use dynamodb_facade::{attribute_definitions, has_attributes, AttributeDefinition, StringAttribute};
///
/// attribute_definitions! {
///     CourseId { "course_id": StringAttribute }
/// }
///
///
/// // Use with the has_attributes! or dynamodb_item! macros:
/// struct MyItem;
/// has_attributes! {
///     MyItem {
///         CourseId { const VALUE: &'static str = "COURSE1234"; }
///     }
/// }
///
/// // Access the attribute name.
/// assert_eq!(CourseId::NAME, "course_id");
/// ```
pub trait AttributeDefinition {
    /// The DynamoDB attribute name (e.g. `"PK"`, `"email"`).
    const NAME: &'static str;
    /// The DynamoDB scalar type: [`StringAttribute`], [`NumberAttribute`], or [`BinaryAttribute`].
    type Type: AttributeType + AttributeValueRef;
}

/// Links an item type to a dynamic DynamoDB attribute.
///
/// Implementing this trait for a pair `(Item, Attr)` declares that `Item`
/// contributes the attribute `Attr` to its DynamoDB representation, where the
/// attribute value is derived from the item at runtime.
///
/// The trait has two key methods:
///
/// - `attribute_id` — extracts an "Id" value from `&self` (e.g. a
///   `&str` field).
/// - `attribute_value` — converts the Id into a Rust value of type
///   [`Self::Value`](HasAttribute::Value) (e.g. produces `"USER#{id}"` as a
///   `String`). This is **not** an [`AttributeValue`](crate::AttributeValue)
///   yet — the library converts it downstream using
///   [`IntoTypedAttributeValue`].
/// - `attribute` — convenience method that calls both in sequence to obtain
///   the `Self::Value` from `&self`.
///
/// Implementations are generated by [`dynamodb_item!`](crate::dynamodb_item)
/// and [`has_attributes!`](crate::has_attributes). Every type that implements
/// [`HasConstAttribute<A>`] automatically gets a blanket `HasAttribute<A>`
/// implementation.
///
/// # Examples
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::HasAttribute;
///
/// let user = sample_user();
///
/// // Retrieve the DynamoDB PK value for this user.
/// let pk_value = <User as HasAttribute<PK>>::attribute(&user);
/// assert_eq!(pk_value, "USER#user-1");
/// ```
pub trait HasAttribute<A: AttributeDefinition> {
    /// The identifier extracted from `&self`, passed to
    /// [`attribute_value`](HasAttribute::attribute_value).
    ///
    /// For constant attributes this is [`NoId`]. For dynamic
    /// attributes it is typically a borrowed field (e.g. `&str`).
    type Id<'id>;
    /// A Rust type convertible to the DynamoDB attribute value for this
    /// attribute.
    ///
    /// Bounded by [`IntoTypedAttributeValue<A::Type>`](crate::IntoTypedAttributeValue),
    /// which guarantees that when this Rust value is converted to an
    /// [`AttributeValue`](crate::AttributeValue) it will produce the correct
    /// DynamoDB scalar type (`S` for [`StringAttribute`], `N` for
    /// [`NumberAttribute`], `B` for [`BinaryAttribute`]).
    type Value: IntoTypedAttributeValue<A::Type>;
    /// Extracts the attribute ID from this item.
    fn attribute_id(&self) -> Self::Id<'_>;
    /// Converts an attribute ID into a Rust value of type [`Self::Value`](HasAttribute::Value)
    /// which can then be converted into the correct [`AttributeValue`](crate::AttributeValue)
    /// at serialization using the via
    /// [`IntoTypedAttributeValue`].
    fn attribute_value(id: Self::Id<'_>) -> Self::Value;
    /// Convenience method: calls [`attribute_id`](HasAttribute::attribute_id)
    /// then [`attribute_value`](HasAttribute::attribute_value), returning a
    /// Rust value of type [`Self::Value`](HasAttribute::Value).
    fn attribute(&self) -> Self::Value {
        <Self as HasAttribute<A>>::attribute_value(self.attribute_id())
    }
}

/// Links an item type to a compile-time constant DynamoDB attribute value.
///
/// Implementing this trait for a pair `(Item, Attr)` declares that every
/// instance of `Item` has the same fixed value for attribute `Attr`. This is
/// the common case for type discriminators (e.g. `ItemType` always `"USER"`).
///
/// Every type that implements [`HasConstAttribute<A>`] automatically gets a
/// blanket [`HasAttribute<A>`] implementation that returns `VALUE` regardless
/// of the instance.
///
/// Implementations are generated by [`dynamodb_item!`](crate::dynamodb_item)
/// and [`has_attributes!`](crate::has_attributes).
///
/// # Examples
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{HasAttribute, HasConstAttribute, NoId};
///
/// // PlatformConfig has a constant PK value.
/// assert_eq!(<PlatformConfig as HasConstAttribute<PK>>::VALUE, "PLATFORM_CONFIG");
/// // Also returned by the blanket HasAttribute<PK> implementation
/// assert_eq!(<PlatformConfig as HasAttribute<PK>>::attribute_value(NoId), "PLATFORM_CONFIG");
///
/// // User has a constant SK value.
/// assert_eq!(<User as HasConstAttribute<SK>>::VALUE, "USER");
/// ```
pub trait HasConstAttribute<A: AttributeDefinition> {
    /// A Rust constant type convertible to the DynamoDB attribute value for this
    /// attribute.
    ///
    /// Bounded by [`IntoTypedAttributeValue<A::Type>`](crate::IntoTypedAttributeValue),
    /// which guarantees that when this Rust value is converted to an
    /// [`AttributeValue`](crate::AttributeValue) it will produce the correct
    /// DynamoDB scalar type (`S` for [`StringAttribute`], `N` for
    /// [`NumberAttribute`], `B` for [`BinaryAttribute`]).
    type Value: IntoTypedAttributeValue<A::Type>;
    /// The constant Rust value shared by all instances of this item type,
    /// later converted to the DynamoDB attribute value by the library.
    const VALUE: Self::Value;
}

impl<A: AttributeDefinition, T: HasConstAttribute<A>> HasAttribute<A> for T {
    type Id<'a> = NoId;
    type Value = <Self as HasConstAttribute<A>>::Value;
    fn attribute_id(&self) -> Self::Id<'_> {
        NoId
    }
    fn attribute_value(_id: Self::Id<'_>) -> Self::Value {
        <Self as HasConstAttribute<A>>::VALUE
    }
}
