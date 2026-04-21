use super::*;

/// Re-exports the SDK `ReturnValue` enum under a shorter alias for internal use.
pub(super) use aws_sdk_dynamodb::types::ReturnValue as SDKReturnValue;

mod state_traits {
    //! Sealed typestate dimension traits for operation builder type parameters.
    pub trait ReturnValue {}
    pub trait OutputFormat {}
    pub trait ConditionState {}
    pub trait FilterState {}
    pub trait ProjectionState {}
}
pub(super) use state_traits::*;

/// Typestate marker: return the item's old (pre-operation) attribute values.
#[doc(hidden)]
pub struct Old;
impl ReturnValue for Old {}

/// Typestate marker: return the item's new (post-operation) attribute values.
#[doc(hidden)]
pub struct New;
impl ReturnValue for New {}

/// Maps a [`ReturnValue`] typestate marker to the corresponding SDK [`SDKReturnValue`] variant.
#[doc(hidden)]
pub trait ReturnValueKind: ReturnValue {
    fn return_value() -> SDKReturnValue;
}

impl ReturnValueKind for Old {
    fn return_value() -> SDKReturnValue {
        SDKReturnValue::AllOld
    }
}

impl ReturnValueKind for New {
    fn return_value() -> SDKReturnValue {
        SDKReturnValue::AllNew
    }
}

/// Typestate marker: the operation does not return item attributes.
#[doc(hidden)]
pub struct ReturnNothing;
impl ReturnValue for ReturnNothing {}

/// Typestate marker: the operation returns item attributes according to `RV` ([`Old`] or [`New`]).
#[doc(hidden)]
pub struct Return<RV: ReturnValueKind>(PhantomData<RV>);
impl<RV: ReturnValueKind> ReturnValue for Return<RV> {}

// -- Output format typestate ------------------------------------------------

/// Typestate marker: terminal methods deserialize the response into `T`.
#[doc(hidden)]
pub struct Typed;
impl OutputFormat for Typed {}

/// Typestate marker: terminal methods return a raw [`Item<TD>`](crate::Item).
#[doc(hidden)]
pub struct Raw;
impl OutputFormat for Raw {}

// -- Expression-set typestate -----------------------------------------------

/// Typestate marker: no condition expression has been set on this builder.
#[doc(hidden)]
pub struct NoCondition;
impl ConditionState for NoCondition {}

/// Typestate marker: a condition expression has already been set; prevents a second call.
#[doc(hidden)]
pub struct AlreadyHasCondition;
impl ConditionState for AlreadyHasCondition {}

/// Typestate marker: no filter expression has been set on this builder.
#[doc(hidden)]
pub struct NoFilter;
impl FilterState for NoFilter {}

/// Typestate marker: a filter expression has already been set; prevents a second call.
#[doc(hidden)]
pub struct AlreadyHasFilter;
impl FilterState for AlreadyHasFilter {}

/// Typestate marker: no projection expression has been set on this builder.
#[doc(hidden)]
pub struct NoProjection;
impl ProjectionState for NoProjection {}

/// Typestate marker: a projection expression has already been set; prevents a second call.
#[doc(hidden)]
pub struct AlreadyHasProjection;
impl ProjectionState for AlreadyHasProjection {}
