use crate::compression::*;
use crate::crypt::{decrypt, hash_string};
use adler32::RollingAdler32;
use byteorder::{ByteOrder, LittleEndian};
use std::fmt;
use std::fs;
use std::io::SeekFrom;
use std::io::{prelude::*, Cursor};
use std::io::{Error, ErrorKind};
use std::mem;
use std::path::Path;

const HEADER_SIZE_V1: usize = 0x20;
//const HEADER_SIZE_V2: usize = 0x2C;
//const HEADER_SIZE_V3: usize = 0x44;
//const HEADER_SIZE_V4: usize = 0xD0;
const USER_HEADER_SIZE: usize = 16;
/// StormLib stops the 0x200 header walk after ~128 MB.
const HEADER_SEARCH_LIMIT: u64 = 0x0800_0000;

const ID_MPQA: &[u8] = b"MPQ\x1A";
const ID_MPQB: &[u8] = b"MPQ\x1B";

const FILE_IMPLODE: u32 = 0x00000100; // implode method by pkware compression library
const FILE_COMPRESS: u32 = 0x00000200; // compress methods by multiple methods
const FILE_ENCRYPTED: u32 = 0x00010000; // file is encrypted
/// StormLib `MPQ_BLOCK_INDEX`: protectors set high bits on `dwBlockIndex`.
const BLOCK_INDEX_MASK: u32 = 0x0FFF_FFFF;
const FILE_FIX_KEY: u32 = 0x00020000; // file decryption key is altered according to position of file in archive
const FILE_PATCH_FILE: u32 = 0x00100000; // file is a patch file. file data begins with patchinfo struct
const FILE_SINGLE_UNIT: u32 = 0x01000000; // file is stored as single unit
const FILE_SECTOR_CRC: u32 = 0x04000000;
const FILE_COMPRESS_MASK: u32 = 0x0000FF00;

#[derive(Debug)]
pub(crate) struct Header {
    _magic: [u8; 4],
    _header_size: u32,
    _archive_size: u32,
    _format_version: u16, // 0 = Original, 1 = Extended
    sector_size_shift: u16,
    pub(crate) hash_table_offset: u32,
    block_table_offset: u32,
    pub(crate) hash_table_count: u32,
    block_table_count: u32,
    // Header v2
    _extended_offset: u64,
    _hash_table_offset_high: u16,
    _block_table_offset_high: u16,
    // ToDo: Header v3 and v4
}

impl Header {
    pub fn new(src: &[u8; HEADER_SIZE_V1]) -> Header {
        Header {
            _magic: [src[0], src[1], src[2], src[3]],
            _header_size: LittleEndian::read_u32(&src[0x04..]),
            _archive_size: LittleEndian::read_u32(&src[0x08..]),
            _format_version: LittleEndian::read_u16(&src[0x0C..]),
            sector_size_shift: LittleEndian::read_u16(&src[0x0E..]),
            hash_table_offset: LittleEndian::read_u32(&src[0x10..]),
            block_table_offset: LittleEndian::read_u32(&src[0x14..]),
            hash_table_count: LittleEndian::read_u32(&src[0x18..]),
            block_table_count: LittleEndian::read_u32(&src[0x1C..]),
            _extended_offset: 0,
            _hash_table_offset_high: 0,
            _block_table_offset_high: 0,
        }
    }
}

#[derive(Debug)]
struct UserDataHeader {
    _magic: [u8; 4],
    user_data_size: u32,
    header_offset: u32,
    _user_data_header_size: u32,
}

impl UserDataHeader {
    pub fn new(src: &[u8]) -> UserDataHeader {
        UserDataHeader {
            _magic: [src[0], src[1], src[2], src[3]],
            user_data_size: LittleEndian::read_u32(&src[0x4..]),
            header_offset: LittleEndian::read_u32(&src[0x8..]),
            _user_data_header_size: LittleEndian::read_u32(&src[0xC..]),
        }
    }
}

#[derive(Debug, Clone)]
struct Hash {
    /// file name hash part A
    hash_a: u32,
    /// file name hash part B
    hash_b: u32,
    /// language of file using windows LANGID type
    _locale: u16,
    /// platform file is used for
    _platform: u16,
    /// index into the block table of file
    block_index: u32,
}

