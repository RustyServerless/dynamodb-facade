/// Generates a sealed marker trait and one or more unit struct implementors.
///
/// Produces a `pub trait $trait: $sealed_trait {}` and, for each `$unit_type`,
/// a `#[derive(Debug, Clone, Copy)] pub struct $unit_type` with both the seal
/// and the marker trait implemented.
macro_rules! impl_sealed_marker_types {
    ($(#[$trait_meta:meta])*  $trait:ident, $sealed_trait:path; $($(#[$type_meta:meta])* $unit_type:ident),+) => {
        $(#[$trait_meta])*
        pub trait $trait: $sealed_trait {}
        $(
            #[derive(Debug, Clone, Copy)]
            $(#[$type_meta])*
            pub struct $unit_type;
            impl $sealed_trait for $unit_type {}
            impl $trait for $unit_type {}
        )+
    };
}
/// Re-exports [`impl_sealed_marker_types!`] for use throughout the crate.
pub(crate) use impl_sealed_marker_types;
