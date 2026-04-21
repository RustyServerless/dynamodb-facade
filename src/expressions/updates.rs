use core::fmt;
use std::borrow::Cow;

use super::{
    super::IntoAttributeValue, ApplyExpressionAttributes, ApplyUpdate, AttrNames, AttrValues,
    AttributeValue, Expression, UpdatableBuilder, fmt_attr_maps, resolve_expression,
    utils::resolve_attr_path,
};

// ---------------------------------------------------------------------------
// Composable Update type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum UpdateSetRhsInner<'a> {
    Value {
        value: AttributeValue,
    },
    Attribute {
        attr: Cow<'a, str>,
    },
    IfNotExists {
        attr: Cow<'a, str>,
        value: AttributeValue,
    },
    Add {
        lhs: Box<UpdateSetRhs<'a>>,
        rhs: Box<UpdateSetRhs<'a>>,
    },
    Sub {
        lhs: Box<UpdateSetRhs<'a>>,
        rhs: Box<UpdateSetRhs<'a>>,
    },
}

/// Advanced right-hand-side expression builder for DynamoDB SET actions.
///
/// `UpdateSetRhs<'a>` represents the right-hand side of a `SET attr = <rhs>`
/// expression. It is used with [`Update::set_custom`] when the simple
/// [`Update::set`] shorthand is insufficient — for example, when you need to
/// reference another attribute, use `if_not_exists`, or build complex
/// arithmetic expressions.
///
/// `UpdateSetRhs` values can be combined with `+` and `-` to ease building
/// of arithmetic expressions:
///
/// ```
/// use dynamodb_facade::{Update, UpdateSetRhs};
///
/// // SET score = other_score + 10
/// let rhs = UpdateSetRhs::attr("other_score") + UpdateSetRhs::value(10);
/// let update = Update::set_custom("score", rhs);
/// assert_eq!(format!("{update}"), r#"SET score = other_score + N("10")"#);
///
/// // SET balance = base_balance - penalty
/// let rhs = UpdateSetRhs::attr("base_balance") - UpdateSetRhs::attr("penalty");
/// let update = Update::set_custom("balance", rhs);
/// assert_eq!(format!("{update}"), "SET balance = base_balance - penalty");
/// ```
#[derive(Debug, Clone)]
#[must_use = "expression does nothing until applied to a request"]
pub struct UpdateSetRhs<'a>(UpdateSetRhsInner<'a>);

impl<'a> UpdateSetRhs<'a> {
    /// Creates an RHS that reference a literal value.
    ///
    /// This is the simplest form. For most cases, [`Update::set`] is more
    /// ergonomic and equivalent.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::{Update, UpdateSetRhs};
    ///
    /// let update = Update::set_custom("score", UpdateSetRhs::value(100));
    /// assert_eq!(format!("{update}"), r#"SET score = N("100")"#);
    ///
    /// // Same as Update::set
    /// let update = Update::set("score", 100);
    /// assert_eq!(format!("{update}"), r#"SET score = N("100")"#);
    /// ```
    pub fn value(value: impl IntoAttributeValue) -> Self {
        Self(UpdateSetRhsInner::Value {
            value: value.into_attribute_value(),
        })
    }

    /// Creates an RHS that references another attribute by name.
    ///
    /// Use this to copy one attribute's value to another, or as part of an
    /// arithmetic expression.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::{Update, UpdateSetRhs};
    ///
    /// // SET old_score = current_score
    /// let update = Update::set_custom("old_score", UpdateSetRhs::attr("current_score"));
    /// assert_eq!(format!("{update}"), "SET old_score = current_score");
    /// ```
    pub fn attr(attr: impl Into<Cow<'a, str>>) -> Self {
        Self(UpdateSetRhsInner::Attribute { attr: attr.into() })
    }

    /// Creates an RHS using `if_not_exists(attr, default)`.
    ///
    /// Evaluates to the current value of `attr` if it exists, or `default`
    /// otherwise. Useful for initializing an attribute on first write without
    /// overwriting an existing value.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::{Update, UpdateSetRhs};
    ///
    /// // SET old_score = if_not_exists(score, 0)
    /// let update = Update::set_custom(
    ///     "old_score",
    ///     UpdateSetRhs::if_not_exists("score", 0),
    /// );
    /// assert_eq!(
    ///     format!("{update}"),
    ///     r#"SET old_score = if_not_exists(score, N("0"))"#
    /// );
    /// ```
    pub fn if_not_exists(attr: impl Into<Cow<'a, str>>, value: impl IntoAttributeValue) -> Self {
        Self(UpdateSetRhsInner::IfNotExists {
            attr: attr.into(),
            value: value.into_attribute_value(),
        })
    }

    fn rhs_expr(self, counter: &mut usize) -> (Expression, AttrNames, AttrValues) {
        match self.0 {
            UpdateSetRhsInner::IfNotExists { attr, value } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "u", counter);
                let val_id = *counter;
                *counter += 1;
                let value_ph = format!(":u{val_id}");
                (
                    format!("if_not_exists({attr_expr}, {value_ph})"),
                    names,
                    vec![(value_ph, value)],
                )
            }
            UpdateSetRhsInner::Value { value } => {
                let val_id = *counter;
                *counter += 1;
                let value_ph = format!(":u{val_id}");
                (value_ph.clone(), vec![], vec![(value_ph, value)])
            }
            UpdateSetRhsInner::Attribute { attr } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "u", counter);
                (attr_expr.into_owned(), names, vec![])
            }
            UpdateSetRhsInner::Add { lhs, rhs } => {
                let mut lhs = lhs.rhs_expr(counter);
                let rhs = rhs.rhs_expr(counter);
                lhs.1.extend(rhs.1);
                lhs.2.extend(rhs.2);
                (format!("{} + {}", lhs.0, rhs.0), lhs.1, lhs.2)
            }
            UpdateSetRhsInner::Sub { lhs, rhs } => {
                let mut lhs = lhs.rhs_expr(counter);
                let rhs = rhs.rhs_expr(counter);
                lhs.1.extend(rhs.1);
                lhs.2.extend(rhs.2);
                (format!("{} - {}", lhs.0, rhs.0), lhs.1, lhs.2)
            }
        }
    }
}

