use super::*;

use aws_sdk_dynamodb::types::{
    TransactWriteItem,
    builders::{ConditionCheckBuilder, DeleteBuilder, PutBuilder, UpdateBuilder},
};

/// Builder for a `Put` operation inside a DynamoDB transaction.
///
/// Constructed via [`DynamoDBItemTransactOp::transact_put`]. Optionally add a
/// condition that must hold for the put to succeed, via
/// [`.condition()`][TransactPutRequest::condition],
/// [`.exists()`][TransactPutRequest::exists], or
/// [`.not_exists()`][TransactPutRequest::not_exists]. DynamoDB accepts a
/// single condition expression per operation, so this can only be called once.
///
/// Call [`.build()`][TransactPutRequest::build] to produce a
/// [`TransactWriteItem`] that can be passed to the SDK's
/// `transact_write_items()` builder.
///
/// # Examples
///
/// Atomically create an enrollment and increment the user's enrollment count:
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{DynamoDBItemTransactOp, KeyId, Update};
///
/// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
/// let enrollment = sample_enrollment();
///
/// client
///     .transact_write_items()
///     .transact_items(enrollment.transact_put().not_exists().build())
///     .transact_items(
///         User::transact_update_by_id(
///             KeyId::pk("user-1"),
///             Update::init_increment("enrollment_count", 0, 1),
///         )
///         .exists()
///         .build(),
///     )
///     .send()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[must_use = "builder does nothing until .build() is called"]
pub struct TransactPutRequest<TD: TableDefinition, T = (), C: ConditionState = NoCondition> {
    builder: PutBuilder,
    _marker: PhantomData<(TD, T, C)>,
}

// -- Common methods (all states) --------------------------------------------

impl<TD: TableDefinition, T, C: ConditionState> TransactPutRequest<TD, T, C> {
    /// Creates a new `TransactPutRequest` from a raw [`Item`].
    ///
    /// Prefer [`DynamoDBItemTransactOp::transact_put`] for typed construction.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{TransactPutRequest, DynamoDBItemTransactOp, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // Create a user from a raw Item and atomically initialize an enrollment
    /// let user_item = sample_user_item();
    /// let enrollment = sample_enrollment();
    /// client
    ///     .transact_write_items()
    ///     .transact_items(
    ///         TransactPutRequest::<PlatformTable>::new(user_item).build(),
    ///     )
    ///     .transact_items(enrollment.transact_put().not_exists().build())
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(item: Item<TD>) -> Self {
        let table_name = TD::table_name();
        tracing::debug!(table_name, "TransactPut");
        Self {
            builder: PutBuilder::default()
                .table_name(table_name)
                .set_item(Some(item.into_inner())),
            _marker: PhantomData,
        }
    }

    /// Consumes the builder and returns the underlying SDK [`PutBuilder`].
    ///
    /// Use this escape hatch when you need to set options not exposed by this
    /// facade.
    pub fn into_inner(self) -> PutBuilder {
        self.builder
    }

    /// Finalizes the builder and returns a [`TransactWriteItem`].
    ///
    /// The returned value can be passed directly to the SDK's
    /// `transact_write_items().transact_items(...)` call.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemTransactOp, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let enrollment = sample_enrollment();
    ///
    /// // Atomically create an enrollment and increment the user's counter
    /// client
    ///     .transact_write_items()
    ///     .transact_items(enrollment.transact_put().not_exists().build())
    ///     .transact_items(
    ///         User::transact_update_by_id(
    ///             KeyId::pk("user-1"),
    ///             Update::init_increment("enrollment_count", 0, 1),
    ///         )
    ///         .exists()
    ///         .build(),
    ///     )
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> TransactWriteItem {
        TransactWriteItem::builder()
            .put(self.builder.build().expect("mandatory attributes set"))
            .build()
    }
}

// -- Condition (NoCondition only) -------------------------------------------

