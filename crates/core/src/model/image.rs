//! Map images: in-memory raster + portable PNG data-URL form.

use std::io::Write;

use base64::{engine::general_purpose, write::EncoderWriter};
use image::{codecs::tga::TgaDecoder, DynamicImage, ImageOutputFormat};
use image_blp::{convert::blp_to_image, parser::load_blp_from_buf};

use crate::api_type;
use crate::parser::error::ParserError;

/// Raster image held in memory (not directly serializable).
#[derive(Debug, Clone)]
pub struct War3Image {
    pub data: DynamicImage,
    pub filename: String,
}

api_type! {
    /// Portable image representation used by CLI dumps and WASM.
    pub struct War3ImageData {
        /// `data:image/png;base64,...`
        pub data_url: String,
        pub width: u32,
        pub height: u32,
        pub filename: String,
    }
}

impl War3Image {
    /// Load a BLP image from a buffer.
    pub fn load_blp<T: AsRef<[u8]>>(buffer: &T, filename: &str) -> Result<Self, ParserError> {
        let blp = load_blp_from_buf(buffer.as_ref())?;
        let image = blp_to_image(&blp, 0)?;
        Ok(Self {
            data: image,
            filename: filename.to_string(),
        })
    }

    /// Load a TGA image from a buffer.
    pub fn load_tga<T: AsRef<[u8]>>(buffer: &T, filename: &str) -> Result<Self, ParserError> {
        let cursor = std::io::Cursor::new(buffer);
        let decoder = TgaDecoder::new(cursor)?;
        let image = DynamicImage::from_decoder(decoder)?;

        Ok(Self {
            data: image,
            filename: filename.to_string(),
        })
    }

    /// Convert a raw binary buffer to a [`War3Image`].
    pub fn from_buffer(data: &[u8], filename: &str) -> Result<Self, ParserError> {
        if let Ok(blp) = Self::load_blp(&data, filename) {
            Ok(blp)
        } else if let Ok(tga) = Self::load_tga(&data, filename) {
            Ok(tga)
        } else {
            Err(ParserError::FailedToConvertBufferToImage)
        }
    }

    /// Encode as a portable PNG data-URL snapshot.
    pub fn to_data(&self) -> Result<War3ImageData, ParserError> {
        War3ImageData::try_from(self)
    }
}

impl TryFrom<&War3Image> for War3ImageData {
    type Error = ParserError;

    fn try_from(image: &War3Image) -> Result<Self, Self::Error> {
        let width = image.data.width();
        let height = image.data.height();
        let mut cursor = std::io::Cursor::new(Vec::new());
        image.data.write_to(&mut cursor, ImageOutputFormat::Png)?;
        let mut encoder = EncoderWriter::new(Vec::new(), &general_purpose::STANDARD);
        encoder.write_all(&cursor.into_inner())?;
        let encoded_data = encoder.finish()?;
        let encoded_data_str = String::from_utf8(encoded_data)?;
        Ok(Self {
            data_url: format!("data:image/png;base64,{encoded_data_str}"),
            width,
            height,
            filename: image.filename.clone(),
        })
    }
}

impl TryFrom<War3Image> for War3ImageData {
    type Error = ParserError;

    fn try_from(image: War3Image) -> Result<Self, Self::Error> {
        Self::try_from(&image)
    }
}

/// Backward-compatible alias — prefer [`War3ImageData`].
#[deprecated(note = "renamed to War3ImageData")]
pub type War3ImageBase64 = War3ImageData;
