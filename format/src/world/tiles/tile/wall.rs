use derive_more::derive::Constructor;
use getset::MutGetters;

use crate::world::tiles::tile::paint::WallPaint;

#[derive(Debug, MutGetters, Constructor, PartialEq, Eq, Clone, Copy)]
#[getset(get_mut = "pub")]
pub struct Wall {
    id: u16,
    is_illuminant: bool,
    is_echo: bool,
    paint: WallPaint,
}

