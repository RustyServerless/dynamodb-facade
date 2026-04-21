use core::fmt;
use std::borrow::Cow;

use super::{
    super::IntoAttributeValue, ApplyCondition, ApplyExpressionAttributes, ApplyFilter,
    ApplyKeyCondition, AttrNames, AttrValues, AttributeValue, ConditionableBuilder, Expression,
    FilterableBuilder, KeyConditionableBuilder, fmt_attr_maps, resolve_expression,
    utils::resolve_attr_path,
};

/// Comparison operators for DynamoDB condition and filter expressions.
///
/// Used with [`Condition::cmp`] and [`Condition::size_cmp`] to build
/// comparison expressions. The convenience constructors generally cover
/// the common cases without needing to name this enum directly.
///
/// # Examples
///
/// ```
/// use dynamodb_facade::{Comparison, Condition};
///
/// // Using the enum directly with cmp():
/// let cond = Condition::cmp("progress", Comparison::Ge, 0.5);
/// assert_eq!(format!("{cond}"), r#"progress >= N("0.5")"#);
///
/// // Equivalent shorthand:
/// let cond = Condition::ge("progress", 0.5);
/// assert_eq!(format!("{cond}"), r#"progress >= N("0.5")"#);
/// ```
#[derive(Debug, Clone, Copy)]
pub enum Comparison {
    /// Equal (`=`).
    Eq,

    /// Not equal (`<>`).
    Neq,

    /// Less than (`<`).
    Lt,

    /// Less than or equal (`<=`).
    Le,

    /// Greater than (`>`).
    Gt,

    /// Greater than or equal (`>=`).
    Ge,
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Comparison::Eq => "=",
            Comparison::Neq => "<>",
            Comparison::Lt => "<",
            Comparison::Le => "<=",
            Comparison::Gt => ">",
            Comparison::Ge => ">=",
        })
    }
}

