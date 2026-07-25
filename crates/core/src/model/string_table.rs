//! Portable string-table entry (sorted list form of `war3map.wts`).

use crate::api_type;
use crate::parser::wts::War3MapWts;

api_type! {
    /// Single WTS entry as exposed to CLI/WASM consumers.
    pub struct StringTableEntry {
        pub id: i32,
        pub value: String,
    }
}

impl War3MapWts {
    /// Flatten the string map into an id-sorted list for stable API output.
    pub fn entries_sorted(&self) -> Vec<StringTableEntry> {
        let mut entries: Vec<StringTableEntry> = self
            .string_map
            .iter()
            .map(|(&id, value)| StringTableEntry {
                id,
                value: value.clone(),
            })
            .collect();
        entries.sort_by_key(|e| e.id);
        entries
    }
}
