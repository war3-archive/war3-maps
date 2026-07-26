//! # war3parser
//!
//! A library for parsing and extracting Warcraft III map files
//! (`.w3x` / `.w3m`).
//!
//! ## Crate layout
//!
//! - [`archive`] — the map container: optional `HM3W` header + embedded MPQ
//! - [`formats`] — parsers for the member files (`w3i`, `wts`, `imp`, `mmp`)
//! - [`model`] — portable data structures shared by CLI / WASM / library users
//! - [`reader`] — bounds-checked little-endian [`reader::ByteReader`]
//! - [`error`] — the crate-wide [`error::Error`] type
//!
//! The `war3map.w3i` parser supports the full format-version ladder
//! **v8 → v33** (Reign of Chaos betas through WC3 2.0); see
//! [`formats::w3i::FormatVersion`].
//!
//! ## Features
//!
//! | Feature | Default | Purpose |
//! |---------|---------|---------|
//! | `serde` | yes     | JSON serialization for dumps and the WASM bridge |
//!
//! ## Example
//!
//! ```ignore
//! use war3parser::prelude::*;
//!
//! let buffer = std::fs::read("path/to/map.w3x")?;
//! let mut metadata = War3MapMetadata::parse(&buffer)?;
//! metadata.resolve_trigger_strings();
//! let snapshot = metadata.snapshot()?;
//! println!("{:?}", snapshot.map_info.map(|i| i.name));
//! ```

pub mod archive;
pub mod error;
pub mod formats;
pub mod model;
pub mod reader;

/// The most commonly used types in one import.
pub mod prelude {
    #[doc(inline)]
    pub use crate::archive::War3MapW3x;
    #[doc(inline)]
    pub use crate::error::Error;
    #[doc(inline)]
    pub use crate::formats::{
        imp::War3MapImp,
        mmp::{MinimapIcon, War3MapMmp},
        w3i::{FormatVersion, War3MapW3i},
        wts::War3MapWts,
    };
    #[doc(inline)]
    pub use crate::model::{
        ImportEntry, MapSnapshot, StringTableEntry, War3Image, War3ImageData, War3MapHeader,
        War3MapMetadata,
    };
}

#[doc(inline)]
pub use crate::{
    archive::War3MapW3x,
    error::Error,
    model::{
        ImportEntry, MapSnapshot, StringTableEntry, War3Image, War3ImageData, War3MapHeader,
        War3MapMetadata,
    },
};
