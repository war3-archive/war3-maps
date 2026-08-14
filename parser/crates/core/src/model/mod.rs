//! Portable, reusable data model shared by the library, CLI, and WASM bindings.
//!
//! Parsing lives in [`crate::formats`] and [`crate::archive`]; this module
//! holds the public shapes that callers should depend on. Prefer these types
//! over re-defining DTOs in downstream crates.

mod header;
mod image;
mod metadata;
mod snapshot;

pub use header::{War3MapHeader, HM3W_MAGIC};
pub use image::{War3Image, War3ImageData};
pub use metadata::War3MapMetadata;
pub use snapshot::{ImportEntry, MapSnapshot, StringTableEntry};
