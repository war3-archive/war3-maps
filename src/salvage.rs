//! Reading an archive whose hash and block tables are unusable.
//!
//! A name-based read needs the hash table to find a file and the block table to
//! find its bytes. Protected Warcraft III maps routinely have both overwritten
//! with noise, and no key explains the region afterwards — the tables are gone,
//! not merely re-encrypted. See `docs/PROTECTED_MAPS.md` for how that was
//! established.
//!
//! The member data survives, and it is self-describing. MPQ lays members out
//! contiguously from the start of the archive, and a sector-based member begins
//! with its own sector offset table: `n + 1` little-endian `u32`s whose first
//! value is `4 * (n + 1)` and whose last value is the packed size. That is
//! enough to chain from one member to the next and read each one, consulting
//! neither table.
//!
//! What this cannot recover is names — those live only in the hash table, as
//! one-way hashes — so callers identify members by content. Measured against 200
//! healthy maps the walk reproduces the real block table exactly (same offsets,
//! same packed sizes, same order) in 99 cases and as a correct prefix in most of
//! the rest; it stops early on zero-length, single-unit or encrypted members,
//! whose sector table it cannot read. Treat the result as salvage, not as an
//! authoritative directory.

use crate::archive::Archive;
use crate::compression::decompress;
use byteorder::{ByteOrder, LittleEndian};
use std::io::{prelude::*, Error, ErrorKind, SeekFrom};

/// A sector offset table longer than this is not a member; Warcraft III's
/// largest sector count sits far below it, and the bound keeps a garbage first
/// dword from allocating.
const MAX_SECTOR_TABLE: u32 = 0x4000;

/// Ceiling on how many members the walk will follow.
///
/// Without it a large archive full of noise is a trap: garbage keeps looking
/// enough like a sector offset table to advance the walk by a few bytes at a
/// time, and each step allocates and validates an offset list. The largest
/// block table in the 10365-map corpus holds 848 entries, so this is far above
/// anything real while keeping a pathological file bounded.
const MAX_MEMBERS: usize = 4096;

/// A member located by walking the data region rather than by a table lookup.
#[derive(Debug, Clone)]
pub struct SalvagedMember {
    /// Offset of the member's data, relative to the MPQ header — the same base
    /// a block entry's `offset` uses.
    pub offset: u32,
    /// Bytes the member occupies on disk, from its own sector offset table.
    pub packed_size: u32,
    /// The member's sector offsets, relative to `offset`.
    pub sector_offsets: Vec<u32>,
}

impl SalvagedMember {
    /// Number of sectors the member is split into.
    pub fn sector_count(&self) -> usize {
        self.sector_offsets.len().saturating_sub(1)
    }
}

impl Archive {
    /// Walk the data region and return every member whose sector offset table
    /// can be followed, in archive order.
    ///
    /// The walk starts immediately after the header and stops at the first
    /// member it cannot make sense of, so a broken chain yields a prefix rather
    /// than an error.
    pub fn salvage_members(&self) -> Vec<SalvagedMember> {
        let buf = self.file.get_ref();
        let total = buf.len() as u64;
        let end = self.data_region_end(total);
        let mut members = Vec::new();
        let mut pos: u64 = 0x20;

        while let Some(member) = self.member_at(buf, pos, end) {
            pos += u64::from(member.packed_size);
            members.push(member);
            if members.len() >= MAX_MEMBERS {
                break;
            }
        }

        members
    }

