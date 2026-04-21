mod impls;
mod typed;

pub use typed::*;

use super::AttributeValue;

/// Converts a Rust value into a DynamoDB [`AttributeValue`].
///
/// This trait is the bridge between Rust types and the DynamoDB wire format.
/// It is implemented for all common scalar types, collections, and the
/// [`AsSet`] and [`AsNumber<T>`] newtypes:
///
/// | Rust type | DynamoDB type |
/// |---|---|
/// | [`String`], [`&str`], `&String` | `S` |
/// | [`bool`] | `BOOL` |
/// | Integer and float primitives | `N` |
/// | [`Vec<u8>`], [`&[u8]`] | `B` |
/// | [`Vec<T>`], [`&[T]`] (where `T` is a scalar) | `L` |
/// | [`HashSet<String>`](std::collections::HashSet), [`BTreeSet<String>`](std::collections::BTreeSet) | `SS` |
/// | [`HashSet<N>`](std::collections::HashSet), [`BTreeSet<N>`](std::collections::BTreeSet) (numeric) | `NS` |
/// | [`HashSet<Vec<u8>>`](std::collections::HashSet), [`BTreeSet<Vec<u8>>`](std::collections::BTreeSet) | `BS` |
/// | [`AsSet<String>`] | `SS` |
/// | [`AsSet<N>`] (numeric) | `NS` |
/// | [`AsSet<Vec<u8>>`] | `BS` |
/// | [`AsNumber<T>`] | `N` |
/// | [`AttributeValue`] | identity |
///
/// Implement this trait for your own domain types to use them directly in
/// expression builders (e.g. `Update::set("field", my_value)`).
///
/// # Examples
///
/// ```
/// use dynamodb_facade::{AttributeValue, IntoAttributeValue};
///
/// // Strings become AttributeValue::S
/// let av = "alice@example.com".into_attribute_value();
/// assert_eq!(av, AttributeValue::S("alice@example.com".to_owned()));
///
/// // Numbers become AttributeValue::N
/// let av = 42.into_attribute_value();
/// assert_eq!(av, AttributeValue::N("42".to_owned()));
///
/// // Custom domain type
/// struct UserId(String);
/// impl IntoAttributeValue for UserId {
///     fn into_attribute_value(self) -> AttributeValue {
///         self.0.into_attribute_value()
///     }
/// }
///
/// let av = UserId("user-1".to_owned()).into_attribute_value();
/// assert_eq!(av, AttributeValue::S("user-1".to_owned()));
/// ```
pub trait IntoAttributeValue {
    /// Converts `self` into a DynamoDB [`AttributeValue`].
    fn into_attribute_value(self) -> AttributeValue;
}

