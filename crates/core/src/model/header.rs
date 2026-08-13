//! `HM3W` container header (absent on bare-MPQ / protected maps).

use crate::error::Result;
use crate::reader::ByteReader;

/// Magic bytes of the optional map header.
pub const HM3W_MAGIC: &[u8; 4] = b"HM3W";

/// Fields of the optional `HM3W` prefix that precedes the embedded MPQ.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct War3MapHeader {
    /// Whether the `HM3W` magic was present.
    pub has_hm3w: bool,
    pub name: Option<String>,
    pub flags: Option<u32>,
    pub max_players: Option<u32>,
    /// Unknown field between the magic and the name.
    pub u1: Option<u32>,
}

impl War3MapHeader {
    /// Parse the `HM3W` prefix straight from a map's bytes.
    ///
    /// Deliberately independent of the embedded MPQ: these fields are stored in
    /// plaintext ahead of the archive, so a map whose archive cannot be opened
    /// at all — a protected one whose hash table is unreadable — still yields
    /// its name and player count here. A buffer without the magic (a bare MPQ,
    /// or a pre-TFT map) yields a default, absent header rather than an error.
    pub fn from_buffer(buffer: &[u8]) -> Result<Self> {
        let r = &mut ByteReader::new(buffer);
        let magic: [u8; 4] = r.bytes()?;
        if &magic != HM3W_MAGIC {
            return Ok(Self::default());
        }
        Ok(Self {
            has_hm3w: true,
            u1: Some(r.u32()?),
            name: Some(r.cstr_lossy()?),
            flags: Some(r.u32()?),
            max_players: Some(r.u32()?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hm3w(name: &[u8], max_players: u32) -> Vec<u8> {
        let mut buffer = Vec::from(*HM3W_MAGIC);
        buffer.extend_from_slice(&0u32.to_le_bytes());
        buffer.extend_from_slice(name);
        buffer.push(0);
        buffer.extend_from_slice(&1u32.to_le_bytes());
        buffer.extend_from_slice(&max_players.to_le_bytes());
        buffer
    }

    #[test]
    fn reads_name_and_players() {
        let header = War3MapHeader::from_buffer(&hm3w("攻守兼备TD".as_bytes(), 10)).unwrap();
        assert!(header.has_hm3w);
        assert_eq!(header.name.as_deref(), Some("攻守兼备TD"));
        assert_eq!(header.max_players, Some(10));
    }

    #[test]
    fn absent_magic_is_not_an_error() {
        let header = War3MapHeader::from_buffer(b"MPQ\x1a____").unwrap();
        assert!(!header.has_hm3w);
        assert_eq!(header.name, None);
    }

    /// The whole point of the standalone entry point: the archive behind the
    /// header is garbage, and the header still parses.
    #[test]
    fn reads_header_of_an_unopenable_archive() {
        let mut buffer = hm3w("守卫剑阁".as_bytes(), 8);
        buffer.resize(512, 0);
        buffer.extend_from_slice(&[0xAB; 4096]);
        let header = War3MapHeader::from_buffer(&buffer).unwrap();
        assert_eq!(header.name.as_deref(), Some("守卫剑阁"));
        assert_eq!(header.max_players, Some(8));
    }

    #[test]
    fn truncated_buffer_errors_instead_of_panicking() {
        assert!(War3MapHeader::from_buffer(b"HM").is_err());
    }
}
