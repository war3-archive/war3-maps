//! Finding member data in an archive whose tables *and* member chain are gone.
//!
//! [`crate::salvage`] walks the data region member by member, which needs every
//! sector offset table along the way to be readable. When that chain breaks —
//! an encrypted member, a member overwritten in place — what is left is the
//! sector data itself, and it is recognisable on sight: an MPQ sector is a
//! one-byte compression mask followed by the compressed stream, so a
//! zlib-compressed sector appears in the file literally as `02 78 9C`.
//!
//! Scanning for that shape recovers members with no structural information at
//! all. It is strictly the last resort: it cannot say which sectors are *live*
//! (a re-saved map leaves orphaned copies of older members behind), it cannot
//! name what it finds, and inflating arbitrary bytes occasionally succeeds. The
//! caller has to judge each candidate on its content.
//!
//! Only zlib is scanned for. The other MPQ compressions are not self-delimiting
//! in a way that lets a scanner learn how many bytes a stream consumed, and
//! consumption is what makes chaining possible — an MPQ member larger than one
//! sector is a *run* of adjacent streams, each deflated on its own, so only the
//! first sector of a long member carries anything recognisable.

use flate2::read::ZlibDecoder;
use std::io::Read;

/// Compression mask for zlib, the method Warcraft III's editor uses.
const MASK_ZLIB: u8 = 0x02;

/// Plausible zlib FLG bytes for deflate with a 32K window.
///
/// FCHECK makes the `CMF FLG` pair divisible by 31, and `0x78` leaves a
/// remainder of 30, so a valid FLG is `1 mod 31`. Of those, the values without
/// the FDICT bit — the only ones a plain deflate stream uses — are exactly these
/// four, one per compression level.
const ZLIB_FLG: [u8; 4] = [0x01, 0x5E, 0x9C, 0xDA];

/// Upper bound on one carved member, so a crafted stream cannot inflate into a
/// decompression bomb. Comfortably above any Warcraft III metadata file.
pub const MAX_CARVED: u64 = 8 << 20;

/// Whether a zlib-compressed MPQ sector could begin at `at`.
pub fn starts_sector(buffer: &[u8], at: usize) -> bool {
    matches!(buffer.get(at..at + 3), Some([MASK_ZLIB, 0x78, flg]) if ZLIB_FLG.contains(flg))
}

/// Every offset that could begin a zlib-compressed MPQ sector.
pub fn sector_offsets(buffer: &[u8]) -> impl Iterator<Item = usize> + '_ {
    (0..buffer.len().saturating_sub(2)).filter(move |&at| starts_sector(buffer, at))
}

/// Inflate the sector at `offset`, skipping its compression mask byte.
///
/// Returns the inflated bytes and how many compressed bytes the stream
/// consumed, which is what lets a caller step to the sector behind it.
pub fn inflate_sector(buffer: &[u8], offset: usize) -> Option<(Vec<u8>, usize)> {
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
pub fn inflate_sector_chain(buffer: &[u8], offset: usize) -> (Vec<u8>, usize) {
    let mut joined = Vec::new();
    let mut at = offset;
    while starts_sector(buffer, at) {
        let Some((data, consumed)) = inflate_sector(buffer, at) else {
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

/// Every sector in the buffer that inflates, as `(offset, data)`.
///
/// Each sector stands alone here; use [`inflate_sector_chain`] on an offset
/// whose content suggests a member that continues past one sector.
pub fn carve_sectors(buffer: &[u8]) -> impl Iterator<Item = (usize, Vec<u8>)> + '_ {
    sector_offsets(buffer).filter_map(move |offset| {
        inflate_sector(buffer, offset).map(|(data, _consumed)| (offset, data))
    })
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

    #[test]
    fn finds_a_sector_between_noise() {
        let mut archive = vec![0xAB; 4096];
        let at = archive.len();
        archive.extend(sector(b"war3map payload"));
        archive.extend(vec![0xCD; 4096]);

        let found: Vec<_> = carve_sectors(&archive).collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, at);
        assert_eq!(found[0].1, b"war3map payload");
    }

    #[test]
    fn chains_the_sectors_of_one_member() {
        let mut archive = vec![0xAB; 16];
        let at = archive.len();
        archive.extend(sector(b"first sector, "));
        archive.extend(sector(b"second sector"));
        archive.extend(vec![0xCD; 16]);

        let (joined, end) = inflate_sector_chain(&archive, at);
        assert_eq!(joined, b"first sector, second sector");
        assert_eq!(end, archive.len() - 16);
    }

    #[test]
    fn a_chain_stops_at_the_first_non_sector() {
        let archive = sector(b"only me");
        let (joined, end) = inflate_sector_chain(&archive, 0);
        assert_eq!(joined, b"only me");
        assert_eq!(end, archive.len());
    }

    #[test]
    fn noise_carves_nothing() {
        let archive = vec![0xAB; 8192];
        assert_eq!(carve_sectors(&archive).count(), 0);
    }
}