impl<TD: TableDefinition, T> TransactPutRequest<TD, T, NoCondition> {
    /// Adds a condition expression that must be satisfied for the put to succeed.
    ///
    /// DynamoDB accepts a single condition expression per operation, so this
    /// method can only be called once. If the condition fails, the entire
    /// transaction is cancelled with `TransactionCanceledException`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, DynamoDBItemTransactOp, Condition};
    ///
    /// let transact_item = sample_enrollment()
    ///     .transact_put()
    ///     .condition(
    ///         Enrollment::not_exists() |
    ///         Condition::not_exists("completed_at")
    ///     )
    ///     .build();
    /// ```
    pub fn condition(
        mut self,
        condition: Condition<'_>,
    ) -> TransactPutRequest<TD, T, AlreadyHasCondition> {
        tracing::debug!(%condition, "TransactPut condition");
        self.builder = condition.apply(self.builder);
        TransactPutRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

impl<TD: TableDefinition, T: DynamoDBItem<TD>> TransactPutRequest<TD, T, NoCondition> {
    /// Adds an `attribute_exists(<PK>)` condition.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemTransactOp;
    ///
    /// let transact_item = sample_enrollment().transact_put().exists().build();
    /// ```
    pub fn exists(mut self) -> TransactPutRequest<TD, T, AlreadyHasCondition> {
        self.builder = T::exists().apply(self.builder);
        TransactPutRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }

    /// Adds an `attribute_not_exists(<PK>)` condition.
    ///
    /// Use this to implement create-only semantics within a transaction.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemTransactOp;
    ///
    /// let transact_item = sample_enrollment().transact_put().not_exists().build();
    /// ```
    pub fn not_exists(mut self) -> TransactPutRequest<TD, T, AlreadyHasCondition> {
        self.builder = T::not_exists().apply(self.builder);
        TransactPutRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

/// Builder for a `Delete` operation inside a DynamoDB transaction.
///
/// Constructed via [`DynamoDBItemTransactOp::transact_delete`] or
/// [`DynamoDBItemTransactOp::transact_delete_by_id`]. Optionally add a
/// condition that must hold for the delete to succeed, via
/// [`.condition()`][TransactDeleteRequest::condition], or
/// [`.exists()`][TransactDeleteRequest::exists]. DynamoDB accepts a
/// single condition expression per operation, so this can only be called once.
///
/// Call [`.build()`][TransactDeleteRequest::build] to produce a
/// [`TransactWriteItem`] that can be passed to the SDK's
/// `transact_write_items()` builder.
///
/// # Examples
///
/// Atomically remove an enrollment and decrement the user's enrollment count:
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{DynamoDBItemTransactOp, Condition, KeyId, Update};
///
/// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
/// client
///     .transact_write_items()
///     .transact_items(
///         Enrollment::transact_delete_by_id(KeyId::pk("user-1").sk("course-42"))
///             .exists()
///             .build(),
///     )
///     .transact_items(
///         User::transact_update_by_id(
///             KeyId::pk("user-1"),
///             Update::decrement("enrollment_count", 1),
///         )
///         .condition(Condition::exists("enrollment_count"))
///         .build(),
///     )
///     .send()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[must_use = "builder does nothing until .build() is called"]
pub struct TransactDeleteRequest<TD: TableDefinition, T = (), C: ConditionState = NoCondition> {
    builder: DeleteBuilder,
    _marker: PhantomData<(TD, T, C)>,
}

// -- Common methods (all states) --------------------------------------------

impl<TD: TableDefinition, T, C: ConditionState> TransactDeleteRequest<TD, T, C> {
    /// Creates a new `TransactDeleteRequest` from a raw [`Key`].
    ///
    /// Prefer [`DynamoDBItemTransactOp::transact_delete`] or
    /// [`DynamoDBItemTransactOp::transact_delete_by_id`] for typed construction.
    pub fn new(key: Key<TD>) -> Self {
        let table_name = TD::table_name();
        tracing::debug!(table_name, key = ?key, "TransactDelete");
        Self {
            builder: DeleteBuilder::default()
                .table_name(table_name)
                .set_key(Some(key.into_inner())),
            _marker: PhantomData,
        }
    }

    /// Consumes the builder and returns the underlying SDK [`DeleteBuilder`].
    ///
    /// Use this escape hatch when you need to set options not exposed by this
    /// facade.
    pub fn into_inner(self) -> DeleteBuilder {
        self.builder
    }

    /// Finalizes the builder and returns a [`TransactWriteItem`].
    ///
    /// The returned value can be passed directly to the SDK's
    /// `transact_write_items().transact_items(...)` call.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemTransactOp, Condition, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// // Atomically remove an enrollment and decrement the user's counter
    /// client
    ///     .transact_write_items()
    ///     .transact_items(
    ///         Enrollment::transact_delete_by_id(KeyId::pk("user-1").sk("course-42"))
    ///             .exists()
    ///             .build(),
    ///     )
    ///     .transact_items(
    ///         User::transact_update_by_id(
    ///             KeyId::pk("user-1"),
    ///             Update::decrement("enrollment_count", 1),
    ///         )
    ///         .condition(Condition::exists("enrollment_count"))
    ///         .build(),
    ///     )
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> TransactWriteItem {
        TransactWriteItem::builder()
            .delete(self.builder.build().expect("mandatory attributes set"))
            .build()
    }
}

// -- Condition (NoCondition only) -------------------------------------------

impl<TD: TableDefinition, T> TransactDeleteRequest<TD, T, NoCondition> {
    /// Adds a condition expression that must be satisfied for the delete to succeed.
    ///
    /// DynamoDB accepts a single condition expression per operation, so this
    /// method can only be called once. If the condition fails, the entire
    /// transaction is cancelled.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, DynamoDBItemTransactOp, Condition};
    ///
    /// let transact_item = sample_enrollment()
    ///     .transact_delete()
    ///     .condition(Enrollment::exists() & Condition::not_exists("completed_at"))
    ///     .build();
    /// ```
    pub fn condition(
        mut self,
        condition: Condition<'_>,
    ) -> TransactDeleteRequest<TD, T, AlreadyHasCondition> {
        tracing::debug!(%condition, "TransactDelete condition");
        self.builder = condition.apply(self.builder);
        TransactDeleteRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

impl<TD: TableDefinition, T: DynamoDBItem<TD>> TransactDeleteRequest<TD, T, NoCondition> {
    /// Adds an `attribute_exists(<PK>)` condition.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemTransactOp;
    ///
    /// let transact_item = sample_enrollment().transact_delete().exists().build();
    /// ```
    pub fn exists(self) -> TransactDeleteRequest<TD, T, AlreadyHasCondition> {
        self.condition(T::exists())
    }
}

/// Builder for an `Update` operation inside a DynamoDB transaction.
///
/// Constructed via [`DynamoDBItemTransactOp::transact_update`] or
/// [`DynamoDBItemTransactOp::transact_update_by_id`]. Optionally add a
/// condition that must hold for the update to succeed, via
/// [`.condition()`][TransactUpdateRequest::condition],
/// [`.exists()`][TransactUpdateRequest::exists], or
/// [`.not_exists()`][TransactUpdateRequest::not_exists]. DynamoDB accepts a
/// single condition expression per operation, so this can only be called once.
///
/// Call [`.build()`][TransactUpdateRequest::build] to produce a
/// [`TransactWriteItem`] that can be passed to the SDK's
/// `transact_write_items()` builder.
///
/// # Examples
///
/// Atomically promote a user to instructor and create their first enrollment:
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{Condition, DynamoDBItemTransactOp, KeyId, Update};
///
/// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
/// let enrollment = sample_enrollment();
///
/// client
///     .transact_write_items()
///     .transact_items(
///         User::transact_update_by_id(
///             KeyId::pk("user-1"),
///             Update::set("role", "instructor"),
///         )
///         .condition(Condition::not_exists("role"))
///         .build(),
///     )
///     .transact_items(enrollment.transact_put().not_exists().build())
///     .send()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[must_use = "builder does nothing until .build() is called"]
pub struct TransactUpdateRequest<TD: TableDefinition, T = (), C: ConditionState = NoCondition> {
    builder: UpdateBuilder,
    _marker: PhantomData<(TD, T, C)>,
}

// -- Common methods (all states) --------------------------------------------

impl<TD: TableDefinition, T, C: ConditionState> TransactUpdateRequest<TD, T, C> {
    /// Creates a new `TransactUpdateRequest` from a raw [`Key`] and an [`Update`] expression.
    ///
    /// Prefer [`DynamoDBItemTransactOp::transact_update`] or
    /// [`DynamoDBItemTransactOp::transact_update_by_id`] for typed construction.
    pub fn new(key: Key<TD>, update: Update<'_>) -> Self {
        let table_name = TD::table_name();
        tracing::debug!(table_name, key = ?key, %update, "TransactUpdate");
        Self {
            builder: update.apply(
                UpdateBuilder::default()
                    .table_name(table_name)
                    .set_key(Some(key.into_inner())),
            ),
            _marker: PhantomData,
        }
    }

    /// Consumes the builder and returns the underlying SDK [`UpdateBuilder`].
    ///
    /// Use this escape hatch when you need to set options not exposed by this
    /// facade.
    pub fn into_inner(self) -> UpdateBuilder {
        self.builder
    }

    /// Finalizes the builder and returns a [`TransactWriteItem`].
    ///
    /// The returned value can be passed directly to the SDK's
    /// `transact_write_items().transact_items(...)` call.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{Condition, DynamoDBItemTransactOp, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let enrollment = sample_enrollment();
    ///
    /// // Atomically promote a user and create an enrollment
    /// client
    ///     .transact_write_items()
    ///     .transact_items(
    ///         User::transact_update_by_id(
    ///             KeyId::pk("user-1"),
    ///             Update::set("role", "instructor"),
    ///         )
    ///         .condition(Condition::not_exists("role"))
    ///         .build(),
    ///     )
    ///     .transact_items(enrollment.transact_put().not_exists().build())
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> TransactWriteItem {
        TransactWriteItem::builder()
            .update(
                self.builder
                    .build()
                    .expect("Update expression is always set"),
            )
            .build()
    }
}

// -- Condition (NoCondition only) -------------------------------------------

impl<TD: TableDefinition, T> TransactUpdateRequest<TD, T, NoCondition> {
    /// Adds a condition expression that must be satisfied for the update to succeed.
    ///
    /// DynamoDB accepts a single condition expression per operation, so this
    /// method can only be called once. If the condition fails, the entire
    /// transaction is cancelled.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemTransactOp, KeyId, Update, Condition};
    ///
    /// let transact_item = User::transact_update_by_id(
    ///     KeyId::pk("user-1"),
    ///     Update::set("role", "instructor"),
    /// )
    /// .condition(Condition::not_exists("role"))
    /// .build();
    /// ```
    pub fn condition(
        mut self,
        condition: Condition<'_>,
    ) -> TransactUpdateRequest<TD, T, AlreadyHasCondition> {
        tracing::debug!(%condition, "TransactUpdate condition");
        self.builder = condition.apply(self.builder);
        TransactUpdateRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

impl<TD: TableDefinition, T: DynamoDBItem<TD>> TransactUpdateRequest<TD, T, NoCondition> {
    /// Adds an `attribute_exists(<PK>)` condition.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemTransactOp, KeyId, Update};
    ///
    /// let transact_item = User::transact_update_by_id(
    ///     KeyId::pk("user-1"),
    ///     Update::increment("enrollment_count", 1),
    /// )
    /// .exists()
    /// .build();
    /// ```
    pub fn exists(mut self) -> TransactUpdateRequest<TD, T, AlreadyHasCondition> {
        self.builder = T::exists().apply(self.builder);
        TransactUpdateRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }

    /// Adds an `attribute_not_exists(<PK>)` condition.
    ///
    /// Useful for upsert-style updates that must only apply when the item does
    /// not yet exist.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemTransactOp, KeyId, Update};
    ///
    /// // Initialize enrollment progress only if the enrollment doesn't exist yet
    /// let transact_item = Enrollment::transact_update_by_id(
    ///     KeyId::pk("user-1").sk("course-42"),
    ///     Update::set("progress", 0.0),
    /// )
    /// .not_exists()
    /// .build();
    /// ```
    pub fn not_exists(mut self) -> TransactUpdateRequest<TD, T, AlreadyHasCondition> {
        self.builder = T::not_exists().apply(self.builder);
        TransactUpdateRequest {
            builder: self.builder,
            _marker: PhantomData,
        }
    }
}

/// Builder for a `ConditionCheck` operation inside a DynamoDB transaction.
///
/// A condition check does not mutate any item — it only asserts that a
/// condition holds. If the condition fails, the entire transaction is
/// cancelled. Use this to enforce invariants on items that are not otherwise
/// being modified in the same transaction.
///
/// Constructed via [`DynamoDBItemTransactOp::transact_condition`] or
/// [`DynamoDBItemTransactOp::transact_condition_by_id`].
///
/// Call [`.build()`][TransactConditionCheckRequest::build] to produce a
/// [`TransactWriteItem`] that can be passed to the SDK's
/// `transact_write_items()` builder.
///
/// # Examples
///
/// Verify the user is an admin before toggling maintenance mode:
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{DynamoDBItemOp, DynamoDBItemTransactOp, Condition, KeyId, Update};
///
/// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
/// let user = sample_user();
///
/// client
///     .transact_write_items()
///     .transact_items(
///         user.transact_condition(
///             User::exists() & Condition::eq("role", "admin"),
///         )
///         .build(),
///     )
///     .transact_items(
///         PlatformConfig::transact_update_by_id(
///             KeyId::NONE,
///             Update::set("maintenance_mode", true),
///         )
///         .exists()
///         .build(),
///     )
///     .send()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[must_use = "builder does nothing until .build() is called"]
pub struct TransactConditionCheckRequest<TD: TableDefinition, T = ()> {
    builder: ConditionCheckBuilder,
    _marker: PhantomData<(TD, T)>,
}

impl<TD: TableDefinition, T> TransactConditionCheckRequest<TD, T> {
    /// Creates a new `TransactConditionCheckRequest` from a raw [`Key`] and a [`Condition`].
    ///
    /// Prefer [`DynamoDBItemTransactOp::transact_condition`] or
    /// [`DynamoDBItemTransactOp::transact_condition_by_id`] for typed construction.
    pub fn new(key: Key<TD>, condition: Condition<'_>) -> Self {
        let table_name = TD::table_name();
        tracing::debug!(table_name, key = ?key, %condition, "TransactConditionCheck");
        Self {
            builder: condition.apply(
                ConditionCheckBuilder::default()
                    .table_name(table_name)
                    .set_key(Some(key.into_inner())),
            ),
            _marker: PhantomData,
        }
    }

    /// Consumes the builder and returns the underlying SDK [`ConditionCheckBuilder`].
    ///
    /// Use this escape hatch when you need to set options not exposed by this
    /// facade.
    pub fn into_inner(self) -> ConditionCheckBuilder {
        self.builder
    }

    /// Finalizes the builder and returns a [`TransactWriteItem`].
    ///
    /// The returned value can be passed directly to the SDK's
    /// `transact_write_items().transact_items(...)` call.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, DynamoDBItemTransactOp, Condition, KeyId, Update};
    ///
    /// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
    /// let user = sample_user();
    ///
    /// // Verify the user is an admin before toggling maintenance mode
    /// client
    ///     .transact_write_items()
    ///     .transact_items(
    ///         user.transact_condition(
    ///             User::exists() & Condition::eq("role", "admin"),
    ///         )
    ///         .build(),
    ///     )
    ///     .transact_items(
    ///         PlatformConfig::transact_update_by_id(
    ///             KeyId::NONE,
    ///             Update::set("maintenance_mode", true),
    ///         )
    ///         .exists()
    ///         .build(),
    ///     )
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> TransactWriteItem {
        TransactWriteItem::builder()
            .condition_check(self.builder.build().expect("mandatory attributes set"))
            .build()
    }
}

// ---------------------------------------------------------------------------
// DynamoDBItemTransactOp trait
// ---------------------------------------------------------------------------

/// Entry points for building DynamoDB `TransactWriteItems` operations.
///
/// This trait is **blanket-implemented** for every type that implements
/// [`DynamoDBItemOp<TD>`]. You never implement it manually.
///
/// Each method returns a typed builder ([`TransactPutRequest`],
/// [`TransactDeleteRequest`], [`TransactUpdateRequest`], or
/// [`TransactConditionCheckRequest`]) that can be configured with optional
/// conditions and then finalized with `.build()` to produce a
/// [`TransactWriteItem`].
///
/// Collect the `TransactWriteItem` values and pass them to the SDK's
/// `client.transact_write_items().transact_items(...)` builder to execute the
/// transaction atomically.
///
/// # Examples
///
/// ```no_run
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{DynamoDBItemTransactOp, KeyId, Update};
///
/// # async fn example(client: aws_sdk_dynamodb::Client) -> dynamodb_facade::Result<()> {
/// let enrollment = sample_enrollment();
///
/// // Atomically create an enrollment and increment the user's enrollment count
/// client
///     .transact_write_items()
///     .transact_items(enrollment.transact_put().not_exists().build())
///     .transact_items(
///         User::transact_update_by_id(
///             KeyId::pk("user-1"),
///             Update::init_increment("enrollment_count", 0, 1),
///         )
///         .exists()
///         .build(),
///     )
///     .send()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub trait DynamoDBItemTransactOp<TD: TableDefinition>: DynamoDBItemOp<TD> {
    /// Creates a [`TransactPutRequest`] for this item.
    ///
    /// Serializes `self` into a DynamoDB item map. Use `.not_exists()` or
    /// `.condition(cond)` to add a guard before calling `.build()`.
    ///
    /// # Panics
    ///
    /// Panics if serializing `self` via [`DynamoDBItem::to_item`] fails. See
    /// [`DynamoDBItem::to_item`] for the conditions under which this can
    /// happen — it is the caller's responsibility to provide a compatible
    /// [`Serialize`] implementation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemTransactOp;
    ///
    /// let transact_item = sample_enrollment().transact_put().not_exists().build();
    /// ```
    fn transact_put(&self) -> TransactPutRequest<TD, Self>
    where
        Self: Serialize,
    {
        TransactPutRequest::new(self.to_item())
    }

    /// Creates a [`TransactDeleteRequest`] for this item's key.
    ///
    /// Use `.exists()` or `.condition(cond)` to add a guard before calling
    /// `.build()`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::DynamoDBItemTransactOp;
    ///
    /// let transact_item = sample_enrollment().transact_delete().exists().build();
    /// ```
    fn transact_delete(&self) -> TransactDeleteRequest<TD, Self> {
        TransactDeleteRequest::new(self.get_key())
    }

    /// Creates a [`TransactDeleteRequest`] from a key ID, without loading the item.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemTransactOp, KeyId};
    ///
    /// let transact_item = Enrollment::transact_delete_by_id(KeyId::pk("user-1").sk("course-42"))
    ///     .exists()
    ///     .build();
    /// ```
    fn transact_delete_by_id(key_id: Self::KeyId<'_>) -> TransactDeleteRequest<TD, Self> {
        TransactDeleteRequest::new(Self::get_key_from_id(key_id))
    }

    /// Creates a [`TransactUpdateRequest`] using the key of `self` and the given [`Update`].
    ///
    /// Use `.exists()` or `.condition(cond)` to add a guard before calling
    /// `.build()`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemTransactOp, Update};
    ///
    /// let transact_item = sample_user()
    ///     .transact_update(Update::set("role", "instructor"))
    ///     .exists()
    ///     .build();
    /// ```
    fn transact_update(&self, update: Update<'_>) -> TransactUpdateRequest<TD, Self> {
        TransactUpdateRequest::new(self.get_key(), update)
    }

    /// Creates a [`TransactUpdateRequest`] from a key ID and an [`Update`] expression.
    ///
    /// Use `.exists()` or `.condition(cond)` to add a guard before calling
    /// `.build()`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemTransactOp, KeyId, Update};
    ///
    /// let transact_item = User::transact_update_by_id(
    ///     KeyId::pk("user-1"),
    ///     Update::increment("enrollment_count", 1),
    /// )
    /// .exists()
    /// .build();
    /// ```
    fn transact_update_by_id(
        key_id: Self::KeyId<'_>,
        update: Update<'_>,
    ) -> TransactUpdateRequest<TD, Self> {
        TransactUpdateRequest::new(Self::get_key_from_id(key_id), update)
    }

    /// Creates a [`TransactConditionCheckRequest`] using the key of `self`.
    ///
    /// The condition check does not mutate any item — it only asserts that the
    /// given condition holds. If it fails, the entire transaction is cancelled.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, DynamoDBItemTransactOp, Condition};
    ///
    /// let transact_check = sample_user()
    ///     .transact_condition(User::exists() & Condition::eq("role", "admin"))
    ///     .build();
    /// ```
    fn transact_condition(
        &self,
        condition: Condition<'_>,
    ) -> TransactConditionCheckRequest<TD, Self> {
        TransactConditionCheckRequest::new(self.get_key(), condition)
    }

    /// Creates a [`TransactConditionCheckRequest`] from a key ID.
    ///
    /// Use this when you have the key components but not a loaded item
    /// instance.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dynamodb_facade::test_fixtures::*;
    /// use dynamodb_facade::{DynamoDBItemOp, DynamoDBItemTransactOp, KeyId, Condition};
    ///
    /// let transact_check = User::transact_condition_by_id(
    ///         KeyId::pk("user-1"),
    ///         User::exists() & Condition::eq("role", "admin"),
    ///     )
    ///     .build();
    /// ```
    fn transact_condition_by_id(
        key_id: Self::KeyId<'_>,
        condition: Condition<'_>,
    ) -> TransactConditionCheckRequest<TD, Self> {
        TransactConditionCheckRequest::new(Self::get_key_from_id(key_id), condition)
    }
}

impl<TD: TableDefinition, DBI: DynamoDBItemOp<TD>> DynamoDBItemTransactOp<TD> for DBI {}