impl Hash {
    pub fn new(src: &[u8]) -> Hash {
        Hash {
            hash_a: LittleEndian::read_u32(src),
            hash_b: LittleEndian::read_u32(&src[4..]),
            _locale: LittleEndian::read_u16(&src[8..]),
            _platform: LittleEndian::read_u16(&src[10..]),
            block_index: LittleEndian::read_u32(&src[12..]),
        }
    }
}

#[derive(Debug, Clone)]
struct Block {
    /// offset of the beginning of the file data, relative to the beginning of the archive
    offset: u32,
    /// compressed file size
    packed_size: u32,
    /// uncompressed file size
    unpacked_size: u32,
    /// flags for file
    flags: u32,
}

impl Block {
    pub fn new(src: &[u8]) -> Block {
        Block {
            offset: LittleEndian::read_u32(src),
            packed_size: LittleEndian::read_u32(&src[0x4..]),
            unpacked_size: LittleEndian::read_u32(&src[0x8..]),
            flags: LittleEndian::read_u32(&src[0xC..]),
        }
    }
}

/// Absolute position of a table whose offset is stored relative to the header.
///
/// Some protectors store that offset as a negative 32-bit value, so following
/// it in 64-bit arithmetic lands past the end of the file while Storm — which
/// adds in 32 bits — wraps back into the archive. Mirror the wrap, but only
/// once the straightforward reading has already fallen outside, so a healthy
/// archive is never resolved by anything but its own header.
fn table_start(header_offset: u64, table_offset: u32, total: u64) -> u64 {
    let plain = header_offset + u64::from(table_offset);
    if plain < total {
        return plain;
    }
    let wrapped = u64::from((header_offset as u32).wrapping_add(table_offset));
    if wrapped < total {
        wrapped
    } else {
        plain
    }
}

/// StormLib `ConvertMpqHeaderToFormat4` rejects a v1 header when the 32-bit
/// wrap of `header + {hash,block}TablePos` lands past EOF, then keeps scanning.
/// Game Storm.dll does not: it takes the first `MPQ\x1A` with `dwHeaderSize >=
/// 0x20`. Skipping the fake header is what finds a later real archive on
/// maps that prepend a decoy at 0x200.
fn is_fake_mpq_header(header_offset: u64, header: &Header, file_size: u64) -> bool {
    let hash_pos = (header_offset as u32).wrapping_add(header.hash_table_offset);
    let block_pos = (header_offset as u32).wrapping_add(header.block_table_offset);
    u64::from(hash_pos) > file_size || u64::from(block_pos) > file_size
}

pub struct Archive {
    pub(crate) file: Cursor<Vec<u8>>,
    pub(crate) header: Header,
    user_data_header: Option<UserDataHeader>,
    hash_table: Vec<Hash>,
    block_table: Vec<Block>,
    pub(crate) sector_size: u32,
    pub(crate) offset: u64,
}

impl Archive {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Archive, Error> {
        let mut file = fs::File::open(&path).expect("no file found");
        let metadata = fs::metadata(&path).expect("unable to read metadata");
        let mut buf = vec![0; metadata.len() as usize];
        file.read_exact(&mut buf).expect("buffer overflow");
        Self::load(buf)
    }

    pub fn load(buf: Vec<u8>) -> Result<Archive, Error> {
        let mut buffer: [u8; HEADER_SIZE_V1] = [0; HEADER_SIZE_V1];
        let mut offset: u64 = 0;
        let mut user_data_header = None;
        let mut file = Cursor::new(buf);
        let total = file.get_ref().len() as u64;

        loop {
            if offset >= HEADER_SEARCH_LIMIT || offset.saturating_add(HEADER_SIZE_V1 as u64) > total
            {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "no MPQ header with in-archive tables",
                ));
            }

            file.seek(SeekFrom::Start(offset))?;

            file.read_exact(&mut buffer)?;

