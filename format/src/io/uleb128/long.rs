use crate::io::uleb128::{FromUleb128Bytes, ToUleb128Bytes, error::{Uleb128Error, Uleb128Result}};

impl FromUleb128Bytes for u64 {
    fn from_uleb128_bytes(bytes: &[u8]) -> Uleb128Result<(Self, usize)> {
        let mut value = 0u64;
        let mut shift = 0u32;

        for (i, &byte) in bytes.iter().take(10).enumerate() {
            let low_bits = (byte & 0x7F) as u64;

            // 10th byte only has 1 valid payload bit (bit 63); anything above overflows.
            if i == 9 && (low_bits >> 1) != 0 {
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

impl ToUleb128Bytes for u64 {
    fn to_uleb128_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(10);
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

