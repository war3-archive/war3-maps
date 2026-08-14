//! Metadata recovery for archives whose MPQ tables cannot be read.
//!
//! Opening a file by name needs the hash table, and finding its bytes needs the
//! block table. Some protected maps have both overwritten with noise, which
//! leaves the archive permanently opaque to any name-based reader — the
//! filename hashes are one-way, so there is nothing to reconstruct them from.
//!
//! The sector *data* is usually untouched, though, and `mpq::carve` finds it by
//! shape: an MPQ sector is a one-byte compression mask followed by the
//! compressed stream, so a zlib-compressed member appears in the file literally
//! as `02 78 ..`. What is left here is the Warcraft III half — deciding whether
//! what inflated is a real `w3i` or a coincidence.
//!
//! A `war3map.w3i` fits in a single sector, but a `war3map.wts` routinely does
//! not: MPQ splits a member into fixed-size sectors that are each deflated on
//! their own, so a large string table is a *run* of adjacent streams and only
//! its first few dozen entries are visible in the first one. Table candidates
//! are therefore inflated as a chain — see [`mpq::carve::inflate_sector_chain`].
//!
//! This is deliberately a separate entry point rather than a fallback inside
//! [`crate::archive::War3MapW3x`]: it is a salvage operation on a broken file,
//! its results are not authoritative the way a real archive read is, and
//! callers should be able to record that difference. Nothing here says which
//! sectors are *live*, either — a map that was re-saved can leave an orphaned
//! copy of an older `w3i` behind, and carving cannot tell the two apart.

use mpq::carve::{carve_sectors, inflate_sector_chain};
use mpq::Archive;

use crate::formats::{War3MapW3i, War3MapWts};

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

/// Recover map metadata from an archive that cannot be read by name.
///
/// Tries the structured route first: MPQ lays members out contiguously and each
/// one begins with its own sector offset table, so `war3-mpq` can walk the data
/// region and hand back real members even with both tables destroyed. That is
/// worth preferring — a member is a whole file, where a scan sees loose sectors
/// and cannot tell a live one from a copy an earlier save left behind.
///
/// This route is cheap: it reads the members the archive points at and stops as
/// soon as it has what it came for. The exhaustive scan is deliberately *not*
/// part of it — see [`carve_deep`], which a caller has to ask for.
///
/// Returns `None` when the walk finds no `w3i`, which happens when the member
/// chain is broken as well — an encrypted member stops it dead, since its
/// sector offset table is encrypted along with the rest.
pub fn carve(buffer: &[u8]) -> Option<Carved> {
    carve_members(buffer)
}

/// [`carve`], and if that finds nothing, scan every byte of the archive for the
/// shape of a compressed sector.
///
/// The scan is exhaustive by nature: it tries to inflate at every offset that
/// could begin a zlib sector, over the whole file. That is the only thing left
/// when the member chain is unreadable, and it recovers maps nothing else
/// reaches, but it costs a full pass per archive — so it belongs behind an
/// explicit call rather than in the default path.
pub fn carve_deep(buffer: &[u8]) -> Option<Carved> {
    carve(buffer).or_else(|| carve_scan(buffer))
}

/// Largest member worth decompressing here. A `w3i` is a few KB and a string
/// table a few hundred; the members above this bound are scripts, terrain and
/// textures.
const MAX_INTERESTING: u32 = 1 << 21;

/// `war3map.w3i` opens with its format version, and the ladder is short. One
/// sector is enough to recognise the number and skip everything that is not it.
const W3I_VERSIONS: [u32; 8] = [8, 10, 11, 15, 18, 23, 25, 28];

/// Recover metadata from the members `war3-mpq` can walk to.
fn carve_members(buffer: &[u8]) -> Option<Carved> {
    let mut archive = Archive::load(buffer.to_vec()).ok()?;
    let members = archive.salvage_members();

    let mut info: Option<War3MapW3i> = None;
    let mut strings: Option<War3MapWts> = None;

    for member in &members {
        if info.is_some() && strings.is_some() {
            break;
        }
        if member.packed_size > MAX_INTERESTING {
            continue;
        }

        // Sniff the head first. Inflating every member in full — scripts,
        // terrain, textures — to look at its opening bytes is what made a carve
        // cost seconds per map. A wts announces itself within its first line,
        // so half a kilobyte covers both checks.
        let Ok(head) = archive.peek_salvaged(member, 512) else {
            continue;
        };

        let looks_like_w3i = info.is_none()
            && head.len() >= 4
            && W3I_VERSIONS.contains(&u32::from_le_bytes([head[0], head[1], head[2], head[3]]));
        let looks_like_wts =
            strings.is_none() && head.windows(WTS_KEYWORD.len()).any(|w| w == WTS_KEYWORD);
        if !looks_like_w3i && !looks_like_wts {
            continue;
        }

        // Only now is the whole member worth having: a w3i can outgrow one
        // sector, and a string table nearly always does.
        let Ok(data) = archive.read_salvaged(member) else {
            continue;
        };

        if looks_like_w3i {
            if let Ok(candidate) = War3MapW3i::parse(&data) {
                if plausible(&candidate) {
                    info = Some(candidate);
                    continue;
                }
            }
        }

        // Unlike the scan, a member is already whole, so the richest-wins rule
        // is not needed: a table that parses here is the table.
        if looks_like_wts {
            strings = parse_table(&data);
        }
    }

    info.map(|info| Carved { info, strings })
}

/// Recover map metadata by scanning raw sector data.
fn carve_scan(buffer: &[u8]) -> Option<Carved> {
    let mut info: Option<War3MapW3i> = None;
    let mut strings: Option<War3MapWts> = None;
    // End of the last sector run folded into a string table, so its inner
    // sectors are not chained all over again.
    let mut table_scanned_until = 0;

    for (offset, data) in carve_sectors(buffer) {
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
        let (joined, end) = inflate_sector_chain(buffer, offset);
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
        let mut out = vec![0x02];
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

        let carved = carve_deep(&archive).expect("w3i should be carved");
        assert_eq!(carved.info.name, "守卫剑阁");
        assert_eq!(carved.info.author, "An Author");
    }

    #[test]
    fn resolves_trigger_strings_from_a_carved_string_table() {
        let mut archive = sector(&w3i("TRIGSTR_001"));
        archive.extend(sector(b"STRING 1\n{\n\xe6\x94\xbb\xe5\xae\x88\n}\n"));

        let mut carved = carve_deep(&archive).expect("w3i should be carved");
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

        let carved = carve_deep(&archive).expect("w3i should be carved");
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

        let mut carved = carve_deep(&archive).expect("w3i should be carved");
        carved.resolve_trigger_strings();
        assert_eq!(carved.info.name, "two");
    }

    #[test]
    fn a_payload_with_no_w3i_yields_nothing() {
        let mut archive = vec![0xAB; 2048];
        archive.extend(sector(b"not a w3i, just bytes"));
        assert!(carve_deep(&archive).is_none());
    }

    /// Inflating arbitrary data can succeed; only content decides.
    #[test]
    fn rejects_an_inflatable_sector_that_is_not_a_w3i() {
        let mut bogus = Vec::new();
        bogus.extend(99u32.to_le_bytes()); // version outside the known ladder
        bogus.extend(b"\0\0\0\0");
        bogus.resize(512, 0);
        assert!(carve_deep(&sector(&bogus)).is_none());
    }
}
