//! Map container: optional `HM3W` header followed by an embedded MPQ archive.

use std::path::Path;

use mpq::{Archive, File};

use crate::error::{Error, Result};
use crate::formats::{War3MapImp, War3MapMmp, War3MapW3i, War3MapWts};
use crate::model::{War3Image, War3MapHeader};

/// Candidate script file paths, in lookup order.
const SCRIPT_PATHS: &[&str] = &[
    "war3map.j",
    "scripts\\war3map.j",
    "war3map.lua",
    "scripts\\war3map.lua",
];

/// A Warcraft III map file (`.w3x` / `.w3m`): optional `HM3W` header plus the
/// embedded MPQ archive.
pub struct War3MapW3x {
    /// Fields of the optional `HM3W` prefix.
    pub header: War3MapHeader,
    /// The underlying MPQ archive.
    pub archive: Archive,
    /// Contents of `(listfile)` when present.
    pub files: Option<Vec<String>>,
}

impl War3MapW3x {
    /// Load a map from a file path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_vec(std::fs::read(path)?)
    }

    /// Load a map from a borrowed buffer (copies once for the MPQ archive).
    pub fn from_buffer(buffer: &[u8]) -> Result<Self> {
        Self::from_vec(buffer.to_vec())
    }

    /// Load a map from an owned buffer.
    ///
    /// Fails when the embedded MPQ cannot be opened. To read the `HM3W` fields
    /// of a map whose archive is unreadable, use [`War3MapHeader::from_buffer`]
    /// directly — it does not depend on the archive.
    pub fn from_vec(buffer: Vec<u8>) -> Result<Self> {
        let header = War3MapHeader::from_buffer(&buffer)?;
        let mut archive = Archive::load(buffer)?;
        let files = Self::read_listfile(&mut archive).ok();
        Ok(Self {
            header,
            archive,
            files,
        })
    }

    fn read_listfile(archive: &mut Archive) -> Result<Vec<String>> {
        let file = archive.open_file("(listfile)")?;
        let mut data = vec![0; file.size() as usize];
        file.read(archive, &mut data)?;
        let listfile = String::from_utf8_lossy(&data);
        Ok(listfile
            .split(['\r', '\n'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect())
    }

    /// Open a file inside the MPQ archive.
    pub fn get(&mut self, filename: &str) -> Result<File> {
        self.archive.open_file(filename).map_err(|error| {
            // Only a genuinely absent file is "not found". Flattening every
            // structural failure into that message hides what is actually
            // wrong — a corrupt block table reads as a missing `w3i`, which
            // sends triage looking for the file rather than for the damage.
            if error.kind() == std::io::ErrorKind::NotFound {
                Error::FileNotFound(filename.to_string())
            } else {
                Error::Io(error)
            }
        })
    }

    /// Whether a file exists inside the MPQ archive.
    pub fn has(&mut self, filename: &str) -> bool {
        self.archive.open_file(filename).is_ok()
    }

    /// Read a file's bytes by name.
    pub fn read_file(&mut self, filename: &str) -> Result<Vec<u8>> {
        let file = self.get(filename)?;
        let mut data = vec![0; file.size() as usize];
        file.read(&mut self.archive, &mut data)?;
        Ok(data)
    }

    /// The map script (`war3map.j` / `war3map.lua`), if any.
    pub fn get_script_file(&mut self) -> Option<File> {
        SCRIPT_PATHS.iter().find_map(|&path| self.get(path).ok())
    }

    /// Parse `war3map.w3i` map information.
    pub fn read_map_info(&mut self) -> Result<War3MapW3i> {
        War3MapW3i::parse(&self.read_file("war3map.w3i")?)
    }

    /// Parse the `war3map.imp` import table.
    pub fn read_imports(&mut self) -> Result<War3MapImp> {
        War3MapImp::parse(&self.read_file("war3map.imp")?)
    }

    /// Parse the `war3map.wts` string table.
    pub fn read_string_table(&mut self) -> Result<War3MapWts> {
        let data = self.read_file("war3map.wts")?;
        War3MapWts::parse(&String::from_utf8_lossy(&data))
    }

    /// Parse `war3map.mmp` minimap icons.
    pub fn read_minimap_icons(&mut self) -> Result<War3MapMmp> {
        War3MapMmp::parse(&self.read_file("war3map.mmp")?)
    }

    /// Decode the minimap image (`war3mapMap.tga` / `.blp`).
    pub fn read_minimap(&mut self) -> Result<War3Image> {
        self.read_image_variants(
            "war3mapMap",
            &[
                "war3mapMap.tga",
                "war3mapMap.blp",
                "war3mapmap.tga",
                "war3mapmap.blp",
            ],
        )
    }

    /// Decode the preview image (`war3mapPreview.tga` / `.blp`).
    pub fn read_preview(&mut self) -> Result<War3Image> {
        self.read_image_variants(
            "war3mapPreview",
            &[
                "war3mapPreview.tga",
                "war3mapPreview.blp",
                "war3mappreview.tga",
                "war3mappreview.blp",
            ],
        )
    }

    fn read_image_variants(&mut self, stem: &str, candidates: &[&str]) -> Result<War3Image> {
        let filename = candidates
            .iter()
            .find(|&&name| self.has(name))
            .ok_or_else(|| Error::FileNotFound(stem.to_string()))?;
        let data = self.read_file(filename)?;
        War3Image::from_buffer(&data, filename)
    }
}
