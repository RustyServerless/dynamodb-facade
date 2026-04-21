use aws_sdk_dynamodb::{
    operation::{
        delete_item::builders::DeleteItemFluentBuilder, get_item::builders::GetItemFluentBuilder,
        put_item::builders::PutItemFluentBuilder, query::builders::QueryFluentBuilder,
        scan::builders::ScanFluentBuilder, update_item::builders::UpdateItemFluentBuilder,
    },
    types::builders::{ConditionCheckBuilder, DeleteBuilder, PutBuilder, UpdateBuilder},
};

use super::{super::IntoAttributeValue, AttrNames, AttrValues};

// ---------------------------------------------------------------------------
// ExpressionAttrNames / ExpressionAttrBuilder — shared attribute plumbing
// ---------------------------------------------------------------------------

mod sealed_traits {
    /// Seals [`ExpressionAttrNames`](super::ExpressionAttrNames) so it cannot be implemented outside this module.
    pub trait ExprAttrNamesSeal {}
}

/// Sealed trait for SDK builders that support `expression_attribute_names`.
pub(crate) trait ExpressionAttrNames: sealed_traits::ExprAttrNamesSeal {
    fn expression_attribute_names(self, k: impl Into<String>, v: impl Into<String>) -> Self;
}

/// Extends [`ExpressionAttrNames`] with `expression_attribute_values` support.
///
/// Implemented for all SDK builders except `GetItemFluentBuilder`, which lacks value substitution.
pub(crate) trait ExpressionAttrBuilder: ExpressionAttrNames {
    fn expression_attribute_values(self, k: impl Into<String>, v: impl IntoAttributeValue) -> Self;
}

// -- Macro: names only (GetItemFluentBuilder) --------------------------------

macro_rules! impl_expr_attr_names {
    ($b:ty) => {
        impl sealed_traits::ExprAttrNamesSeal for $b {}
        impl ExpressionAttrNames for $b {
            fn expression_attribute_names(
                self,
                k: impl Into<String>,
                v: impl Into<String>,
            ) -> Self {
                self.expression_attribute_names(k, v)
            }
        }
    };
}

// -- Macro: names + values (all other builders) ------------------------------

macro_rules! impl_expr_attr_builder {
    ($b:ty) => {
        impl_expr_attr_names!($b);
        impl ExpressionAttrBuilder for $b {
            fn expression_attribute_values(
                self,
                k: impl Into<String>,
                v: impl IntoAttributeValue,
            ) -> Self {
                self.expression_attribute_values(k, v.into_attribute_value())
            }
        }
    };
}

// GetItemFluentBuilder — names only (no expression_attribute_values on this SDK type)
impl_expr_attr_names!(GetItemFluentBuilder);

// Fluent builders (online operations)
impl_expr_attr_builder!(UpdateItemFluentBuilder);
impl_expr_attr_builder!(DeleteItemFluentBuilder);
impl_expr_attr_builder!(PutItemFluentBuilder);
impl_expr_attr_builder!(QueryFluentBuilder);
impl_expr_attr_builder!(ScanFluentBuilder);

// Type builders (transactions)
impl_expr_attr_builder!(UpdateBuilder);
impl_expr_attr_builder!(DeleteBuilder);
impl_expr_attr_builder!(PutBuilder);
impl_expr_attr_builder!(ConditionCheckBuilder);

// -- Fold helpers ------------------------------------------------------------

/// Folds a collection of name placeholders onto a builder via [`ExpressionAttrNames`].
pub(crate) trait ApplyExpressionNames: ExpressionAttrNames + Sized {
    fn apply_names(self, names: AttrNames) -> Self {
        names
            .into_iter()
            .fold(self, |b, (k, v)| b.expression_attribute_names(k, v))
    }
}

impl<B: ExpressionAttrNames> ApplyExpressionNames for B {}

/// Folds both name and value placeholders onto a builder via [`ExpressionAttrBuilder`].
pub(crate) trait ApplyExpressionAttributes: ExpressionAttrBuilder + Sized {
    fn apply_names_and_values(self, names: AttrNames, values: AttrValues) -> Self {
        let builder = self.apply_names(names);
        values
            .into_iter()
            .fold(builder, |b, (k, v)| b.expression_attribute_values(k, v))
    }
}

impl<B: ExpressionAttrBuilder> ApplyExpressionAttributes for B {}

// ---------------------------------------------------------------------------
// Specialised expression traits
// ---------------------------------------------------------------------------

// -- ConditionableBuilder: condition_expression (put/delete/update) ----------

