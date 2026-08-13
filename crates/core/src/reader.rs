//! Zero-copy little-endian cursor over a byte slice.
//!
//! All Warcraft III map formats are little-endian. Every read is
//! bounds-checked and returns [`Error::UnexpectedEof`] instead of panicking,
//! which matters for protected / truncated maps.

use crate::error::{Error, Result};

/// Little-endian cursor over borrowed bytes.
#[derive(Debug, Clone)]
pub struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize,
    empty_strings_unterminated: bool,
}

impl<'a> ByteReader<'a> {
    /// Create a reader over `data`, positioned at the start.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            empty_strings_unterminated: false,
        }
    }

    /// Read an empty string as zero bytes rather than as a lone terminator.
    ///
    /// Some third-party map optimizers emit nothing at all for an empty string
    /// instead of the NUL that terminates it, which shifts every field after
    /// it. The two encodings are locally indistinguishable — a zero byte is
    /// either the terminator or the first byte of whatever follows — so this is
    /// only ever a retry after the format-conformant read has already failed.
    pub fn set_empty_strings_unterminated(&mut self, unterminated: bool) {
        self.empty_strings_unterminated = unterminated;
    }

    /// Current byte offset from the start of the input.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Number of unread bytes.
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Whether the reader is exhausted.
    pub fn is_at_end(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Look at the next byte without consuming it.
    pub fn peek_u8(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    /// Consume `n` bytes, returning them as a slice.
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.remaining() < n {
            return Err(Error::UnexpectedEof {
                offset: self.pos,
                needed: n - self.remaining(),
            });
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    /// Skip `n` bytes.
    pub fn skip(&mut self, n: usize) -> Result<()> {
        self.take(n).map(|_| ())
    }

    /// Read a single byte.
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    /// Read a little-endian `u32`.
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    /// Read a little-endian `i32`.
    pub fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    /// Read a little-endian `f32`.
    pub fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    /// Read exactly `N` raw bytes.
    pub fn bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        Ok(self.take(N)?.try_into().unwrap())
    }

    /// Read `N` little-endian `u32`s.
    pub fn u32s<const N: usize>(&mut self) -> Result<[u32; N]> {
        let mut out = [0u32; N];
        for v in &mut out {
            *v = self.u32()?;
        }
        Ok(out)
    }

    /// Read `N` little-endian `i32`s.
    pub fn i32s<const N: usize>(&mut self) -> Result<[i32; N]> {
        let mut out = [0i32; N];
        for v in &mut out {
            *v = self.i32()?;
        }
        Ok(out)
    }

    /// Read `N` little-endian `f32`s.
    pub fn f32s<const N: usize>(&mut self) -> Result<[f32; N]> {
        let mut out = [0f32; N];
        for v in &mut out {
            *v = self.f32()?;
        }
        Ok(out)
    }

    /// Read a NUL-terminated string, consuming the terminator.
    ///
    /// Invalid UTF-8 is replaced lossily. If no terminator exists before the
    /// end of input the remaining bytes are returned as the string — protected
    /// maps sometimes truncate the final field.
    pub fn cstr_lossy(&mut self) -> Result<String> {
        if self.empty_strings_unterminated && self.peek_u8() == Some(0) {
            return Ok(String::new());
        }
        let rest = &self.data[self.pos..];
        match rest.iter().position(|&b| b == 0) {
            Some(nul) => {
                let s = String::from_utf8_lossy(&rest[..nul]).into_owned();
                self.pos += nul + 1;
                Ok(s)
            }
            None => {
                let s = String::from_utf8_lossy(rest).into_owned();
                self.pos = self.data.len();
                Ok(s)
            }
        }
    }
}

/// Read a `u32` count followed by `count` records.
///
/// Returns an empty vector when fewer than 4 bytes remain — truncated
/// trailing sections are common on protected maps and are not an error.
pub fn parse_counted<T>(
    r: &mut ByteReader<'_>,
    mut parse_one: impl FnMut(&mut ByteReader<'_>) -> Result<T>,
) -> Result<Vec<T>> {
    if r.remaining() < 4 {
        return Ok(Vec::new());
    }
    let count = r.u32()? as usize;
    // Clamp pre-allocation so a corrupt count cannot balloon memory.
    let mut items = Vec::with_capacity(count.min(4096));
    for _ in 0..count {
        items.push(parse_one(r)?);
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_and_position() {
        let data = [0x01, 0x02, 0x00, 0x00, 0x00, 0xFF];
        let mut r = ByteReader::new(&data);
        assert_eq!(r.u8().unwrap(), 1);
        assert_eq!(r.u32().unwrap(), 2);
        assert_eq!(r.remaining(), 1);
        assert_eq!(r.peek_u8(), Some(0xFF));
        assert_eq!(r.u8().unwrap(), 0xFF);
        assert!(r.is_at_end());
    }

    #[test]
    fn eof_is_an_error_not_a_panic() {
        let mut r = ByteReader::new(&[0x01, 0x02]);
        let err = r.u32().unwrap_err();
        assert!(matches!(
            err,
            Error::UnexpectedEof {
                offset: 0,
                needed: 2
            }
        ));
    }

    #[test]
    fn cstr_reads_terminator_and_tolerates_missing_nul() {
        let mut r = ByteReader::new(b"abc\0def");
        assert_eq!(r.cstr_lossy().unwrap(), "abc");
        assert_eq!(r.cstr_lossy().unwrap(), "def");
        assert!(r.is_at_end());
    }

    #[test]
    fn counted_returns_empty_on_truncation() {
        let mut r = ByteReader::new(&[0x01]);
        let items = parse_counted(&mut r, |r| r.u8()).unwrap();
        assert!(items.is_empty());
    }
}
