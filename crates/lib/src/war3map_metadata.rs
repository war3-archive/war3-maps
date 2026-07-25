//! Helper struct that includes all supported metadata of a map file

use std::path::Path;

use crate::parser::{
    error::ParserError, img::War3Image, imp::War3MapImp, w3i::War3MapW3i, w3x::War3MapW3x,
    wts::War3MapWts,
};

/// HM3W container header fields (absent on pure-MPQ / some protected maps)
#[cfg_attr(
    feature = "wasm",
    derive(tsify_next::Tsify),
    tsify(into_wasm_abi, from_wasm_abi)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct War3MapHeader {
    pub has_hm3w: bool,
    pub name: Option<String>,
    pub flags: Option<u32>,
    pub max_players: Option<u32>,
    pub u1: Option<u32>,
}

/// If a file exists and was extracted successfully, the field is `Some(..)`.
/// Otherwise it is `None`.
pub struct War3MapMetadata {
    pub header: War3MapHeader,
    pub map_info: Option<War3MapW3i>,
    pub imp: Option<War3MapImp>,
    pub wts: Option<War3MapWts>,
    pub images: Vec<War3Image>,
    pub files: Option<Vec<String>>,
}

impl War3MapMetadata {
    /// Load metadata from a buffer
    ///
    /// # Example
    ///
    /// ```ignore
    /// use war3parser::war3map_metadata::War3MapMetadata;
    ///
    /// let buffer = std::fs::read("path/to/map.w3x").unwrap();
    /// let metadata = War3MapMetadata::from(&buffer).unwrap();
    /// ```
    pub fn from(buffer: &[u8]) -> Option<Self> {
        let w3x = War3MapW3x::from_buffer(buffer).ok()?;
        let mut w3x = Box::new(w3x);
        let mut images = Vec::new();
        if let Ok(minimap) = w3x.read_minimap() {
            images.push(minimap);
        }
        if let Ok(preview) = w3x.read_preview() {
            images.push(preview);
        }

        Some(Self {
            header: War3MapHeader {
                has_hm3w: w3x.has_hm3w,
                name: w3x.name.clone(),
                flags: w3x.flags,
                max_players: w3x.max_players,
                u1: w3x.u1,
            },
            map_info: w3x.read_map_info().ok(),
            imp: w3x.read_imports().ok(),
            wts: w3x.read_string_table().ok(),
            images,
            files: w3x.files.clone(),
        })
    }

    /// Update trigger strings in `w3i` file if `wts` file is available
    pub fn update_string_table(&mut self) -> Result<(), ParserError> {
        let map_info = self
            .map_info
            .as_ref()
            .ok_or(ParserError::MapFileNotFound("w3i".to_string()))?;
        let mut map_info_json = serde_json::to_string_pretty(map_info)?;
        let trigger_string_map = map_info.trigger_string_map()?;

        let string_table = &self
            .wts
            .as_ref()
            .ok_or(ParserError::MapFileNotFound("wts".to_string()))?
            .string_map;

        let default = "Unknown".to_string();

        trigger_string_map.iter().try_for_each(
            |(key, value)| -> Result<(), serde_json::Error> {
                let replace_str = string_table.get(value).unwrap_or(&default);
                let replace_str = serde_json::to_string(replace_str)?;
                map_info_json = map_info_json.replace(key, replace_str.as_str());
                Ok(())
            },
        )?;

        self.map_info = Some(serde_json::from_str(&map_info_json)?);

        Ok(())
    }

    /// Save metadata to files
    pub fn save(&self, out_dir: &str) -> Result<(), ParserError> {
        let out_dir = Path::new(out_dir);
        std::fs::create_dir_all(out_dir)?;

        #[cfg(feature = "serde")]
        {
            let header_path = out_dir.join("header.json");
            std::fs::write(header_path, serde_json::to_string_pretty(&self.header)?)?;

            if let Some(map_info) = &self.map_info {
                let map_info_path = out_dir.join("war3map.w3i.json");
                std::fs::write(map_info_path, serde_json::to_string_pretty(map_info)?)?;
            }
            if let Some(wts) = &self.wts {
                let wts_path = out_dir.join("war3map.wts.json");
                std::fs::write(wts_path, serde_json::to_string_pretty(wts)?)?;
            }
            if let Some(imp) = &self.imp {
                let imp_path = out_dir.join("war3map.imp.json");
                std::fs::write(imp_path, serde_json::to_string_pretty(imp)?)?;
            }
            if let Some(files) = &self.files {
                let files_path = out_dir.join("listfile.json");
                std::fs::write(files_path, serde_json::to_string_pretty(files)?)?;
            }
        }

        #[cfg(not(feature = "serde"))]
        {
            return Err(ParserError::MapFileNotFound(
                "serde feature required for save()".to_string(),
            ));
        }

        for (index, image) in self.images.iter().enumerate() {
            let image_path = out_dir.join(format!("{}_{}.png", image.filename, index));
            image.data.save(image_path)?;
        }
        Ok(())
    }
}
