use crate::io::{BinaryReader, error::{BinaryReadError, ReadResult}};

pub struct BinaryCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BinaryCursor<'a> {
    pub fn from(data: &'a [u8]) -> Self {
        Self::with_pos(data, 0)
    }

    pub fn with_pos(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }
}

impl<'a> BinaryReader for BinaryCursor<'a> {
    fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn advance(&mut self, count: usize) {
        self.pos += count;
    }

    fn read_n(&mut self, length: usize) -> ReadResult<&[u8]> {
        let end = self.pos.checked_add(length).ok_or(BinaryReadError::Overflow { pos: self.pos, length })?;
        if end > self.data.len() {
            return Err(BinaryReadError::UnexpectedEof {
                pos: self.pos,
                requested: length,
                available: self.data.len(),
            });
        }
        let start = self.pos;
        self.seek(end);
        Ok(&self.data[start..end])
    }

    fn read_at(&self, offset: usize, length: usize) -> ReadResult<&[u8]> {
        let end = offset.checked_add(length).ok_or(BinaryReadError::Overflow { pos: offset, length })?;
        if end > self.data.len() {
            return Err(BinaryReadError::UnexpectedEof {
                pos: offset,
                requested: length,
                available: self.data.len(),
            });
        }
        Ok(&self.data[offset..end])
    }

    fn peek_up_to(&self, max_length: usize) -> &[u8] {
        let start = self.pos;
        let end = (start + max_length).min(self.data.len());
        &self.data[start..end]
    }

    fn pos(&self) -> usize {
        self.pos
    }

    fn size(&self) -> usize {
        self.data.len()
    }
}

