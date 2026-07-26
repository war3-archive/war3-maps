//! Crate-wide error type.

use thiserror::Error;

/// Errors produced while parsing map files or converting assets.
#[derive(Debug, Error)]
pub enum Error {
    /// The input ended before a field could be fully read.
    #[error("unexpected end of data at offset {offset} (needed {needed} more byte(s))")]
    UnexpectedEof {
        /// Byte offset where the read started.
        offset: usize,
        /// Number of bytes that were still required.
        needed: usize,
    },

    /// The input violated a structural invariant of the format.
    #[error("invalid data: {0}")]
    InvalidData(&'static str),

    /// A file was not found inside the MPQ archive.
    #[error("file `{0}` not found in archive")]
    FileNotFound(String),

    /// A buffer could not be decoded as any supported image format.
    #[error("failed to decode `{0}` as BLP or TGA")]
    UnsupportedImage(String),

    /// Raster decode/encode error from the `image` crate.
    #[error(transparent)]
    Image(#[from] image::ImageError),

    /// BLP parsing error.
    #[error(transparent)]
    BlpParse(#[from] image_blp::parser::LoadError),

    /// BLP-to-raster conversion error.
    #[error(transparent)]
    BlpConvert(#[from] image_blp::convert::Error),

    /// I/O error (also covers MPQ archive access).
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// The operation needs a crate feature that is disabled.
    #[error("feature `{0}` is required for this operation")]
    FeatureRequired(&'static str),

    /// JSON serialization error.
    #[cfg(feature = "serde")]
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Convenience alias used throughout the crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;