            if buffer.starts_with(ID_MPQA) {
                // Warcraft III's Storm.dll (Frozen Throne build, ordinal 266)
                // accepts a header only when dwHeaderSize >= 0x20; a smaller
                // size is treated as a decoy and the 0x200 scan continues.
                // Huge garbage sizes (Spazzler etc.) still pass this check.
                let header_size = LittleEndian::read_u32(&buffer[0x04..]);
                if header_size >= HEADER_SIZE_V1 as u32 {
                    let candidate = Header::new(&buffer);
                    // StormLib additionally drops headers whose 32-bit table
                    // positions sit past EOF (ERROR_FAKE_MPQ_HEADER) and keeps
                    // walking. That is how a decoy at 0x200 loses to a real
                    // archive later in the file.
                    if !is_fake_mpq_header(offset, &candidate, total) {
                        break;
                    }
                }
            }

            if buffer.starts_with(ID_MPQB) {
                let header = UserDataHeader::new(&buffer);
                let candidate = offset + u64::from(header.header_offset);

                // Map protectors write a user data header whose offset points
                // nowhere, so that a reader following it gives up before
                // reaching the real header a sector or two later. Treat a user
                // data header that does not land on `MPQ\x1a` as decoration and
                // keep scanning.
                let mut probe: [u8; HEADER_SIZE_V1] = [0; HEADER_SIZE_V1];
                let landed = file.seek(SeekFrom::Start(candidate)).is_ok()
                    && file.read_exact(&mut probe).is_ok()
                    && probe.starts_with(ID_MPQA);

                if landed {
                    let landed_header = Header::new(&probe);
                    if !is_fake_mpq_header(candidate, &landed_header, total) {
                        buffer = probe;
                        offset = candidate;
                        user_data_header = Some(header);
                        break;
                    }
                }
            }

