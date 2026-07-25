use std::fmt;

use derive_more::derive::{Constructor, Deref, DerefMut};

use crate::{common::codec::{BinaryDecode, RLEBinaryDecode}, io::BinaryReader, util::coordinates::{Coordinates, Size}, world::{frame_important::WorldFrameImportant, tiles::{error::WorldTilesError, tile::Tile}}};

pub mod tile;
pub mod error;

#[derive(PartialEq, Eq, Clone)]
pub struct Tiles {
    width: usize,
    height: usize,
    tiles: Vec<Tile>,
}

impl fmt::Debug for Tiles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Tiles {{ width: {}, height: {}, tiles: [", self.width, self.height)?;

        if self.tiles.is_empty() {
            return writeln!(f, "] }}");
        }

        let mut i = 0;
        while i < self.tiles.len() {
            let current = &self.tiles[i];
            let mut count = 1;

            // count consecutive equal tiles in column-major order
            while i + count < self.tiles.len() && &self.tiles[i + count] == current {
                count += 1;
            }

            // also print the coordinate range for readability
            let start_idx = i;
            let end_idx = i + count - 1;
            let (sx, sy) = (start_idx / self.height, start_idx % self.height);
            let (ex, ey) = (end_idx / self.height, end_idx % self.height);

            if count > 1 {
                writeln!(f, " [{},{}]..=[{},{}] {:?} x{}", sx, sy, ex, ey, current, count)?;
            } else {
                writeln!(f, " [{},{}] {:?}", sx, sy, current)?;
            }

            i += count;
        }

        writeln!(f, "] }}")
    }
}

impl Tiles {
    pub fn with_default(width: usize, height: usize, default_tile: Tile) -> Self {
        Self {
            width,
            height,
            tiles: vec![default_tile; width * height],
        }
    }

    pub fn new(width: usize, height: usize) -> Self
    where
        Tile: Default,
    {
        Self::with_default(width, height, Tile::default())
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [Tile] {
        &mut self.tiles
    }

    #[inline(always)]
    pub fn get(&self, coords: &Coordinates<usize, usize>) -> Option<&Tile> {
        self.get_index(coords).map(|idx| unsafe {
            self.tiles.get_unchecked(idx)
        })
    }

    #[inline(always)]
    pub fn get_mut(&mut self, coords: &Coordinates<usize, usize>) -> Option<&mut Tile> {
        self.get_index(coords).map(|idx| unsafe {
            self.tiles.get_unchecked_mut(idx)
        })
    }

    #[inline(always)]
    fn get_index(&self, coords: &Coordinates<usize, usize>) -> Option<usize> {
        if coords.x() < self.width && coords.y() < self.height {
            Some(coords.x() * self.height + coords.y())
        } else {
            None
        }
    }
}

#[derive(Constructor)]
pub struct WorldTilesDecodeContext<'a> {
    size: &'a Size<i32, i32>,
    frame_important: &'a WorldFrameImportant
}

#[derive(Debug, Deref, DerefMut, PartialEq, Eq, Clone)]
pub struct WorldTiles(pub Tiles);

impl BinaryDecode for WorldTiles {
    type Error = WorldTilesError;
    type Context<'a> = WorldTilesDecodeContext<'a>;

    fn decode<R: BinaryReader>(
        ctx: Option<&Self::Context<'_>>,
        reader: &mut R,
    ) -> Result<Self, Self::Error> {
        let ctx = ctx.ok_or_else(|| anyhow::anyhow!("Failed to decode WorldTiles: context was None"))?;

        let width = ctx.size.width() as usize;
        let height = ctx.size.height() as usize;

        let mut tiles = Tiles::new(width, height);
        let tiles_slice = tiles.as_mut_slice();

        let mut x = 0;
        while x < width {
            let mut y = 0;
            let col_offset = x * height;

            while y < height {
                let i = col_offset + y;
                let (tile, repeat) = Tile::rle_decode(
                    Some(ctx.frame_important),
                    reader
                )?;
                tiles_slice[i] = tile.clone();

                if repeat > 0 {
                    let repeat_count = repeat as usize;
                    let start = i + 1;
                    
                    let end = (start + repeat_count).min(col_offset + height);
                    tiles_slice[start..end].fill(tile);
                    
                    y += repeat_count;
                }

                y += 1;
            }
            x += 1;
        }

        Ok(WorldTiles(tiles))
    }
}

