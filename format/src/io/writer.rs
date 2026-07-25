use crate::io::{BinaryWriter, error::WriteResult};

pub struct BinaryBuffer {
    data: Vec<u8>,
    pos: usize,
}

impl BinaryBuffer {
    pub fn new() -> Self {
        Self { data: Vec::new(), pos: 0 }
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }
}

impl BinaryWriter for BinaryBuffer {
    fn seek(&mut self, pos: usize) {
        self.pos = pos;
        if self.data.len() < pos {
            self.data.resize(pos, 0);
        }
    }

    fn advance(&mut self, count: usize) {
        self.pos += count;
        if self.data.len() < self.pos {
            self.data.resize(self.pos, 0);
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> WriteResult<()> {
        let end = self.pos + bytes.len();

        if self.data.len() < end {
            self.data.resize(end, 0);
        }

        self.data[self.pos..end].copy_from_slice(bytes);
        self.pos = end;

        Ok(())
    }

    fn pos(&self) -> usize {
        self.pos
    }

    fn as_slice(&self) -> &[u8] {
        &self.data
    }
}

