use crate::error::{ProtocolError, ProtocolResult};

#[derive(Debug, Clone)]
pub struct Reader<'a> {
    data: &'a [u8],
}

impl<'a> Reader<'a> {
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn u8(&mut self) -> ProtocolResult<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn array<const N: usize>(&mut self) -> ProtocolResult<[u8; N]> {
        self.take(N)?.first_chunk().copied().ok_or(ProtocolError::UnexpectedEof)
    }

    pub fn uleb128(&mut self) -> ProtocolResult<u32> {
        let mut value = 0u32;
        for shift in (0u32..35).step_by(7) {
            let byte = self.u8()?;
            let bits = u32::from(byte & 0x7F);
            if shift == 28 && bits > 0x0F {
                return Err(ProtocolError::VarintOverflow);
            }
            value |= bits << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(ProtocolError::VarintOverflow)
    }

    pub fn string(&mut self) -> ProtocolResult<&'a str> {
        let len = self.uleb128()? as usize;
        Ok(std::str::from_utf8(self.take(len)?)?)
    }

    fn take(&mut self, len: usize) -> ProtocolResult<&'a [u8]> {
        let (head, rest) = self.data.split_at_checked(len).ok_or(ProtocolError::UnexpectedEof)?;
        self.data = rest;
        Ok(head)
    }
}

#[derive(Debug)]
pub struct Writer<'a> {
    out: &'a mut Vec<u8>,
}

impl<'a> Writer<'a> {
    pub const fn new(out: &'a mut Vec<u8>) -> Self {
        Self { out }
    }

    pub fn u8(&mut self, value: u8) {
        self.out.push(value);
    }

    pub fn raw(&mut self, bytes: &[u8]) {
        self.out.extend_from_slice(bytes);
    }

    pub fn uleb128(&mut self, mut value: u32) {
        while value >= 0x80 {
            self.out.push(((value & 0x7F) as u8) | 0x80);
            value >>= 7;
        }
        self.out.push(value as u8);
    }

    pub fn string(&mut self, value: &str) -> ProtocolResult {
        let len = u16::try_from(value.len())
            .map_err(|_| ProtocolError::PacketTooLarge(value.len()))?;
        self.uleb128(u32::from(len));
        self.out.extend_from_slice(value.as_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uleb128_round_trips() {
        for value in [0, 1, 127, 128, 624_485, u32::MAX] {
            let mut out = Vec::new();
            Writer::new(&mut out).uleb128(value);
            assert_eq!(Reader::new(&out).uleb128().unwrap(), value);
        }
    }

    #[test]
    fn uleb128_rejects_overflow() {
        assert!(matches!(
            Reader::new(&[0xFF, 0xFF, 0xFF, 0xFF, 0x1F]).uleb128(),
            Err(ProtocolError::VarintOverflow)
        ));
        assert!(matches!(Reader::new(&[0x80; 5]).uleb128(), Err(ProtocolError::VarintOverflow)));
    }

    #[test]
    fn strings_round_trip() {
        let mut out = Vec::new();
        Writer::new(&mut out).string("héllo").unwrap();
        assert_eq!(Reader::new(&out).string().unwrap(), "héllo");
    }

    #[test]
    fn strings_reject_truncation_and_bad_utf8() {
        assert!(matches!(Reader::new(&[5, b'a']).string(), Err(ProtocolError::UnexpectedEof)));
        assert!(matches!(Reader::new(&[1, 0xFF]).string(), Err(ProtocolError::InvalidString(_))));
    }

    #[test]
    fn oversized_strings_are_rejected() {
        let mut out = Vec::new();
        let long = "a".repeat(usize::from(u16::MAX) + 1);
        assert!(matches!(
            Writer::new(&mut out).string(&long),
            Err(ProtocolError::PacketTooLarge(_))
        ));
        assert!(out.is_empty());
    }
}