/// Combines two RHS expressions with addition: `expr1 + expr2`.
///
/// Produces a `expr1 + expr2` arithmetic expression fragment for use in a SET
/// action.
///
/// # Examples
///
/// ```
/// use dynamodb_facade::{Update, UpdateSetRhs};
///
/// // SET score = base_score + 10
/// let update = Update::set_custom(
///     "score",
///     UpdateSetRhs::attr("base_score") + UpdateSetRhs::value(10),
/// );
/// assert_eq!(format!("{update}"), r#"SET score = base_score + N("10")"#);
///
/// // SET balance = base_balance + last_payment
/// let rhs = UpdateSetRhs::attr("base_balance") + UpdateSetRhs::attr("last_payment");
/// let update = Update::set_custom("balance", rhs);
/// assert_eq!(format!("{update}"), "SET balance = base_balance + last_payment");
/// ```
impl<'a> core::ops::Add<UpdateSetRhs<'a>> for UpdateSetRhs<'a> {
    type Output = UpdateSetRhs<'a>;

    fn add(self, rhs: UpdateSetRhs<'a>) -> Self::Output {
        Self(UpdateSetRhsInner::Add {
            lhs: Box::new(self),
            rhs: Box::new(rhs),
        })
    }
}

/// Combines two RHS expressions with subtraction: `expr1 - expr2`.
///
/// Produces a `expr1 - expr2` arithmetic expression fragment for use in a SET
/// action.
///
/// # Examples
///
/// ```
/// use dynamodb_facade::{Update, UpdateSetRhs};
///
/// // SET balance = base_balance - 5
/// let update = Update::set_custom(
///     "balance",
///     UpdateSetRhs::attr("base_balance") - UpdateSetRhs::value(5),
/// );
/// assert_eq!(format!("{update}"), r#"SET balance = base_balance - N("5")"#);
///
/// // SET balance = base_balance - penalty
/// let rhs = UpdateSetRhs::attr("base_balance") - UpdateSetRhs::attr("penalty");
/// let update = Update::set_custom("balance", rhs);
/// assert_eq!(format!("{update}"), "SET balance = base_balance - penalty");
/// ```
impl<'a> core::ops::Sub<UpdateSetRhs<'a>> for UpdateSetRhs<'a> {
    type Output = UpdateSetRhs<'a>;

    fn sub(self, rhs: UpdateSetRhs<'a>) -> Self::Output {
        Self(UpdateSetRhsInner::Sub {
            lhs: Box::new(self),
            rhs: Box::new(rhs),
        })
    }
}

#[derive(Debug, Clone)]
enum UpdateInner<'a> {
    Combine(Vec<Update<'a>>),

    // -- SET actions ----------------------------------------------------------
    Set {
        attr: Cow<'a, str>,
        value: AttributeValue,
    },

    SetIfNotExists {
        attr: Cow<'a, str>,
        value: AttributeValue,
    },

    Increment {
        attr: Cow<'a, str>,
        by: AttributeValue,
    },

    Decrement {
        attr: Cow<'a, str>,
        by: AttributeValue,
    },

    InitIncrement {
        attr: Cow<'a, str>,
        initial: AttributeValue,
        by: AttributeValue,
    },

    InitDecrement {
        attr: Cow<'a, str>,
        initial: AttributeValue,
        by: AttributeValue,
    },

    ListAppend {
        attr: Cow<'a, str>,
        value: AttributeValue,
    },

    ListPrepend {
        attr: Cow<'a, str>,
        value: AttributeValue,
    },

    SetCustom {
        attr: Cow<'a, str>,
        rhs: UpdateSetRhs<'a>,
    },

    // -- REMOVE actions -------------------------------------------------------
    Remove {
        attr: Cow<'a, str>,
    },

    ListRemove {
        attr: Cow<'a, str>,
        index: usize,
    },

    // -- ADD actions ----------------------------------------------------------
    Add {
        attr: Cow<'a, str>,
        value: AttributeValue,
    },

    // -- DELETE actions -------------------------------------------------------
    Delete {
        attr: Cow<'a, str>,
        set: AttributeValue,
    },
}

