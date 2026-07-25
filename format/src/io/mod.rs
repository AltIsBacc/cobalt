use uuid::Uuid;

use crate::io::{csharp_datetime::CSharpDateTime, error::{BinaryReadError, ReadResult, WriteResult}, uleb128::{FromUleb128Bytes, ToUleb128Bytes}};

pub mod reader;
pub mod writer;
pub mod error;
pub mod uleb128;
pub mod csharp_datetime;

pub trait BinaryReader {
    fn seek(&mut self, pos: usize);
    fn advance(&mut self, count: usize);

    fn read_n(&mut self, length: usize) -> ReadResult<&[u8]>;
    fn read_at(&self, offset: usize, length: usize) -> ReadResult<&[u8]>;

    fn peek_up_to(&self, max_length: usize) -> &[u8];

    fn pos(&self) -> usize;
    fn size(&self) -> usize;

    #[inline]
    fn read<const N: usize>(&mut self) -> ReadResult<[u8; N]> {
        // Safety: read_n guarantees exactly N bytes
        Ok(self.read_n(N)?.try_into().unwrap())
    }

    #[inline]
    fn read_array<T, F>(&mut self, mut value_reader: F) -> ReadResult<Vec<T>>
    where
        F: FnMut(&mut Self) -> ReadResult<T>,
    {
        (0..self.u16()?).map(|_| value_reader(self)).collect()
    }

    #[inline]
    fn read_array_n<T, F>(&mut self, length: u16, mut value_reader: F) -> ReadResult<Vec<T>>
    where
        F: FnMut(&mut Self) -> ReadResult<T>,
    {
        (0..length).map(|_| value_reader(self)).collect()
    }

    #[inline]
    fn peek(&self, length: usize) -> ReadResult<&[u8]> {
        self.read_at(self.pos(), length)
    }

    #[inline]
    fn u8(&mut self) -> ReadResult<u8> {
        Ok(self.read::<1>()?[0])
    }

    #[inline]
    fn u16(&mut self) -> ReadResult<u16> {
        Ok(u16::from_le_bytes(self.read()?))
    }

    #[inline]
    fn u32(&mut self) -> ReadResult<u32> {
        Ok(u32::from_le_bytes(self.read()?))
    }

    #[inline]
    fn uleb128_u32(&mut self) -> ReadResult<u32> {
        let window = self.peek_up_to(5);
        let (value, advance) = u32::from_uleb128_bytes(window)?;
        self.advance(advance);

        Ok(value)
    }

    #[inline]
    fn u64(&mut self) -> ReadResult<u64> {
        Ok(u64::from_le_bytes(self.read()?))
    }

    #[inline]
    fn uleb128_u64(&mut self) -> ReadResult<u64> {
        let window = self.peek_up_to(10);
        let (value, advance) = u64::from_uleb128_bytes(window)?;
        self.advance(advance);

        Ok(value)
    }

    #[inline]
    fn i8(&mut self) -> ReadResult<i8> {
        Ok(self.u8()? as i8)
    }

    #[inline]
    fn i16(&mut self) -> ReadResult<i16> {
        Ok(i16::from_le_bytes(self.read()?))
    }

    #[inline]
    fn i32(&mut self) -> ReadResult<i32> {
        Ok(i32::from_le_bytes(self.read()?))
    }

    #[inline]
    fn i64(&mut self) -> ReadResult<i64> {
        Ok(i64::from_le_bytes(self.read()?))
    }

    #[inline]
    fn f32(&mut self) -> ReadResult<f32> {
        Ok(f32::from_le_bytes(self.read()?))
    }

    #[inline]
    fn f64(&mut self) -> ReadResult<f64> {
        Ok(f64::from_le_bytes(self.read()?))
    }

    #[inline]
    fn bool(&mut self) -> ReadResult<bool> {
        Ok(self.u8()? != 0)
    }

    #[inline]
    fn bits(&mut self) -> ReadResult<[bool; 8]> {
        let byte = self.u8()?;
        Ok(std::array::from_fn(|i| (byte & (1 << i)) != 0))
    }

