use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(u8);

impl ClientId {
    pub(crate) const fn new(index: u8) -> Self {
        Self(index)
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