/// Composable DynamoDB update expression builder.
///
/// `Update<'a>` represents a single DynamoDB update expression that can be
/// used as an `UpdateExpression`. It supports all four DynamoDB update clauses
/// — `SET`, `REMOVE`, `ADD`, and `DELETE` — and multiple actions can be
/// combined into a single expression with [`and`](Update::and),
/// [`combine`](Update::combine), or [`try_combine`](Update::try_combine).
///
/// Attribute names that are DynamoDB reserved words are automatically escaped
/// with `#` expression attribute name placeholders.
///
/// # Display
///
/// `Update` implements [`Display`](core::fmt::Display) in two modes:
///
/// - **Default (`{}`)** — resolves all placeholders inline for human-readable
///   debugging: `SET balance = N("100")`
/// - **Alternate (`{:#}`)** — shows the raw expression with `#name` / `:value`
///   placeholders and separate maps: `SET balance = :u0\n  values: { :u0 = N("100") }`
///
/// # Examples
///
/// Simple SET:
///
/// ```
/// use dynamodb_facade::Update;
///
/// let update = Update::set("role", "instructor");
/// assert_eq!(format!("{update}"), r#"SET role = S("instructor")"#);
/// ```
///
/// Combining multiple actions:
///
/// ```
/// use dynamodb_facade::Update;
///
/// let update = Update::set("name", "Alice")
///     .and(Update::remove("legacy_field"))
///     .and(Update::increment("login_count", 1));
/// let rendered = format!("{update}");
/// assert_eq!(
///     format!("{update}"),
///     r#"SET name = S("Alice"), login_count = login_count + N("1") REMOVE legacy_field"#,
/// );
/// ```
#[derive(Debug, Clone)]
#[must_use = "expression does nothing until applied to a request"]
pub struct Update<'a>(UpdateInner<'a>);

// -- Constructor methods ------------------------------------------------------

impl<'a> Update<'a> {
    // SET

    /// Creates a SET action with a custom right-hand-side expression.
    ///
    /// Use this when [`set`](Update::set) is insufficient — for example, to
    /// reference another attribute, use `if_not_exists`, or build complex
    /// arithmetic expressions. See [`UpdateSetRhs`] for the available RHS forms.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::{Update, UpdateSetRhs};
    ///
    /// // SET old_score = if_not_exists(score, 0)
    /// let update = Update::set_custom(
    ///     "old_score",
    ///     UpdateSetRhs::if_not_exists("score", 0),
    /// );
    /// assert_eq!(
    ///     format!("{update}"),
    ///     r#"SET old_score = if_not_exists(score, N("0"))"#
    /// );
    /// ```
    pub fn set_custom(attr: impl Into<Cow<'a, str>>, rhs: UpdateSetRhs<'a>) -> Self {
        Self(UpdateInner::SetCustom {
            attr: attr.into(),
            rhs,
        })
    }

    /// Creates a SET action that assigns a literal value: `SET attr = value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// let update = Update::set("role", "instructor");
    /// assert_eq!(format!("{update}"), r#"SET role = S("instructor")"#);
    /// ```
    pub fn set(attr: impl Into<Cow<'a, str>>, value: impl IntoAttributeValue) -> Self {
        Self(UpdateInner::Set {
            attr: attr.into(),
            value: value.into_attribute_value(),
        })
    }

    /// Creates a SET action that only writes if the attribute does not already exist.
    ///
    /// Generates `SET attr = if_not_exists(attr, value)`. This is an atomic
    /// "initialize if absent" operation — it will not overwrite an existing value.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// let update = Update::set_if_not_exists("created_at", "2024-01-01");
    /// assert_eq!(
    ///     format!("{update}"),
    ///     r#"SET created_at = if_not_exists(created_at, S("2024-01-01"))"#
    /// );
    /// ```
    pub fn set_if_not_exists(
        attr: impl Into<Cow<'a, str>>,
        value: impl IntoAttributeValue,
    ) -> Self {
        Self(UpdateInner::SetIfNotExists {
            attr: attr.into(),
            value: value.into_attribute_value(),
        })
    }

    // Arithmetic

    /// Creates a SET action that increments a numeric attribute: `SET attr = attr + by`.
    ///
    /// The attribute must already exist and be a number. To safely initialize
    /// and increment in one operation, use [`init_increment`](Update::init_increment).
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// let update = Update::increment("login_count", 1);
    /// assert_eq!(
    ///     format!("{update}"),
    ///     r#"SET login_count = login_count + N("1")"#
    /// );
    /// ```
    pub fn increment(attr: impl Into<Cow<'a, str>>, by: impl IntoAttributeValue) -> Self {
        Self(UpdateInner::Increment {
            attr: attr.into(),
            by: by.into_attribute_value(),
        })
    }

    /// Creates a SET action that decrements a numeric attribute: `SET attr = attr - by`.
    ///
    /// The attribute must already exist and be a number. To safely initialize
    /// and decrement in one operation, use [`init_decrement`](Update::init_decrement).
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// let update = Update::decrement("credits", 10);
    /// assert_eq!(
    ///     format!("{update}"),
    ///     r#"SET credits = credits - N("10")"#
    /// );
    /// ```
    pub fn decrement(attr: impl Into<Cow<'a, str>>, by: impl IntoAttributeValue) -> Self {
        Self(UpdateInner::Decrement {
            attr: attr.into(),
            by: by.into_attribute_value(),
        })
    }

    /// Creates a SET action that initializes and increments atomically.
    ///
    /// Generates `SET attr = if_not_exists(attr, initial) + by`. If the
    /// attribute does not exist, it is initialized to `initial` before the
    /// increment is applied. This is safe to call even on a new item.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// // SET enrollment_count = if_not_exists(enrollment_count, 0) + 1
    /// let update = Update::init_increment("enrollment_count", 0, 1);
    /// assert_eq!(
    ///     format!("{update}"),
    ///     r#"SET enrollment_count = if_not_exists(enrollment_count, N("0")) + N("1")"#
    /// );
    /// ```
    pub fn init_increment(
        attr: impl Into<Cow<'a, str>>,
        initial: impl IntoAttributeValue,
        by: impl IntoAttributeValue,
    ) -> Self {
        Self(UpdateInner::InitIncrement {
            attr: attr.into(),
            initial: initial.into_attribute_value(),
            by: by.into_attribute_value(),
        })
    }

