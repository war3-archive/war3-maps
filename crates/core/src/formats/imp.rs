//! `war3map.imp` — imported files table.

use crate::error::Result;
use crate::reader::{parse_counted, ByteReader};

/// One imported file entry.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Import {
    /// Flag byte: WC3MapSpec uses 8 = standard, 13 = custom path;
    /// older tools write 0/1 or 10.
    pub is_custom: u8,
    /// Path as stored in the file (standard imports omit the
    /// `war3mapimported\` prefix).
    pub path: String,
}

impl Import {
    /// Whether the stored path is implicitly under `war3mapimported\`.
    pub fn is_standard_path(&self) -> bool {
        matches!(self.is_custom, 0 | 1 | 8)
    }

    /// Full in-archive path (standard imports gain the
    /// `war3mapimported\` prefix).
    pub fn resolved_path(&self) -> String {
        if self.is_standard_path() {
            format!("war3mapimported\\{}", self.path)
        } else {
            self.path.clone()
        }
    }
}

/// Import table parsed from `war3map.imp`, in file order.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct War3MapImp {
    pub version: u32,
    pub entries: Vec<Import>,
}

impl War3MapImp {
    /// Parse a complete `war3map.imp` buffer.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let r = &mut ByteReader::new(data);
        Ok(Self {
            version: r.u32()?,
            entries: parse_counted(r, |r| {
                Ok(Import {
                    is_custom: r.u8()?,
                    path: r.cstr_lossy()?,
                })
            })?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries_and_resolves_paths() {
        let mut data = Vec::new();
        data.extend(1u32.to_le_bytes()); // version
        data.extend(2u32.to_le_bytes()); // count
        data.push(8); // standard
        data.extend(b"icon.blp\0");
        data.push(13); // custom
        data.extend(b"units\\custom.mdx\0");

        let imp = War3MapImp::parse(&data).unwrap();
        assert_eq!(imp.version, 1);
        assert_eq!(imp.entries.len(), 2);
        assert_eq!(imp.entries[0].resolved_path(), "war3mapimported\\icon.blp");
        assert_eq!(imp.entries[1].resolved_path(), "units\\custom.mdx");
    }
}
