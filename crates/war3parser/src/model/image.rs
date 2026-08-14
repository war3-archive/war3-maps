//! Map images: in-memory raster + portable PNG data-URL form.

use std::io::Write;

use base64::{engine::general_purpose, write::EncoderWriter};
use image::{codecs::tga::TgaDecoder, DynamicImage, ImageOutputFormat};
use image_blp::{convert::blp_to_image, parser::load_blp_from_buf};

use crate::error::{Error, Result};

/// Raster image held in memory (not directly serializable).
#[derive(Debug, Clone)]
pub struct War3Image {
    pub data: DynamicImage,
    pub filename: String,
}

/// Portable image representation used by CLI dumps and WASM.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct War3ImageData {
    /// `data:image/png;base64,...`
    pub data_url: String,
    pub width: u32,
    pub height: u32,
    pub filename: String,
}

impl War3Image {
    /// Decode a BLP buffer.
    pub fn load_blp(buffer: &[u8], filename: &str) -> Result<Self> {
        let blp = load_blp_from_buf(buffer)?;
        Ok(Self {
            data: blp_to_image(&blp, 0)?,
            filename: filename.to_string(),
        })
    }

    /// Decode a TGA buffer.
    pub fn load_tga(buffer: &[u8], filename: &str) -> Result<Self> {
        let decoder = TgaDecoder::new(std::io::Cursor::new(buffer))?;
        Ok(Self {
            data: DynamicImage::from_decoder(decoder)?,
            filename: filename.to_string(),
        })
    }

    /// Decode a buffer, trying BLP first, then TGA.
    pub fn from_buffer(data: &[u8], filename: &str) -> Result<Self> {
        Self::load_blp(data, filename)
            .or_else(|_| Self::load_tga(data, filename))
            .map_err(|_| Error::UnsupportedImage(filename.to_string()))
    }

    /// Encode as a portable PNG data-URL snapshot.
    pub fn to_data(&self) -> Result<War3ImageData> {
        War3ImageData::try_from(self)
    }
}

impl TryFrom<&War3Image> for War3ImageData {
    type Error = Error;

    fn try_from(image: &War3Image) -> Result<Self> {
        let mut png = std::io::Cursor::new(Vec::new());
        image.data.write_to(&mut png, ImageOutputFormat::Png)?;

        let mut encoder = EncoderWriter::new(Vec::new(), &general_purpose::STANDARD);
        encoder.write_all(&png.into_inner())?;
        let encoded = encoder.finish()?;
        let encoded = String::from_utf8(encoded).expect("base64 output is always ASCII");

        Ok(Self {
            data_url: format!("data:image/png;base64,{encoded}"),
            width: image.data.width(),
            height: image.data.height(),
            filename: image.filename.clone(),
        })
    }
}
