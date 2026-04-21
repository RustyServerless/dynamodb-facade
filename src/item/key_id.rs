/// Zero-sized placeholder used when a key component is a compile-time constant.
///
/// `NoId` is the `PkId` or `SkId` type parameter of [`KeyId`] for item types
/// whose partition key or sort key is a constant (e.g. singleton items like
/// the `PlatformConfig` in the crate examples). It carries no data because
/// the key value is baked into the type's
/// [`HasConstAttribute`](crate::HasConstAttribute) impl.
///
/// You will encounter `NoId` as part of [`KeyId::NONE`] for singleton items,
/// and as the `SkId` of [`KeyId::pk`] for items with a variable PK but a
/// constant SK.
///
/// # Examples
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{KeyId, NoId};
///
/// // Singleton item — both PK and SK are constants.
/// let key_id: KeyId<NoId, NoId> = KeyId::NONE;
///
/// // Variable PK, constant SK — SkId is NoId.
/// let key_id: KeyId<&str, NoId> = KeyId::pk("user-1");
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct NoId;

/// Type-safe builder for a DynamoDB partition key + sort key pair.
///
/// `KeyId<PkId, SkId>` holds the logical identifiers used to construct a
/// [`Key<TD>`](crate::Key) for a specific item. The type parameters encode
/// which key components are present:
///
/// | `PkId` | `SkId` | Meaning |
/// |---|---|---|
/// | [`NoId`] | [`NoId`] | Both keys are constants — use [`KeyId::NONE`] |
/// | `T` | [`NoId`] | Variable PK, constant SK — use [`KeyId::pk`] |
/// | `T` | `U` | Both keys are variable — use [`KeyId::pk`]`.`[`sk`](KeyId::sk) |
///
/// The concrete `PkId` and `SkId` types are determined by the item type's
/// `HasAttribute` impls. For example, a `User` whose PK is derived from a
/// `&str` user ID has `KeyId<&str, NoId>`.
///
/// # Examples
///
/// Singleton item (constant PK + SK):
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{DynamoDBItemOp, KeyId};
///
/// # async fn example(client: dynamodb_facade::Client) -> dynamodb_facade::Result<()> {
/// // PlatformConfig has const PK and SK — use KeyId::NONE.
/// let config = PlatformConfig::get(client, KeyId::NONE).await?;
/// # Ok(())
/// # }
/// ```
///
/// Variable PK, constant SK:
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{DynamoDBItemOp, KeyId};
///
/// # async fn example(client: dynamodb_facade::Client) -> dynamodb_facade::Result<()> {
/// // User has a PK and a const SK.
/// let user = User::get(client, KeyId::pk("user-1")).await?;
/// # Ok(())
/// # }
/// ```
///
/// Variable PK + variable SK:
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{DynamoDBItemOp, KeyId};
///
/// # async fn example(client: dynamodb_facade::Client) -> dynamodb_facade::Result<()> {
/// // Enrollment has a PK and a SK.
/// let user_enrollment = Enrollment::get(client, KeyId::pk("user-1").sk("course-42")).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyId<PkId, SkId> {
    /// The partition key identifier.
    pub(super) pk: PkId,
    /// The sort key identifier (`NoId` for simple-key tables).
    pub(super) sk: SkId,
}
impl KeyId<NoId, NoId> {
    /// Shorthand for singleton items whose PK and SK are both compile-time constants.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::{KeyId, NoId};
    ///
    /// let key_id: KeyId<NoId, NoId> = KeyId::NONE;
    /// ```
    pub const NONE: Self = KeyId { pk: NoId, sk: NoId };
}
impl<PkId> KeyId<PkId, NoId> {
    /// Creates a [`KeyId`] from a partition key ID, leaving the sort key as [`NoId`].
    ///
    /// Use this for item types with a variable PK and a constant SK. Chain
    /// [`sk`](KeyId::sk) afterwards if the sort key is also variable.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::{KeyId, NoId};
    ///
    /// let key_id: KeyId<&str, NoId> = KeyId::pk("user-1");
    /// ```
    pub fn pk(pk: PkId) -> Self {
        Self { pk, sk: NoId }
    }

    /// Adds a sort key ID to produce a composite [`KeyId<PkId, SkId>`].
    ///
    /// Use this for item types where both PK and SK are variable.
    ///
    /// # Examples
    ///
    /// ```
    /// use dynamodb_facade::KeyId;
    ///
    /// let key_id: KeyId<&str, &str>  = KeyId::pk("user-1").sk("course-42");
    /// ```
    pub fn sk<SkId>(self, sk: SkId) -> KeyId<PkId, SkId> {
        KeyId { pk: self.pk, sk }
    }
}
