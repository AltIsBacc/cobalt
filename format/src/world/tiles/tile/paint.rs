use num_enum::{FromPrimitive, IntoPrimitive};

pub type BlockPaint = BlockPaintKind;
pub type WallPaint = WallPaintKind;

#[derive(Debug, FromPrimitive, IntoPrimitive, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum BlockPaintKind {
    #[num_enum(default)]
    Unpainted,
    Red
}

impl Default for BlockPaintKind {
    fn default() -> Self { BlockPaintKind::Unpainted }
}

#[derive(Debug, FromPrimitive, IntoPrimitive, PartialEq, Eq, Clone, Copy)]
#[repr(u16)]
pub enum WallPaintKind {
    #[num_enum(default)]
    Unpainted,
    Red
}

impl Default for WallPaintKind {
    fn default() -> Self { WallPaintKind::Unpainted }
}

