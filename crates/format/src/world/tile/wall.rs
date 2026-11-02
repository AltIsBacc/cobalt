use crate::world::tile::paint::Paint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wall {
    pub id: u16,
    pub is_illuminant: bool,
    pub is_echo: bool,
    pub paint: Paint,
}
