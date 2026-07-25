//! Portable, reusable data model shared by the library, CLI, and WASM bindings.
//!
//! Parsing lives in [`crate::parser`]; this module holds the public shapes that
//! callers should depend on. Prefer these types over re-defining DTOs in
//! downstream crates.

mod header;
mod image;
mod import;
mod metadata;
mod string_table;

pub use header::War3MapHeader;
#[allow(deprecated)]
pub use image::War3ImageBase64;
pub use image::{War3Image, War3ImageData};
pub use import::ImportEntry;
pub use metadata::{MapSnapshot, War3MapMetadata};
pub use string_table::StringTableEntry;

/// Common derives for API-facing types (serde + optional TypeScript/WASM ABI).
#[macro_export]
macro_rules! api_type {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident $($rest:tt)*
    ) => {
        $(#[$meta])*
        #[cfg_attr(
            feature = "typescript",
            derive(tsify_next::Tsify),
            tsify(into_wasm_abi, from_wasm_abi)
        )]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[derive(Debug, Clone)]
        $vis struct $name $($rest)*
    };
}
