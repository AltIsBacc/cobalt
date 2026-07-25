use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Eq, Clone, Copy)]
#[repr(i32)]
pub enum Difficulty {
    Normal,
    Expert,
    Master
}

