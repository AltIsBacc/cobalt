use derive_more::derive::Constructor;
use getset::MutGetters;
use num_enum::{FromPrimitive, IntoPrimitive};

use crate::{util::coordinates::UVCoordinates, world::tiles::tile::paint::BlockPaint};

#[derive(Debug, FromPrimitive, IntoPrimitive, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum BlockShape {
    #[num_enum(default)]
    Normal = 0,
    HalfBrick = 1,
    SlopeBottomRight = 2,
    SlopeBottomLeft = 3,
    SlopeTopRight = 4,
    SlopeTopLeft = 5,
}

impl Default for BlockShape {
    fn default() -> Self { BlockShape::Normal }
}

#[derive(Debug, MutGetters, Constructor, PartialEq, Eq, Clone, Copy, Default)]
#[getset(get_mut = "pub")]
pub struct Block {
    id: u16,
    frame: Option<UVCoordinates>, // u,v — only Some if this id is frame-important
    shape: BlockShape,
    is_actuated: bool,
    is_illuminant: bool,
    is_echo: bool,
    paint: BlockPaint,
}

