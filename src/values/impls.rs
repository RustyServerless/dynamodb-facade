use std::{
    borrow::Cow,
    collections::{BTreeSet, HashSet},
};

use aws_sdk_dynamodb::primitives::Blob;

use super::*;

// -- Identity -----------------------------------------------------------------

impl IntoAttributeValue for AttributeValue {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        self
    }
}

// -- Strings ------------------------------------------------------------------

impl IntoAttributeValue for String {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::S(self)
    }
}
impl IntoStringAttributeValue for String {}

impl IntoAttributeValue for &str {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::S(self.to_owned())
    }
}
impl IntoStringAttributeValue for &str {}

impl IntoAttributeValue for &String {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::S(self.to_owned())
    }
}
impl IntoStringAttributeValue for &String {}

impl IntoAttributeValue for Cow<'_, str> {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::S(self.into_owned())
    }
}
impl IntoStringAttributeValue for Cow<'_, str> {}

// -- Bool ---------------------------------------------------------------------

impl IntoAttributeValue for bool {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::Bool(self)
    }
}

// -- Numerics -----------------------------------------------------------------

/// Implements [`IntoAttributeValue`] (→ `N`) and [`IntoNumberAttributeValue`] for numeric types.
macro_rules! impl_numeric {
    ($($t:ty),*) => {
        $(
            impl IntoAttributeValue for $t {
                #[inline]
                fn into_attribute_value(self) -> AttributeValue {
                    AttributeValue::N(self.to_string())
                }
            }
            impl IntoNumberAttributeValue for $t {}
        )*
    };
}

impl_numeric!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);

impl<T: Into<String>> IntoAttributeValue for AsNumber<T> {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::N(self.0.into())
    }
}
impl<T: Into<String>> IntoNumberAttributeValue for AsNumber<T> {}

// -- Binary (Vec<u8>) → B ----------------------------------------------------

impl IntoAttributeValue for Vec<u8> {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::B(Blob::new(self))
    }
}
impl IntoBinaryAttributeValue for Vec<u8> {}

impl IntoAttributeValue for &[u8] {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::B(Blob::new(self))
    }
}
impl IntoBinaryAttributeValue for &[u8] {}

// -- Vec<T> → L (list) -------------------------------------------------------
// Explicit impls for each supported inner type to avoid blanket conflicts.

// -- Vec<Vec<u8>> → L(B, B, …) -----------------------------------------------

/// Implements [`IntoAttributeValue`] (→ `L`) for `Vec<T>` by mapping each element.
macro_rules! impl_vec_list {
    ($($t:ty),*) => {
        $(
            impl IntoAttributeValue for Vec<$t> {
                #[inline]
                fn into_attribute_value(self) -> AttributeValue {
                    AttributeValue::L(
                        self.into_iter()
                            .map(IntoAttributeValue::into_attribute_value)
                            .collect(),
                    )
                }
            }
        )*
    };
}

impl_vec_list!(
    String,
    &str,
    bool,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u16,
    u32,
    u64,
    u128,
    usize,
    f32,
    f64,
    AttributeValue,
    Vec<u8>
);

// -- &[T] → L (list) ---------------------------------------------------------

/// Implements [`IntoAttributeValue`] (→ `L`) for `&[T]` by cloning and mapping each element.
macro_rules! impl_slice_list {
    ($($t:ty),*) => {
        $(
            impl IntoAttributeValue for &[$t] {
                #[inline]
                fn into_attribute_value(self) -> AttributeValue {
                    AttributeValue::L(
                        self.into_iter()
                            .cloned()
                            .map(IntoAttributeValue::into_attribute_value)
                            .collect(),
                    )
                }
            }
        )*
    };
}

// Numeric & bool slices — each element is cloned/converted via to_string().
impl_slice_list!(
    String,
    &str,
    bool,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u16,
    u32,
    u64,
    u128,
    usize,
    f32,
    f64,
    Vec<u8>,
    &[u8]
);

impl IntoAttributeValue for &[AttributeValue] {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::L(self.to_vec())
    }
}

// -- HashSet<String> → Ss -----------------------------------------------------

impl IntoAttributeValue for HashSet<String> {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::Ss(self.into_iter().collect())
    }
}

impl IntoAttributeValue for HashSet<&str> {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::Ss(self.into_iter().map(ToOwned::to_owned).collect())
    }
}

impl IntoAttributeValue for BTreeSet<String> {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::Ss(self.into_iter().collect())
    }
}

impl IntoAttributeValue for BTreeSet<&str> {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::Ss(self.into_iter().map(ToOwned::to_owned).collect())
    }
}

impl IntoAttributeValue for AsSet<String> {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::Ss(self.0)
    }
}

impl IntoAttributeValue for AsSet<&str> {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::Ss(self.into_iter().map(ToOwned::to_owned).collect())
    }
}

// -- HashSet<number> → Ns -----------------------------------------------------

/// Implements [`IntoAttributeValue`] (→ `Ns`) for set collections of numeric types.
macro_rules! impl_number_set {
    ($col:ident, $($t:ty),*) => {
        $(
            impl IntoAttributeValue for $col<$t> {
                #[inline]
                fn into_attribute_value(self) -> AttributeValue {
                    AttributeValue::Ns(self.into_iter().map(|v| v.to_string()).collect())
                }
            }
        )*
    };
}

impl_number_set!(
    HashSet, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);
impl_number_set!(
    BTreeSet, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);
impl_number_set!(
    AsSet, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);

// -- HashSet<Vec<u8>> → Bs ---------------------------------------------------

impl IntoAttributeValue for HashSet<Vec<u8>> {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::Bs(self.into_iter().map(Blob::new).collect())
    }
}

impl IntoAttributeValue for BTreeSet<Vec<u8>> {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::Bs(self.into_iter().map(Blob::new).collect())
    }
}

impl IntoAttributeValue for AsSet<Vec<u8>> {
    #[inline]
    fn into_attribute_value(self) -> AttributeValue {
        AttributeValue::Bs(self.into_iter().map(Blob::new).collect())
    }
}
