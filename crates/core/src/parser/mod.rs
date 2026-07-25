/// Binary reader helpers.
pub mod binary_reader;

/// Custom error types.
pub mod error;

/// [`War3MapImp`] parser for `war3map.imp`.
pub mod imp;

/// [`War3MapW3i`] parser for `war3map.w3i`.
pub mod w3i;

/// [`War3MapWts`] parser for `war3map.wts`.
pub mod wts;

/// [`War3MapW3x`] container / MPQ access.
pub mod w3x;

/// Image loading re-exports (implementation lives in [`crate::model`]).
pub mod img;

/// [`War3MapMmp`] parser for `war3map.mmp` minimap icons.
pub mod mmp;

/// Experimental / incomplete WTG trigger parser.
pub mod wtg;

#[doc(inline)]
pub use {
    binary_reader::BinaryReadable, error::ParserError, imp::War3MapImp, mmp::War3MapMmp,
    w3i::War3MapW3i, w3x::War3MapW3x, wts::War3MapWts,
};