    /// Creates a SET action that initializes and decrements atomically.
    ///
    /// Generates `SET attr = if_not_exists(attr, initial) - by`. If the
    /// attribute does not exist, it is initialized to `initial` before the
    /// decrement is applied. This is safe to call even on a new item.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// // SET balance = if_not_exists(balance, 1000) - 50
    /// let update = Update::init_decrement("balance", 1000, 50);
    /// assert_eq!(
    ///     format!("{update}"),
    ///     r#"SET balance = if_not_exists(balance, N("1000")) - N("50")"#
    /// );
    /// ```
    pub fn init_decrement(
        attr: impl Into<Cow<'a, str>>,
        initial: impl IntoAttributeValue,
        by: impl IntoAttributeValue,
    ) -> Self {
        Self(UpdateInner::InitDecrement {
            attr: attr.into(),
            initial: initial.into_attribute_value(),
            by: by.into_attribute_value(),
        })
    }

    // Lists

    /// Creates a SET action that appends elements to a list attribute.
    ///
    /// Generates `SET attr = list_append(attr, value)`. The `value` must be a
    /// DynamoDB List (`L`) attribute value.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// let update = Update::list_append("tags", vec!["rust"]);
    /// assert_eq!(
    ///     format!("{update}"),
    ///     r#"SET tags = list_append(tags, L([S("rust")]))"#
    /// );
    /// ```
    pub fn list_append(attr: impl Into<Cow<'a, str>>, value: impl IntoAttributeValue) -> Self {
        Self(UpdateInner::ListAppend {
            attr: attr.into(),
            value: value.into_attribute_value(),
        })
    }

    /// Creates a SET action that prepends elements to a list attribute.
    ///
    /// Generates `SET attr = list_append(value, attr)`. The `value` must be a
    /// DynamoDB List (`L`) attribute value.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// let update = Update::list_prepend("notifications", vec!["new_event"]);
    /// assert_eq!(
    ///     format!("{update}"),
    ///     r#"SET notifications = list_append(L([S("new_event")]), notifications)"#
    /// );
    /// ```
    pub fn list_prepend(attr: impl Into<Cow<'a, str>>, value: impl IntoAttributeValue) -> Self {
        Self(UpdateInner::ListPrepend {
            attr: attr.into(),
            value: value.into_attribute_value(),
        })
    }

    // REMOVE

    /// Creates a REMOVE action that deletes an attribute from the item.
    ///
    /// Generates `REMOVE attr`. The attribute path may include a list index
    /// (e.g. `"tags[2]"`) to remove a specific list element.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// let update = Update::remove("legacy_field");
    /// assert_eq!(format!("{update}"), "REMOVE legacy_field");
    ///
    /// let update = Update::remove("tags[2]");
    /// assert_eq!(format!("{update}"), "REMOVE tags[2]");
    /// ```
    pub fn remove(attr: impl Into<Cow<'a, str>>) -> Self {
        Self(UpdateInner::Remove { attr: attr.into() })
    }

    /// Creates a REMOVE action that deletes a specific element from a list attribute.
    ///
    /// Generates `REMOVE attr[index]`. The `index` is zero-based.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// let update = Update::list_remove("tags", 2);
    /// assert_eq!(format!("{update}"), "REMOVE tags[2]");
    /// ```
    pub fn list_remove(attr: impl Into<Cow<'a, str>>, index: usize) -> Self {
        Self(UpdateInner::ListRemove {
            attr: attr.into(),
            index,
        })
    }

    // ADD

    /// Creates an ADD action for numeric attributes or set attributes.
    ///
    /// For numeric attributes, generates `ADD attr value` which atomically
    /// adds `value` to the current attribute value (initializing to zero if
    /// absent). For DynamoDB Set types (`SS`, `NS`, `BS`), adds elements to
    /// the set.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// let update = Update::add("visitor_count", 5);
    /// assert_eq!(format!("{update}"), r#"ADD visitor_count N("5")"#);
    /// ```
    pub fn add(attr: impl Into<Cow<'a, str>>, value: impl IntoAttributeValue) -> Self {
        Self(UpdateInner::Add {
            attr: attr.into(),
            value: value.into_attribute_value(),
        })
    }

    // DELETE

    /// Creates a DELETE action that removes elements from a DynamoDB Set attribute.
    ///
    /// Generates `DELETE attr set`. The `value` must be a DynamoDB Set type
    /// (`SS`, `NS`, or `BS`). Elements present in `value` are removed from the
    /// set attribute.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::{AsSet, Update};
    ///
    /// // Remove "rust" from the tag_set (SS attribute).
    /// let update = Update::delete("tag_set", AsSet(vec!["rust"]));
    /// assert_eq!(format!("{update}"), r#"DELETE tag_set Ss(["rust"])"#);
    /// ```
    pub fn delete(attr: impl Into<Cow<'a, str>>, value: impl IntoAttributeValue) -> Self {
        Self(UpdateInner::Delete {
            attr: attr.into(),
            set: value.into_attribute_value(),
        })
    }

    // Combinators

    /// Chains another update action onto this one.
    ///
    /// The resulting expression merges all SET, REMOVE, ADD, and DELETE actions
    /// from both updates into a single `UpdateExpression`. Existing `Combine`
    /// wrappers are flattened.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// let update = Update::set("name", "Alice")
    ///     .and(Update::set("role", "instructor"))
    ///     .and(Update::remove("legacy_field"));
    ///
    /// assert_eq!(
    ///     format!("{update}"),
    ///     r#"SET name = S("Alice"), role = S("instructor") REMOVE legacy_field"#,
    /// );
    /// ```
    pub fn and(self, other: Update<'a>) -> Self {
        Self(UpdateInner::Combine(match self.0 {
            UpdateInner::Combine(mut updates) => {
                updates.push(other);
                updates
            }
            _ => vec![self, other],
        }))
    }

