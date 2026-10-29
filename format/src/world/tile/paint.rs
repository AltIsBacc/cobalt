#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Paint(pub u8);

impl Paint {
    pub const NONE: Self = Self(0);
}
