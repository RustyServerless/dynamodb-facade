use core::fmt;
use std::{borrow::Cow, collections::BTreeSet, marker::PhantomData};

use super::{
    ApplyExpressionNames, ApplyProjection, AttrNames, Expression, ProjectionableBuilder,
    fmt_attr_maps, resolve_expression, utils::resolve_attr_path,
};
use crate::{
    AttributeDefinition, CompositeKey, CompositeKeySchema, KeySchema, KeySchemaKind, SimpleKey,
    SimpleKeySchema, TableDefinition,
};

// ---------------------------------------------------------------------------
// Composable Projection type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct BuiltProjection {
    expression: Expression,
    names: AttrNames,
}

/// Projection expression builder that automatically includes the table's key attributes.
///
/// `Projection<'a, TD>` builds a DynamoDB `ProjectionExpression` that limits
/// which attributes are returned by a Get or Query/Scan operation. It always
/// includes the table's partition key (and sort key for composite-key tables)
/// so that the resulting [`Item<TD>`](crate::Item) upholds its invariant of
/// always containing the key attributes.
///
/// Attribute names that are DynamoDB reserved words are automatically escaped
/// with `#` expression attribute name placeholders.
///
/// # Examples
///
/// Projecting a subset of user attributes:
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::Projection;
///
/// // Request only "name" and "email" — PK and SK are added automatically.
/// let proj = Projection::<PlatformTable>::new(["name", "email"]);
///
/// // The rendered expression includes PK, SK, and the requested fields.
/// let rendered = format!("{proj}");
/// assert_eq!(format!("{proj}"), "PK,SK,email,name");
/// let rendered_with_placeholders = format!("{proj:#}");
/// assert!(rendered_with_placeholders.contains("#p0 = name"));
/// ```
#[derive(Debug, Clone)]
#[must_use = "expression does nothing until applied to a request"]
pub struct Projection<'a, TD> {
    attrs: BTreeSet<Cow<'a, str>>,
    _marker: PhantomData<TD>,
}

// -- Constructor --------------------------------------------------------------

impl<'a, TD: TableDefinition> Projection<'a, TD>
where
    Self: key_schema_projection::KeySchemaProjection<
            'a,
            TD::KeySchema,
            <TD::KeySchema as KeySchema>::Kind,
        >,
{
    /// Creates a projection from an iterator of attribute names.
    ///
    /// The table's key attributes (PK, and SK for composite-key tables) are
    /// **always** prepended to the provided list, ensuring the resulting
    /// [`Item<TD>`](crate::Item) is always valid for the table schema.
    ///
    /// Duplicate attribute names are deduplicated automatically.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::Projection;
    ///
    /// // Project "name" and "email"; PK + SK are added automatically.
    /// let proj = Projection::<PlatformTable>::new(["name", "email"]);
    ///
    /// let rendered = format!("{proj}");
    /// assert!(rendered.contains("PK"));
    /// assert!(rendered.contains("SK"));
    /// assert!(rendered.contains("name"));
    /// assert!(rendered.contains("email"));
    /// let rendered_with_placeholders = format!("{proj:#}");
    /// assert!(rendered_with_placeholders.contains("#p0 = name"));
    /// ```
    pub fn new(attrs: impl IntoIterator<Item = impl Into<Cow<'a, str>>>) -> Self {
        Self {
            attrs: <Self as key_schema_projection::KeySchemaProjection<
                'a,
                TD::KeySchema,
                <TD::KeySchema as KeySchema>::Kind,
            >>::key_schema_names()
            .chain(attrs.into_iter().map(Into::into))
            .collect(),
            _marker: PhantomData,
        }
    }

    /// Creates a projection that contains **only** the table's key attributes.
    ///
    /// The result is a projection that includes exactly PK (for simple-key tables)
    /// or PK + SK (for composite-key tables).
    ///
    /// This is useful when you want to list or scan matching items without
    /// fetching any payload data — for example, collecting the keys of all
    /// enrollments for a user before issuing a batch delete.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::Projection;
    ///
    /// // PlatformTable has composite key PK + SK — those are the only attributes returned.
    /// let proj = Projection::<PlatformTable>::keys_only();
    ///
    /// let rendered = format!("{proj}");
    /// assert_eq!(rendered, "PK,SK");
    /// ```
    pub fn keys_only() -> Self {
        Self::new([] as [Cow<'a, str>; 0])
    }
}

mod key_schema_projection {
    //! Provides key attribute names to seed every [`Projection`], ensuring key attributes
    //! are always included regardless of the caller-supplied attribute list.
    use super::*;

    /// Returns the key attribute names that must always be present in a projection.
    pub trait KeySchemaProjection<'a, KS: KeySchema, KSK: KeySchemaKind> {
        fn key_schema_names() -> impl Iterator<Item = Cow<'a, str>>;
    }

    impl<'a, TD: TableDefinition> KeySchemaProjection<'a, TD::KeySchema, SimpleKey>
        for Projection<'a, TD>
    where
        TD::KeySchema: SimpleKeySchema,
    {
        fn key_schema_names() -> impl Iterator<Item = Cow<'a, str>> {
            [<<TD::KeySchema as KeySchema>::PartitionKey as AttributeDefinition>::NAME.into()]
                .into_iter()
        }
    }
    impl<'a, TD: TableDefinition> KeySchemaProjection<'a, TD::KeySchema, CompositeKey>
        for Projection<'a, TD>
    where
        TD::KeySchema: CompositeKeySchema,
    {
        fn key_schema_names() -> impl Iterator<Item = Cow<'a, str>> {
            [
                <<TD::KeySchema as KeySchema>::PartitionKey as AttributeDefinition>::NAME.into(),
                <<TD::KeySchema as CompositeKeySchema>::SortKey as AttributeDefinition>::NAME
                    .into(),
            ]
            .into_iter()
        }
    }
}