    /// Combines an iterator of updates into a single update expression.
    ///
    /// All SET, REMOVE, ADD, and DELETE actions from the iterator are merged
    /// into one `UpdateExpression`.
    ///
    /// # Panics
    ///
    /// Panics if the iterator is empty. Use [`try_combine`](Update::try_combine)
    /// for a non-panicking alternative.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// let update = Update::combine([
    ///     Update::set("name", "Alice"),
    ///     Update::set("role", "instructor"),
    ///     Update::remove("legacy_field"),
    /// ]);
    ///
    /// assert_eq!(
    ///     format!("{update}"),
    ///     r#"SET name = S("Alice"), role = S("instructor") REMOVE legacy_field"#,
    /// );
    /// ```
    ///
    /// Combining optional updates from an iterator:
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// let new_name: Option<&str> = Some("Alice");
    /// let new_email: Option<&str> = None;
    ///
    /// let update = Update::combine(
    ///     [
    ///         new_name.map(|n| Update::set("name", n)),
    ///         new_email.map(|e| Update::set("email", e)),
    ///     ]
    ///     .into_iter()
    ///     .flatten(),
    /// );
    /// assert_eq!(format!("{update}"), r#"SET name = S("Alice")"#);
    /// ```
    pub fn combine(updates: impl IntoIterator<Item = Update<'a>>) -> Self {
        let updates: Vec<_> = updates.into_iter().collect();
        assert!(
            !updates.is_empty(),
            "Update::combine requires at least one update"
        );
        Self(UpdateInner::Combine(updates))
    }

    /// Combines an iterator of updates into a single update expression, returning `None` if empty.
    ///
    /// This is the non-panicking version of [`combine`](Update::combine). Returns
    /// `None` when the iterator yields no items, which is useful when all updates
    /// are conditional and none may apply.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::Update;
    ///
    /// let new_name: Option<&str> = None;
    /// let new_address: Option<&str> = None;
    ///
    /// // No updates — returns None.
    /// let update = Update::try_combine(
    ///     [
    ///         new_name.map(|n| Update::set("name", n)),
    ///         new_address.map(|a| Update::set("address", a))
    ///     ]
    ///     .into_iter()
    ///     .flatten(),
    /// );
    /// assert!(update.is_none());
    ///
    /// // At least one update — returns Some.
    /// let update = Update::try_combine([Update::set("role", "admin")]);
    /// assert!(update.is_some());
    /// ```
    pub fn try_combine(updates: impl IntoIterator<Item = Update<'a>>) -> Option<Self> {
        let updates: Vec<_> = updates.into_iter().collect();
        if updates.is_empty() {
            None
        } else {
            Some(Self(UpdateInner::Combine(updates)))
        }
    }
}

// -- Internal build machinery -------------------------------------------------

/// Compiled output of an [`Update`], with actions grouped by clause (SET, REMOVE, ADD, DELETE).
#[derive(Debug, Default)]
struct BuiltUpdate {
    set_actions: Vec<String>,
    remove_actions: Vec<String>,
    add_actions: Vec<String>,
    delete_actions: Vec<String>,
    names: AttrNames,
    values: AttrValues,
}

impl BuiltUpdate {
    /// Merges another `BuiltUpdate` into `self`, extending all action lists and placeholder maps.
    fn merge(&mut self, other: BuiltUpdate) {
        self.set_actions.extend(other.set_actions);
        self.remove_actions.extend(other.remove_actions);
        self.add_actions.extend(other.add_actions);
        self.delete_actions.extend(other.delete_actions);
        self.names.extend(other.names);
        self.values.extend(other.values);
    }

    /// Assembles the final update expression string from the grouped action lists.
    fn into_expression(self) -> (String, AttrNames, AttrValues) {
        use core::fmt::Write;
        let mut expression = String::new();

        if !self.set_actions.is_empty() {
            let _ = write!(expression, "SET {}", self.set_actions.join(", "));
        }
        if !self.remove_actions.is_empty() {
            if !expression.is_empty() {
                let _ = write!(expression, " ");
            }
            let _ = write!(expression, "REMOVE {}", self.remove_actions.join(", "));
        }
        if !self.add_actions.is_empty() {
            if !expression.is_empty() {
                let _ = write!(expression, " ");
            }
            let _ = write!(expression, "ADD {}", self.add_actions.join(", "));
        }
        if !self.delete_actions.is_empty() {
            if !expression.is_empty() {
                let _ = write!(expression, " ");
            }
            let _ = write!(expression, "DELETE {}", self.delete_actions.join(", "));
        }

        (expression, self.names, self.values)
    }
}