// ---------------------------------------------------------------------------
// Composable Condition type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum ConditionInner<'a> {
    // -- Logical combinators --------------------------------------------------
    And(Vec<Condition<'a>>),

    Or(Vec<Condition<'a>>),

    Not(Box<Condition<'a>>),

    // -- Comparisons ----------------------------------------------------------
    Compare {
        attr: Cow<'a, str>,
        cmp: Comparison,
        value: AttributeValue,
    },

    // -- Range / set operators ------------------------------------------------
    Between {
        attr: Cow<'a, str>,
        low: AttributeValue,
        high: AttributeValue,
    },

    In {
        attr: Cow<'a, str>,
        values: Vec<AttributeValue>,
    },

    // -- Functions ------------------------------------------------------------
    Exists(Cow<'a, str>),

    NotExists(Cow<'a, str>),

    BeginsWith {
        attr: Cow<'a, str>,
        prefix: AttributeValue,
    },

    Contains {
        attr: Cow<'a, str>,
        value: AttributeValue,
    },

    // -- Size -----------------------------------------------------------------
    SizeCompare {
        attr: Cow<'a, str>,
        cmp: Comparison,
        value: AttributeValue,
    },
}

/// Composable DynamoDB condition expression builder.
///
/// `Condition<'a>` represents a single DynamoDB condition expression that can
/// be used as a `ConditionExpression`, or `FilterExpression`. Conditions are
/// built from static constructor methods and composed with the `&`
/// ([`BitAnd`](core::ops::BitAnd)), `|` ([`BitOr`](core::ops::BitOr)), and `!`
/// ([`Not`](core::ops::Not)) operators, or with the variadic
/// [`and`](Condition::and) and [`or`](Condition::or) methods.
///
/// Attribute names that are DynamoDB reserved words are automatically escaped
/// with `#` expression attribute name placeholders. You never need to manage
/// placeholder names manually.
///
/// # Display
///
/// `Condition` implements [`Display`](core::fmt::Display) in two modes:
///
/// - **Default (`{}`)** — resolves all placeholders inline for human-readable
///   debugging: `PK = S("USER#user-1")`
/// - **Alternate (`{:#}`)** — shows the raw expression with `#name` / `:value`
///   placeholders and separate name/value maps, matching what DynamoDB receives:
///   `PK = :c0\n  values: { :c0 = S("USER#user-1") }`
///
/// # Examples
///
/// Simple equality condition:
///
/// ```
/// use dynamodb_facade::Condition;
///
/// let cond = Condition::eq("role", "instructor");
/// assert_eq!(format!("{cond}"), r#"role = S("instructor")"#);
/// ```
///
/// Composing conditions with operators:
///
/// ```
/// use dynamodb_facade::Condition;
///
/// let cond = Condition::exists("email")
///     & !Condition::eq("role", "banned");
/// assert_eq!(
///     format!("{cond}"),
///     r#"(attribute_exists(email) AND (NOT role = S("banned")))"#
/// );
/// ```
///
/// Variadic composition:
///
/// ```
/// use dynamodb_facade::{Comparison, Condition};
///
/// let cond = Condition::and([
///     Condition::eq("role", "student"),
///     Condition::size_gt("tags", 0),
///     Condition::exists("enrolled_at"),
/// ]);
/// assert_eq!(
///     format!("{cond}"),
///     r#"(role = S("student") AND size(tags) > N("0") AND attribute_exists(enrolled_at))"#
/// );
/// ```
#[derive(Debug, Clone)]
#[must_use = "condition does nothing until applied to a request"]
pub struct Condition<'a>(ConditionInner<'a>);

// -- Constructor methods ------------------------------------------------------

impl<'a> Condition<'a> {
    // Comparisons

    /// Creates a condition that compares an attribute to a value using the given operator.
    ///
    /// This is the general form underlying the convenience methods [`eq`](Condition::eq),
    /// [`ne`](Condition::ne), [`lt`](Condition::lt), [`le`](Condition::le),
    /// [`gt`](Condition::gt), and [`ge`](Condition::ge).
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::{Comparison, Condition};
    ///
    /// let cond = Condition::cmp("progress", Comparison::Ge, 0.75);
    /// assert_eq!(format!("{cond}"), r#"progress >= N("0.75")"#);
    /// ```
    pub fn cmp(
        attr: impl Into<Cow<'a, str>>,
        cmp: Comparison,
        value: impl IntoAttributeValue,
    ) -> Self {
        Self(ConditionInner::Compare {
            attr: attr.into(),
            cmp,
            value: value.into_attribute_value(),
        })
    }

    /// Creates an equality condition: `attr = value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::eq("role", "instructor");
    /// assert_eq!(format!("{cond}"), r#"role = S("instructor")"#);
    /// ```
    pub fn eq(attr: impl Into<Cow<'a, str>>, value: impl IntoAttributeValue) -> Self {
        Self::cmp(attr, Comparison::Eq, value)
    }

    /// Creates a not-equal condition: `attr <> value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::ne("role", "banned");
    /// assert_eq!(format!("{cond}"), r#"role <> S("banned")"#);
    /// ```
    pub fn ne(attr: impl Into<Cow<'a, str>>, value: impl IntoAttributeValue) -> Self {
        Self::cmp(attr, Comparison::Neq, value)
    }

    /// Creates a less-than condition: `attr < value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::lt("progress", 1.0);
    /// assert_eq!(format!("{cond}"), r#"progress < N("1")"#);
    /// ```
    pub fn lt(attr: impl Into<Cow<'a, str>>, value: impl IntoAttributeValue) -> Self {
        Self::cmp(attr, Comparison::Lt, value)
    }

    /// Creates a less-than-or-equal condition: `attr <= value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::le("max_enrollments", 100);
    /// assert_eq!(format!("{cond}"), r#"max_enrollments <= N("100")"#);
    /// ```
    pub fn le(attr: impl Into<Cow<'a, str>>, value: impl IntoAttributeValue) -> Self {
        Self::cmp(attr, Comparison::Le, value)
    }

    /// Creates a greater-than condition: `attr > value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::gt("enrolled_at", 0);
    /// assert_eq!(format!("{cond}"), r#"enrolled_at > N("0")"#);
    /// ```
    pub fn gt(attr: impl Into<Cow<'a, str>>, value: impl IntoAttributeValue) -> Self {
        Self::cmp(attr, Comparison::Gt, value)
    }

    /// Creates a greater-than-or-equal condition: `attr >= value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::ge("progress", 0.5);
    /// assert_eq!(format!("{cond}"), r#"progress >= N("0.5")"#);
    /// ```
    pub fn ge(attr: impl Into<Cow<'a, str>>, value: impl IntoAttributeValue) -> Self {
        Self::cmp(attr, Comparison::Ge, value)
    }

    // Range / set

    /// Creates a range condition: `attr BETWEEN low AND high` (inclusive).
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::between("enrolled_at", 1_700_000_000, 1_800_000_000);
    /// assert_eq!(
    ///     format!("{cond}"),
    ///     r#"enrolled_at BETWEEN N("1700000000") AND N("1800000000")"#
    /// );
    /// ```
    pub fn between(
        attr: impl Into<Cow<'a, str>>,
        low: impl IntoAttributeValue,
        high: impl IntoAttributeValue,
    ) -> Self {
        Self(ConditionInner::Between {
            attr: attr.into(),

            low: low.into_attribute_value(),
            high: high.into_attribute_value(),
        })
    }

    /// Creates a membership condition: `attr IN (val1, val2, ...)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::is_in("role", ["student", "instructor"]);
    /// assert_eq!(
    ///     format!("{cond}"),
    ///     r#"role IN (S("student"), S("instructor"))"#
    /// );
    /// ```
    pub fn is_in(
        attr: impl Into<Cow<'a, str>>,
        values: impl IntoIterator<Item = impl IntoAttributeValue>,
    ) -> Self {
        Self(ConditionInner::In {
            attr: attr.into(),
            values: values
                .into_iter()
                .map(IntoAttributeValue::into_attribute_value)
                .collect(),
        })
    }

    // Functions

    /// Creates an attribute-existence condition: `attribute_exists(attr)`.
    ///
    /// Evaluates to true when the named attribute is present on the item,
    /// regardless of its value.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::exists("email");
    /// assert_eq!(format!("{cond}"), "attribute_exists(email)");
    /// ```
    pub fn exists(attr: impl Into<Cow<'a, str>>) -> Self {
        Self(ConditionInner::Exists(attr.into()))
    }

    /// Creates an attribute-absence condition: `attribute_not_exists(attr)`.
    ///
    /// Evaluates to true when the named attribute is absent from the item.
    /// Commonly used to implement create-only semantics (e.g. "put only if
    /// item does not exist").
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::not_exists("deleted_at");
    /// assert_eq!(format!("{cond}"), "attribute_not_exists(deleted_at)");
    /// ```
    pub fn not_exists(attr: impl Into<Cow<'a, str>>) -> Self {
        Self(ConditionInner::NotExists(attr.into()))
    }

    /// Creates a prefix condition: `begins_with(attr, prefix)`.
    ///
    /// Evaluates to true when the string attribute starts with the given prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::begins_with("SK", "ENROLL#");
    /// assert_eq!(format!("{cond}"), r#"begins_with(SK, S("ENROLL#"))"#);
    /// ```
    pub fn begins_with(attr: impl Into<Cow<'a, str>>, prefix: impl IntoAttributeValue) -> Self {
        Self(ConditionInner::BeginsWith {
            attr: attr.into(),
            prefix: prefix.into_attribute_value(),
        })
    }

    /// Creates a containment condition: `contains(attr, value)`.
    ///
    /// For string attributes, evaluates to true when the attribute contains
    /// `value` as a substring. For set attributes, evaluates to true when the
    /// set contains `value` as an element.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::contains("tags", "rust");
    /// assert_eq!(format!("{cond}"), r#"contains(tags, S("rust"))"#);
    /// ```
    pub fn contains(attr: impl Into<Cow<'a, str>>, value: impl IntoAttributeValue) -> Self {
        Self(ConditionInner::Contains {
            attr: attr.into(),
            value: value.into_attribute_value(),
        })
    }

    // Size

    /// Creates a size comparison condition: `size(attr) <op> value`.
    ///
    /// Compares the size of an attribute (string length, list/map/set
    /// cardinality, or binary length) to a [`usize`] using the given
    /// [`Comparison`] operator. This is the general form underlying
    /// the convenience methods [`size_eq`](Condition::size_eq),
    /// [`size_ne`](Condition::size_ne), [`size_lt`](Condition::size_lt),
    /// [`size_le`](Condition::size_le), [`size_gt`](Condition::size_gt), and
    /// [`size_ge`](Condition::size_ge).
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::{Comparison, Condition};
    ///
    /// let cond = Condition::size_cmp("tags", Comparison::Ge, 1);
    /// assert_eq!(format!("{cond}"), r#"size(tags) >= N("1")"#);
    /// ```
    pub fn size_cmp(attr: impl Into<Cow<'a, str>>, cmp: Comparison, value: usize) -> Self {
        Self(ConditionInner::SizeCompare {
            attr: attr.into(),
            cmp,
            value: value.into_attribute_value(),
        })
    }

    /// Creates a size-equal condition: `size(attr) = value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::size_eq("tags", 3);
    /// assert_eq!(format!("{cond}"), r#"size(tags) = N("3")"#);
    /// ```
    pub fn size_eq(attr: impl Into<Cow<'a, str>>, value: usize) -> Self {
        Self::size_cmp(attr, Comparison::Eq, value)
    }

    /// Creates a size-not-equal condition: `size(attr) <> value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::size_ne("tags", 0);
    /// assert_eq!(format!("{cond}"), r#"size(tags) <> N("0")"#);
    /// ```
    pub fn size_ne(attr: impl Into<Cow<'a, str>>, value: usize) -> Self {
        Self::size_cmp(attr, Comparison::Neq, value)
    }

    /// Creates a size-less-than condition: `size(attr) < value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::size_lt("content", 1000);
    /// assert_eq!(format!("{cond}"), r#"size(content) < N("1000")"#);
    /// ```
    pub fn size_lt(attr: impl Into<Cow<'a, str>>, value: usize) -> Self {
        Self::size_cmp(attr, Comparison::Lt, value)
    }

    /// Creates a size-less-than-or-equal condition: `size(attr) <= value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::size_le("bio", 500);
    /// assert_eq!(format!("{cond}"), r#"size(bio) <= N("500")"#);
    /// ```
    pub fn size_le(attr: impl Into<Cow<'a, str>>, value: usize) -> Self {
        Self::size_cmp(attr, Comparison::Le, value)
    }

    /// Creates a size-greater-than condition: `size(attr) > value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::size_gt("tags", 0);
    /// assert_eq!(format!("{cond}"), r#"size(tags) > N("0")"#);
    /// ```
    pub fn size_gt(attr: impl Into<Cow<'a, str>>, value: usize) -> Self {
        Self::size_cmp(attr, Comparison::Gt, value)
    }
    /// Creates a size-greater-than-or-equal condition: `size(attr) >= value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::size_ge("email", 5);
    /// assert_eq!(format!("{cond}"), r#"size(email) >= N("5")"#);
    /// ```
    pub fn size_ge(attr: impl Into<Cow<'a, str>>, value: usize) -> Self {
        Self::size_cmp(attr, Comparison::Ge, value)
    }

    // Logical combinators

    /// Creates a condition that is true when **all** of the given conditions are true.
    ///
    /// Nested `And` conditions are flattened automatically. An empty iterator
    /// produces a no-op condition that renders as `<none>` and is silently
    /// dropped when applied to a builder.
    ///
    /// For combining a fixed number of conditions, the `&` operator is more ergonomic.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::and([
    ///     Condition::exists("email"),
    ///     Condition::eq("role", "student"),
    ///     Condition::gt("enrolled_at", 0),
    /// ]);
    /// assert_eq!(
    ///     format!("{cond}"),
    ///     r#"(attribute_exists(email) AND role = S("student") AND enrolled_at > N("0"))"#
    /// );
    /// ```
    ///
    /// Empty iterator produces a no-op:
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::and([] as [Condition; 0]);
    /// assert_eq!(format!("{cond}"), "<none>");
    /// ```
    ///
    /// The & operator produce the same result:
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond1 = Condition::and([
    ///     Condition::exists("email"),
    ///     Condition::eq("role", "student"),
    ///     Condition::gt("enrolled_at", 0),
    /// ]);
    ///
    /// let cond2 = Condition::exists("email")
    ///     & Condition::eq("role", "student")
    ///     & Condition::gt("enrolled_at", 0);
    /// assert_eq!(format!("{cond1}"), format!("{cond2}"));
    /// ```
    pub fn and(conditions: impl IntoIterator<Item = Condition<'a>>) -> Self {
        let iterator = conditions.into_iter();
        let est_size = iterator.size_hint().0;
        Self(ConditionInner::And(iterator.fold(
            Vec::with_capacity(est_size),
            |mut conditions, c| {
                match c.0 {
                    ConditionInner::And(conds) => {
                        conditions.extend(conds);
                    }
                    _ => {
                        conditions.push(c);
                    }
                };
                conditions
            },
        )))
    }

    /// Creates a condition that is true when **any** of the given conditions is true.
    ///
    /// Nested `Or` conditions are flattened automatically. An empty iterator
    /// produces a no-op condition that renders as `<none>` and is silently
    /// dropped when applied to a builder.
    ///
    /// For combining a fixed number of conditions, the `|` operator is more ergonomic.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::or([
    ///     Condition::not_exists("deleted_at"),
    ///     Condition::eq("role", "admin"),
    /// ]);
    /// assert_eq!(
    ///     format!("{cond}"),
    ///     r#"(attribute_not_exists(deleted_at) OR role = S("admin"))"#
    /// );
    /// ```
    ///
    /// Empty iterator produces a no-op:
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond = Condition::or([] as [Condition; 0]);
    /// assert_eq!(format!("{cond}"), "<none>");
    /// ```
    ///
    /// The | operator produce the same result:
    ///
    /// ```
    /// use dynamodb_facade::Condition;
    ///
    /// let cond1 = Condition::or([
    ///     Condition::not_exists("deleted_at"),
    ///     Condition::eq("role", "admin"),
    /// ]);
    ///
    /// let cond2 = Condition::not_exists("deleted_at")
    ///     | Condition::eq("role", "admin");
    /// assert_eq!(format!("{cond1}"), format!("{cond2}"));
    /// ```
    pub fn or(conditions: impl IntoIterator<Item = Condition<'a>>) -> Self {
        let iterator = conditions.into_iter();
        let est_size = iterator.size_hint().0;
        Self(ConditionInner::Or(iterator.fold(
            Vec::with_capacity(est_size),
            |mut conditions, c| {
                match c.0 {
                    ConditionInner::Or(conds) => {
                        conditions.extend(conds);
                    }
                    _ => {
                        conditions.push(c);
                    }
                };
                conditions
            },
        )))
    }
}

/// Negates a condition: `NOT cond`.
///
/// Applying `!` twice cancels out — `!!cond` returns the original condition.
///
/// # Examples
///
/// ```
/// use dynamodb_facade::Condition;
///
/// let cond = !Condition::eq("role", "banned");
/// assert_eq!(format!("{cond}"), r#"(NOT role = S("banned"))"#);
///
/// // Double negation cancels out.
/// let cond = !!Condition::exists("email");
/// assert_eq!(format!("{cond}"), "attribute_exists(email)");
/// ```
impl<'a> core::ops::Not for Condition<'a> {
    type Output = Condition<'a>;

    fn not(self) -> Self::Output {
        match self.0 {
            ConditionInner::Not(condition) => *condition,
            _ => Self(ConditionInner::Not(Box::new(self))),
        }
    }
}

/// Combines two conditions with AND: `lhs & rhs`.
///
/// Equivalent to `Condition::and([lhs, rhs])`. Nested `And` conditions are
/// flattened automatically.
///
/// # Examples
///
/// ```
/// use dynamodb_facade::Condition;
///
/// let cond = Condition::exists("email") & Condition::eq("role", "student");
/// assert!(format!("{cond}").contains("AND"));
/// assert_eq!(
///     format!("{cond}"),
///     r#"(attribute_exists(email) AND role = S("student"))"#
/// );
/// ```
impl<'a> core::ops::BitAnd for Condition<'a> {
    type Output = Condition<'a>;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self::and([self, rhs])
    }
}

/// Combines two conditions with OR: `lhs | rhs`.
///
/// Equivalent to `Condition::or([lhs, rhs])`. Nested `Or` conditions are
/// flattened automatically.
///
/// # Examples
///
/// ```
/// use dynamodb_facade::Condition;
///
/// let cond = Condition::not_exists("deleted_at") | Condition::eq("role", "admin");
/// assert_eq!(
///     format!("{cond}"),
///     r#"(attribute_not_exists(deleted_at) OR role = S("admin"))"#
/// );
/// ```
impl<'a> core::ops::BitOr for Condition<'a> {
    type Output = Condition<'a>;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self::or([self, rhs])
    }
}

// -- Internal build machinery -------------------------------------------------

#[derive(Debug, Default)]
struct BuiltCondition {
    expression: Expression,
    names: AttrNames,
    values: AttrValues,
}
impl BuiltCondition {
    const EMPTY: Self = Self {
        expression: String::new(),
        names: vec![],
        values: vec![],
    };
}

impl Condition<'_> {
    /// Builds the condition starting the placeholder counter at zero.
    fn build(self) -> BuiltCondition {
        self.build_with_counter(&mut 0)
    }

    /// Builds the condition using a shared counter for unique placeholder names.
    fn build_with_counter(self, counter: &mut usize) -> BuiltCondition {
        match self.0 {
            ConditionInner::And(conditions) => Self::build_logical(conditions, " AND ", counter),
            ConditionInner::Or(conditions) => Self::build_logical(conditions, " OR ", counter),

            ConditionInner::Not(inner) => {
                let mut built = inner.build_with_counter(counter);
                if !built.expression.is_empty() {
                    built.expression = format!("(NOT {})", built.expression);
                }
                built
            }

            ConditionInner::Compare { attr, cmp, value } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "c", counter);
                let val_id = *counter;
                *counter += 1;
                let val_ph = format!(":c{val_id}");
                BuiltCondition {
                    expression: format!("{attr_expr} {cmp} {val_ph}"),
                    names,
                    values: vec![(val_ph, value)],
                }
            }

            ConditionInner::Between { attr, low, high } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "c", counter);
                let val_id = *counter;
                *counter += 1;
                let lo_ph = format!(":c{val_id}lo");
                let hi_ph = format!(":c{val_id}hi");
                BuiltCondition {
                    expression: format!("{attr_expr} BETWEEN {lo_ph} AND {hi_ph}"),
                    names,
                    values: vec![(lo_ph, low), (hi_ph, high)],
                }
            }

            ConditionInner::In { attr, values } => {
                if values.is_empty() {
                    BuiltCondition::EMPTY
                } else {
                    let (attr_expr, names) = resolve_attr_path(&attr, "c", counter);
                    let val_id = *counter;
                    *counter += 1;
                    let val_phs: Vec<String> = (0..values.len())
                        .map(|i| format!(":c{val_id}i{i}"))
                        .collect();
                    let in_list = val_phs.join(", ");
                    BuiltCondition {
                        expression: format!("{attr_expr} IN ({in_list})"),
                        names,
                        values: val_phs.into_iter().zip(values.iter().cloned()).collect(),
                    }
                }
            }

            ConditionInner::Exists(attr) => {
                let (attr_expr, names) = resolve_attr_path(&attr, "c", counter);
                BuiltCondition {
                    expression: format!("attribute_exists({attr_expr})"),
                    names,
                    values: vec![],
                }
            }

            ConditionInner::NotExists(attr) => {
                let (attr_expr, names) = resolve_attr_path(&attr, "c", counter);
                BuiltCondition {
                    expression: format!("attribute_not_exists({attr_expr})"),
                    names,
                    values: vec![],
                }
            }

            ConditionInner::BeginsWith { attr, prefix } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "c", counter);
                let val_id = *counter;
                *counter += 1;
                let prefix_ph = format!(":c{val_id}");
                BuiltCondition {
                    expression: format!("begins_with({attr_expr}, {prefix_ph})"),
                    names,
                    values: vec![(prefix_ph, prefix)],
                }
            }

            ConditionInner::Contains { attr, value } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "c", counter);
                let val_id = *counter;
                *counter += 1;
                let val_ph = format!(":c{val_id}");
                BuiltCondition {
                    expression: format!("contains({attr_expr}, {val_ph})"),
                    names,
                    values: vec![(val_ph, value)],
                }
            }

            ConditionInner::SizeCompare { attr, cmp, value } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "c", counter);
                let val_id = *counter;
                *counter += 1;
                let val_ph = format!(":c{val_id}");
                BuiltCondition {
                    expression: format!("size({attr_expr}) {cmp} {val_ph}"),
                    names,
                    values: vec![(val_ph, value)],
                }
            }
        }
    }

    /// Builds a logical AND/OR expression by joining sub-conditions with `operator`.
    fn build_logical(
        conditions: Vec<Condition>,
        operator: &str,
        counter: &mut usize,
    ) -> BuiltCondition {
        let mut result = BuiltCondition::default();
        let mut parts = Vec::with_capacity(conditions.len());

        for cond in conditions {
            let BuiltCondition {
                expression,
                names,
                values,
            } = cond.build_with_counter(counter);
            if !expression.is_empty() {
                parts.push(expression);
                result.names.extend(names);
                result.values.extend(values);
            }
        }

        result.expression = match parts.len() {
            0 => String::new(),
            1 => parts.pop().expect("parts has exactly one element"),
            _ => {
                let joined = parts.join(operator);
                format!("({joined})")
            }
        };

        result
    }
}

