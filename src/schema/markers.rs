use super::*;

/// Marker trait asserting a type carries all key attributes required by `KS`.
///
/// Blanket-implemented for any type that satisfies [`HasKeyAttributesImpl`] for the schema's kind.
#[doc(hidden)]
pub trait HasKeyAttributes<KS: KeySchema>: HasKeyAttributesImpl<KS, KS::Kind> {}

/// Implementation-level marker: requires the partition key attribute, plus the sort key for composite schemas.
#[doc(hidden)]
pub trait HasKeyAttributesImpl<KS: KeySchema, K: KeySchemaKind>:
    HasAttribute<KS::PartitionKey>
{
}

impl<S, T> HasKeyAttributesImpl<S, SimpleKey> for T
where
    S: SimpleKeySchema,
    T: HasAttribute<S::PartitionKey>,
{
}
impl<S, T> HasKeyAttributesImpl<S, CompositeKey> for T
where
    S: CompositeKeySchema,
    T: HasAttribute<S::PartitionKey>,
    T: HasAttribute<S::SortKey>,
{
}
impl<KS, T> HasKeyAttributes<KS> for T
where
    KS: KeySchema,
    T: HasKeyAttributesImpl<KS, KS::Kind>,
{
}

/// Marker trait asserting a type carries all key attributes for table `TD`.
#[doc(hidden)]
pub trait HasTableKeyAttributes<TD: TableDefinition>: HasKeyAttributes<TD::KeySchema> {}
impl<TD: TableDefinition, T> HasTableKeyAttributes<TD> for T where T: HasKeyAttributes<TD::KeySchema>
{}

/// Marker trait asserting a type carries both the table key attributes and the index key attributes.
#[doc(hidden)]
pub trait HasIndexKeyAttributes<TD: TableDefinition, I: IndexDefinition<TD>>:
    HasKeyAttributes<TD::KeySchema> + HasKeyAttributes<I::KeySchema>
{
}
impl<TD: TableDefinition, I: IndexDefinition<TD>, T> HasIndexKeyAttributes<TD, I> for T where
    T: HasKeyAttributes<TD::KeySchema> + HasKeyAttributes<I::KeySchema>
{
}
