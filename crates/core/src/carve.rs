//! Metadata recovery for archives whose MPQ tables cannot be read.
//!
//! Opening a file by name needs the hash table, and finding its bytes needs the
//! block table. Some protected maps have both overwritten with noise, which
//! leaves the archive permanently opaque to any name-based reader — the
//! filename hashes are one-way, so there is nothing to reconstruct them from.
//!
//! The sector *data* is usually untouched, though. An MPQ sector is a one-byte
//! compression mask followed by the compressed stream, so a zlib-compressed
//! member appears in the file literally as `02 78 ..`. Scanning for that and
//! validating whatever inflates recovers the metadata without consulting either
//! table.
//!
//! A `war3map.w3i` fits in a single sector, but a `war3map.wts` routinely does
//! not: MPQ splits a member into fixed-size sectors that are each deflated on
//! their own, so a large string table is a *run* of adjacent streams and only
//! its first few dozen entries are visible in the first one. Table candidates
//! are therefore inflated as a chain — see [`inflate_chain`].
//!
//! This is deliberately a separate entry point rather than a fallback inside
//! [`crate::archive::War3MapW3x`]: it is a salvage operation on a broken file,
//! its results are not authoritative the way a real archive read is, and
//! callers should be able to record that difference. Nothing here says which
//! sectors are *live*, either — a map that was re-saved can leave an orphaned
//! copy of an older `w3i` behind, and carving cannot tell the two apart.

use flate2::read::ZlibDecoder;
use std::io::Read;

use crate::formats::{War3MapW3i, War3MapWts};

/// Compression mask for zlib, the method Warcraft III's editor uses.
const MASK_ZLIB: u8 = 0x02;

/// Plausible zlib FLG bytes for deflate with a 32K window.
///
/// FCHECK makes the `CMF FLG` pair divisible by 31, and `0x78` leaves a
/// remainder of 30, so a valid FLG is `1 mod 31`. Of those, the values without
/// the FDICT bit — the only ones a plain deflate stream uses — are exactly these
/// four, one per compression level.
const ZLIB_FLG: [u8; 4] = [0x01, 0x5E, 0x9C, 0xDA];

/// Upper bound on one carved member. A `w3i` is a few KB; a string table can be
/// larger, but nothing here justifies inflating a decompression bomb.
const MAX_CARVED: u64 = 8 << 20;

/// The `war3map.wts` record keyword, used to skip sectors cheaply before paying
/// for a lossy UTF-8 conversion.
const WTS_KEYWORD: &[u8] = b"STRING ";

/// What could be salvaged from an unreadable archive.
#[derive(Debug)]
pub struct Carved {
    /// The recovered map information.
    pub info: War3MapW3i,
    /// The string table, when one was also recoverable. Maps whose `w3i` holds
    /// `TRIGSTR_*` references need it to resolve them.
    pub strings: Option<War3MapWts>,
}

/// Whether a parsed `w3i` looks like a real one rather than a coincidence.
///
/// Inflating arbitrary sector data occasionally succeeds, and the parser is
/// deliberately lenient about unknown versions, so the result has to be judged
/// on its content: a genuine map declares a version the format ladder knows and
/// carries a printable title.
fn plausible(info: &War3MapW3i) -> bool {
    info.version.is_known()
        && !info.name.trim().is_empty()
        && !info.name.chars().any(char::is_control)
}

/// Whether a zlib-compressed MPQ sector could begin at `at`.
fn starts_sector(buffer: &[u8], at: usize) -> bool {
    matches!(buffer.get(at..at + 3), Some([MASK_ZLIB, 0x78, flg]) if ZLIB_FLG.contains(flg))
}

/// Every offset that could begin a zlib-compressed MPQ sector.
fn sector_offsets(buffer: &[u8]) -> impl Iterator<Item = usize> + '_ {
    (0..buffer.len().saturating_sub(2)).filter(|&at| starts_sector(buffer, at))
}

/// Inflate the sector at `offset`, skipping its compression mask byte, and
/// report how many compressed bytes the stream consumed.
fn inflate(buffer: &[u8], offset: usize) -> Option<(Vec<u8>, usize)> {
    let mut out = Vec::new();
    let mut decoder = ZlibDecoder::new(&buffer[offset + 1..]);
    (&mut decoder).take(MAX_CARVED).read_to_end(&mut out).ok()?;
    let consumed = decoder.total_in() as usize;
    (!out.is_empty()).then_some((out, consumed))
}

/// Inflate the sector at `offset` together with every sector that directly
/// follows it, which is how MPQ stores a member too large for one sector.
///
/// Returns the offset just past the run as well, so a caller walking the file
/// can skip the sectors already folded into this member.
fn inflate_chain(buffer: &[u8], offset: usize) -> (Vec<u8>, usize) {
    let mut joined = Vec::new();
    let mut at = offset;
    while starts_sector(buffer, at) {
        let Some((data, consumed)) = inflate(buffer, at) else {
            break;
        };
        joined.extend_from_slice(&data);
        at += 1 + consumed;
        if joined.len() as u64 >= MAX_CARVED {
            break;
        }
    }
    (joined, at)
}

/// Parse a string table out of carved bytes, rejecting the empty result.
///
/// `War3MapWts::parse` accepts anything, so an unrelated sector that merely
/// mentions `STRING ` yields a table with no entries. Taking that at face value
/// would shadow the real table later in the file.
fn parse_table(data: &[u8]) -> Option<War3MapWts> {
    let text = String::from_utf8_lossy(data);
    let table = War3MapWts::parse(&text).ok()?;
    (!table.string_map.is_empty()).then_some(table)
}