impl Update<'_> {
    /// Builds the update into grouped action lists using a shared placeholder counter.
    fn build(self, counter: &mut usize) -> BuiltUpdate {
        match self.0 {
            UpdateInner::Combine(updates) => {
                let mut result = BuiltUpdate::default();
                for update in updates {
                    result.merge(update.build(counter));
                }
                result
            }

            UpdateInner::SetCustom { attr, rhs } => {
                let (attr_expr, mut names) = resolve_attr_path(&attr, "u", counter);
                let (rhs_expr, rhs_names, rhs_values) = rhs.rhs_expr(counter);
                names.extend(rhs_names);
                BuiltUpdate {
                    set_actions: vec![format!("{attr_expr} = {rhs_expr}")],
                    names,
                    values: rhs_values,
                    ..Default::default()
                }
            }

            UpdateInner::Set { attr, value } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "u", counter);
                let val_id = *counter;
                *counter += 1;
                let value_ph = format!(":u{val_id}");
                BuiltUpdate {
                    set_actions: vec![format!("{attr_expr} = {value_ph}")],
                    names,
                    values: vec![(value_ph, value)],
                    ..Default::default()
                }
            }

            UpdateInner::SetIfNotExists { attr, value } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "u", counter);
                let val_id = *counter;
                *counter += 1;
                let value_ph = format!(":u{val_id}");
                BuiltUpdate {
                    set_actions: vec![format!(
                        "{attr_expr} = if_not_exists({attr_expr}, {value_ph})"
                    )],
                    names,
                    values: vec![(value_ph, value)],
                    ..Default::default()
                }
            }

            UpdateInner::Increment { attr, by } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "u", counter);
                let val_id = *counter;
                *counter += 1;
                let by_ph = format!(":u{val_id}");
                BuiltUpdate {
                    set_actions: vec![format!("{attr_expr} = {attr_expr} + {by_ph}")],
                    names,
                    values: vec![(by_ph, by)],
                    ..Default::default()
                }
            }

            UpdateInner::Decrement { attr, by } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "u", counter);
                let val_id = *counter;
                *counter += 1;
                let by_ph = format!(":u{val_id}");
                BuiltUpdate {
                    set_actions: vec![format!("{attr_expr} = {attr_expr} - {by_ph}")],
                    names,
                    values: vec![(by_ph, by)],
                    ..Default::default()
                }
            }

            UpdateInner::InitIncrement { attr, initial, by } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "u", counter);
                let val_id = *counter;
                *counter += 1;
                let by_ph = format!(":u{val_id}");
                let init_ph = format!(":u{val_id}init");
                BuiltUpdate {
                    set_actions: vec![format!(
                        "{attr_expr} = if_not_exists({attr_expr}, {init_ph}) + {by_ph}"
                    )],
                    names,
                    values: vec![(init_ph, initial), (by_ph, by)],
                    ..Default::default()
                }
            }

            UpdateInner::InitDecrement { attr, initial, by } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "u", counter);
                let val_id = *counter;
                *counter += 1;
                let by_ph = format!(":u{val_id}");
                let init_ph = format!(":u{val_id}init");
                BuiltUpdate {
                    set_actions: vec![format!(
                        "{attr_expr} = if_not_exists({attr_expr}, {init_ph}) - {by_ph}"
                    )],
                    names,
                    values: vec![(init_ph, initial), (by_ph, by)],
                    ..Default::default()
                }
            }

            UpdateInner::ListAppend { attr, value } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "u", counter);
                let val_id = *counter;
                *counter += 1;
                let val_ph = format!(":u{val_id}");
                BuiltUpdate {
                    set_actions: vec![format!("{attr_expr} = list_append({attr_expr}, {val_ph})")],
                    names,
                    values: vec![(val_ph, value)],
                    ..Default::default()
                }
            }

            UpdateInner::ListPrepend { attr, value } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "u", counter);
                let val_id = *counter;
                *counter += 1;
                let val_ph = format!(":u{val_id}");
                BuiltUpdate {
                    set_actions: vec![format!("{attr_expr} = list_append({val_ph}, {attr_expr})")],
                    names,
                    values: vec![(val_ph, value)],
                    ..Default::default()
                }
            }

            UpdateInner::Remove { attr } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "u", counter);
                BuiltUpdate {
                    remove_actions: vec![attr_expr.into_owned()],
                    names,
                    ..Default::default()
                }
            }

            UpdateInner::ListRemove { attr, index } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "u", counter);
                BuiltUpdate {
                    remove_actions: vec![format!("{attr_expr}[{index}]")],
                    names,
                    ..Default::default()
                }
            }

            UpdateInner::Add { attr, value } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "u", counter);
                let val_id = *counter;
                *counter += 1;
                let val_ph = format!(":u{val_id}");
                BuiltUpdate {
                    add_actions: vec![format!("{attr_expr} {val_ph}")],
                    names,
                    values: vec![(val_ph, value)],
                    ..Default::default()
                }
            }

            UpdateInner::Delete { attr, set } => {
                let (attr_expr, names) = resolve_attr_path(&attr, "u", counter);
                let val_id = *counter;
                *counter += 1;
                let set_ph = format!(":u{val_id}");
                BuiltUpdate {
                    delete_actions: vec![format!("{attr_expr} {set_ph}")],
                    names,
                    values: vec![(set_ph, set)],
                    ..Default::default()
                }
            }
        }
    }
}

// -- Display ------------------------------------------------------------------

impl fmt::Display for Update<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut counter = 0;
        let (expression, names, values) = self.clone().build(&mut counter).into_expression();
        if f.alternate() {
            f.write_str(&expression)?;
            fmt_attr_maps(f, &names, &values)
        } else {
            f.write_str(&resolve_expression(&expression, &names, &values))
        }
    }
}

// -- ApplyUpdate impl ---------------------------------------------------------

impl<B: UpdatableBuilder> ApplyUpdate<B> for Update<'_> {
    fn apply(self, builder: B) -> B {
        let mut counter = 0;
        let (expression, names, values) = self.build(&mut counter).into_expression();
        builder
            .update_expression(expression)
            .apply_names_and_values(names, values)
    }
}