    /// Decompress a salvaged member.
    ///
    /// Sectors are inflated by their own compression mask; a sector as long as
    /// the archive's sector size is stored verbatim, which is how MPQ marks an
    /// incompressible one.
    pub fn read_salvaged(&mut self, member: &SalvagedMember) -> Result<Vec<u8>, Error> {
        if self.sector_size == 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "MPQ sector size is zero",
            ));
        }

        let sector_size = self.sector_size as usize;
        let mut out: Vec<u8> = Vec::new();
        let mut raw: Vec<u8> = Vec::new();
        let mut scratch: Vec<u8> = vec![0; sector_size];

        for pair in member.sector_offsets.windows(2) {
            let (from, to) = (pair[0], pair[1]);
            // The walk already checked that offsets rise and stay within the
            // declared sector size, so this cannot underflow or over-allocate.
            let len = (to - from) as usize;
            if len == 0 {
                continue;
            }

            raw.resize(len, 0);
            self.file.seek(SeekFrom::Start(
                self.offset + u64::from(member.offset) + u64::from(from),
            ))?;
            self.file.read_exact(&mut raw)?;

            if len == sector_size {
                out.extend_from_slice(&raw);
                continue;
            }

            let written = decompress(&mut raw, &mut scratch)?;
            out.extend_from_slice(&scratch[..written]);
        }

        Ok(out)
    }

    /// Where the member data stops: the hash table when the header points
    /// somewhere plausible, the end of the file otherwise.
    fn data_region_end(&self, total: u64) -> u64 {
        let declared = self.offset + u64::from(self.header.hash_table_offset);
        if declared > self.offset + 0x20 && declared <= total {
            declared
        } else {
            total
        }
    }

    /// Parse a member's sector offset table at `pos`, or `None` if what is there
    /// cannot be one.
    fn member_at(&self, buf: &[u8], pos: u64, end: u64) -> Option<SalvagedMember> {
        let start = self.offset.checked_add(pos)?;
        if start.checked_add(8)? > end {
            return None;
        }

        let first = LittleEndian::read_u32(&buf[start as usize..]);
        if first < 8 || first % 4 != 0 || first > MAX_SECTOR_TABLE {
            return None;
        }
        let count = first / 4; // sectors + 1
        if start + u64::from(first) > end {
            return None;
        }

        let mut offsets: Vec<u32> = Vec::with_capacity(count as usize);
        for i in 0..count {
            let at = (start + u64::from(i) * 4) as usize;
            offsets.push(LittleEndian::read_u32(&buf[at..]));
        }

        // A sector table rises monotonically and no sector expands beyond the
        // archive's sector size; noise satisfies neither.
        for pair in offsets.windows(2) {
            if pair[1] < pair[0] || pair[1] - pair[0] > self.sector_size {
                return None;
            }
        }

        let packed_size = *offsets.last()?;
        if packed_size == 0 || start + u64::from(packed_size) > end {
            return None;
        }

        Some(SalvagedMember {
            offset: pos as u32,
            packed_size,
            sector_offsets: offsets,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypt::{encrypt, hash_string};
    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    /// Build a one-member archive: 32-byte header, one zlib member, then tables.
    fn archive_with_member(payload: &[u8], wreck_tables: bool) -> Vec<u8> {
        let sector_size: usize = 4096;
        let sectors: Vec<&[u8]> = payload.chunks(sector_size).collect();

        let mut bodies: Vec<Vec<u8>> = Vec::new();
        for sector in &sectors {
            let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
            enc.write_all(sector).unwrap();
            let mut body = vec![0x02u8];
            body.extend_from_slice(&enc.finish().unwrap());
            bodies.push(body);
        }

        let table_len = (sectors.len() + 1) * 4;
        let mut member: Vec<u8> = Vec::new();
        let mut at = table_len as u32;
        member.extend_from_slice(&at.to_le_bytes());
        for body in &bodies {
            at += body.len() as u32;
            member.extend_from_slice(&at.to_le_bytes());
        }
        for body in &bodies {
            member.extend_from_slice(body);
        }
        let packed = member.len() as u32;

        let hash_pos = 0x20 + packed;
        let block_pos = hash_pos + 16;

        let mut hash_raw = vec![0u8; 16];
        let name = "war3map.w3i";
        hash_raw[0..4].copy_from_slice(&hash_string(name, 0x100).to_le_bytes());
        hash_raw[4..8].copy_from_slice(&hash_string(name, 0x200).to_le_bytes());
        hash_raw[12..16].copy_from_slice(&0u32.to_le_bytes());
        encrypt(&mut hash_raw, hash_string("(hash table)", 0x300));

        let mut block_raw = vec![0u8; 16];
        block_raw[0..4].copy_from_slice(&0x20u32.to_le_bytes());
        block_raw[4..8].copy_from_slice(&packed.to_le_bytes());
        block_raw[8..12].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        block_raw[12..16].copy_from_slice(&0x8000_0200u32.to_le_bytes());
        encrypt(&mut block_raw, hash_string("(block table)", 0x300));

        if wreck_tables {
            // What a protector leaves behind: noise where the tables were.
            for (i, byte) in hash_raw.iter_mut().enumerate() {
                *byte = (i as u8).wrapping_mul(31).wrapping_add(7);
            }
            for (i, byte) in block_raw.iter_mut().enumerate() {
                *byte = (i as u8).wrapping_mul(17).wrapping_add(3);
            }
        }

        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"MPQ\x1a");
        out.extend_from_slice(&0x20u32.to_le_bytes());
        out.extend_from_slice(&(block_pos + 16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // format version
        out.extend_from_slice(&3u16.to_le_bytes()); // 512 << 3 == 4096
        out.extend_from_slice(&hash_pos.to_le_bytes());
        out.extend_from_slice(&block_pos.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        assert_eq!(out.len(), 0x20);
        out.extend_from_slice(&member);
        out.extend_from_slice(&hash_raw);
        out.extend_from_slice(&block_raw);
        out
    }

    #[test]
    fn walks_a_member_whose_tables_are_noise() {
        let payload: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
        let mut archive = Archive::load(archive_with_member(&payload, true)).unwrap();

        // The name-based path is dead: nothing resolves.
        assert!(archive.open_file("war3map.w3i").is_err());

        let members = archive.salvage_members();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].offset, 0x20);
        assert_eq!(members[0].sector_count(), 3);
        assert_eq!(archive.read_salvaged(&members[0]).unwrap(), payload);
    }

    #[test]
    fn salvage_agrees_with_the_real_block_entry() {
        let payload: Vec<u8> = (0..5_000u32).map(|i| (i % 97) as u8).collect();
        let mut archive = Archive::load(archive_with_member(&payload, false)).unwrap();

        let by_name = archive.open_file("war3map.w3i").unwrap();
        let mut expected = vec![0; by_name.size() as usize];
        by_name.read(&mut archive, &mut expected).unwrap();

        let members = archive.salvage_members();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].packed_size, by_name.packed_size());
        assert_eq!(archive.read_salvaged(&members[0]).unwrap(), expected);
    }

    /// Noise that keeps looking like a short sector table must not walk forever.
    #[test]
    fn the_walk_is_bounded() {
        // 0x08 0x00 0x00 0x00 reads as a one-sector member eight bytes long, so
        // a file full of it advances the walk eight bytes at a time.
        let mut archive = archive_with_member(b"payload", true);
        archive.truncate(0x20);
        for _ in 0..(MAX_MEMBERS * 2) {
            archive.extend_from_slice(&8u32.to_le_bytes());
            archive.extend_from_slice(&8u32.to_le_bytes());
        }
        // The data region runs to the end of the file, so the walk has no
        // table position to stop at either.
        let len = archive.len() as u32 - 16;
        archive[0x10..0x14].copy_from_slice(&len.to_le_bytes());

        let archive = Archive::load(archive).unwrap();
        assert_eq!(archive.salvage_members().len(), MAX_MEMBERS);
    }

    #[test]
    fn a_walk_that_cannot_start_yields_nothing() {
        let mut archive = archive_with_member(b"payload", true);
        // Encrypted members keep their sector table encrypted, which is what the
        // walk trips over on 76 maps in the corpus.
        for byte in archive[0x20..0x28].iter_mut() {
            *byte = 0xAB;
        }
        let archive = Archive::load(archive).unwrap();
        assert!(archive.salvage_members().is_empty());
    }
}