/// Recover map metadata by scanning raw sector data.
///
/// Returns `None` when nothing that parses as a `w3i` is present — which is
/// itself informative: it means the file's payload is transformed too, not just
/// its tables.
pub fn carve(buffer: &[u8]) -> Option<Carved> {
    let mut info: Option<War3MapW3i> = None;
    let mut strings: Option<War3MapWts> = None;
    // End of the last sector run folded into a string table, so its inner
    // sectors are not chained all over again.
    let mut table_scanned_until = 0;

    for offset in sector_offsets(buffer) {
        let Some((data, _)) = inflate(buffer, offset) else {
            continue;
        };

        if info.is_none() {
            if let Ok(candidate) = War3MapW3i::parse(&data) {
                if plausible(&candidate) {
                    info = Some(candidate);
                    continue;
                }
            }
        }

        if offset < table_scanned_until
            || !data.windows(WTS_KEYWORD.len()).any(|w| w == WTS_KEYWORD)
        {
            continue;
        }

        // The keyword is here, so this may be the head of a multi-sector table.
        // Keep the richest candidate rather than the first: protected maps carry
        // leftovers and imported tables alongside the real one.
        let (joined, end) = inflate_chain(buffer, offset);
        table_scanned_until = end;
        if let Some(table) = parse_table(&joined) {
            let best = strings.as_ref().map_or(0, |t| t.string_map.len());
            if table.string_map.len() > best {
                strings = Some(table);
            }
        }
    }

    info.map(|info| Carved { info, strings })
}

impl Carved {
    /// Resolve the `TRIGSTR_*` references in the recovered metadata in place.
    pub fn resolve_trigger_strings(&mut self) {
        let Some(strings) = self.strings.as_ref() else {
            return;
        };
        self.info.visit_strings(|text| {
            if let Some(value) = strings.resolve(text) {
                *text = value.to_string();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn sector(payload: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(payload).unwrap();
        let mut out = vec![MASK_ZLIB];
        out.extend(encoder.finish().unwrap());
        out
    }

    /// A minimal v25 w3i: version, saves, editor version, then the four strings
    /// and enough trailing space for the fields the validator reaches past.
    fn w3i(name: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend(25u32.to_le_bytes());
        buf.extend(0u32.to_le_bytes());
        buf.extend(0u32.to_le_bytes());
        for field in [name, "An Author", "", ""] {
            buf.extend(field.as_bytes());
            buf.push(0);
        }
        buf.resize(buf.len() + 256, 0);
        buf
    }

    fn wts_entry(id: u32, value: &str) -> String {
        format!("STRING {id}\n{{\n{value}\n}}\n")
    }

    #[test]
    fn carves_a_w3i_out_of_surrounding_noise() {
        let mut archive = vec![0xAB; 4096];
        archive.extend(sector(&w3i("守卫剑阁")));
        archive.extend(vec![0xCD; 4096]);

        let carved = carve(&archive).expect("w3i should be carved");
        assert_eq!(carved.info.name, "守卫剑阁");
        assert_eq!(carved.info.author, "An Author");
    }

    #[test]
    fn resolves_trigger_strings_from_a_carved_string_table() {
        let mut archive = sector(&w3i("TRIGSTR_001"));
        archive.extend(sector(b"STRING 1\n{\n\xe6\x94\xbb\xe5\xae\x88\n}\n"));

        let mut carved = carve(&archive).expect("w3i should be carved");
        assert_eq!(carved.info.name, "TRIGSTR_001");
        carved.resolve_trigger_strings();
        assert_eq!(carved.info.name, "攻守");
    }

    /// A real `wts` spans several sectors; only the first carries the keyword,
    /// so the rest have to be pulled in behind it.
    #[test]
    fn joins_a_string_table_split_across_sectors() {
        let mut archive = sector(&w3i("TRIGSTR_900"));
        archive.extend(sector(wts_entry(1, "first sector").as_bytes()));
        archive.extend(sector(wts_entry(900, "跨扇区的标题").as_bytes()));

        let carved = carve(&archive).expect("w3i should be carved");
        let strings = carved.strings.as_ref().expect("table should be carved");
        assert_eq!(strings.get(1), Some("first sector"));
        assert_eq!(strings.get(900), Some("跨扇区的标题"));
    }

    /// An unrelated sector that merely mentions the keyword must not shadow the
    /// real table that follows it.
    #[test]
    fn prefers_the_richest_string_table() {
        let mut archive = sector(&w3i("TRIGSTR_002"));
        archive.extend(vec![0xAB; 16]);
        archive.extend(sector(b"call BJDebugMsg(\"STRING \")"));
        archive.extend(vec![0xAB; 16]);
        archive.extend(sector(
            format!("{}{}", wts_entry(1, "one"), wts_entry(2, "two")).as_bytes(),
        ));

        let mut carved = carve(&archive).expect("w3i should be carved");
        carved.resolve_trigger_strings();
        assert_eq!(carved.info.name, "two");
    }

    #[test]
    fn a_payload_with_no_w3i_yields_nothing() {
        let mut archive = vec![0xAB; 2048];
        archive.extend(sector(b"not a w3i, just bytes"));
        assert!(carve(&archive).is_none());
    }

    /// Inflating arbitrary data can succeed; only content decides.
    #[test]
    fn rejects_an_inflatable_sector_that_is_not_a_w3i() {
        let mut bogus = Vec::new();
        bogus.extend(99u32.to_le_bytes()); // version outside the known ladder
        bogus.extend(b"\0\0\0\0");
        bogus.resize(512, 0);
        assert!(carve(&sector(&bogus)).is_none());
    }
}