/// SDK builders that accept a `condition_expression` (put, delete, update, transact).
pub(crate) trait ConditionableBuilder: ExpressionAttrBuilder {
    fn condition_expression(self, input: impl Into<String>) -> Self;
}

macro_rules! impl_conditionable_builder {
    ($b:ty) => {
        impl ConditionableBuilder for $b {
            fn condition_expression(self, input: impl Into<String>) -> Self {
                self.condition_expression(input)
            }
        }
    };
}

impl_conditionable_builder!(UpdateItemFluentBuilder);
impl_conditionable_builder!(DeleteItemFluentBuilder);
impl_conditionable_builder!(PutItemFluentBuilder);

impl_conditionable_builder!(UpdateBuilder);
impl_conditionable_builder!(DeleteBuilder);
impl_conditionable_builder!(PutBuilder);
impl_conditionable_builder!(ConditionCheckBuilder);

/// Applies a condition expression to a [`ConditionableBuilder`].
pub(crate) trait ApplyCondition<B: ConditionableBuilder> {
    fn apply(self, builder: B) -> B;
}

// -- KeyConditionableBuilder: key_condition_expression (query) ---------------

/// SDK builders that accept a `key_condition_expression` (query only).
pub(crate) trait KeyConditionableBuilder: ExpressionAttrBuilder {
    fn key_condition_expression(self, input: impl Into<String>) -> Self;
}

impl KeyConditionableBuilder for QueryFluentBuilder {
    fn key_condition_expression(self, input: impl Into<String>) -> Self {
        self.key_condition_expression(input)
    }
}

/// Applies a key condition expression to a [`KeyConditionableBuilder`].
pub(crate) trait ApplyKeyCondition<B: KeyConditionableBuilder> {
    fn apply_key_condition(self, builder: B) -> B;
}

// -- FilterableBuilder: filter_expression (query/scan) ----------------------

/// SDK builders that accept a `filter_expression` (query, scan).
pub(crate) trait FilterableBuilder: ExpressionAttrBuilder {
    fn filter_expression(self, input: impl Into<String>) -> Self;
}

macro_rules! impl_filterable_builder {
    ($b:ty) => {
        impl FilterableBuilder for $b {
            fn filter_expression(self, input: impl Into<String>) -> Self {
                self.filter_expression(input)
            }
        }
    };
}

impl_filterable_builder!(QueryFluentBuilder);
impl_filterable_builder!(ScanFluentBuilder);

/// Applies a filter expression to a [`FilterableBuilder`].
pub(crate) trait ApplyFilter<B: FilterableBuilder> {
    fn apply_filter(self, builder: B) -> B;
}

// -- UpdatableBuilder: update_expression (update) ---------------------------

/// SDK builders that accept an `update_expression` (update item only).
pub(crate) trait UpdatableBuilder: ExpressionAttrBuilder {
    fn update_expression(self, input: impl Into<String>) -> Self;
}

macro_rules! impl_updatable_builder {
    ($b:ty) => {
        impl UpdatableBuilder for $b {
            fn update_expression(self, input: impl Into<String>) -> Self {
                self.update_expression(input)
            }
        }
    };
}

impl_updatable_builder!(UpdateItemFluentBuilder);
impl_updatable_builder!(UpdateBuilder);

/// Applies an update expression to an [`UpdatableBuilder`].
pub(crate) trait ApplyUpdate<B: UpdatableBuilder> {
    fn apply(self, builder: B) -> B;
}

// -- ProjectionableBuilder: projection_expression (get/query/scan) -----------

/// SDK builders that accept a `projection_expression` (get, query, scan).
///
/// Extends [`ExpressionAttrNames`] only — projection expressions never use value placeholders.
pub(crate) trait ProjectionableBuilder: ExpressionAttrNames {
    fn projection_expression(self, input: impl Into<String>) -> Self;
}

macro_rules! impl_projectionable_builder {
    ($b:ty) => {
        impl ProjectionableBuilder for $b {
            fn projection_expression(self, input: impl Into<String>) -> Self {
                self.projection_expression(input)
            }
        }
    };
}

impl_projectionable_builder!(GetItemFluentBuilder);
impl_projectionable_builder!(QueryFluentBuilder);
impl_projectionable_builder!(ScanFluentBuilder);

/// Applies a projection expression to a [`ProjectionableBuilder`].
pub(crate) trait ApplyProjection<B: ProjectionableBuilder> {
    fn apply_projection(self, builder: B) -> B;
}