// -- Internal build machinery -------------------------------------------------

impl<TD> Projection<'_, TD> {
    /// Resolves all attribute paths and assembles the projection expression string.
    fn build(&self) -> BuiltProjection {
        let mut counter = 0;
        let mut all_names = Vec::new();
        let mut resolved_parts = Vec::new();

        for attr in &self.attrs {
            let (expr, names) = resolve_attr_path(attr, "p", &mut counter);
            all_names.extend(names);
            resolved_parts.push(expr.into_owned());
        }

        BuiltProjection {
            expression: resolved_parts.join(","),
            names: all_names,
        }
    }
}

// -- Display ------------------------------------------------------------------

impl<TD> fmt::Display for Projection<'_, TD> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let built = self.build();
        if built.expression.is_empty() {
            return f.write_str("<none>");
        }
        let no_values = vec![];
        if f.alternate() {
            f.write_str(&built.expression)?;
            fmt_attr_maps(f, &built.names, &no_values)
        } else {
            f.write_str(&resolve_expression(
                &built.expression,
                &built.names,
                &no_values,
            ))
        }
    }
}

// -- ApplyProjection impl -----------------------------------------------------

impl<TD, B: ProjectionableBuilder> ApplyProjection<B> for Projection<'_, TD> {
    fn apply_projection(self, builder: B) -> B {
        let built = self.build();
        if built.expression.is_empty() {
            return builder;
        }
        builder
            .projection_expression(built.expression)
            .apply_names(built.names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::*;

    // Local simple-key table for tests that need a non-composite key schema.
    crate::attribute_definitions! {
        MyPk { "MyPk": crate::StringAttribute }
    }
    crate::table_definitions! {
        SimpleTable {
            type PartitionKey = MyPk;
            fn table_name() -> String { "simple".to_owned() }
        }
    }

    // -- Auto-inclusion + dedup -----------------------------------------------

    #[test]
    fn test_projection_new_simple_key_auto_includes_pk() {
        // BTreeSet ordering (ASCII): uppercase before lowercase.
        // "MyPk" < "email" < "name" → sorted: MyPk, email, name
        let proj = Projection::<SimpleTable>::new(["name", "email"]);
        let output = format!("{proj}");
        assert_eq!(output, "MyPk,email,name");
    }

    #[test]
    fn test_projection_new_composite_key_auto_includes_pk_and_sk() {
        // BTreeSet ordering: "PK" < "SK" < "email" < "role"
        let proj = Projection::<PlatformTable>::new(["email", "role"]);
        let output = format!("{proj}");
        assert_eq!(output, "PK,SK,email,role");
    }

    #[test]
    fn test_projection_new_dedup_when_user_supplies_pk() {
        // User supplies "PK" and "SK" explicitly — BTreeSet deduplicates them.
        // Result should be identical to supplying only "email".
        let proj = Projection::<PlatformTable>::new(["PK", "SK", "email"]);
        let output = format!("{proj}");
        assert_eq!(output, "PK,SK,email");
    }

    // -- Display --------------------------------------------------------------

    #[test]
    fn test_projection_display_default_with_reserved_word() {
        // "Status" is a DynamoDB reserved word.
        // BTreeSet ordering: "PK" < "SK" < "Status" < "email"
        // Default mode resolves placeholders inline → "Status" appears as-is.
        let proj = Projection::<PlatformTable>::new(["email", "Status"]);
        let output = format!("{proj}");
        assert_eq!(output, "PK,SK,Status,email");
    }

    #[test]
    fn test_projection_display_alternate_with_reserved_word() {
        // Alternate mode: raw expression with #p0 placeholder + name map.
        // BTreeSet ordering: "PK" < "SK" < "Status" < "email"
        // "Status" is reserved → replaced with #p0 (counter starts at 0,
        // iterates BTreeSet in order: PK (not reserved), SK (not reserved),
        // Status (reserved → #p0), email (not reserved)).
        let proj = Projection::<PlatformTable>::new(["email", "Status"]);
        let output = format!("{proj:#}");
        assert_eq!(output, "PK,SK,#p0,email\n  names: { #p0 = Status }");
    }

    // -- Empty projection -----------------------------------------------------

    // NOTE: `Projection::new` always auto-inserts the table's key attributes
    // (PK for simple-key tables, PK + SK for composite-key tables), so it is
    // impossible to construct an empty `Projection` via the public constructor.
    // The `<none>` branch in `Display` and the no-op path in `apply_projection`
    // are only reachable if `attrs` is empty after `build()`, which cannot
    // happen through `Projection::new`. No test is written for this branch
    // because there is no public API to reach it.
}
