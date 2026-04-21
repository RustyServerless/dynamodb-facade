use crate::AttributeValue;

use super::attributes::*;

/// Extracts a typed reference from an [`AttributeValue`] for a given attribute type.
///
/// Implemented for [`StringAttribute`], [`NumberAttribute`], and [`BinaryAttribute`].
pub trait AttributeValueRef: sealed_traits::AttributeTypeSeal {
    type Ref<'a>;

    fn attribute_value_ref(av: &AttributeValue) -> Self::Ref<'_>;
}

impl AttributeValueRef for StringAttribute {
    type Ref<'a> = &'a str;

    fn attribute_value_ref(av: &AttributeValue) -> &str {
        av.as_s().expect("expected S attribute")
    }
}

impl AttributeValueRef for NumberAttribute {
    type Ref<'a> = &'a str;

    fn attribute_value_ref(av: &AttributeValue) -> &str {
        av.as_n().expect("expected N attribute")
    }
}

impl AttributeValueRef for BinaryAttribute {
    type Ref<'a> = &'a [u8];

    fn attribute_value_ref(av: &AttributeValue) -> &[u8] {
        av.as_b().expect("expected B attribute").as_ref()
    }
}