// -- Display ------------------------------------------------------------------

/// Formats the condition for display.
///
/// Two modes are supported:
///
/// - **Default (`{}`)** — resolves all `#name` and `:value` placeholders
///   inline, producing a human-readable string useful for debugging:
///   `PK = S("USER#user-1")`
/// - **Alternate (`{:#}`)** — shows the raw DynamoDB expression with
///   placeholder names, followed by the name and value maps on separate lines.
///   This matches what is actually sent to DynamoDB:
///   `PK = :c0\n  values: { :c0 = S("USER#user-1") }`
///
/// An empty condition (e.g. `Condition::and([])`) renders as `<none>` in both
/// modes.
///
/// # Examples
///
/// ```
/// use dynamodb_facade::Condition;
///
/// let cond = Condition::eq("PK", "USER#user-1");
///
/// // Default: placeholders resolved inline.
/// assert_eq!(format!("{cond}"), r#"PK = S("USER#user-1")"#);
///
/// // Alternate: raw expression + maps.
/// assert_eq!(format!("{cond:#}"), "PK = :c0\n  values: { :c0 = S(\"USER#user-1\") }");
/// ```
impl fmt::Display for Condition<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let built = self.clone().build();
        if built.expression.is_empty() {
            return f.write_str("<none>");
        }
        if f.alternate() {
            f.write_str(&built.expression)?;
            fmt_attr_maps(f, &built.names, &built.values)
        } else {
            f.write_str(&resolve_expression(
                &built.expression,
                &built.names,
                &built.values,
            ))
        }
    }
}

