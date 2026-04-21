use crate::{AttributeType, BinaryAttribute, NumberAttribute, StringAttribute};

use super::*;

mod sealed_traits {
    /// Seals [`IntoTypedAttributeValue`](super::IntoTypedAttributeValue) so it can only be implemented via the marker sub-traits.
    pub trait IntoTypedAttributeValueSeal<KA: super::AttributeType> {}
}

/// A type-safe variant of [`IntoAttributeValue`] that guarantees the produced
/// [`AttributeValue`] matches a specific DynamoDB
/// scalar type.
///
/// This trait is sealed and cannot be implemented directly. It is automatically
/// implemented for any type that implements both [`IntoAttributeValue`] and the
/// corresponding internal marker trait:
///
/// | Rust type | `KA` parameter |
/// |---|---|
/// | [`String`], [`&str`], `&String` | [`StringAttribute`] |
/// | Integer and float primitives, [`String`], [`&str`] | [`NumberAttribute`] |
/// | [`Vec<u8>`], [`&[u8]`] | [`BinaryAttribute`] |
///
/// `IntoTypedAttributeValue<KA>` is used as the bound on
/// [`HasAttribute::Value`](crate::HasAttribute::Value) and
/// [`HasConstAttribute::Value`](crate::HasConstAttribute::Value), ensuring at compile
/// time that the value type for an attribute matches the declared
/// [`AttributeDefinition::Type`](crate::AttributeDefinition::Type). For
/// example, you cannot accidentally store a number in a `StringAttribute`
/// column.
///
/// # Examples
///
/// ```
/// use dynamodb_facade::{IntoTypedAttributeValue, StringAttribute, NumberAttribute};
///
/// // String implements IntoTypedAttributeValue<StringAttribute>
/// fn accepts_string_attr<V: IntoTypedAttributeValue<StringAttribute>>(_v: V) {}
/// accepts_string_attr("alice@example.com");
/// accepts_string_attr("user-1".to_owned());
///
/// // u32 implements IntoTypedAttributeValue<NumberAttribute>
/// fn accepts_number_attr<V: IntoTypedAttributeValue<NumberAttribute>>(_v: V) {}
/// accepts_number_attr(42);
/// accepts_number_attr(3.14);
/// ```
pub trait IntoTypedAttributeValue<KA: AttributeType>:
    IntoAttributeValue + sealed_traits::IntoTypedAttributeValueSeal<KA>
{
}

/// Marker for types that produce a DynamoDB `S` (string) attribute value.
pub(super) trait IntoStringAttributeValue {}
impl<T: IntoStringAttributeValue + IntoAttributeValue>
    sealed_traits::IntoTypedAttributeValueSeal<StringAttribute> for T
{
}
impl<T: IntoStringAttributeValue + IntoAttributeValue> IntoTypedAttributeValue<StringAttribute>
    for T
{
}

/// Marker for types that produce a DynamoDB `N` (number) attribute value.
pub(super) trait IntoNumberAttributeValue {}
impl<T: IntoNumberAttributeValue + IntoAttributeValue>
    sealed_traits::IntoTypedAttributeValueSeal<NumberAttribute> for T
{
}
impl<T: IntoNumberAttributeValue + IntoAttributeValue> IntoTypedAttributeValue<NumberAttribute>
    for T
{
}

/// Marker for types that produce a DynamoDB `B` (binary) attribute value.
pub(super) trait IntoBinaryAttributeValue {}
impl<T: IntoBinaryAttributeValue + IntoAttributeValue>
    sealed_traits::IntoTypedAttributeValueSeal<BinaryAttribute> for T
{
}
impl<T: IntoBinaryAttributeValue + IntoAttributeValue> IntoTypedAttributeValue<BinaryAttribute>
    for T
{
}
