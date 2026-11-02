use std::fmt;

use crate::error::Result;
use crate::reader::Reader;
use crate::world::frame_important::FrameImportant;
use crate::world::header::Size;
use crate::world::tile::Tile;

#[derive(Clone, PartialEq, Eq)]
pub struct Tiles {
    size: Size,
    data: Vec<Tile>,
}

impl Tiles {
    pub fn new(size: Size) -> Self {
        Self { size, data: vec![Tile::default(); size.area()] }
    }

    pub fn read(
        reader: &mut Reader<'_>,
        size: Size,
        frame_important: &FrameImportant,
    ) -> Result<Self> {
        let mut data = Vec::with_capacity(size.area());

        for _ in 0..size.width {
            let mut remaining = size.height;
            while remaining > 0 {
                let (tile, repeat) = Tile::read(reader, frame_important)?;
                let run = (repeat + 1).min(remaining);
                data.resize(data.len() + run, tile);
                remaining -= run;
            }
        }

        Ok(Self { size, data })
    }

    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    #[must_use]
    pub fn get(&self, x: usize, y: usize) -> Option<&Tile> {
        self.index(x, y).and_then(|i| self.data.get(i))
    }

    #[must_use]
    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Tile> {
        self.index(x, y).and_then(|i| self.data.get_mut(i))
    }

    #[must_use]
    pub fn as_slice(&self) -> &[Tile] {
        &self.data
    }

    fn index(&self, x: usize, y: usize) -> Option<usize> {
        (x < self.size.width && y < self.size.height).then(|| x * self.size.height + y)
    }
}

impl fmt::Debug for Tiles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tiles").field("size", &self.size).finish_non_exhaustive()
    }
}
