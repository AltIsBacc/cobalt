#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiquidKind {
    Water,
    Lava,
    Honey,
    Shimmer,
}

impl LiquidKind {
    pub(crate) const fn header_bits(self) -> u8 {
        match self {
            Self::Water | Self::Shimmer => 1,
            Self::Lava => 2,
            Self::Honey => 3,
        }
    }

    pub(crate) const fn from_header_bits(bits: u8) -> Self {
        match bits {
            1 => Self::Water,
            2 => Self::Lava,
            _ => Self::Honey,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Liquid {
    pub kind: LiquidKind,
    pub level: u8,
}
