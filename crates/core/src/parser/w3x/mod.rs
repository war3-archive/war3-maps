use std::path::PathBuf;

use binary_reader::{BinaryReader, Endian};
use mpq::{Archive, File};

use crate::model::{War3Image, War3MapHeader};
use crate::parser::{
    binary_reader::{AutoReadable, BinaryReadable},
    error::ParserError,
    imp::War3MapImp,
    mmp::War3MapMmp,
    w3i::War3MapW3i,
    wts::War3MapWts,
};

/// Warcraft 3 map entry (optional HM3W header + embedded MPQ).
pub struct War3MapW3x {
    /// Optional `HM3W` prefix fields (shared type with metadata / WASM).
    pub header: War3MapHeader,
    /// MPQ archive
    pub archive: Archive,
    /// List of files in `(listfile)` when present
    pub files: Option<Vec<String>>,
}

impl BinaryReadable for War3MapW3x {
    fn load(stream: &mut BinaryReader, _version: u32) -> Result<Self, ParserError> {
        let magic: [u8; 4] = AutoReadable::read(stream)?;

        let header = if magic == [b'H', b'M', b'3', b'W'] {
            War3MapHeader {
                has_hm3w: true,
                u1: Some(AutoReadable::read(stream)?),
                name: Some(AutoReadable::read(stream)?),
                flags: Some(AutoReadable::read(stream)?),
                max_players: Some(AutoReadable::read(stream)?),
            }
        } else {
            // Pure MPQ / protected maps without HM3W
            War3MapHeader::default()
        };

        let mut archive = Archive::load(stream.data.clone())?;
        let files = Self::get_file_names(&mut archive).ok();
        Ok(Self {
            header,
            archive,
            files,
        })
    }
}

impl War3MapW3x {
    /// Load a map file from a path.
    pub fn new(path: PathBuf) -> Result<Self, ParserError> {
        let buffer = std::fs::read(path)?;
        Self::from_buffer(&buffer)
    }

    /// Load a map file from a buffer.
    pub fn from_buffer(buffer: &[u8]) -> Result<Self, ParserError> {
        let mut binary_reader = BinaryReader::from_u8(buffer);
        binary_reader.set_endian(Endian::Little);
        War3MapW3x::load(&mut binary_reader, 0)
    }

    /// Get list of files in `(listfile)`.
    pub fn get_file_names(archive: &mut Archive) -> Result<Vec<String>, ParserError> {
        let file = archive.open_file("(listfile)")?;
        let mut data = vec![0; file.size() as usize];
        file.read(archive, &mut data)?;
        let listfile = String::from_utf8_lossy(&data);
        Ok(listfile
            .split(['\r', '\n'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect())
    }

    /// Get a file from the MPQ archive.
    pub fn get(&mut self, filename: &str) -> Result<File, ParserError> {
        self.archive.open_file(filename).map_err(ParserError::from)
    }

    /// Check if a file exists in the MPQ archive.
    pub fn has(&mut self, filename: &str) -> bool {
        self.archive.open_file(filename).is_ok()
    }

    /// Read file bytes by name.
    pub fn read_file(&mut self, filename: &str) -> Result<Vec<u8>, ParserError> {
        let file = self.get(filename)?;
        let mut data = vec![0; file.size() as usize];
        file.read(&mut self.archive, &mut data)?;
        Ok(data)
    }

    /// Get the script file from the MPQ archive.
    pub fn get_script_file(&mut self) -> Option<File> {
        [
            "war3map.j",
            "scripts\\war3map.j",
            "war3map.lua",
            "scripts\\war3map.lua",
        ]
        .iter()
        .find_map(|&filename| self.get(filename).ok())
    }

    /// Read the `w3i` map info from the MPQ archive.
    pub fn read_map_info(&mut self) -> Result<War3MapW3i, ParserError> {
        let data = self.read_file("war3map.w3i")?;
        let mut reader = BinaryReader::from_vec(&data);
        reader.set_endian(Endian::Little);
        War3MapW3i::load(&mut reader, 0)
    }

    /// Read the `imp` map imports from the MPQ archive.
    pub fn read_imports(&mut self) -> Result<War3MapImp, ParserError> {
        let data = self.read_file("war3map.imp")?;
        let mut reader = BinaryReader::from_vec(&data);
        reader.set_endian(Endian::Little);
        War3MapImp::load(&mut reader, 0)
    }

    /// Read the `wts` string table from the MPQ archive.
    pub fn read_string_table(&mut self) -> Result<War3MapWts, ParserError> {
        let data = self.read_file("war3map.wts")?;
        let buffer = String::from_utf8_lossy(&data).into_owned();
        War3MapWts::load(&buffer)
    }

    /// Read the minimap from the MPQ archive.
    pub fn read_minimap(&mut self) -> Result<War3Image, ParserError> {
        let filename = [
            "war3mapMap.tga",
            "war3mapMap.blp",
            "war3mapmap.tga",
            "war3mapmap.blp",
        ]
        .iter()
        .find(|&&filename| self.has(filename))
        .ok_or_else(|| ParserError::MapFileNotFound("war3mapMap".to_string()))?;
        let data = self.read_file(filename)?;
        War3Image::from_buffer(&data, filename)
    }

    /// Read the preview image from the MPQ archive.
    pub fn read_preview(&mut self) -> Result<War3Image, ParserError> {
        let filename = [
            "war3mapPreview.tga",
            "war3mapPreview.blp",
            "war3mappreview.tga",
            "war3mappreview.blp",
        ]
        .iter()
        .find(|&&filename| self.has(filename))
        .ok_or_else(|| ParserError::MapFileNotFound("war3mapPreview".to_string()))?;
        let data = self.read_file(filename)?;
        War3Image::from_buffer(&data, filename)
    }

    /// Read minimap icons (`war3map.mmp`) — gold mines, houses, player starts.
    pub fn read_minimap_icons(&mut self) -> Result<War3MapMmp, ParserError> {
        let data = self.read_file("war3map.mmp")?;
        War3MapMmp::load_bytes(&data)
    }
}
