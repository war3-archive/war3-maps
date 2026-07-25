use std::collections::HashMap;

use binary_reader::BinaryReader;

use super::{
    binary_reader::{AutoReadable, BinaryReadable},
    error::ParserError,
};

/// Import entry
#[cfg_attr(
    feature = "wasm",
    derive(tsify_next::Tsify),
    tsify(into_wasm_abi, from_wasm_abi)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Import {
    /// Flag byte: WC3MapSpec uses 8=standard, 13=custom; older tools use 0/1 or 10
    pub is_custom: u8,
    pub path: String,
}

/// Import table
#[cfg_attr(
    feature = "wasm",
    derive(tsify_next::Tsify),
    tsify(into_wasm_abi, from_wasm_abi)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone)]
pub struct War3MapImp {
    pub version: u32,
    pub entries: HashMap<String, Import>,
}

impl Import {
    /// Whether this path should be treated as under `war3mapimported\`
    pub fn is_standard_path(&self) -> bool {
        matches!(self.is_custom, 0 | 1 | 8)
    }
}

impl BinaryReadable for Import {
    fn load(stream: &mut BinaryReader, _version: u32) -> Result<Self, ParserError> {
        Ok(Import {
            is_custom: AutoReadable::read(stream)?,
            path: AutoReadable::read(stream)?,
        })
    }
}

impl BinaryReadable for War3MapImp {
    fn load(stream: &mut BinaryReader, _version: u32) -> Result<Self, ParserError> {
        let version: u32 = AutoReadable::read(stream)?;
        let count: u32 = AutoReadable::read(stream)?;
        let mut entries = HashMap::new();
        for _ in 0..count {
            let import = Import::load(stream, version)?;
            let key = if import.is_standard_path() {
                format!("war3mapimported\\{}", import.path)
            } else {
                import.path.clone()
            };
            entries.insert(key, import);
        }
        Ok(War3MapImp { version, entries })
    }
}