impl<B: UpdatableBuilder> ApplyUpdate<B> for Option<Update<'_>> {
    fn apply(self, builder: B) -> B {
        match self {
            Some(u) => u.apply(builder),
            None => builder,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::values::AsSet;
    use super::*;

    #[test]
    fn test_update_display_default_set() {
        let u = Update::set("balance", 100u32);
        let display = format!("{u}");
        assert_eq!(display, r#"SET balance = N("100")"#);
    }

    #[test]
    fn test_update_display_default_set_reserved_word() {
        let u = Update::set("Status", "active");
        let display = format!("{u}");
        assert_eq!(display, r#"SET Status = S("active")"#);
    }

    #[test]
    fn test_update_display_default_increment() {
        let u = Update::increment("login_count", 1u32);
        let display = format!("{u}");
        assert_eq!(display, r#"SET login_count = login_count + N("1")"#);
    }

    #[test]
    fn test_update_display_default_remove() {
        let u = Update::remove("legacy_field");
        let display = format!("{u}");
        assert_eq!(display, "REMOVE legacy_field");
    }

    #[test]
    fn test_update_display_default_combined() {
        let u = Update::combine([
            Update::set("balance", 100u32),
            Update::increment("login_count", 1u32),
            Update::remove("legacy_field"),
        ]);
        let display = format!("{u}");
        assert_eq!(
            display,
            r#"SET balance = N("100"), login_count = login_count + N("1") REMOVE legacy_field"#
        );
    }

    #[test]
    fn test_update_display_default_add_action() {
        let u = Update::add("visitor_count", 5u32);
        let display = format!("{u}");
        assert_eq!(display, r#"ADD visitor_count N("5")"#);
    }

    #[test]
    fn test_update_display_default_set_if_not_exists() {
        let u = Update::set_if_not_exists("created_at", "2024-01-01");
        let display = format!("{u}");
        assert_eq!(
            display,
            r#"SET created_at = if_not_exists(created_at, S("2024-01-01"))"#
        );
    }

    #[test]
    fn test_update_display_default_init_increment() {
        let u = Update::init_increment("counter", 0u32, 1u32);
        let display = format!("{u}");
        assert_eq!(
            display,
            r#"SET counter = if_not_exists(counter, N("0")) + N("1")"#
        );
    }

    #[test]
    fn test_update_display_alternate_set() {
        let u = Update::set("balance", 100u32);
        let display = format!("{u:#}");
        assert_eq!(display, "SET balance = :u0\n  values: { :u0 = N(\"100\") }");
    }

    #[test]
    fn test_update_display_alternate_reserved_word() {
        let u = Update::set("Status", "active");
        let display = format!("{u:#}");
        assert_eq!(
            display,
            "SET #u0 = :u1\n  names: { #u0 = Status }\n  values: { :u1 = S(\"active\") }"
        );
    }

    #[test]
    fn test_update_display_alternate_remove() {
        let u = Update::remove("legacy_field");
        let display = format!("{u:#}");
        // No names (not reserved) and no values
        assert_eq!(display, "REMOVE legacy_field");
    }

    #[test]
    fn test_update_display_alternate_combined() {
        let u = Update::combine([Update::set("balance", 100u32), Update::remove("Name")]);
        let display = format!("{u:#}");
        assert_eq!(
            display,
            "SET balance = :u0 REMOVE #u1\n  names: { #u1 = Name }\n  values: { :u0 = N(\"100\") }"
        );
    }

    // -- Decrement ------------------------------------------------------------

    #[test]
    fn test_decrement_display_default() {
        // Non-reserved attribute: no name placeholder
        let u = Update::decrement("credits", 1u32);
        assert_eq!(format!("{u}"), r#"SET credits = credits - N("1")"#);
    }

    #[test]
    fn test_decrement_display_alternate() {
        // Non-reserved attribute: only a value placeholder
        let u = Update::decrement("credits", 1u32);
        assert_eq!(
            format!("{u:#}"),
            "SET credits = credits - :u0\n  values: { :u0 = N(\"1\") }"
        );
    }

    #[test]
    fn test_decrement_display_default_reserved_word() {
        // Reserved word "Count" must be aliased with a name placeholder
        let u = Update::decrement("Count", 1u32);
        assert_eq!(format!("{u}"), r#"SET Count = Count - N("1")"#);
    }

    #[test]
    fn test_decrement_display_alternate_reserved_word() {
        let u = Update::decrement("Count", 1u32);
        assert_eq!(
            format!("{u:#}"),
            "SET #u0 = #u0 - :u1\n  names: { #u0 = Count }\n  values: { :u1 = N(\"1\") }"
        );
    }

    // -- InitDecrement --------------------------------------------------------

    #[test]
    fn test_init_decrement_display_default() {
        let u = Update::init_decrement("credits", 100u32, 1u32);
        assert_eq!(
            format!("{u}"),
            r#"SET credits = if_not_exists(credits, N("100")) - N("1")"#
        );
    }

    #[test]
    fn test_init_decrement_display_alternate() {
        let u = Update::init_decrement("credits", 100u32, 1u32);
        assert_eq!(
            format!("{u:#}"),
            "SET credits = if_not_exists(credits, :u0init) - :u0\n  values: { :u0init = N(\"100\"), :u0 = N(\"1\") }"
        );
    }

    // -- ListAppend -----------------------------------------------------------

    #[test]
    fn test_list_append_display_default() {
        let u = Update::list_append("tags", vec!["a", "b"]);
        assert_eq!(
            format!("{u}"),
            r#"SET tags = list_append(tags, L([S("a"), S("b")]))"#
        );
    }

    #[test]
    fn test_list_append_display_alternate() {
        let u = Update::list_append("tags", vec!["a", "b"]);
        assert_eq!(
            format!("{u:#}"),
            "SET tags = list_append(tags, :u0)\n  values: { :u0 = L([S(\"a\"), S(\"b\")]) }"
        );
    }

    // -- ListPrepend ----------------------------------------------------------

    #[test]
    fn test_list_prepend_display_default() {
        // list_prepend reverses argument order: list_append(:u0, attr)
        let u = Update::list_prepend("tags", vec!["a", "b"]);
        assert_eq!(
            format!("{u}"),
            r#"SET tags = list_append(L([S("a"), S("b")]), tags)"#
        );
    }

    #[test]
    fn test_list_prepend_display_alternate() {
        let u = Update::list_prepend("tags", vec!["a", "b"]);
        assert_eq!(
            format!("{u:#}"),
            "SET tags = list_append(:u0, tags)\n  values: { :u0 = L([S(\"a\"), S(\"b\")]) }"
        );
    }

    // -- ListRemove -----------------------------------------------------------

    #[test]
    fn test_list_remove_display_default() {
        let u = Update::list_remove("tags", 0);
        assert_eq!(format!("{u}"), "REMOVE tags[0]");
    }

    #[test]
    fn test_list_remove_display_alternate() {
        // No names or values — alternate mode is identical to default
        let u = Update::list_remove("tags", 0);
        assert_eq!(format!("{u:#}"), "REMOVE tags[0]");
    }

    // -- Delete ---------------------------------------------------------------

    #[test]
    fn test_delete_display_default() {
        let u = Update::delete("tag_set", AsSet(vec!["old".to_owned()]));
        assert_eq!(format!("{u}"), r#"DELETE tag_set Ss(["old"])"#);
    }

    #[test]
    fn test_delete_display_alternate() {
        let u = Update::delete("tag_set", AsSet(vec!["old".to_owned()]));
        assert_eq!(
            format!("{u:#}"),
            "DELETE tag_set :u0\n  values: { :u0 = Ss([\"old\"]) }"
        );
    }

    // -- SetCustom / UpdateSetRhs variants ------------------------------------

    #[test]
    fn test_set_custom_value_display_default() {
        let u = Update::set_custom("score", UpdateSetRhs::value("Alice"));
        assert_eq!(format!("{u}"), r#"SET score = S("Alice")"#);
    }

    #[test]
    fn test_set_custom_value_display_alternate() {
        let u = Update::set_custom("score", UpdateSetRhs::value("Alice"));
        assert_eq!(
            format!("{u:#}"),
            "SET score = :u0\n  values: { :u0 = S(\"Alice\") }"
        );
    }

    #[test]
    fn test_set_custom_attr_display_default() {
        // RHS is another attribute reference — no value placeholder, name placeholder for "name"
        let u = Update::set_custom("display_name", UpdateSetRhs::attr("name"));
        assert_eq!(format!("{u}"), "SET display_name = name");
    }

    #[test]
    fn test_set_custom_attr_display_alternate() {
        // "name" is a reserved word → name placeholder in RHS
        let u = Update::set_custom("display_name", UpdateSetRhs::attr("name"));
        assert_eq!(
            format!("{u:#}"),
            "SET display_name = #u0\n  names: { #u0 = name }"
        );
    }

    #[test]
    fn test_set_custom_if_not_exists_display_default() {
        let u = Update::set_custom("score", UpdateSetRhs::if_not_exists("score", 0u32));
        assert_eq!(
            format!("{u}"),
            r#"SET score = if_not_exists(score, N("0"))"#
        );
    }

    #[test]
    fn test_set_custom_if_not_exists_display_alternate() {
        let u = Update::set_custom("score", UpdateSetRhs::if_not_exists("score", 0u32));
        assert_eq!(
            format!("{u:#}"),
            "SET score = if_not_exists(score, :u0)\n  values: { :u0 = N(\"0\") }"
        );
    }

    #[test]
    fn test_set_custom_composite_rhs_display_default() {
        // (attr("a") + value(5u32)) - attr("b") → "total = a + N("5") - b"
        let rhs = UpdateSetRhs::attr("a") + UpdateSetRhs::value(5u32) - UpdateSetRhs::attr("b");
        let u = Update::set_custom("total", rhs);
        assert_eq!(format!("{u}"), r#"SET total = a + N("5") - b"#);
    }

    // -- combine / try_combine / and ------------------------------------------

    #[test]
    #[should_panic(expected = "Update::combine requires at least one update")]
    fn test_update_combine_empty_panics() {
        let _ = Update::combine(std::iter::empty::<Update>());
    }

    #[test]
    fn test_update_try_combine_empty_none() {
        let result = Update::try_combine(std::iter::empty::<Update>());
        assert!(result.is_none());
    }

    #[test]
    fn test_update_try_combine_non_empty_some() {
        let result = Update::try_combine([Update::set("score", 1u32)]);
        assert!(result.is_some());
        assert_eq!(format!("{}", result.unwrap()), r#"SET score = N("1")"#);
    }

    #[test]
    fn test_update_and_flattens_nested_combine() {
        // Chaining .and() should flatten into a single Combine, not nest them
        let u = Update::set("score", 1u32)
            .and(Update::set("balance", 2u32))
            .and(Update::remove("legacy_field"));
        assert_eq!(
            format!("{u}"),
            r#"SET score = N("1"), balance = N("2") REMOVE legacy_field"#
        );
    }
}
