mod builders;
mod conditions;
mod key_conditions;
mod projections;
mod updates;
mod utils;

use super::AttributeValue;
use utils::{fmt_attr_maps, resolve_expression};

/// A compiled DynamoDB expression string.
type Expression = String;

/// Collected `(placeholder, real_name)` pairs for `expression_attribute_names`.
type AttrNames = Vec<(String, String)>;

/// Collected `(placeholder, value)` pairs for `expression_attribute_values`.
type AttrValues = Vec<(String, AttributeValue)>;

pub(crate) use builders::*;
pub use conditions::*;
pub use key_conditions::*;
pub use projections::*;
pub use updates::*;
