//! # War3Parser
//!
//! `war3parser` is a library for parsing and extracting Warcraft III map files.
//!
//! ## Crate layout
//!
//! - [`parser`] — binary format readers (`w3x`, `w3i`, `wts`, `imp`, `mmp`, images, …)
//! - [`model`] — portable data structures shared by CLI / WASM / library users
//!
//! Downstream crates should depend on [`model`] types (especially [`MapSnapshot`])
//! instead of inventing parallel DTOs.
//!
//! ## Features
//!
//! | Feature | Default | Purpose |
//! |---------|---------|---------|
//! | `serde` | yes | JSON serialization for dumps and TRIGSTR resolution |
//! | `typescript` | no | Tsify / wasm-bindgen ABI derives (used by `war3parser-wasm`) |
//! | `wasm` | no | Deprecated alias of `typescript` |
//!
//! ## Example
//!
//! ```ignore
//! use war3parser::prelude::War3MapMetadata;
//!
//! let buffer = std::fs::read("path/to/map.w3x")?;
//! let mut metadata = War3MapMetadata::from(&buffer).unwrap();
//! metadata.update_string_table().ok();
//! let snapshot = metadata.snapshot()?;
//! ```

/// Binary format parsers.
pub mod parser;

/// Portable, reusable public data model.
pub mod model;

pub mod prelude {
    #[doc(inline)]
    pub use crate::model::{
        ImportEntry, MapSnapshot, StringTableEntry, War3Image, War3ImageData, War3MapHeader,
        War3MapMetadata,
    };
    #[doc(inline)]
    pub use crate::parser::{
        binary_reader::BinaryReadable,
        error::ParserError,
        imp::War3MapImp,
        mmp::{MinimapIcon, War3MapMmp},
        w3i::War3MapW3i,
        w3x::War3MapW3x,
        wts::War3MapWts,
    };
}

// Re-export top-level modules for a stable public API surface.
#[doc(inline)]
pub use model::{
    ImportEntry, MapSnapshot, StringTableEntry, War3Image, War3ImageData, War3MapHeader,
    War3MapMetadata,
};

/// Backward-compatible path used by older callers / tests.
#[deprecated(note = "use war3parser::model or war3parser::prelude instead")]
pub mod war3map_metadata {
    pub use crate::model::{MapSnapshot, War3MapHeader, War3MapMetadata};
}