            offset += 0x200;
        }

        let header = Header::new(&buffer);

        // Table sizes come from the header and are routinely inflated by map
        // protectors: a block count of 33410 in a 1.7 MB file makes a reader
        // that trusts it fail on a short read. Clamp both tables to what the
        // archive actually contains and read what is there.
        let entries_after =
            |start: u64, entry: usize| -> u64 { total.saturating_sub(start) / entry as u64 };

        // read hash table
        let hash_start = table_start(offset, header.hash_table_offset, total);
        let hash_room = entries_after(hash_start, mem::size_of::<Hash>());
        if hash_room == 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "MPQ hash table lies outside the archive",
            ));
        }
        // Read what is there, but keep the declared count: Storm masks the
        // bucket index with `dwHashTableSize - 1`, so shrinking the count to fit
        // the file would change the mask and send every lookup to the wrong
        // bucket. Slots past the end of the file are simply unreachable.
        let hash_count = header.hash_table_count.min(hash_room as u32);
        let mut hash_buff: Vec<u8> = vec![0; (hash_count as usize) * mem::size_of::<Hash>()];
        let mut hash_table: Vec<Hash> = Vec::with_capacity(hash_count as usize);

        file.seek(SeekFrom::Start(hash_start))?;

        file.read_exact(&mut hash_buff)?;

        decrypt(&mut hash_buff, hash_string("(hash table)", 0x300));

        for x in 0..hash_count {
            hash_table.push(Hash::new(&hash_buff[x as usize * mem::size_of::<Hash>()..]));
        }

        // read block table
        let block_start = table_start(offset, header.block_table_offset, total);
        let block_room = entries_after(block_start, mem::size_of::<Block>());
        let block_count = header.block_table_count.min(block_room as u32);
        let mut block_buff: Vec<u8> = vec![0; (block_count as usize) * mem::size_of::<Block>()];
        let mut block_table: Vec<Block> = Vec::with_capacity(block_count as usize);

        file.seek(SeekFrom::Start(block_start))?;

        file.read_exact(&mut block_buff)?;

        decrypt(&mut block_buff, hash_string("(block table)", 0x300));

        for x in 0..block_count {
            block_table.push(Block::new(
                &block_buff[x as usize * mem::size_of::<Block>()..],
            ));
        }

        // Two maps in the corpus carry a sector shift of 65292; shifting by that
        // panics a debug build and silently masks to 5 bits in release. A zero
        // sector size is already handled as "cannot read this file".
        let sector_size = 512u32
            .checked_shl(u32::from(header.sector_size_shift))
            .unwrap_or(0);

        Ok(Archive {
            file,
            header,
            user_data_header,
            hash_table,
            block_table,
            sector_size,
            offset,
        })
    }

    pub fn open_file(&mut self, filename: &str) -> Result<File, Error> {
        if self.hash_table.is_empty() {
            return Err(Error::new(ErrorKind::InvalidData, "empty MPQ hash table"));
        }
        // Storm masks with the *declared* size, which protectors inflate past
        // what the file holds. Keep its arithmetic and let the slots we do not
        // have behave as empty, rather than remasking against a shorter table.
        let declared = if self.header.hash_table_count == 0 {
            self.hash_table.len() as u32
        } else {
            self.header.hash_table_count
        };
        let start_index = (hash_string(filename, 0x0) & declared.wrapping_sub(1)) as usize;
        let mut hash;

        let hash_a = hash_string(filename, 0x100);
        let hash_b = hash_string(filename, 0x200);
        let mut file_key = 0;

        // The hash table is probed circularly from the home bucket: an entry
        // whose bucket is late in the table wraps around to the front, and
        // stopping at the end silently loses those files.
        let table_len = self.hash_table.len();
        for probe in 0..table_len {
            let i = (start_index + probe) % table_len;
            hash = &self.hash_table[i];

            if hash.hash_a == hash_a && hash.hash_b == hash_b {
                // The block index comes straight from the archive: a protected
                // or corrupt map can point it past the end of the block table.
                // StormLib keeps only the low 28 bits — W3x protectors stash
                // flags in the top nibble (0x800003F2, 0x400003BE, …).
                let block =
                    self.block_table
                        .get((hash.block_index & BLOCK_INDEX_MASK) as usize)
                        .ok_or_else(|| {
                            Error::new(
                        ErrorKind::InvalidData,
                        format!("MPQ hash entry for {filename:?} points outside the block table"),
                    )
                        })?;

                // A block entry whose data does not fit inside the archive is
                // not a file we can read, and its flags are not worth
                // interpreting either — protectors overwrite whole block tables
                // with noise, where a random bit lands on FILE_PATCH_FILE or
                // FILE_ENCRYPTED and produces an error describing the wrong
                // problem. Say what is actually wrong instead.
                // Only `packed_size` is on disk; `unpacked_size` is what the
                // data expands to and is routinely far larger.
                let total = self.file.get_ref().len() as u64;
                let data_end = self.offset + u64::from(block.offset) + u64::from(block.packed_size);
                if data_end > total {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "MPQ block entry for {filename:?} lies outside the archive \
                             (offset {}, packed {} bytes, archive {total} bytes)",
                            block.offset, block.packed_size,
                        ),
                    ));
                }

                let mut sector_offsets: Vec<u32> = Vec::new();
                let mut sector_checksums: Vec<u32> = Vec::new();

                // file if encrypted, generate decryption key
                if block.flags & FILE_ENCRYPTED != 0 {
                    match filename.split(&['\\', '/'][..]).next_back() {
                        Some(basename) => file_key = hash_string(basename, 0x300),
                        None => {
                            return Err(Error::other("Unable to extract filename from path"));
                        }
                    }

                    // fix decryption key
                    if block.flags & FILE_FIX_KEY != 0 {
                        file_key = (file_key + (block.offset)) ^ block.unpacked_size;
                    }
                }

                // block split into sectors, read sector offsets
                if block.flags & FILE_SINGLE_UNIT == 0 {
                    // FixMe: handle empty files, packed and unpacked size should be 0

                    if block.unpacked_size == 0 || self.sector_size == 0 {
                        return Err(Error::new(ErrorKind::UnexpectedEof, filename));
                    }

                    let num_sectors = ((block.unpacked_size - 1) / self.sector_size) + 1;

                    let mut sector_buff: Vec<u8> = vec![0; ((num_sectors as usize) + 1) * 4];

                    self.file
                        .seek(SeekFrom::Start(u64::from(block.offset) + self.offset))?;
                    self.file.read_exact(&mut sector_buff)?;

                    if block.flags & FILE_ENCRYPTED != 0 {
                        decrypt(&mut sector_buff, file_key - 1);
                    }

                    let mut x = 0;
                    while x < sector_buff.len() - 3 {
                        sector_offsets.push(LittleEndian::read_u32(&sector_buff[x..]));
                        x += 4;
                    }

                    // load sector checksums
                    if block.flags & FILE_COMPRESS != 0 && block.flags & FILE_SECTOR_CRC != 0 {
                        let mut buff: Vec<u8> = vec![0; 4];

                        self.file.read_exact(&mut buff)?;

                        let last_offset = LittleEndian::read_u32(&buff);
                        let checksum_offset = sector_offsets[num_sectors as usize];
                        let sector_size = last_offset - checksum_offset;
                        let expected_size = num_sectors * mem::size_of::<u32>() as u32;

                        // is checksum sector the expected size
                        if sector_size == expected_size {
                            self.file.seek(SeekFrom::Start(
                                u64::from(block.offset) + u64::from(checksum_offset),
                            ))?;

                            for _ in 0..num_sectors {
                                self.file.read_exact(&mut buff)?;

                                sector_checksums.push(LittleEndian::read_u32(&buff));
                            }
                        }
                    }
                }

                return Ok(File {
                    _name: String::from(filename),
                    _hash: hash.clone(),
                    block: block.clone(),
                    sector_offsets,
                    sector_checksums,
                    file_key,
                });
            }
        }

        Err(Error::new(ErrorKind::NotFound, filename))
    }

    pub fn read_user_data(&mut self) -> Result<Option<Vec<u8>>, Error> {
        match self.user_data_header {
            Some(ref header) => {
                let mut buf: Vec<u8> = vec![0; header.user_data_size as usize];

                self.file.seek(SeekFrom::Start(USER_HEADER_SIZE as u64))?;
                self.file.read_exact(&mut buf)?;

                Ok(Some(buf))
            }
            None => Ok(None),
        }
    }
}

