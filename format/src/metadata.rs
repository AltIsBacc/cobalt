use enumflags2::{BitFlags, bitflags};
use num_enum::TryFromPrimitive;

use crate::{
    error::{Error, Result},
    reader::Reader,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(i32)]
pub enum SaveVersion {
    V1_4_5_6 = 319,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveSignature {
    Relogic,
    Xindong,
}

impl SaveSignature {
    pub const fn from_bytes(bytes: &[u8; 7]) -> Option<Self> {
        match bytes {
            b"relogic" => Some(Self::Relogic),
            b"xindong" => Some(Self::Xindong),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum SaveKind {
    Invalid = 0,
    Minimap = 1,
    World = 2,
    Player = 3,
}

#[bitflags]
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveFlags {
    Favorite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveMetadata {
    pub version: SaveVersion,
    pub signature: SaveSignature,
    pub kind: SaveKind,
    pub revisions: i32,
    pub flags: BitFlags<SaveFlags>,
}

impl SaveMetadata {
    pub fn read(reader: &mut Reader<'_>) -> Result<Self> {
        let version = SaveVersion::try_from(reader.i32()?)
            .map_err(|e| Error::UnsupportedVersion(e.number))?;

        let raw_signature = reader.array::<7>()?;
        let signature = SaveSignature::from_bytes(&raw_signature)
            .ok_or(Error::UnknownSignature(raw_signature))?;

        let kind = SaveKind::try_from(reader.u8()?).map_err(|e| Error::UnknownKind(e.number))?;
        let revisions = reader.i32()?;
        let flags = BitFlags::from_bits_truncate(reader.u64()?);

        Ok(Self { version, signature, kind, revisions, flags })
    }
}
