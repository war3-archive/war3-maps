//! # War3Parser
//!
//! `war3parser` is a library for parsing and extracting Warcraft III map files.
//!
//! ## Features
//!
//! - Extract files from MPQ archives. _(As long as the file name is known and the file exists in the MPQ)_
//! - Parse file formats into Rust structs
//!
//! ### Supported parse targets
//!
//! - W3X: [`War3MapW3x`]
//! - W3I: [`War3MapW3i`] (ROC v18 → Reforged / WC3 2.0 v31–33)
//! - WTS: [`War3MapWts`]
//! - IMP: [`War3MapImp`]
//! - BLP: [`BlpImage`]
//! - TGA: [`TgaImage`]
//!
//! And a helper struct to include all supported metadata of a map file: [`War3MapMetadata`]
//!
//! ### Implementation
//!
//! Most of the struct implemented [`BinaryReadable`] trait, which provides a `load` function to load the struct from a binary reader.
//!
//! We use the trait implementation chain to load the struct from a file.

/// Parsers to do the actual parsing
pub mod parser;

/// Helper struct that includes all supported metadata of a map file
pub mod war3map_metadata;

pub mod prelude {
    #[doc(inline)]
    pub use crate::parser::{
        binary_reader::BinaryReadable,
        error::ParserError,
        img::{War3Image, War3ImageBase64},
        imp::War3MapImp,
        mmp::{MinimapIcon, War3MapMmp},
        w3i::War3MapW3i,
        w3x::War3MapW3x,
        wts::War3MapWts,
    };
    #[doc(inline)]
    pub use crate::war3map_metadata::{War3MapHeader, War3MapMetadata};
}
