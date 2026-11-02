use uuid::Uuid;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

macro_rules! read_int {
    ($($name:ident => $ty:ty),* $(,)?) => {
        $(
            pub fn $name(&mut self) -> Result<$ty> {
                Ok(<$ty>::from_le_bytes(self.array()?))
            }
        )*
    };
}

impl<'a> Reader<'a> {
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub const fn pos(&self) -> usize {
        self.pos
    }

    pub const fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn seek(&mut self, pos: usize) -> Result {
        if pos > self.data.len() {
            return Err(Error::SeekOutOfBounds { pos, len: self.data.len() });
        }
        self.pos = pos;
        Ok(())
    }

    pub fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let available = self.remaining();
        if len > available {
            return Err(Error::UnexpectedEof { pos: self.pos, requested: len, available });
        }
        let data = self.data;
        let bytes = &data[self.pos..self.pos + len];
        self.pos += len;
        Ok(bytes)
    }

    pub fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    read_int! {
        u16 => u16,
        i16 => i16,
        i32 => i32,
        i64 => i64,
        u64 => u64,
        f32 => f32,
        f64 => f64,
    }

    pub fn bool(&mut self) -> Result<bool> {
        Ok(self.u8()? != 0)
    }

    pub fn i32s<const N: usize>(&mut self) -> Result<[i32; N]> {
        let mut out = [0; N];
        for value in &mut out {
            *value = self.i32()?;
        }
        Ok(out)
    }

    pub fn count_i16(&mut self) -> Result<usize> {
        let raw = self.i16()?;
        usize::try_from(raw).map_err(|_| Error::NegativeCount(i32::from(raw)))
    }

    pub fn count_i32(&mut self) -> Result<usize> {
        let raw = self.i32()?;
        usize::try_from(raw).map_err(|_| Error::NegativeCount(raw))
    }

    pub fn repeat<T>(
        &mut self,
        count: usize,
        mut read: impl FnMut(&mut Self) -> Result<T>,
    ) -> Result<Vec<T>> {
        (0..count).map(|_| read(self)).collect()
    }

    pub fn uleb128_u32(&mut self) -> Result<u32> {
        let mut value = 0u32;
        for shift in (0u32..35).step_by(7) {
            let byte = self.u8()?;
            let bits = u32::from(byte & 0x7F);
            if shift == 28 && bits > 0x0F {
                return Err(Error::VarintOverflow);
            }
            value |= bits << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(Error::VarintOverflow)
    }

    pub fn string(&mut self) -> Result<String> {
        let len = self.uleb128_u32()? as usize;
        Ok(std::str::from_utf8(self.take(len)?)?.to_owned())
    }

    pub fn uuid(&mut self) -> Result<Uuid> {
        Ok(Uuid::from_bytes_le(self.array()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_little_endian_integers() {
        let mut reader = Reader::new(&[0x34, 0x12, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(reader.u16().unwrap(), 0x1234);
        assert_eq!(reader.i32().unwrap(), -1);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn reports_eof_without_advancing() {
        let mut reader = Reader::new(&[1, 2]);
        assert!(matches!(
            reader.i32(),
            Err(Error::UnexpectedEof { pos: 0, requested: 4, available: 2 })
        ));
        assert_eq!(reader.pos(), 0);
    }

    #[test]
    fn decodes_uleb128() {
        assert_eq!(Reader::new(&[0xE5, 0x8E, 0x26]).uleb128_u32().unwrap(), 624_485);
        assert_eq!(Reader::new(&[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]).uleb128_u32().unwrap(), u32::MAX);
        assert!(matches!(
            Reader::new(&[0xFF, 0xFF, 0xFF, 0xFF, 0x1F]).uleb128_u32(),
            Err(Error::VarintOverflow)
        ));
        assert!(matches!(
            Reader::new(&[0x80; 5]).uleb128_u32(),
            Err(Error::VarintOverflow)
        ));
    }

    #[test]
    fn decodes_strings() {
        assert_eq!(Reader::new(&[3, b'a', b'b', b'c']).string().unwrap(), "abc");
        assert_eq!(Reader::new(&[0]).string().unwrap(), "");
        assert!(matches!(
            Reader::new(&[5, b'a']).string(),
            Err(Error::UnexpectedEof { .. })
        ));
        assert!(matches!(
            Reader::new(&[1, 0xFF]).string(),
            Err(Error::InvalidString(_))
        ));
    }
}