    #[inline]
    fn string(&mut self) -> ReadResult<String> {
        let size = self.uleb128_u32()? as usize;
        if size == 0 { return Ok(String::new()) }

        let s = std::str::from_utf8(
            self.read_n(size)?
        ).map_err(|_| BinaryReadError::InvalidString)?;

        Ok(s.to_string())
    }

    #[inline]
    fn datetime(&mut self) -> ReadResult<CSharpDateTime> {
        Ok(CSharpDateTime::from_dotnet_date_data(self.u64()?)?)
    }

    #[inline]
    fn uuid(&mut self) -> ReadResult<Uuid> {
        Ok(Uuid::from_bytes_le(self.read()?))
    }
}

pub trait BinaryWriter {
    fn seek(&mut self, pos: usize);
    fn advance(&mut self, count: usize);
    
    fn write_bytes(&mut self, bytes: &[u8]) -> WriteResult;

    fn pos(&self) -> usize;
    fn as_slice(&self) -> &[u8];

    #[inline]
    fn write<const N: usize>(&mut self, bytes: [u8; N]) -> WriteResult {
        self.write_bytes(&bytes)
    }

    #[inline]
    fn write_array<T, F>(&mut self, items: &[T], mut value_writer: F) -> WriteResult<()>
    where
        F: FnMut(&mut Self, &T) -> WriteResult<()>,
    {
        self.u16(items.len() as u16)?;
        for item in items {
            value_writer(self, item)?;
        }
        Ok(())
    }

    #[inline]
    fn u8(&mut self, value: u8) -> WriteResult {
        self.write_bytes(&[value])
    }

    #[inline]
    fn u16(&mut self, value: u16) -> WriteResult {
        self.write(value.to_le_bytes())
    }

    #[inline]
    fn u32(&mut self, value: u32) -> WriteResult {
        self.write(value.to_le_bytes())
    }

    #[inline]
    fn u64(&mut self, value: u64) -> WriteResult {
        self.write(value.to_le_bytes())
    }

    #[inline]
    fn i8(&mut self, value: i8) -> WriteResult {
        self.u8(value as u8)
    }

    #[inline]
    fn i16(&mut self, value: i16) -> WriteResult {
        self.write(value.to_le_bytes())
    }

    #[inline]
    fn i32(&mut self, value: i32) -> WriteResult {
        self.write(value.to_le_bytes())
    }

    #[inline]
    fn i64(&mut self, value: i64) -> WriteResult {
        self.write(value.to_le_bytes())
    }

    #[inline]
    fn f32(&mut self, value: f32) -> WriteResult {
        self.write(value.to_le_bytes())
    }

    #[inline]
    fn f64(&mut self, value: f64) -> WriteResult {
        self.write(value.to_le_bytes())
    }

    #[inline]
    fn bool(&mut self, value: bool) -> WriteResult {
        self.u8(if value { 1 } else { 0 })
    }

    #[inline]
    fn bits(&mut self, bits: [bool; 8]) -> WriteResult {
        let byte = bits
            .iter()
            .enumerate()
            .fold(0u8, |acc, (i, &b)| acc | ((b as u8) << i));
        self.u8(byte)
    }

    #[inline]
    fn uleb128_u32(&mut self, value: u32) -> WriteResult {
        let bytes = value.to_uleb128_bytes();
        self.write_bytes(&bytes)
    }

    #[inline]
    fn uleb128_u64(&mut self, value: u64) -> WriteResult {
        let bytes = value.to_uleb128_bytes();
        self.write_bytes(&bytes)
    }

    #[inline]
    fn string<S>(&mut self, value: S) -> WriteResult
    where
        S: AsRef<str>,
    {
        self.uleb128_u32(value.as_ref().len() as u32)?;
        self.write_bytes(value.as_ref().as_bytes())
    }

    #[inline]
    fn datetime(&mut self, value: CSharpDateTime) -> WriteResult {
        self.u64(value.to_dotnet_date_data())
    }

    #[inline]
    fn uuid(&mut self, value: Uuid) -> WriteResult {
        self.write(value.to_bytes_le())
    }
}

