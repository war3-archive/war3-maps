/// Binary reader to read binary data
pub mod binary_reader;

/// Custom error types
pub mod error;

/// [`War3MapImp`] parser for `war3map.imp` file
pub mod imp;

/// [`War3MapW3i`] parser for `war3map.w3i` file
pub mod w3i;

/// [`War3MapWts`] parser for `war3map.wts` file
pub mod wts;

/// [`War3MapW3x`] parser
pub mod w3x;

/// [`War3Image`] parser for images
pub mod img;

/// [`War3MapMmp`] parser for `war3map.mmp` minimap icons
pub mod mmp;

#[doc(inline)]
pub use {
    binary_reader::BinaryReadable, error::ParserError, imp::War3MapImp, mmp::War3MapMmp,
    w3i::War3MapW3i, w3x::War3MapW3x, wts::War3MapWts,
};
