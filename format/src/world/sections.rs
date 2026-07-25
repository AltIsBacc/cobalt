use derive_more::derive::Constructor;
use getset::{CopyGetters, Setters};

use crate::{common::codec::{BinaryDecode, BinaryEncode}, io::{BinaryReader, BinaryWriter}, world::error::{WorldLoadingError, WorldSavingError}};

#[derive(Debug, Constructor, CopyGetters, Setters, PartialEq, Eq, Clone, Copy)]
#[getset(get_copy = "pub", set = "pub")]
pub struct WorldSections {
    metadata: i32, // do not write to file!

    header: i32,
    tiles: i32,
    chests: i32,
    signs: i32,
    npcs: i32,
    tile_entities: i32,
    pressure_plates: i32,
    rooms: i32,
    bestiary: i32,
    journey: i32,
    footer: i32,
}

impl WorldSections {
    const SECTION_COUNT: usize = 11;
}

impl BinaryDecode for WorldSections {
    type Error = WorldLoadingError;
    type Context<'a> = ();

    fn decode<R: BinaryReader>(
        _ctx: Option<&Self::Context<'_>>,
        reader: &mut R
    ) -> Result<Self, Self::Error> {
        let pointers = reader
            .read_array(|r| r.i32())
            .map_err(|_| WorldLoadingError::MalformedData("sections"))?;

        if pointers.len() != Self::SECTION_COUNT {
            return Err(WorldLoadingError::MalformedData("sections"));
        }

        Ok(Self {
            metadata: 0, // always zero
            header: pointers[0],
            tiles: pointers[1],
            chests: pointers[2],
            signs: pointers[3],
            npcs: pointers[4],
            tile_entities: pointers[5],
            pressure_plates: pointers[6],
            rooms: pointers[7],
            bestiary: pointers[8],
            journey: pointers[9],
            footer: pointers[10],
        })
    }
}

impl BinaryEncode for WorldSections {
    type Error = WorldSavingError;
    type Context<'a> = ();

    fn encode<W: BinaryWriter>(
        &self,
        _ctx: Option<&Self::Context<'_>>,
        writer: &mut W
    ) -> Result<(), Self::Error> {
        let pointers = [
            self.header,
            self.tiles,
            self.chests,
            self.signs,
            self.npcs,
            self.tile_entities,
            self.pressure_plates,
            self.rooms,
            self.bestiary,
            self.journey,
            self.footer,
        ];

        writer
            .write_array(&pointers, |w, p| w.i32(*p))
            .map_err(|_| WorldSavingError::WriteFailed("sections"))?;

        Ok(())
    }
}

