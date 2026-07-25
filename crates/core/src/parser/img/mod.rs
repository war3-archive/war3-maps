//! Image types live in [`crate::model`]; this module re-exports them for
//! backward-compatible `parser::img::*` paths.

pub use crate::model::{War3Image, War3ImageData};

#[allow(deprecated)]
pub use crate::model::War3ImageBase64;
