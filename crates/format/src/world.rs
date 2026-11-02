use std::{fs, path::Path};

use enumflags2::BitFlags;

use crate::{
    error::{Error, Result},
    metadata::{SaveKind, SaveMetadata, SaveSignature, SaveVersion},
    reader::Reader,
};

pub mod chest;
pub mod footer;
pub mod frame_important;
pub mod header;
pub mod item;
pub mod npcs;
pub mod point;
pub mod sections;
pub mod sign;
pub mod tile;
pub mod tile_entity;
pub mod tiles;

use chest::Chest;
use frame_important::FrameImportant;
use header::{Difficulty, Header, Size};
use npcs::{Npcs, Room};
use point::Point;
use sections::Sections;
use sign::Sign;
use tile_entity::TileEntity;
use tiles::Tiles;

#[derive(Debug)]
pub struct World {
    pub metadata: SaveMetadata,
    pub sections: Sections,
    pub frame_important: FrameImportant,
    pub header: Header,
    pub tiles: Tiles,
    pub chests: Vec<Chest>,
    pub signs: Vec<Sign>,
    pub npcs: Npcs,
    pub tile_entities: Vec<TileEntity>,
    pub pressure_plates: Vec<Point>,
    pub rooms: Vec<Room>,
}

fn expect_at(reader: &Reader<'_>, section: &'static str, expected: usize) -> Result {
    match reader.pos() {
        found if found == expected => Ok(()),
        found => Err(Error::SectionMismatch { section, expected, found }),
    }
}

impl World {
    pub fn new(name: impl Into<String>, size: Size, difficulty: Difficulty) -> Result<Self> {
        Ok(Self {
            metadata: SaveMetadata {
                version: SaveVersion::V1_4_5_6,
                signature: SaveSignature::Relogic,
                kind: SaveKind::World,
                revisions: 0,
                flags: BitFlags::empty(),
            },
            sections: Sections::default(),
            frame_important: FrameImportant::default(),
            header: Header::new(name, size, difficulty)?,
            tiles: Tiles::new(size),
            chests: Vec::new(),
            signs: Vec::new(),
            npcs: Npcs::default(),
            tile_entities: Vec::new(),
            pressure_plates: Vec::new(),
            rooms: Vec::new(),
        })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_bytes(&fs::read(path)?)
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut reader = Reader::new(data);

        let metadata = SaveMetadata::read(&mut reader)?;
        if metadata.kind != SaveKind::World {
            return Err(Error::UnexpectedKind { expected: SaveKind::World, found: metadata.kind });
        }

        let sections = Sections::read(&mut reader)?;
        let frame_important = FrameImportant::read(&mut reader)?;

        reader.seek(sections.header)?;
        let header = Header::read(&mut reader)?;
        expect_at(&reader, "header", sections.tiles)?;

        let tiles = Tiles::read(&mut reader, header.size, &frame_important)?;
        expect_at(&reader, "tiles", sections.chests)?;

        let chests = Chest::read_all(&mut reader)?;
        expect_at(&reader, "chests", sections.signs)?;

        let signs = Sign::read_all(&mut reader)?;
        expect_at(&reader, "signs", sections.npcs)?;

        let npcs = Npcs::read(&mut reader)?;
        expect_at(&reader, "npcs", sections.tile_entities)?;

        let tile_entities = TileEntity::read_all(&mut reader)?;
        expect_at(&reader, "tile entities", sections.pressure_plates)?;

        let pressure_plates = Point::read_all(&mut reader)?;
        expect_at(&reader, "pressure plates", sections.rooms)?;

        let rooms = Room::read_all(&mut reader)?;
        expect_at(&reader, "rooms", sections.bestiary)?;

        reader.seek(sections.footer)?;
        footer::verify(&mut reader, &header)?;

        Ok(Self {
            metadata,
            sections,
            frame_important,
            header,
            tiles,
            chests,
            signs,
            npcs,
            tile_entities,
            pressure_plates,
            rooms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PREFIX_LEN: usize = 72;

    const HEADER_TAIL_LEN: usize = 384;

    fn world_bytes(kind: u8) -> Vec<u8> {
        let mut header = Vec::new();
        header.extend([1, b'w']);
        header.push(0);
        header.extend(0u64.to_le_bytes());
        header.extend([0u8; 16]);
        header.extend(7i32.to_le_bytes());
        for bound in [0i32, 16, 0, 32] {
            header.extend(bound.to_le_bytes());
        }
        header.extend(2i32.to_le_bytes());
        header.extend(1i32.to_le_bytes());
        header.extend(1i32.to_le_bytes());
        header.extend([0u8; HEADER_TAIL_LEN]);

        let mut footer = vec![1, 1, b'w'];
        footer.extend(7i32.to_le_bytes());

        let sections: [Vec<u8>; Sections::COUNT] = [
            header,
            vec![0x40, 1],
            vec![0; 2],
            vec![0; 2],
            vec![0; 6],
            vec![0; 4],
            vec![0; 4],
            vec![0; 4],
            Vec::new(),
            Vec::new(),
            footer,
        ];

        let mut bytes = Vec::new();
        bytes.extend(319i32.to_le_bytes());
        bytes.extend(*b"relogic");
        bytes.push(kind);
        bytes.extend(0i32.to_le_bytes());
        bytes.extend(0u64.to_le_bytes());
        bytes.extend(11u16.to_le_bytes());
        let mut offset = PREFIX_LEN;
        for section in &sections {
            bytes.extend((offset as i32).to_le_bytes());
            offset += section.len();
        }
        bytes.extend(0u16.to_le_bytes());
        assert_eq!(bytes.len(), PREFIX_LEN);

        bytes.extend(sections.concat());
        bytes
    }

    #[test]
    fn loads_a_minimal_world() {
        let world = World::from_bytes(&world_bytes(2)).unwrap();

        assert_eq!(world.metadata.kind, SaveKind::World);
        assert_eq!(world.header.name, "w");
        assert_eq!(world.header.id, 7);
        assert_eq!(world.header.difficulty, Difficulty::Expert);
        assert_eq!(world.header.size, Size::new(1, 2).unwrap());
        assert_eq!(world.tiles.size(), world.header.size);
        assert!(world.tiles.get(0, 1).is_some());
        assert!(world.tiles.get(1, 0).is_none());
    }

    #[test]
    fn creates_an_empty_world() {
        let size = Size::new(4, 3).unwrap();
        let world = World::new("fresh", size, Difficulty::Master).unwrap();

        assert_eq!(world.header.name, "fresh");
        assert_eq!(world.header.difficulty, Difficulty::Master);
        assert_eq!(world.header.bounds.right, 64);
        assert_eq!(world.header.bounds.bottom, 48);
        assert_eq!(world.tiles.size(), size);
        assert_eq!(world.tiles.as_slice().len(), 12);
    }

    #[test]
    fn rejects_overlong_world_names() {
        let size = Size::new(1, 1).unwrap();
        assert!(matches!(
            World::new("x".repeat(28), size, Difficulty::Normal),
            Err(Error::NameTooLong { len: 28, max: 27 })
        ));
    }

    #[test]
    fn rejects_non_world_saves() {
        assert!(matches!(
            World::from_bytes(&world_bytes(3)),
            Err(Error::UnexpectedKind { found: SaveKind::Player, .. })
        ));
    }

    #[test]
    fn rejects_truncated_data() {
        let bytes = world_bytes(2);
        assert!(World::from_bytes(&bytes[..bytes.len() - 1]).is_err());
    }
}
