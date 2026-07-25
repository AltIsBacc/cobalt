use derive_more::derive::Constructor;
use getset::{Getters, Setters};
use uuid::Uuid;

use crate::{common::codec::BinaryDecode, io::BinaryReader, util::{bounds::Bounds, coordinates::Size}, world::header::{difficulty::Difficulty, error::WorldHeaderError, generator_info::GeneratorInfo}};

pub mod generator_info;
pub mod difficulty;

pub mod error;

#[derive(Debug, Constructor, Getters, Setters, PartialEq, Eq, Clone)]
#[getset(get = "pub", set = "pub")]
pub struct WorldHeader {
    name: String,
    generator_info: GeneratorInfo,
    uuid: Uuid,
    id: i32,
    bounds: Bounds,
    size: Size,
    difficulty: Difficulty,
}

impl WorldHeader {
    const MAX_WORLD_NAME: usize = 27;
}

impl BinaryDecode for WorldHeader {
    type Error = WorldHeaderError;
    type Context<'a> = ();

    fn decode<R: BinaryReader>(
        _ctx: Option<&Self::Context<'_>>,
        reader: &mut R
    ) -> Result<Self, Self::Error> {
        let name = reader.string()?;
        if name.len() > Self::MAX_WORLD_NAME {
            return Err(WorldHeaderError::NameTooLong)
        }

        Ok(Self {
            name,
            generator_info: GeneratorInfo::new(
                reader.string()?,
                reader.u64()?
            ),
            uuid: reader.uuid()?,
            id: reader.i32()?,
            bounds: Bounds::new(
                reader.i32()?, reader.i32()?,
                reader.i32()?, reader.i32()?,
            ),
            size: Size::new_rev(reader.i32()?, reader.i32()?),
            difficulty: Difficulty::try_from(reader.i32()?)?,
        })
    }
}

