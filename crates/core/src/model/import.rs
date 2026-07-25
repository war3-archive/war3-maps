//! Portable import-table entry (sorted list form of `war3map.imp`).

use crate::api_type;
use crate::parser::imp::{Import, War3MapImp};

api_type! {
    /// Single import path as exposed to CLI/WASM consumers.
    pub struct ImportEntry {
        pub path: String,
        /// Flag byte: WC3MapSpec uses 8=standard, 13=custom; older tools use 0/1 or 10.
        pub is_custom: u8,
    }
}

impl From<&Import> for ImportEntry {
    fn from(import: &Import) -> Self {
        Self {
            path: import.path.clone(),
            is_custom: import.is_custom,
        }
    }
}

impl War3MapImp {
    /// Flatten entries into a path-sorted list for stable API output.
    ///
    /// Paths use the resolved keys (standard imports are prefixed with
    /// `war3mapimported\`).
    pub fn entries_sorted(&self) -> Vec<ImportEntry> {
        let mut entries: Vec<ImportEntry> = self
            .entries
            .iter()
            .map(|(path, import)| ImportEntry {
                path: path.clone(),
                is_custom: import.is_custom,
            })
            .collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        entries
    }
}
