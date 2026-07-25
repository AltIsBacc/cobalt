use derive_more::derive::Constructor;
use getset::MutGetters;
use num_enum::{FromPrimitive, IntoPrimitive};

#[derive(Debug, IntoPrimitive, FromPrimitive, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum LiquidKind {
    None = 0,
    Water = 1,
    Lava = 2,
    Honey = 3,
    Shimmer = 4,

    #[num_enum(default)]
    Unknown = 5
}

#[derive(Debug, MutGetters, Constructor, PartialEq, Eq, Clone, Copy)]
#[getset(get_mut = "pub")]
pub struct Liquid {
    kind: LiquidKind,
    level: u8
}

