use std::fmt;
use std::ops::Range;

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
        let mut tiles = Self::new(size);

        for x in 0..size.width {
            let mut y = 0;
            while y < size.height {
                let (tile, repeat) = Tile::read(reader, frame_important)?;
                let end = (y + repeat + 1).min(size.height);
                for row in y..end {
                    tiles.data[row * size.width + x] = tile;
                }
                y = end;
            }
        }

        Ok(tiles)
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

    /// Tiles in row-major order, the layout of a network tile section.
    #[must_use]
    pub fn as_slice(&self) -> &[Tile] {
        &self.data
    }

    pub fn write_region(
        &self,
        columns: Range<usize>,
        rows: Range<usize>,
        frame_important: &FrameImportant,
        out: &mut Vec<u8>,
    ) {
        let Size { width, height } = self.size;
        let columns = columns.start.min(width)..columns.end.min(width);
        let rows = rows.start.min(height)..rows.end.min(height);

        let mut tiles = rows.flat_map(|y| self.data[y * width..][columns.clone()].iter());
        let Some(mut current) = tiles.next() else { return };
        let mut run = 0u16;
        for tile in tiles {
            if tile == current && run < u16::MAX && current.allows_batching() {
                run += 1;
            } else {
                current.write(out, frame_important, run);
                current = tile;
                run = 0;
            }
        }
        current.write(out, frame_important, run);
    }

    fn index(&self, x: usize, y: usize) -> Option<usize> {
        (x < self.size.width && y < self.size.height).then(|| y * self.size.width + x)
    }
}

impl fmt::Debug for Tiles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tiles").field("size", &self.size).finish_non_exhaustive()
    }
}
