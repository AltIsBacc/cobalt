use std::fmt;

use crate::error::Result;
use crate::reader::Reader;

#[derive(Default, Clone, PartialEq, Eq)]
pub struct FrameImportant(Box<[bool]>);

impl FrameImportant {
    pub fn read(reader: &mut Reader<'_>) -> Result<Self> {
        let count = usize::from(reader.u16()?);
        let bytes = reader.take(count.div_ceil(8))?;
        let flags: Box<[bool]> = (0..count).map(|i| ((bytes[i / 8] >> (i % 8)) & 1) != 0).collect();
        Ok(Self(flags))
    }

    #[must_use]
    pub fn contains(&self, id: u16) -> bool {
        self.0.get(usize::from(id)).copied().unwrap_or(false)
    }
}

impl fmt::Debug for FrameImportant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameImportant").field("len", &self.0.len()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_bits_least_significant_first() {
        let mut reader = Reader::new(&[10, 0, 0b0000_0101, 0b0000_0010]);
        let frames = FrameImportant::read(&mut reader).unwrap();

        assert!(frames.contains(0));
        assert!(!frames.contains(1));
        assert!(frames.contains(2));
        assert!(frames.contains(9));
        assert!(!frames.contains(10));
        assert!(!frames.contains(u16::MAX));
        assert_eq!(reader.remaining(), 0);
    }
}
