use crate::error::Result;
use crate::reader::Reader;
use crate::world::frame_important::FrameImportant;

pub mod block;
pub mod liquid;
pub mod mechanicals;
pub mod paint;
pub mod wall;

use block::{Block, BlockShape, Frame};
use liquid::{Liquid, LiquidKind};
use mechanicals::{MechanicalState, Mechanicals};
use paint::Paint;
use wall::Wall;

const H1_HAS_H2: u8 = 1 << 0;
const H1_ACTIVE: u8 = 1 << 1;
const H1_WALL: u8 = 1 << 2;
const H1_LIQUID_MASK: u8 = 0b0001_1000;
const H1_WIDE_BLOCK: u8 = 1 << 5;
const H1_RLE_SHIFT: u8 = 6;

const H2_HAS_H3: u8 = 1 << 0;
const H2_RED_WIRE: u8 = 1 << 1;
const H2_BLUE_WIRE: u8 = 1 << 2;
const H2_GREEN_WIRE: u8 = 1 << 3;
const H2_SHAPE_MASK: u8 = 0b0111_0000;

const H3_HAS_H4: u8 = 1 << 0;
const H3_ACTUATOR: u8 = 1 << 1;
const H3_ACTUATED: u8 = 1 << 2;
const H3_BLOCK_PAINT: u8 = 1 << 3;
const H3_WALL_PAINT: u8 = 1 << 4;
const H3_YELLOW_WIRE: u8 = 1 << 5;
const H3_WIDE_WALL: u8 = 1 << 6;
const H3_SHIMMER: u8 = 1 << 7;

const H4_BLOCK_ECHO: u8 = 1 << 1;
const H4_WALL_ECHO: u8 = 1 << 2;
const H4_BLOCK_ILLUMINANT: u8 = 1 << 3;
const H4_WALL_ILLUMINANT: u8 = 1 << 4;

const BLOCK_ID_FRAME_V_ZERO: u16 = 144;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub block: Option<Block>,
    pub wall: Option<Wall>,
    pub liquid: Option<Liquid>,
    pub mechanicals: Mechanicals,
}

