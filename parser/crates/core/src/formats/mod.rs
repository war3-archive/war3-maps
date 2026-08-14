//! Parsers for the individual file formats found inside a map archive.

pub mod imp;
pub mod mmp;
pub mod w3i;
pub mod wts;

#[doc(inline)]
pub use {
    imp::War3MapImp,
    mmp::{MinimapIcon, War3MapMmp},
    w3i::{FormatVersion, War3MapW3i},
    wts::War3MapWts,
};
