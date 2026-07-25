use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum SaveKind {
    Invalid = 0,
    Minimap = 1,
    World = 2,
    Player = 3,
}

