use enumflags2::{BitFlag, BitFlags};
use getset::MutGetters;

use crate::{common::{codec::{BinaryDecode, BinaryEncode}, metadata::{error::SaveMetadataError, flags::SaveFlags, kinds::SaveKind, signatures::SaveSignature, versions::SaveVersion}}, io::{BinaryReader, BinaryWriter, error::BinaryWriteError}};

pub mod versions;
pub mod signatures;
pub mod kinds;
pub mod flags;
pub mod error;

#[derive(Debug, MutGetters, PartialEq, Eq, Clone, Copy)]
#[getset(get_mut = "pub")]
pub struct SaveMetadata {
    version: SaveVersion,
    signature: SaveSignature,
    kind: SaveKind,
    revisions: i32,
    flags: BitFlags<SaveFlags, u64>,
}

impl BinaryDecode for SaveMetadata {
    type Error = SaveMetadataError;
    type Context<'a> = ();

    fn decode<R: BinaryReader>(
        _ctx: Option<&Self::Context<'_>>,
        reader: &mut R
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            version: SaveVersion::try_from(reader.i32()?)?,
            signature: SaveSignature::try_from(
                String::from_utf8_lossy(
                    reader.read_n(7)?
                ).as_ref()
            ).map_err(|e| SaveMetadataError::InvalidSignature(e))?,
            kind: SaveKind::try_from(reader.u8()?)?,
            revisions: reader.i32()?,
            flags: SaveFlags::from_bits_truncate(reader.u64()?),
        })
    }
}

impl BinaryEncode for SaveMetadata {
    type Error = BinaryWriteError;
    type Context<'a> = ();

    fn encode<W: BinaryWriter>(
        &self,
        _ctx: Option<&Self::Context<'_>>,
        writer: &mut W
    ) -> Result<(), Self::Error> {
        writer.i32(self.version.into())?;
        writer.write_bytes(self.signature.as_ref().as_bytes())?;
        writer.u8(self.kind.into())?;
        writer.i32(self.revisions)?;
        writer.u64(self.flags.bits())?;

        Ok(())
    }
}

