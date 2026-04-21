use std::collections::HashMap;

use super::{AttributeDefinition, HasAttribute, HasTableKeyAttributes, TableDefinition};
use crate::{AttributeValue, IntoAttributeValue};

mod sealed_traits {
    use super::*;
    /// Seals [`AttributeList`] and provides the recursive attribute-collection machinery.
    pub trait AttributeListSeal<TD: TableDefinition, T: HasTableKeyAttributes<TD>> {
        const ATTRIBUTE_LIST_LEN: usize;
        fn enrich(source: &T, item: &mut HashMap<String, AttributeValue>);
    }
}

/// A sealed, recursive tuple-list of additional DynamoDB attributes for an item.
///
/// This trait is sealed and automatically implemented for right-nested tuples of
/// [`AttributeDefinition`] types — for example `(ItemType, (Expiration, ()))`.
/// It is the type-level representation of
/// [`DynamoDBItem::AdditionalAttributes`](crate::DynamoDBItem::AdditionalAttributes):
/// the set of attributes that are written to DynamoDB in addition to the
/// primary key attributes and the serialization of the underlying type.
///
/// The [`attr_list!`](crate::attr_list) macro builds these nested tuple types
/// conveniently: `attr_list![ItemType, Expiration]` expands to
/// `(ItemType, (Expiration, ()))`.
///
/// # Examples
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{attr_list, AttributeList, DynamoDBItem};
///
/// // The AdditionalAttributes type for PlatformConfig is (ItemType, ()).
/// type ConfigAttrs = <PlatformConfig as DynamoDBItem<PlatformTable>>::AdditionalAttributes;
///
/// // AttributeList::get_attributes collects all additional attributes into a HashMap.
/// let config = sample_config();
/// let attrs = <ConfigAttrs as AttributeList<PlatformTable, PlatformConfig>>::get_attributes(&config);
/// // Only 1 additional attribute in this case
/// assert_eq!(attrs.len(), 1);
/// // It is "_TYPE" and its value for PlatformConfig is "PLATFORM_CONFIG"
/// assert!(attrs.get("_TYPE").is_some_and(|v| v.as_s().unwrap() == "PLATFORM_CONFIG"));
/// ```
pub trait AttributeList<TD: TableDefinition, T: HasTableKeyAttributes<TD>>:
    sealed_traits::AttributeListSeal<TD, T>
{
    /// Collects all attributes in this list from `source` into a
    /// `HashMap<String, AttributeValue>`.
    fn get_attributes(source: &T) -> HashMap<String, AttributeValue>;
}

// Base case: empty list
impl<TD: TableDefinition, T: HasTableKeyAttributes<TD>> sealed_traits::AttributeListSeal<TD, T>
    for ()
{
    const ATTRIBUTE_LIST_LEN: usize = 0;
    fn enrich(_source: &T, _item: &mut HashMap<String, AttributeValue>) {}
}

// Recursive case: head + tail
impl<TD, A, Rest, T> sealed_traits::AttributeListSeal<TD, T> for (A, Rest)
where
    TD: TableDefinition,
    A: AttributeDefinition,
    T: HasTableKeyAttributes<TD> + HasAttribute<A>,
    Rest: sealed_traits::AttributeListSeal<TD, T>,
{
    const ATTRIBUTE_LIST_LEN: usize = Rest::ATTRIBUTE_LIST_LEN + 1;

    fn enrich(source: &T, item: &mut HashMap<String, AttributeValue>) {
        item.insert(
            A::NAME.to_owned(),
            <T as HasAttribute<A>>::attribute(source).into_attribute_value(),
        );
        Rest::enrich(source, item);
    }
}
impl<TD, T, AL> AttributeList<TD, T> for AL
where
    TD: TableDefinition,
    T: HasTableKeyAttributes<TD>,
    AL: sealed_traits::AttributeListSeal<TD, T>,
{
    fn get_attributes(source: &T) -> HashMap<String, AttributeValue> {
        let mut attributes = HashMap::with_capacity(AL::ATTRIBUTE_LIST_LEN);
        Self::enrich(source, &mut attributes);
        attributes
    }
}
