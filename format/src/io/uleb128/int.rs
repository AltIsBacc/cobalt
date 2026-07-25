use crate::io::uleb128::{FromUleb128Bytes, ToUleb128Bytes, error::{Uleb128Error, Uleb128Result}};

impl FromUleb128Bytes for u32 {
    fn from_uleb128_bytes(bytes: &[u8]) -> Uleb128Result<(Self, usize)> {
        let mut value = 0u32;
        let mut shift = 0u32;

        for (i, &byte) in bytes.iter().take(5).enumerate() {
            let low_bits = (byte & 0x7F) as u32;

            // 5th byte only has 4 valid payload bits (bits 28-31); anything above overflows.
            if i == 4 && (low_bits >> 4) != 0 {
                return Err(Uleb128Error::Overflow);
            }

            value |= low_bits << shift;

            if byte & 0x80 == 0 {
                return Ok((value, i + 1));
            }

            shift += 7;
        }

        Err(Uleb128Error::Unterminated)
    }
}

impl ToUleb128Bytes for u32 {
    fn to_uleb128_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(5);
        let mut value = *self;

        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            bytes.push(byte);
            if value == 0 {
                break;
            }
        }

        bytes
    }
}

