use derive_more::derive::{Deref, DerefMut};

use crate::{common::codec::{BinaryDecode, BinaryEncode}, io::{BinaryReader, BinaryWriter}, world::error::{WorldLoadingError, WorldSavingError}};

#[derive(Debug, Deref, DerefMut, PartialEq, Eq, Clone)]
pub struct WorldFrameImportant(pub Vec<bool>);

impl BinaryDecode for WorldFrameImportant {
    type Error = WorldLoadingError;
    type Context<'a> = ();

    fn decode<R: BinaryReader>(
        _ctx: Option<&Self::Context<'_>>,
        reader: &mut R
    ) -> Result<Self, Self::Error> {
        let count = reader
            .u16()
            .map_err(|_| WorldLoadingError::MalformedData("frame_important"))? as usize;

        let count_bytes = count.div_ceil(8);

        let mut value: Vec<bool> = Vec::with_capacity(count_bytes * 8);
        for _ in 0..count_bytes {
            let bits = reader
                .bits() // reads 1 byte -> [bool; 8]
                .map_err(|_| WorldLoadingError::MalformedData("frame_important"))?;
            value.extend_from_slice(&bits);
        }

        value.truncate(count);
        Ok(Self(value))
    }
}

impl BinaryEncode for WorldFrameImportant {
    type Error = WorldSavingError;
    type Context<'a> = ();

    fn encode<W: BinaryWriter>(
        &self,
        _ctx: Option<&Self::Context<'_>>,
        writer: &mut W
    ) -> Result<(), Self::Error> {
        writer
            .u16(self.len() as u16)
            .map_err(|_| WorldSavingError::WriteFailed("frame_important"))?;

        for chunk in self.chunks(8) {
            let mut byte = [false; 8];
            byte[..chunk.len()].copy_from_slice(chunk);
            writer
                .bits(byte)
                .map_err(|_| WorldSavingError::WriteFailed("frame_important"))?;
        }

        Ok(())
    }
}