// -- ApplyCondition impl (condition_expression) -------------------------------

impl<B: ConditionableBuilder> ApplyCondition<B> for Condition<'_> {
    fn apply(self, builder: B) -> B {
        let built = self.build();
        if built.expression.is_empty() {
            return builder;
        }
        builder
            .condition_expression(built.expression)
            .apply_names_and_values(built.names, built.values)
    }
}

impl<B: ConditionableBuilder> ApplyCondition<B> for Option<Condition<'_>> {
    fn apply(self, builder: B) -> B {
        match self {
            Some(c) => c.apply(builder),
            None => builder,
        }
    }
}

// -- ApplyKeyCondition impl (key_condition_expression) ------------------------

impl<B: KeyConditionableBuilder> ApplyKeyCondition<B> for Condition<'_> {
    fn apply_key_condition(self, builder: B) -> B {
        let built = self.build();
        if built.expression.is_empty() {
            return builder;
        }
        builder
            .key_condition_expression(built.expression)
            .apply_names_and_values(built.names, built.values)
    }
}

// -- ApplyFilter impl (filter_expression) -------------------------------------

impl<B: FilterableBuilder> ApplyFilter<B> for Condition<'_> {
    fn apply_filter(self, builder: B) -> B {
        let built = self.build();
        if built.expression.is_empty() {
            return builder;
        }
        builder
            .filter_expression(built.expression)
            .apply_names_and_values(built.names, built.values)
    }
}