impl Tile {
    pub fn read(
        reader: &mut Reader<'_>,
        frame_important: &FrameImportant,
    ) -> Result<(Self, usize)> {
        let h1 = reader.u8()?;
        let h2 = if h1 & H1_HAS_H2 != 0 { reader.u8()? } else { 0 };
        let h3 = if h2 & H2_HAS_H3 != 0 { reader.u8()? } else { 0 };
        let h4 = if h3 & H3_HAS_H4 != 0 { reader.u8()? } else { 0 };

        let block = if h1 & H1_ACTIVE != 0 {
            let id = if h1 & H1_WIDE_BLOCK != 0 {
                reader.u16()?
            } else {
                u16::from(reader.u8()?)
            };

            let frame = if frame_important.contains(id) {
                let u = reader.i16()?;
                let v = reader.i16()?;
                Some(Frame { u, v: if id == BLOCK_ID_FRAME_V_ZERO { 0 } else { v } })
            } else {
                None
            };

            let paint = if h3 & H3_BLOCK_PAINT != 0 { Paint(reader.u8()?) } else { Paint::NONE };

            Some(Block {
                id,
                frame,
                shape: BlockShape::from((h2 & H2_SHAPE_MASK) >> 4),
                is_actuated: h3 & H3_ACTUATED != 0,
                is_illuminant: h4 & H4_BLOCK_ILLUMINANT != 0,
                is_echo: h4 & H4_BLOCK_ECHO != 0,
                paint,
            })
        } else {
            None
        };

        let wall = if h1 & H1_WALL != 0 {
            let mut id = u16::from(reader.u8()?);
            if h3 & H3_WIDE_WALL != 0 {
                id |= u16::from(reader.u8()?) << 8;
            }

            let paint = if h3 & H3_WALL_PAINT != 0 { Paint(reader.u8()?) } else { Paint::NONE };

            Some(Wall {
                id,
                is_illuminant: h4 & H4_WALL_ILLUMINANT != 0,
                is_echo: h4 & H4_WALL_ECHO != 0,
                paint,
            })
        } else {
            None
        };

        let liquid = match (h1 & H1_LIQUID_MASK) >> 3 {
            0 => None,
            bits => {
                let kind = if h3 & H3_SHIMMER != 0 {
                    LiquidKind::Shimmer
                } else {
                    LiquidKind::from_header_bits(bits)
                };
                Some(Liquid { kind, level: reader.u8()? })
            }
        };

        let mut mechanicals = Mechanicals::empty();
        mechanicals.set(MechanicalState::HasRedWire, h2 & H2_RED_WIRE != 0);
        mechanicals.set(MechanicalState::HasGreenWire, h2 & H2_GREEN_WIRE != 0);
        mechanicals.set(MechanicalState::HasBlueWire, h2 & H2_BLUE_WIRE != 0);
        mechanicals.set(MechanicalState::HasYellowWire, h3 & H3_YELLOW_WIRE != 0);
        mechanicals.set(MechanicalState::HasActuator, h3 & H3_ACTUATOR != 0);

        let repeat = match h1 >> H1_RLE_SHIFT {
            0 => 0,
            1 => usize::from(reader.u8()?),
            _ => usize::from(reader.u16()?),
        };

        Ok((Self { block, wall, liquid, mechanicals }, repeat))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::world::header::Size;
    use crate::world::tiles::Tiles;

    fn read(bytes: &[u8], frame_important: &FrameImportant) -> Result<(Tile, usize)> {
        Tile::read(&mut Reader::new(bytes), frame_important)
    }

    #[test]
    fn empty_tile_is_a_single_byte() {
        let (tile, repeat) = read(&[0], &FrameImportant::default()).unwrap();
        assert_eq!(tile, Tile::default());
        assert_eq!(repeat, 0);
    }

    #[test]
    fn decodes_wide_block_ids_and_paint() {
        let bytes =
            [H1_HAS_H2 | H1_ACTIVE | H1_WIDE_BLOCK, H2_HAS_H3, H3_BLOCK_PAINT, 0x2c, 0x01, 9];
        let (tile, _) = read(&bytes, &FrameImportant::default()).unwrap();
        let block = tile.block.unwrap();

        assert_eq!(block.id, 0x012c);
        assert_eq!(block.paint, Paint(9));
        assert!(tile.wall.is_none());
    }

    #[test]
    fn decodes_block_shape_from_header2() {
        let half = [H1_HAS_H2 | H1_ACTIVE, 0b0001_0000, 1];
        let slope = [H1_HAS_H2 | H1_ACTIVE, 0b0011_0000, 1];

        let (tile, _) = read(&half, &FrameImportant::default()).unwrap();
        assert_eq!(tile.block.unwrap().shape, BlockShape::HalfBrick);

        let (tile, _) = read(&slope, &FrameImportant::default()).unwrap();
        assert_eq!(tile.block.unwrap().shape, BlockShape::SlopeBottomLeft);
    }

    #[test]
    fn decodes_wall_liquid_and_wires() {
        let bytes = [
            H1_HAS_H2 | H1_WALL | (2 << 3),
            H2_HAS_H3 | H2_RED_WIRE,
            H3_SHIMMER | H3_YELLOW_WIRE,
            7,
            100,
        ];
        let (tile, _) = read(&bytes, &FrameImportant::default()).unwrap();

        assert_eq!(tile.wall.unwrap().id, 7);
        assert_eq!(tile.liquid, Some(Liquid { kind: LiquidKind::Shimmer, level: 100 }));
        assert!(tile.mechanicals.contains(MechanicalState::HasRedWire));
        assert!(tile.mechanicals.contains(MechanicalState::HasYellowWire));
        assert!(!tile.mechanicals.contains(MechanicalState::HasBlueWire));
    }

    #[test]
    fn decodes_both_rle_widths() {
        let (_, short) = read(&[1 << H1_RLE_SHIFT, 200], &FrameImportant::default()).unwrap();
        let (_, long) = read(&[2 << H1_RLE_SHIFT, 0x2c, 0x01], &FrameImportant::default()).unwrap();
        let (_, wide) = read(&[3 << H1_RLE_SHIFT, 0x2c, 0x01], &FrameImportant::default()).unwrap();

        assert_eq!((short, long, wide), (200, 300, 300));
    }

    #[test]
    fn tiles_expand_runs_within_columns() {
        let bytes = [H1_ACTIVE | (1 << H1_RLE_SHIFT), 5, 2, 1 << H1_RLE_SHIFT, 2];
        let mut reader = Reader::new(&bytes);
        let size = Size::try_new(2, 3).unwrap();
        let tiles = Tiles::read(&mut reader, size, &FrameImportant::default()).unwrap();

        assert_eq!(reader.remaining(), 0);
        assert_eq!(tiles.as_slice().len(), 6);
        assert_eq!(tiles.get(0, 0).unwrap().block.unwrap().id, 5);
        assert_eq!(tiles.get(0, 2).unwrap().block.unwrap().id, 5);
        assert_eq!(tiles.get(1, 1), Some(&Tile::default()));
        assert!(tiles.get(2, 0).is_none());
        assert!(tiles.get(0, 3).is_none());
    }

    #[test]
    fn tiles_clamp_runs_at_column_end() {
        let bytes = [1 << H1_RLE_SHIFT, 250, 0, 0];
        let mut reader = Reader::new(&bytes);
        let size = Size::try_new(2, 2).unwrap();
        let tiles = Tiles::read(&mut reader, size, &FrameImportant::default()).unwrap();

        assert_eq!(tiles.as_slice().len(), 4);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn truncated_tiles_fail() {
        let bytes = [H1_ACTIVE];
        let result = Tiles::read(
            &mut Reader::new(&bytes),
            Size::try_new(1, 1).unwrap(),
            &FrameImportant::default(),
        );
        assert!(matches!(result, Err(Error::UnexpectedEof { .. })));
    }
}