/// Converts a [`serde::Serialize`] value into a DynamoDB [`AttributeValue`]
/// using [`serde_dynamo`].
///
/// This is a convenience wrapper around [`try_to_attribute_value`] that panics
/// on failure. Use it when you are confident the serialization cannot fail
/// (e.g. for well-known types like `&[&str]` or simple structs).
///
/// Prefer [`try_to_attribute_value`] in contexts where you want to propagate
/// errors rather than panic.
///
/// # Panics
///
/// Panics if [`serde_dynamo::to_attribute_value`] returns an error. This
/// should be rare in practice — it can happen for types that serialize to
/// formats unsupported by DynamoDB (e.g. maps with non-string keys).
///
/// # Examples
///
/// Updating the `platform_config` field of a `MainPlatformConfig` item via
/// [`Update::set`](crate::Update::set):
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use serde::{Serialize, Deserialize};
/// use dynamodb_facade::{
///     to_attribute_value, DynamoDBItemOp, KeyId, Update,
///     StringAttribute, dynamodb_item,
/// };
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct MainPlatformConfig {
///     platform_config: PlatformConfig,
///     main_since_ts: u64,
/// }
///
/// dynamodb_item! {
///     #[table = PlatformTable]
///     MainPlatformConfig {
///         #[partition_key]
///         PK { const VALUE: &'static str = "MAIN_PLATFORM_CONFIG"; }
///         #[sort_key]
///         SK { const VALUE: &'static str = "MAIN_PLATFORM_CONFIG"; }
///     }
/// }
///
/// # async fn example(client: dynamodb_facade::Client) -> dynamodb_facade::Result<()> {
/// let new_config = PlatformConfig {
///     max_enrollments: 50,
///     maintenance_mode: false,
/// };
///
/// // PlatformConfig is a Serialize type — to_attribute_value bridges it
/// // to an AttributeValue for use in Update::set.
/// MainPlatformConfig::update_by_id(
///     client,
///     KeyId::NONE,
///     Update::set("platform_config", to_attribute_value(&new_config)),
/// )
/// .await?;
/// # Ok(())
/// # }
/// ```
pub fn to_attribute_value<T: serde::Serialize>(value: T) -> AttributeValue {
    try_to_attribute_value(value)
        .expect("should be infallible, use `try_to_attribute_value` instead")
}

/// Converts a [`serde::Serialize`] value into a DynamoDB [`AttributeValue`]
/// using [`serde_dynamo`], returning a [`Result`](crate::Result) on failure.
///
/// Use this when you need to handle serialization errors gracefully. For
/// infallible cases, [`to_attribute_value`] is more ergonomic.
///
/// # Errors
///
/// Returns [`Error::Serde`](crate::Error::Serde) if [`serde_dynamo`] cannot
/// convert the value — for example, if the type serializes to a map with
/// non-string keys, which DynamoDB does not support.
///
/// # Examples
///
/// Updating the `platform_config` field of a `MainPlatformConfig` item via
/// [`Update::set`](crate::Update::set), propagating any serialization error with `?`:
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use serde::{Serialize, Deserialize};
/// use dynamodb_facade::{
///     try_to_attribute_value, DynamoDBItemOp, KeyId, Update,
///     StringAttribute, dynamodb_item,
/// };
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct MainPlatformConfig {
///     platform_config: PlatformConfig,
///     main_since_ts: u64,
/// }
///
/// dynamodb_item! {
///     #[table = PlatformTable]
///     MainPlatformConfig {
///         #[partition_key]
///         PK { const VALUE: &'static str = "MAIN_PLATFORM_CONFIG"; }
///         #[sort_key]
///         SK { const VALUE: &'static str = "MAIN_PLATFORM_CONFIG"; }
///     }
/// }
///
/// # async fn example(client: dynamodb_facade::Client) -> dynamodb_facade::Result<()> {
/// let new_config = PlatformConfig {
///     max_enrollments: 50,
///     maintenance_mode: false,
/// };
///
/// // PlatformConfig is a Serialize type — try_to_attribute_value bridges it
/// // to an AttributeValue for use in Update::set.
/// MainPlatformConfig::update_by_id(
///     client,
///     KeyId::NONE,
///     Update::set("platform_config", try_to_attribute_value(&new_config)?),
/// )
/// .await?;
/// # Ok(())
/// # }
/// ```
pub fn try_to_attribute_value<T: serde::Serialize>(value: T) -> crate::Result<AttributeValue> {
    Ok(serde_dynamo::to_attribute_value(value)?)
}

/// A newtype wrapper around [`Vec<T>`] that serializes as a DynamoDB Set type
/// (`SS`, `NS`, or `BS`) instead of a List (`L`).
///
/// DynamoDB distinguishes between ordered lists (`L`) and unordered sets
/// (`SS`/`NS`/`BS`). A plain `Vec<String>` converts to `L`, but
/// `AsSet(vec)` converts to `SS`. This matters for `ADD` and `DELETE` update
/// expressions, which operate on Set types.
///
/// [`IntoAttributeValue`] is implemented for:
/// - `AsSet<String>` → `SS`
/// - `AsSet<N>` (any numeric primitive) → `NS`
///
/// `AsSet<T>` derefs to `&Vec<T>` and implements [`IntoIterator`], so you can
/// use it anywhere a `Vec<T>` is expected for reading.
///
/// # Examples
///
/// ```
/// use dynamodb_facade::{AsSet, AttributeValue, IntoAttributeValue};
///
/// // Vec<String> → L (list)
/// let list_av = vec!["rust".to_owned(), "dynamodb".to_owned()].into_attribute_value();
/// assert!(matches!(list_av, AttributeValue::L(_)));
///
/// // AsSet<String> → SS (string set)
/// let set_av = AsSet(vec!["rust".to_owned(), "dynamodb".to_owned()]).into_attribute_value();
/// assert!(matches!(set_av, AttributeValue::Ss(_)));
///
/// // AsSet<u32> → NS (number set)
/// let num_set_av = AsSet(vec![1, 2, 3]).into_attribute_value();
/// assert!(matches!(num_set_av, AttributeValue::Ns(_)));
/// ```
///
/// Using `AsSet` with the `add` update expression to atomically add tags:
///
/// ```no_run
/// // Requires a live DynamoDB connection
/// use dynamodb_facade::{AsSet, IntoAttributeValue};
/// // Update::add("tags", AsSet(vec!["rust".to_owned()]).into_attribute_value())
/// ```
#[derive(Debug)]
// New type wrapper for a Vec that will cause it to be serialized as a HashSet
// See impls.rs.
pub struct AsSet<T>(pub Vec<T>);
impl<T> core::ops::Deref for AsSet<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> IntoIterator for AsSet<T> {
    type Item = T;
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// A generic newtype wrapper that converts any `T: Into<String>` directly to
/// a DynamoDB `N` (number) attribute value without parsing.
///
/// Use `AsNumber` when you already have a correctly-formatted numeric string
/// and want to pass it to DynamoDB as-is — for example, a high-precision
/// decimal from an external API, a value from a financial system that must
/// not be rounded through an `f64`, or a number string received from another
/// DynamoDB client. `T` can be `&str`, [`String`], [`Cow<str>`](std::borrow::Cow),
/// or any other type that implements `Into<String>`.
///
/// `AsNumber` implements [`IntoAttributeValue`] (producing
/// [`AttributeValue::N`]) and
/// [`IntoTypedAttributeValue<NumberAttribute>`](crate::IntoTypedAttributeValue),
/// so it can be used anywhere a `NumberAttribute` value is expected (e.g. in
/// [`has_attributes!`](crate::has_attributes) blocks or expression builders).
///
/// `AsNumber<T>` also implements [`Deref<Target = T>`](core::ops::Deref),
/// so you can use it anywhere a `&T` is accepted.
///
/// ## DynamoDB number constraints
///
/// DynamoDB numbers can be positive, negative, or zero, with up to 38 digits
/// of precision (exceeding this causes a runtime error). The supported ranges
/// are:
///
/// - Positive: `1E-130` to `9.9999999999999999999999999999999999999E+125`
/// - Negative: `-9.9999999999999999999999999999999999999E+125` to `-1E-130`
///
/// Leading and trailing zeroes are trimmed by DynamoDB. Numbers are
/// transmitted as strings over the wire but treated as numeric types for
/// mathematical operations.
///
/// > **Warning:** No validation is performed on the wrapped value. An
/// > invalid number string (e.g. `"not-a-number"`) will be accepted by
/// > `AsNumber` but rejected by DynamoDB at runtime.
///
/// # Examples
///
/// Basic usage — converting a pre-formatted decimal string to `AttributeValue::N`:
///
/// ```
/// use dynamodb_facade::{AsNumber, AttributeValue, IntoAttributeValue};
///
/// // A high-precision decimal that would lose precision as f64
/// let price = AsNumber("12345678.90123456789099");
/// let av = price.into_attribute_value();
/// assert_eq!(av, AttributeValue::N("12345678.90123456789099".to_owned()));
/// ```
///
/// Using `AsNumber` where a [`NumberAttribute`](crate::NumberAttribute) value
/// is required — the type system accepts it just like any numeric primitive:
///
/// ```
/// use dynamodb_facade::{AsNumber, IntoTypedAttributeValue, NumberAttribute};
///
/// fn store_score<V: IntoTypedAttributeValue<NumberAttribute>>(_v: V) {}
///
/// // AsNumber satisfies the NumberAttribute bound
/// store_score(AsNumber("99.5"));
/// // So do ordinary numeric primitives
/// store_score(42);
/// ```
pub struct AsNumber<T: Into<String>>(pub T);

impl<T: Into<String>> core::ops::Deref for AsNumber<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    // -- to_attribute_value ---------------------------------------------------

    #[derive(serde::Serialize)]
    struct Thing {
        a: u32,
        b: String,
    }

    #[test]
    fn test_to_attribute_value_happy_path() {
        // Scalar: u32 → N
        let av = to_attribute_value(42u32);
        assert_eq!(av, AttributeValue::N("42".to_owned()));

        // Struct → M with expected keys
        let thing = Thing {
            a: 7,
            b: "hello".to_owned(),
        };
        let av = to_attribute_value(thing);
        if let AttributeValue::M(map) = av {
            assert_eq!(map.get("a"), Some(&AttributeValue::N("7".to_owned())));
            assert_eq!(map.get("b"), Some(&AttributeValue::S("hello".to_owned())));
        } else {
            panic!("expected AttributeValue::M, got something else");
        }
    }

    #[test]
    #[should_panic]
    fn test_to_attribute_value_panics_on_unserializable() {
        // serde_dynamo rejects HashMap<Option<&str>, &str> — None cannot be represented DynamoDB map.
        let wrong = HashMap::from([(Some("key"), "v1"), (None, "v2")]);
        to_attribute_value(wrong);
    }

    // -- try_to_attribute_value -----------------------------------------------

    #[test]
    fn test_try_to_attribute_value_happy_path() {
        let av = try_to_attribute_value("hello").unwrap();
        assert_eq!(av, AttributeValue::S("hello".to_owned()));
    }

    #[test]
    fn test_try_to_attribute_value_error_path() {
        // serde_dynamo rejects HashMap<Option<&str>, &str> — None cannot be represented DynamoDB map.
        let wrong = HashMap::from([(Some("key"), "v1"), (None, "v2")]);
        let result = try_to_attribute_value(wrong);
        assert!(result.is_err());
    }

    // -- AsSet vs Vec ---------------------------------------------------------

    #[test]
    fn test_as_set_vs_vec_produce_different_variants() {
        // Vec<String> → L (ordered list)
        let list_av = vec!["a".to_owned(), "b".to_owned()].into_attribute_value();
        if let AttributeValue::L(items) = &list_av {
            assert_eq!(items.len(), 2);
            assert!(items.iter().all(|v| matches!(v, AttributeValue::S(_))));
        } else {
            panic!("expected AttributeValue::L for Vec<String>");
        }

        // AsSet<String> → Ss (string set)
        let set_av = AsSet(vec!["a".to_owned(), "b".to_owned()]).into_attribute_value();
        if let AttributeValue::Ss(strings) = set_av {
            assert_eq!(strings.len(), 2);
            assert!(strings.contains(&"a".to_owned()));
            assert!(strings.contains(&"b".to_owned()));
        } else {
            panic!("expected AttributeValue::Ss for AsSet<String>");
        }
    }

    // -- AsNumber -------------------------------------------------------------

    #[test]
    fn test_as_number_preserves_exact_string() {
        // The string "1e10" must survive round-trip without reformatting.
        let av = AsNumber("1e10").into_attribute_value();
        assert_eq!(av, AttributeValue::N("1e10".to_owned()));

        // High-precision decimal that would lose precision through f64
        let av = AsNumber("12345678.90123456789099").into_attribute_value();
        assert_eq!(av, AttributeValue::N("12345678.90123456789099".to_owned()));
    }
}
