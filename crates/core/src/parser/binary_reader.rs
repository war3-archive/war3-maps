use binary_reader::BinaryReader;

use crate::parser::error::ParserError;

/// Trait to read a value from a binary reader, and parse it into a type.
///
/// This trait is implemented for most of the primitive types.
pub trait AutoReadable: Sized {
    fn read(stream: &mut BinaryReader) -> Result<Self, ParserError>;
}

impl AutoReadable for u8 {
    fn read(stream: &mut BinaryReader) -> Result<Self, ParserError> {
        Ok(stream.read_u8()?)
    }
}

impl AutoReadable for u32 {
    fn read(stream: &mut BinaryReader) -> Result<Self, ParserError> {
        Ok(stream.read_u32()?)
    }
}

impl AutoReadable for i32 {
    fn read(stream: &mut BinaryReader) -> Result<Self, ParserError> {
        Ok(stream.read_i32()?)
    }
}

impl AutoReadable for String {
    fn read(stream: &mut BinaryReader) -> Result<Self, ParserError> {
        Ok(stream.read_cstr_lossy()?)
    }
}

impl AutoReadable for f32 {
    fn read(stream: &mut BinaryReader) -> Result<Self, ParserError> {
        Ok(stream.read_f32()?)
    }
}

impl<T: AutoReadable + Default + Copy, const N: usize> AutoReadable for [T; N] {
    fn read(stream: &mut BinaryReader) -> Result<Self, ParserError> {
        let mut array = [T::default(); N];
        for it in array.iter_mut().take(N) {
            *it = T::read(stream)?;
        }
        Ok(array)
    }
}

/// Trait to load from binary array and parse it into a struct
pub trait BinaryReadable: Sized {
    fn load(stream: &mut BinaryReader, version: u32) -> Result<Self, ParserError>;
    fn size(&self, _version: u32) -> usize {
        std::mem::size_of::<Self>()
    }
}