impl<B: FilterableBuilder> ApplyFilter<B> for Option<Condition<'_>> {
    fn apply_filter(self, builder: B) -> B {
        match self {
            Some(c) => c.apply_filter(builder),
            None => builder,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_display_default_simple_eq() {
        let c = Condition::eq("PK", "USER#123");
        let display = format!("{c}");
        assert_eq!(display, r#"PK = S("USER#123")"#);
    }

    #[test]
    fn test_condition_display_default_reserved_word() {
        // "Status" is a reserved word → gets a #c0 placeholder internally
        let c = Condition::eq("Status", "active");
        let display = format!("{c}");
        assert_eq!(display, r#"Status = S("active")"#);
    }

    #[test]
    fn test_condition_display_default_and_with_begins_with() {
        let c = Condition::and([
            Condition::eq("PK", "USER#123"),
            Condition::begins_with("SK", "ORDER#"),
        ]);
        let display = format!("{c}");
        assert_eq!(
            display,
            r#"(PK = S("USER#123") AND begins_with(SK, S("ORDER#")))"#
        );
    }

    #[test]
    fn test_condition_display_default_or() {
        let c = Condition::or([
            Condition::eq("attr1", "value1"),
            Condition::ne("attr2", 2345u32),
        ]);
        let display = format!("{c}");
        assert_eq!(display, r#"(attr1 = S("value1") OR attr2 <> N("2345"))"#);
    }

    #[test]
    fn test_condition_display_default_not() {
        let c = !Condition::exists("PK");
        let display = format!("{c}");
        assert_eq!(display, "(NOT attribute_exists(PK))");
    }

    #[test]
    fn test_condition_display_default_between() {
        let c = Condition::between("age", 18u32, 65u32);
        let display = format!("{c}");
        assert_eq!(display, r#"age BETWEEN N("18") AND N("65")"#);
    }

    #[test]
    fn test_condition_display_default_in() {
        let c = Condition::is_in("color", ["red", "green", "blue"]);
        let display = format!("{c}");
        assert_eq!(display, r#"color IN (S("red"), S("green"), S("blue"))"#);
    }

    #[test]
    fn test_condition_display_default_size() {
        let c = Condition::size_cmp("tags", Comparison::Ge, 3);
        let display = format!("{c}");
        assert_eq!(display, r#"size(tags) >= N("3")"#);
    }

    #[test]
    fn test_condition_display_default_contains() {
        let c = Condition::contains("description", "rust");
        let display = format!("{c}");
        assert_eq!(display, r#"contains(description, S("rust"))"#);
    }

    #[test]
    fn test_condition_display_default_empty() {
        let c = Condition::and([]);
        let display = format!("{c}");
        assert_eq!(display, "<none>");
    }

    #[test]
    fn test_condition_display_alternate_simple() {
        let c = Condition::eq("PK", "USER#123");
        let display = format!("{c:#}");
        assert_eq!(display, "PK = :c0\n  values: { :c0 = S(\"USER#123\") }");
    }

    #[test]
    fn test_condition_display_alternate_reserved_word() {
        let c = Condition::eq("Status", "active");
        let display = format!("{c:#}");
        assert_eq!(
            display,
            "#c0 = :c1\n  names: { #c0 = Status }\n  values: { :c1 = S(\"active\") }"
        );
    }

    #[test]
    fn test_condition_display_alternate_and_with_begins_with() {
        let c = Condition::and([
            Condition::eq("PK", "USER#123"),
            Condition::begins_with("SK", "ORDER#"),
        ]);
        let display = format!("{c:#}");
        assert_eq!(
            display,
            "(PK = :c0 AND begins_with(SK, :c1))\n  values: { :c0 = S(\"USER#123\"), :c1 = S(\"ORDER#\") }"
        );
    }

    #[test]
    fn test_condition_display_alternate_no_values() {
        let c = Condition::exists("PK");
        let display = format!("{c:#}");
        // No names (PK is not reserved) and no values
        assert_eq!(display, "attribute_exists(PK)");
    }

    #[test]
    fn test_condition_display_alternate_empty() {
        let c = Condition::and([]);
        let display = format!("{c:#}");
        assert_eq!(display, "<none>");
    }
}
