use num_enum::FromPrimitive;

use crate::world::tile::paint::Paint;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u8)]
pub enum BlockShape {
    #[default]
    Normal = 0,
    HalfBrick = 1,
    SlopeBottomRight = 2,
    SlopeBottomLeft = 3,
    SlopeTopRight = 4,
    SlopeTopLeft = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    pub u: i16,
    pub v: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    pub id: u16,
    pub frame: Option<Frame>,
    pub shape: BlockShape,
    pub is_actuated: bool,
    pub is_illuminant: bool,
    pub is_echo: bool,
    pub paint: Paint,
}
