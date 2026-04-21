/// Declares one or more DynamoDB attribute definitions as zero-sized types.
///
/// Each invocation generates a `pub struct` that implements
/// [`AttributeDefinition`](crate::AttributeDefinition), providing a
/// compile-time `const NAME: &str` and an associated `type Type` (one of
/// [`StringAttribute`](crate::StringAttribute),
/// [`NumberAttribute`](crate::NumberAttribute), or
/// [`BinaryAttribute`](crate::BinaryAttribute)).
///
/// These zero-sized types serve as type-level identifiers throughout the library: they
/// are used as generic parameters in [`HasAttribute`](crate::HasAttribute),
/// [`HasConstAttribute`](crate::HasConstAttribute),
/// [`KeySchema`](crate::KeySchema), and [`IndexDefinition`](crate::IndexDefinition).
///
/// # Syntax
///
/// ```text
/// attribute_definitions! {
///     [doc comments and attributes]
///     TypeName { "dynamo_attribute_name": AttributeTypeMarker }
///     ...
/// }
/// ```
///
/// Multiple definitions can appear in a single invocation.
///
/// # Examples
///
/// ```
/// use dynamodb_facade::{attribute_definitions, StringAttribute, NumberAttribute};
///
/// attribute_definitions! {
///     /// Partition key for the platform mono-table.
///     PK { "PK": StringAttribute }
///
///     /// Sort key for the platform mono-table.
///     SK { "SK": StringAttribute }
///
///     /// Item type discriminator (single-table design).
///     ItemType { "_TYPE": StringAttribute }
///
///     /// TTL attribute for expiring items.
///     Expiration { "expiration_timestamp": NumberAttribute }
///
///     /// Email attribute, used as a GSI partition key.
///     Email { "email": StringAttribute }
/// }
///
/// use dynamodb_facade::AttributeDefinition;
/// // Each generated type exposes its DynamoDB attribute name as a constant.
/// assert_eq!(PK::NAME, "PK");
/// assert_eq!(SK::NAME, "SK");
/// assert_eq!(ItemType::NAME, "_TYPE");
/// assert_eq!(Expiration::NAME, "expiration_timestamp");
/// assert_eq!(Email::NAME, "email");
/// ```
#[macro_export]
macro_rules! attribute_definitions {
    {
        $(
            $(#[$meta:meta])*
            $tname:ident {
                $name:literal: $t:ty
            }
        )+
    } => {
        $(
            $(#[$meta])*
            pub struct $tname;
            impl $crate::AttributeDefinition for $tname {
                const NAME: &'static str = $name;
                type Type = $t;
            }
        )+
    };
    // === diagnostic arm: catch-all for malformed input ===
    ($($tt:tt)*) => {
        ::core::compile_error!(concat!(
            "`attribute_definitions!` expected:\n",
            "    TypeName { \"dynamo_attribute_name\": StringAttribute|NumberAttribute|BinaryAttribute }\n",
            "    ... (one or more)"
        ));
    };
}

/// Builds the nested tuple type used to represent a list of
/// [`AttributeDefinition`](crate::AttributeDefinition) types.
///
/// `attr_list![A, B, C]` expands to the right-nested tuple
/// `(A, (B, (C, ())))`, which is the representation expected by
/// [`AttributeList`](crate::AttributeList) and the
/// [`DynamoDBItem::AdditionalAttributes`](crate::DynamoDBItem::AdditionalAttributes)
/// associated type.
///
/// You rarely need to invoke this macro directly — [`dynamodb_item!`](crate::dynamodb_item) calls it
/// internally. It is exposed for use in manual [`DynamoDBItem`](crate::DynamoDBItem)
/// implementations where you need to spell out the `AdditionalAttributes` type
/// explicitly.
///
/// # Syntax
///
/// ```text
/// attr_list![AttrType1, AttrType2, ...]
/// ```
///
/// An empty list `attr_list![]` expands to `()`.
///
/// # Examples
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::attr_list;
///
/// // Equivalent to (ItemType, (Expiration, ()))
/// type MyAttrs = attr_list![ItemType, Expiration];
/// ```
#[macro_export]
macro_rules! attr_list {
    // Expands to the nested tuple type
    [$($attr:ty),*] => {
        // just the type
        $crate::attr_list!(@nest $($attr),*)
    };
    (@nest) => { () };
    (@nest $head:ty $(, $tail:ty)*) => {
        ($head, $crate::attr_list!(@nest $($tail),*))
    };
}

/// Defines a key schema struct implementing [`KeySchema`](crate::KeySchema).
///
/// This is a lower-level macro used internally by [`table_definitions!`](crate::table_definitions) and
/// [`index_definitions!`](crate::index_definitions). You can use it directly when you need a named key
/// schema type outside of a table or index definition.
///
/// Generates a `pub struct` that implements:
/// - [`KeySchema`](crate::KeySchema) — always
/// - [`SimpleKeySchema`](crate::SimpleKeySchema) — when only `PartitionKey` is given
/// - [`CompositeKeySchema`](crate::CompositeKeySchema) — when both `PartitionKey` and `SortKey` are given
///
/// # Syntax
///
/// Simple key (partition key only):
/// ```text
/// key_schema! {
///     MySchema {
///         type PartitionKey = MyPkAttr;
///     }
/// }
/// ```
///
/// Composite key (partition + sort key):
/// ```text
/// key_schema! {
///     MySchema {
///         type PartitionKey = MyPkAttr;
///         type SortKey = MySkAttr;
///     }
/// }
/// ```
///
/// The `PartitionKey` and `SortKey` fields may appear in either order.
///
/// The given types must implement [`AttributeDefinition`](crate::AttributeDefinition) and will
/// typically have been created using [`attribute_definitions!`]
///
/// # Examples
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::{key_schema, CompositeKeySchema, KeySchema, SimpleKeySchema};
///
/// key_schema! {
///     UserSchema {
///         type PartitionKey = PK;
///         type SortKey = SK;
///     }
/// }
///
/// key_schema! {
///     ConfigSchema {
///         type PartitionKey = PK;
///     }
/// }
///
/// fn _assert_composite<KS: CompositeKeySchema>() {}
/// fn _assert_simple<KS: SimpleKeySchema>() {}
///
/// _assert_composite::<UserSchema>();
/// _assert_simple::<ConfigSchema>();
/// ```
#[macro_export]
macro_rules! key_schema {
    // Syntaxic QoL
    {
        $(#[$meta:meta])*
        $ksname:ident {
            type SortKey = $skty:ty;
            type PartitionKey = $pkty:ty;
        }
    } => {
        $crate::key_schema!{
            $(#[$meta])*
            $ksname {
                type PartitionKey = $pkty;
                type SortKey = $skty;
            }
        }
    };
    // Processing
    {
        $(#[$meta:meta])*
        $ksname:ident {
            type PartitionKey = $pkty:ty;
            type SortKey = $skty:ty;
        }
    } => {
        $(#[$meta])*
        pub struct $ksname;
        impl $crate::KeySchema for $ksname {
            type Kind = $crate::CompositeKey;
            type PartitionKey = $pkty;
        }
        impl $crate::CompositeKeySchema for $ksname {
            type SortKey = $skty;
        }
    };
    {
        $(#[$meta:meta])*
        $ksname:ident {
            type PartitionKey = $pkty:ty;
        }
    } => {
        $(#[$meta])*
        pub struct $ksname;
        impl $crate::KeySchema for $ksname {
            type Kind = $crate::SimpleKey;
            type PartitionKey = $pkty;
        }
        impl $crate::SimpleKeySchema for $ksname {}
    };
    // === diagnostic arm: catch-all for malformed input ===
    ($($tt:tt)*) => {
        ::core::compile_error!(concat!(
            "`key_schema!` expected:\n",
            "    SchemaName {\n",
            "        type PartitionKey = PkAttr;\n",
            "        [type SortKey = SkAttr;]\n",
            "    }"
        ));
    };
}

/// Manually implements [`HasAttribute`](crate::HasAttribute) or
/// [`HasConstAttribute`](crate::HasConstAttribute) for a type.
///
/// Use this macro when you need to implement the attribute traits without going
/// through [`dynamodb_item!`](crate::dynamodb_item) — for example, when writing a manual
/// [`DynamoDBItem`](crate::DynamoDBItem) implementation or when adding
/// attribute bindings to a type that is already wired to a table.
///
/// # Syntax
///
/// Each attribute block uses one of two forms:
///
/// **Constant attribute** — implements [`HasConstAttribute`](crate::HasConstAttribute):
/// ```text
/// has_attributes! {
///     MyType {
///         MyAttr { const VALUE: AttrValueType = expr; }
///     }
/// }
/// ```
///
/// **Dynamic attribute** — implements [`HasAttribute`](crate::HasAttribute):
/// ```text
/// has_attributes! {
///     MyType {
///         MyAttr {
///             fn attribute_id(&self) -> IdType { ... }
///             fn attribute_value(id) -> ValueType { ... }
///         }
///     }
/// }
/// ```
///
/// The `attribute_id` and `attribute_value` functions may appear in either
/// order. If `attribute_id` is omitted, it defaults to returning
/// [`NoId`](crate::NoId).
///
/// Multiple attribute blocks can appear in a single invocation.
///
/// # The `attribute_id` → `attribute_value` pipeline
///
/// For dynamic attributes, the return type of `attribute_id(&self)` is
/// **always** the input type of `attribute_value(id)`. The two functions
/// form a pipeline: `attribute_id` extracts a lightweight identifier from
/// `&self`, and `attribute_value` transforms it into the final DynamoDB
/// value. This separation allows for independant usages of the methods,
/// in particular it powers the "_by_id" variants of the get/update/delete
/// operations.
///
/// # The `'id` lifetime
///
/// When `attribute_id` returns a reference — typically `&str` — you must
/// annotate it with the **`'id`** lifetime: `&'id str`. This lifetime
/// comes from the [`Id<'id>`](crate::HasAttribute::Id) associated type on
/// [`HasAttribute`](crate::HasAttribute) and must be used exactly as-is.
///
/// The typical use-case is when the identifier is a `String` field on the
/// struct and the final attribute value is a formatted composition of that
/// field (e.g. `format!("USER#{id}")`). Returning `&'id str` lets
/// `attribute_id` borrow the field without cloning it, and
/// `attribute_value` can then use the reference to produce an owned
/// `String`.
///
/// If the attribute does not need data from `&self` (e.g. the value is
/// always a constant), you can omit `attribute_id` entirely — the macro
/// defaults it to returning [`NoId`](crate::NoId).
///
/// # Examples
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::has_attributes;
///
/// struct CourseStatus(String);
///
/// has_attributes! {
///     CourseStatus {
///         // Constant attribute: always the same value
///         ItemType { const VALUE: &'static str = "COURSE_STATUS"; }
///
///         // Dynamic attribute: attribute_id borrows &self.0 as &'id str,
///         // then attribute_value receives that same &str to format the
///         // DynamoDB value — no .clone() needed.
///         SK {
///             fn attribute_id(&self) -> &'id str { &self.0 }
///             fn attribute_value(id) -> String { format!("STATUS#{id}") }
///         }
///     }
/// }
///
/// use dynamodb_facade::HasConstAttribute;
/// assert_eq!(<CourseStatus as HasConstAttribute<ItemType>>::VALUE, "COURSE_STATUS");
///
/// use dynamodb_facade::HasAttribute;
/// let status = CourseStatus("draft".to_owned());
/// assert_eq!(
///     <CourseStatus as HasAttribute<SK>>::attribute(&status),
///     "STATUS#draft".to_owned()
/// );
/// ```
#[macro_export]
macro_rules! has_attributes {
    {
        $item:ty {
            $(
                $attr:path {$($blk:tt)+}
            )+
        }
    } => {
        $(
            $crate::has_attributes! {
                @inner $attr ; $item {$($blk)+}
            }
        )+
    };
    // Syntaxic QoL
    // Re-order functions
    {
        @inner $attr:path ; $item:ty {
            fn attribute_value ($id:ident) -> $outty:ty $produce:block
            fn attribute_id ($(&)?$self:ident) -> $idty:ty $extract:block
        }
    } => {
        $crate::has_attributes! {
            @inner $attr ; $item {
                fn attribute_id ($self) -> $idty $extract
                fn attribute_value ($id) -> $outty $produce
            }
        }
    };
    // Default attribute_id
    {
        @inner $attr:path ; $item:ty {
            fn attribute_value ($id:ident) -> $outty:ty $produce:block
        }
    } => {
        $crate::has_attributes! {
            @inner $attr ; $item {
                fn attribute_id(&self) -> $crate::NoId {
                    $crate::NoId
                }
                fn attribute_value ($id) -> $outty $produce
            }
        }
    };
    // Process
    // HasAttribute
    {
        @inner $attr:path ; $item:ty {
            fn $id_fct:ident ($(&)?$self:ident) -> $idty:ty $extract:block
            fn $value_fct:ident ($id:ident) -> $outty:ty $produce:block
        }
    } => {
        impl $crate::HasAttribute<$attr> for $item {
            type Id<'id> = $idty;
            type Value = $outty;
            fn $id_fct(& $self) -> Self::Id<'_> $extract
            fn $value_fct($id: Self::Id<'_>) -> Self::Value $produce
        }
    };
    // HasConstAttribute
    {
        @inner $attr:path ; $item:ty {
            const VALUE: $t:ty = $v:expr;
        }
    } => {
        impl $crate::HasConstAttribute<$attr> for $item {
            type Value = $t;
            const VALUE: Self::Value = $v;
        }
    };
    // === diagnostic arm: catch-all for malformed input ===
    ($($tt:tt)*) => {
        ::core::compile_error!(concat!(
            "`has_attributes!` expected:\n",
            "    ItemType {\n",
            "        AttrType { const VALUE: T = expr; }\n",
            "        AttrType {\n",
            "            fn attribute_id(&self) -> &'id str { ... }\n",
            "            fn attribute_value(id) -> T { ... }\n",
            "        }\n",
            "        ... (one or more attribute blocks)\n",
            "    }"
        ));
    };
}

/// Wires a Rust struct to a DynamoDB table by implementing
/// [`DynamoDBItem`](crate::DynamoDBItem) and the attribute traits.
///
/// This is the primary macro for defining how a Rust type maps to a DynamoDB
/// item. It generates:
///
/// - [`DynamoDBItem<TD>`](crate::DynamoDBItem) — with the correct
///   `AdditionalAttributes` type derived from the non-key, non-`#[marker_only]`
///   attribute blocks.
/// - [`HasAttribute`](crate::HasAttribute) or
///   [`HasConstAttribute`](crate::HasConstAttribute) for every attribute block,
///   including the partition key and sort key.
///
/// # Syntax
///
/// ```text
/// dynamodb_item! {
///     #[table = TableType]
///     StructType {
///         #[partition_key]
///         PkAttr { ... }
///
///         #[sort_key]           // optional
///         SkAttr { ... }
///
///         #[marker_only]        // optional; implements HasAttribute but excluded from AdditionalAttributes
///         OtherAttr { ... }
///
///         AdditionalAttr { ... }
///         ...
///     }
/// }
/// ```
///
/// Each attribute block uses the same syntax as [`has_attributes!`]:
/// either `const VALUE: T = expr;` for constant attributes, or
/// `fn attribute_id(&self) -> T { ... }` + `fn attribute_value(id) -> T { ... }`
/// for dynamic attributes. The return type of `attribute_id` is always the
/// input type of `attribute_value` — the two form a pipeline.
///
/// When `attribute_id` returns a reference (typically borrowing a `String`
/// field to avoid cloning), annotate it with the **`'id`** lifetime:
/// `&'id str`. This lifetime is dictated by the
/// [`Id<'id>`](crate::HasAttribute::Id) associated type.
/// See [`has_attributes!`] for a detailed explanation.
///
/// ## Attribute modifiers
///
/// - `#[partition_key]` — marks the partition key attribute. **Required.**
/// - `#[sort_key]` — marks the sort key attribute. Optional; omit for simple-key tables.
/// - `#[marker_only]` — implements [`HasAttribute`](crate::HasAttribute) for
///   the attribute (e.g. for GSI membership) but does **not** add it to
///   `AdditionalAttributes`, because the attribute is already serialized as
///   part of the struct's serde representation.
///
/// # Examples
///
/// **Singleton item** (constant PK + SK):
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::dynamodb_item;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct AppConfig {
///     pub feature_flags: Vec<String>,
/// }
///
/// dynamodb_item! {
///     #[table = PlatformTable]
///     AppConfig {
///         #[partition_key]
///         PK { const VALUE: &'static str = "APP_CONFIG"; }
///         #[sort_key]
///         SK { const VALUE: &'static str = "APP_CONFIG"; }
///         ItemType { const VALUE: &'static str = "APP_CONFIG"; }
///     }
/// }
/// ```
///
/// **Variable PK, constant SK**:
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::dynamodb_item;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct Course {
///     pub id: String,
///     pub title: String,
///     pub email: String,
/// }
///
/// dynamodb_item! {
///     #[table = PlatformTable]
///     Course {
///         #[partition_key]
///         PK {
///             // Borrows self.id as &'id str — no clone needed.
///             // attribute_value then receives that same &str.
///             fn attribute_id(&self) -> &'id str { &self.id }
///             fn attribute_value(id) -> String { format!("COURSE#{id}") }
///         }
///         #[sort_key]
///         SK { const VALUE: &'static str = "COURSE"; }
///         // email is already part of the struct and serialized by serde,
///         // so use #[marker_only] to exclude it from AdditionalAttributes
///         #[marker_only]
///         Email {
///             fn attribute_id(&self) -> &'id str { &self.email }
///             fn attribute_value(id) -> String { id.to_owned() }
///         }
///         // The constant ItemType attribute is part of AdditionalAttributes
///         // and will be added to each Course item
///         ItemType { const VALUE: &'static str = "COURSE"; }
///     }
/// }
/// ```
#[macro_export]
macro_rules! dynamodb_item {
    // Syntaxic QoL
    // Bubble-up #[...] modified attributes
    {
        #[table = $table:path]
        $item:ty {
            $(
                #[$attr_mod:ident]
                $modified_attr:path {$($modified_blk:tt)+}
            )+
            $(
                $attr:path {$($blk:tt)+}
            )*
        }
    } => {
        $crate::dynamodb_item! {
            @modtop
            #[table = $table]
            $item {
                $(
                    #[$attr_mod]
                    $modified_attr {$($modified_blk)+}
                )+
                $(
                    $attr {$($blk)+}
                )*
            }
        }
    };
    {
        #[table = $table:path]
        $item:ty {
            $(
                #[$attr_mod:ident]
                $modified_attr:path {$($modified_blk:tt)+}
            )*
            $(
                $attr_before:path {$($blk_before:tt)+}
            )+
            #[$attr_mod_after:ident]
            $modified_attr_after:path {$($modified_blk_after:tt)+}
            $($rest:tt)*
        }
    } => {
        $crate::dynamodb_item! {
            #[table = $table]
            $item {
                $(
                    #[$attr_mod]
                    $modified_attr {$($modified_blk)+}
                )*
                #[$attr_mod_after]
                $modified_attr_after {$($modified_blk_after)+}
                $(
                    $attr_before {$($blk_before)+}
                )+
                $($rest)*
            }
        }
    };
    // Bubble-up PK
    {
        @modtop
        #[table = $table:path]
        $item:ty {
            #[partition_key]
            $pk_attr:path {$($pk_blk:tt)+}
            $(
                #[$attr_mod:ident]
                $modified_attr:path {$($modified_blk:tt)+}
            )*
            $(
                $attr:path {$($blk:tt)+}
            )*
        }
    } => {
        $crate::dynamodb_item! {
            @pktop
            #[table = $table]
            $item {
                #[partition_key]
                $pk_attr {$($pk_blk)+}
                $(
                    #[$attr_mod]
                    $modified_attr {$($modified_blk)+}
                )*
                $(
                    $attr {$($blk)+}
                )*
            }
        }
    };
    {
        @modtop
        #[table = $table:path]
        $item:ty {
            #[$first_attr_mod:ident]
            $first_modified_attr:path {$($first_modified_blk:tt)+}
            $(
                #[$attr_mod:ident]
                $modified_attr:path {$($modified_blk:tt)+}
            )+
            $(
                $attr:path {$($blk:tt)+}
            )*
        }
    } => {
        $crate::dynamodb_item! {
            @modtop
            #[table = $table]
            $item {
                $(
                    #[$attr_mod]
                    $modified_attr {$($modified_blk)+}
                )+
                #[$first_attr_mod]
                $first_modified_attr {$($first_modified_blk)+}
                $(
                    $attr {$($blk)+}
                )*
            }
        }
    };
    // Optionaly Bubble-up SK
    {
        @pktop
        #[table = $table:path]
        $item:ty {
            #[partition_key]
            $pk_attr:path {$($pk_blk:tt)+}
            #[sort_key]
            $sk_attr:path {$($sk_blk:tt)+}
            $(
                #[$attr_mod:ident]
                $modified_attr:path {$($modified_blk:tt)+}
            )*
            $(
                $attr:path {$($blk:tt)+}
            )*
            $(
                @barier
                $(
                    #[$attr_mod_after:ident]
                    $modified_attr_after:path {$($modified_blk_after:tt)+}
                )+
            )?
        }
    } => {
        $crate::dynamodb_item! {
            @allsorted
            #[table = $table]
            $item {
                #[partition_key]
                $pk_attr {$($pk_blk)+}
                #[sort_key]
                $sk_attr {$($sk_blk)+}
                $(
                    $(
                        #[$attr_mod_after]
                        $modified_attr_after {$($modified_blk_after)+}
                    )+
                )?
                $(
                    #[$attr_mod]
                    $modified_attr {$($modified_blk)+}
                )*
                $(
                    $attr {$($blk)+}
                )*
            }
        }
    };
    {
        @pktop
        #[table = $table:path]
        $item:ty {
            #[partition_key]
            $pk_attr:path {$($pk_blk:tt)+}
            #[$first_attr_mod:ident]
            $first_modified_attr:path {$($first_modified_blk:tt)+}
            $(
                #[$attr_mod:ident]
                $modified_attr:path {$($modified_blk:tt)+}
            )*
            $(
                $attr:path {$($blk:tt)+}
            )*
            $(
                @barier
                $(
                    #[$attr_mod_after:ident]
                    $modified_attr_after:path {$($modified_blk_after:tt)+}
                )+
            )?
        }
    } => {
        $crate::dynamodb_item! {
            @pktop
            #[table = $table]
            $item {
                #[partition_key]
                $pk_attr {$($pk_blk)+}
                $(
                    #[$attr_mod]
                    $modified_attr {$($modified_blk)+}
                )*
                $(
                    $attr {$($blk)+}
                )*
                @barier
                $(
                    $(
                        #[$attr_mod_after]
                        $modified_attr_after {$($modified_blk_after)+}
                    )+
                )?
                #[$first_attr_mod]
                $first_modified_attr {$($first_modified_blk)+}
            }
        }
    };
    {
        @pktop
        #[table = $table:path]
        $item:ty {
            #[partition_key]
            $pk_attr:path {$($pk_blk:tt)+}
            $(
                $attr:path {$($blk:tt)+}
            )*
            $(
                @barier
                $(
                    #[$attr_mod_after:ident]
                    $modified_attr_after:path {$($modified_blk_after:tt)+}
                )+
            )?
        }
    } => {
        $crate::dynamodb_item! {
            @allsorted
            #[table = $table]
            $item {
                #[partition_key]
                $pk_attr {$($pk_blk)+}
                $(
                    $(
                        #[$attr_mod_after]
                        $modified_attr_after {$($modified_blk_after)+}
                    )+
                )?
                $(
                    $attr {$($blk)+}
                )*
            }
        }
    };
    // Processing
    {
        @allsorted
        #[table = $table:path]
        $item:ty {
            #[partition_key]
            $pk_attr:path {$($pk_blk:tt)+}
            $(
                #[sort_key]
                $sk_attr:path {$($sk_blk:tt)+}
            )?
            $(
                #[marker_only]
                $marker_only_attr:path {$($marker_only_blk:tt)+}
            )*
            $(
                $attr:path {$($blk:tt)+}
            )*
        }
    } => {
        $crate::dynamodb_item! {
            @dbitem $table ; $item {
                $($attr)*
            }
        }
        $crate::has_attributes! {
            $item {
                $pk_attr {$($pk_blk)+}
                $($sk_attr {$($sk_blk)+})?
                $(
                    $marker_only_attr {$($marker_only_blk)+}
                )*
                $(
                    $attr {$($blk)+}
                )*
            }
        }
    };
    {
        @dbitem $table:path; $item:ty {
            $($attr:path)*
        }
    } => {
        impl $crate::DynamoDBItem<$table> for $item {
            type AdditionalAttributes = $crate::attr_list![$($attr),*];
        }
    };
    // === diagnostic arms: catch-all for malformed input ===
    // User-form catch-all (table attribute present, body malformed —
    // most commonly a missing `#[partition_key]`).
    {
        #[table = $table:path]
        $item:ty {$($tt:tt)*}
    } => {
        ::core::compile_error!(concat!(
            "`dynamodb_item!`: malformed body. Most common cause is a missing ",
            "`#[partition_key]` annotation — exactly one key attribute block must ",
            "be marked `#[partition_key]`. Expected shape:\n",
            "    #[table = TableType]\n",
            "    ItemType {\n",
            "        #[partition_key]\n",
            "        PkAttr { ... }\n",
            "        [#[sort_key]]\n",
            "        [SkAttr { ... }]\n",
            "        [#[marker_only]]\n",
            "        [OtherAttr { ... }]\n",
            "        AdditionalAttr { ... }\n",
            "        ...\n",
            "    }"
        ));
    };
    // Generic fallback (missing `#[table = ...]` or otherwise unrecognised).
    ($($tt:tt)*) => {
        ::core::compile_error!(concat!(
            "`dynamodb_item!` expected:\n",
            "    #[table = TableType]\n",
            "    ItemType {\n",
            "        #[partition_key]\n",
            "        PkAttr { ... }\n",
            "        [#[sort_key]]\n",
            "        [SkAttr { ... }]\n",
            "        AdditionalAttr { ... }\n",
            "        ...\n",
            "    }"
        ));
    };
}

/// Defines one or more DynamoDB table zero-sized types implementing
/// [`TableDefinition`](crate::TableDefinition).
///
/// Each definition generates a `pub struct` with an internal key schema and
/// a `table_name()` function. The key schema is derived from the `type`
/// declarations:
///
/// - `type PartitionKey = ...` only → [`SimpleKeySchema`](crate::SimpleKeySchema)
/// - `type PartitionKey = ...` + `type SortKey = ...` → [`CompositeKeySchema`](crate::CompositeKeySchema)
///
/// Multiple table definitions can appear in a single invocation.
///
/// # Syntax
///
/// ```text
/// table_definitions! {
///     [doc comments and attributes]
///     TableName {
///         type PartitionKey = PkAttr;
///         type SortKey = SkAttr;   // optional
///         fn table_name() -> String { ... }
///     }
///     ...
/// }
/// ```
///
/// # Examples
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::table_definitions;
///
/// table_definitions! {
///     /// The platform mono-table with composite key (PK + SK).
///     LearningTable {
///         type PartitionKey = PK;
///         type SortKey = SK;
///         fn table_name() -> String {
///             std::env::var("TABLE_NAME").unwrap_or_else(|_| "learning".to_owned())
///         }
///     }
/// }
///
/// use dynamodb_facade::TableDefinition;
/// assert_eq!(LearningTable::table_name(), "learning");
/// ```
#[macro_export]
macro_rules! table_definitions {
    {
        $(
            $(#[$meta:meta])*
            $table:ident {
                $(type $ident_before:ident = $identty_before:ty;)*
                fn $table_name_fct:ident() -> String $table_name:block
                $(type $ident_after:ident = $identty_after:ty;)*
            }
        )+
    } => {
        $(
            $(#[$meta])*
            pub struct $table;
            const _: () = {
                $crate::key_schema! {
                    __TableKeySchema {
                        $(type $ident_before = $identty_before;)*
                        $(type $ident_after = $identty_after;)*
                    }
                }

                impl $crate::TableDefinition for $table {
                    type KeySchema = __TableKeySchema;
                    fn $table_name_fct() -> String $table_name
                }
            };
        )+
    };
    // === diagnostic arm: catch-all for malformed input ===
    ($($tt:tt)*) => {
        ::core::compile_error!(concat!(
            "`table_definitions!` expected:\n",
            "    TableName {\n",
            "        type PartitionKey = PkAttr;\n",
            "        [type SortKey = SkAttr;]\n",
            "        fn table_name() -> String { ... }\n",
            "    }\n",
            "    ... (one or more)"
        ));
    };
}

/// Defines one or more DynamoDB Secondary Index (LSI or GSI) zero-sized types implementing
/// [`IndexDefinition`](crate::IndexDefinition).
///
/// Each definition generates a `pub struct` associated with a specific table
/// type. The `#[table = TableType]` attribute is required and links the index
/// to its parent table. The key schema follows the same rules as
/// [`table_definitions!`](crate::table_definitions): `PartitionKey` alone gives a simple-key index;
/// adding `SortKey` gives a composite-key index.
///
/// Multiple index definitions can appear in a single invocation.
///
/// # Syntax
///
/// ```text
/// index_definitions! {
///     [doc comments and attributes]
///     #[table = TableType]
///     IndexName {
///         type PartitionKey = PkAttr;
///         type SortKey = SkAttr;   // optional
///         fn index_name() -> String { ... }
///     }
///     ...
/// }
/// ```
///
/// # Examples
///
/// ```
/// # use dynamodb_facade::test_fixtures::*;
/// use dynamodb_facade::index_definitions;
///
/// index_definitions! {
///     /// GSI on item type — query all items of a given type.
///     #[table = PlatformTable]
///     CourseTypeIndex {
///         type PartitionKey = ItemType;
///         fn index_name() -> String { "iType".to_owned() }
///     }
///
///     /// GSI on email with composite key.
///     #[table = PlatformTable]
///     EmailSkIndex {
///         type PartitionKey = Email;
///         type SortKey = SK;
///         fn index_name() -> String { "iEmailSk".to_owned() }
///     }
/// }
///
/// use dynamodb_facade::IndexDefinition;
/// assert_eq!(CourseTypeIndex::index_name(), "iType");
/// assert_eq!(EmailSkIndex::index_name(), "iEmailSk");
/// ```
#[macro_export]
macro_rules! index_definitions {
    // Syntaxic QoL: Bubble $[table = ...] up
    {
        $(
            $(#[$($meta:tt)+])*
            $index:ident {$($rest:tt)+}
        )+
    } => {
        $(
            $crate::index_definitions!{
                @solo
                $(#[$($meta)+])*
                $index {$($rest)+}
            }
        )+
    };
    // Syntaxic QoL: Bubble $[table = ...] up
    {
        @solo
        #[table = $table:ty]
        $(#[$meta:meta])*
        $index:ident {$($rest:tt)+}
        $(#[$firsts:meta])*
    } => {
        $crate::index_definitions!{
            @tableup $table;
            $(#[$firsts])*
            $(#[$meta])*
            $index {$($rest)+}
        }
    };
    {
        @solo
        #[$first:meta]
        $(#[$($others:tt)+])*
        $index:ident {$($rest:tt)+}
        $(#[$($firsts:tt)+])*
    } => {
        $crate::index_definitions!{
            @solo
            $(#[$($others)+])*
            $index {$($rest)+}
            $(#[$($firsts)+])*
            #[$first]
        }
    };
    // Processing
    {
        @tableup $table:ty;
        $(#[$meta:meta])*
        $index:ident {
            $(type $ident_before:ident = $identty_before:ty;)*
            fn $index_name_fct:ident() -> String $index_name:block
            $(type $ident_after:ident = $identty_after:ty;)*
        }
    } => {
        $(#[$meta])*
        pub struct $index;
        const _: () = {
            $crate::key_schema! {
                __IndexKeySchema {
                    $(type $ident_before = $identty_before;)*
                    $(type $ident_after = $identty_after;)*
                }
            }

            impl $crate::IndexDefinition<$table> for $index {
                type KeySchema = __IndexKeySchema;
                fn $index_name_fct() -> String $index_name
            }
        };
    };
    // === diagnostic arm: catch-all for malformed input ===
    ($($tt:tt)*) => {
        ::core::compile_error!(concat!(
            "`index_definitions!` expected:\n",
            "    #[table = TableType]\n",
            "    IndexName {\n",
            "        type PartitionKey = PkAttr;\n",
            "        [type SortKey = SkAttr;]\n",
            "        fn index_name() -> String { ... }\n",
            "    }\n",
            "    ... (one or more)"
        ));
    };
}