impl fmt::Debug for Archive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{{\nfile: {:#?},\nheader: {:#?}\nsector_size:{}\n}}",
            self.file, self.header, self.sector_size
        )
    }
}

#[derive(Debug)]
pub struct File {
    _name: String,
    _hash: Hash,
    block: Block,
    sector_offsets: Vec<u32>,
    sector_checksums: Vec<u32>,
    file_key: u32,
}

impl File {
    pub fn size(&self) -> u32 {
        self.block.unpacked_size
    }

    /// Bytes the file occupies inside the archive.
    pub fn packed_size(&self) -> u32 {
        self.block.packed_size
    }

    // read data from file
    pub fn read(&self, archive: &mut Archive, buf: &mut [u8]) -> Result<usize, Error> {
        if self.block.flags & FILE_PATCH_FILE != 0 {
            Err(Error::other("Patch file not supported"))
        } else if self.block.flags & FILE_SINGLE_UNIT != 0 {
            // file is single block file
            self.read_single_unit_file(
                self.block.packed_size as usize,
                &mut archive.file,
                archive.offset,
                buf,
            )
        } else {
            // read as sector based MPQ file
            self.read_sector_file(archive, buf)
        }
    }

    fn read_sector_file(&self, archive: &mut Archive, out: &mut [u8]) -> Result<usize, Error> {
        let mut buff: Vec<u8> = vec![0; archive.sector_size as usize];
        let mut read: usize = 0;

        if self.block.flags & FILE_COMPRESS_MASK != 0 {
            for i in 0..self.sector_offsets.len() - 1 {
                let sector_offset = self.sector_offsets[i];
                // Sector offsets come from the archive and are not validated by
                // the format: a protected or truncated map can make them
                // decrease, or claim a sector larger than the sector buffer.
                let sector_size = self.sector_offsets[i + 1]
                    .checked_sub(sector_offset)
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::InvalidData,
                            "MPQ sector offsets are not monotonic",
                        )
                    })?;
                if sector_size as usize > buff.len() || read > out.len() {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "MPQ sector exceeds the declared sector size",
                    ));
                }

                let in_buf: &mut [u8] = &mut buff[0..sector_size as usize];
                let out_buf: &mut [u8] = &mut out[read..];

                archive.file.seek(SeekFrom::Start(
                    u64::from(self.block.offset) + u64::from(sector_offset) + archive.offset,
                ))?;

                archive.file.read_exact(in_buf)?;

                if self.block.flags & FILE_ENCRYPTED != 0 {
                    decrypt(in_buf, self.file_key + i as u32);
                }

                // checksum verification
                if !self.sector_checksums.is_empty() && self.sector_checksums[i] != 0 {
                    let mut adler = RollingAdler32::from_value(0);

                    adler.update_buffer(in_buf);

                    if self.sector_checksums[i] != adler.hash() {
                        return Err(Error::other("Sector checksum error"));
                    }
                }

                if self.block.flags & FILE_COMPRESS != 0 {
                    if in_buf.len() == archive.sector_size as usize || in_buf.len() == out_buf.len()
                    {
                        for (dst, src) in out_buf.iter_mut().zip(in_buf) {
                            *dst = *src;
                            read += 1;
                        }
                    } else {
                        read += decompress(in_buf, out_buf)?;
                    }
                } else if self.block.flags & FILE_IMPLODE != 0 {
                    if in_buf.len() == archive.sector_size as usize || in_buf.len() == out_buf.len()
                    {
                        for (dst, src) in out_buf.iter_mut().zip(in_buf) {
                            *dst = *src;
                            read += 1;
                        }
                    } else {
                        read += explode(in_buf, out_buf)?;
                    }
                }
            }
        } else {
            archive.file.seek(SeekFrom::Start(
                u64::from(self.block.offset) + archive.offset,
            ))?;
            archive.file.read_exact(out)?;

            read = out.len();
        }

        Ok(read)
    }

    fn read_single_unit_file(
        &self,
        buff_size: usize,
        file: &mut Cursor<Vec<u8>>,
        offset: u64,
        out_buf: &mut [u8],
    ) -> Result<usize, Error> {
        let mut in_buff: Vec<u8> = vec![0; buff_size];

        file.seek(SeekFrom::Start(u64::from(self.block.offset) + offset))?;

        file.read_exact(&mut in_buff)?;

        if self.block.flags & FILE_ENCRYPTED != 0 {
            decrypt(&mut in_buff, self.file_key);
        }

        if self.block.flags & FILE_COMPRESS != 0 && out_buf.len() > in_buff.len() {
            decompress(&mut in_buff, out_buf)
        } else if self.block.flags & FILE_IMPLODE != 0 {
            explode(&mut in_buff, out_buf)
        } else {
            for (dst, src) in out_buf.iter_mut().zip(&in_buff) {
                *dst = *src
            }

            Ok(self.block.unpacked_size as usize)
        }
    }

    // extract file from archive to the local filesystem
    pub fn extract<P: AsRef<Path>>(&self, archive: &mut Archive, path: P) -> Result<usize, Error> {
        let mut buf: Vec<u8> = vec![0; self.size() as usize];

        self.read(archive, &mut buf)?;

        fs::create_dir_all(path.as_ref().parent().unwrap())?;

        if path.as_ref().exists() {
            return Err(Error::new(ErrorKind::AlreadyExists, "File already exists"));
        }

        // The path is known not to exist (checked above), so truncation never
        // discards anything; stating it keeps the intent unambiguous.
        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)
            .unwrap();

        file.write(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypt::encrypt;

    fn header(header_size: u32, hash_pos: u32) -> [u8; 32] {
        let mut buf = [0u8; 32];
        buf[0..4].copy_from_slice(ID_MPQA);
        buf[4..8].copy_from_slice(&header_size.to_le_bytes());
        buf[14..16].copy_from_slice(&3u16.to_le_bytes());
        buf[16..20].copy_from_slice(&hash_pos.to_le_bytes());
        buf[20..24].copy_from_slice(&hash_pos.to_le_bytes());
        buf
    }

    #[test]
    fn storm_skips_a_header_smaller_than_0x20() {
        let mut archive = vec![0u8; 0x400];
        archive[0..32].copy_from_slice(&header(0x10, 0x20));
        archive[0x200..0x220].copy_from_slice(&header(0x20, 0x20));

        let opened = Archive::load(archive).expect("second header should be accepted");
        assert_eq!(opened.offset, 0x200);
    }

    #[test]
    fn storm_accepts_a_huge_header_size() {
        let mut archive = vec![0u8; 0x400];
        archive[0x200..0x220].copy_from_slice(&header(1_347_385_430, 0x20));

        let opened = Archive::load(archive).expect("Spazzler-sized header is still a header");
        assert_eq!(opened.offset, 0x200);
    }

    #[test]
    fn stormlib_skips_a_header_whose_tables_sit_past_eof() {
        let mut archive = vec![0u8; 0x400];
        archive[0..32].copy_from_slice(&header(0x20, 0x10000));
        archive[0x200..0x220].copy_from_slice(&header(0x20, 0x20));

        let opened = Archive::load(archive).expect("decoy with out-of-range tables is skipped");
        assert_eq!(opened.offset, 0x200);
    }

    #[test]
    fn wrapped_table_offsets_are_not_fake_headers() {
        // offset 0x200 + 0xFFFF_FF00 wraps to 0x100, which is inside a 0x400 file.
        // StormLib keeps this header; it is not ERROR_FAKE_MPQ_HEADER.
        let mut archive = vec![0u8; 0x400];
        archive[0x200..0x220].copy_from_slice(&header(0x20, 0xFFFF_FF00));

        let opened = Archive::load(archive).expect("32-bit wrap back into the file is valid");
        assert_eq!(opened.offset, 0x200);
    }

    #[test]
    fn stormlib_masks_high_bits_of_the_block_index() {
        // 4-slot hash table + 1 block, both immediately after a 32-byte header.
        // The w3i hash entry lives at its home bucket with block_index =
        // 0x80000000 (FILE_EXISTS bit set on the index, protector style).
        let name = "war3map.w3i";
        let hash_count = 4u32;
        let home = hash_string(name, 0x0) & (hash_count - 1);
        let mut hash_raw = vec![0u8; 4 * 16];
        let slot = (home as usize) * 16;
        hash_raw[slot..slot + 4].copy_from_slice(&hash_string(name, 0x100).to_le_bytes());
        hash_raw[slot + 4..slot + 8].copy_from_slice(&hash_string(name, 0x200).to_le_bytes());
        hash_raw[slot + 12..slot + 16].copy_from_slice(&0x8000_0000u32.to_le_bytes());
        encrypt(&mut hash_raw, hash_string("(hash table)", 0x300));

        let mut block_raw = vec![0u8; 16];
        // offset 0x70, 4 packed/unpacked bytes, no compression
        block_raw[0..4].copy_from_slice(&0x70u32.to_le_bytes());
        block_raw[4..8].copy_from_slice(&4u32.to_le_bytes());
        block_raw[8..12].copy_from_slice(&4u32.to_le_bytes());
        block_raw[12..16].copy_from_slice(&FILE_SINGLE_UNIT.to_le_bytes());
        encrypt(&mut block_raw, hash_string("(block table)", 0x300));

        let mut archive = vec![0u8; 0x80];
        let mut hdr = header(0x20, 0x20);
        hdr[20..24].copy_from_slice(&0x60u32.to_le_bytes()); // block table after 4 hash slots
        hdr[24..28].copy_from_slice(&hash_count.to_le_bytes());
        hdr[28..32].copy_from_slice(&1u32.to_le_bytes());
        archive[0..32].copy_from_slice(&hdr);
        archive[0x20..0x60].copy_from_slice(&hash_raw);
        archive[0x60..0x70].copy_from_slice(&block_raw);
        archive[0x70..0x74].copy_from_slice(&[1, 2, 3, 4]);

        let mut opened = Archive::load(archive).expect("archive with masked block index");
        let file = opened
            .open_file(name)
            .expect("0x80000000 block index must resolve to 0");
        assert_eq!(file.size(), 4);
    }
}
