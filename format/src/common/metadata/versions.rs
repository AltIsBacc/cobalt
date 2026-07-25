use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Eq, Clone, Copy)]
#[repr(i32)]
pub enum SaveVersion {
    V1_4_5_6 = 319,
}

