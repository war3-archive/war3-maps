use std::io::Write;

use super::ParserError;
use base64::{engine::general_purpose, write::EncoderWriter};
use image::{codecs::tga::TgaDecoder, DynamicImage, ImageOutputFormat};
use image_blp::{convert::blp_to_image, parser::load_blp_from_buf};

#[derive(Debug, Clone)]
pub struct War3Image {
    pub data: DynamicImage,
    pub filename: String,
}

#[cfg_attr(
    feature = "wasm",
    derive(tsify_next::Tsify),
    tsify(into_wasm_abi, from_wasm_abi)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct War3ImageBase64 {
    pub data: String,
    pub filename: String,
}

impl War3Image {
    /// Load a BLP image from a buffer
    pub fn load_blp<T: AsRef<[u8]>>(buffer: &T, filename: &str) -> Result<Self, ParserError> {
        let blp = load_blp_from_buf(buffer.as_ref())?;
        let image = blp_to_image(&blp, 0)?;
        Ok(Self {
            data: image,
            filename: filename.to_string(),
        })
    }

    /// Load a TGA image from a buffer
    pub fn load_tga<T: AsRef<[u8]>>(buffer: &T, filename: &str) -> Result<Self, ParserError> {
        let cursor = std::io::Cursor::new(buffer);
        let decoder = TgaDecoder::new(cursor)?;
        let image = DynamicImage::from_decoder(decoder)?;

        Ok(Self {
            data: image,
            filename: filename.to_string(),
        })
    }

    /// Convert a raw binary buffer to an [`War3Image`]
    pub fn from_buffer(data: &[u8], filename: &str) -> Result<Self, ParserError> {
        if let Ok(blp) = Self::load_blp(&data, filename) {
            Ok(blp)
        } else if let Ok(tga) = Self::load_tga(&data, filename) {
            Ok(tga)
        } else {
            Err(ParserError::FailedToConvertBufferToImage)
        }
    }
}

impl TryFrom<War3Image> for War3ImageBase64 {
    type Error = ParserError;

    fn try_from(image: War3Image) -> Result<Self, Self::Error> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        image.data.write_to(&mut cursor, ImageOutputFormat::Png)?;
        let mut encoder = EncoderWriter::new(Vec::new(), &general_purpose::STANDARD);
        encoder.write_all(&cursor.into_inner())?;
        let encoded_data = encoder.finish()?;
        let encoded_data_str = String::from_utf8(encoded_data.to_vec())?;
        Ok(Self {
            data: format!("data:image/png;base64,{}", encoded_data_str),
            filename: image.filename,
        })
    }
}
