use num_enum::TryFromPrimitive;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    reader::Reader,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(i32)]
pub enum Difficulty {
    Normal = 0,
    Expert = 1,
    Master = 2,
    Creative = 3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generator {
    pub seed: String,
    pub version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: usize,
    pub height: usize,
}

impl Size {
    pub const MAX_TILES: usize = 25_000_000;

    pub fn new(width: i32, height: i32) -> Result<Self> {
        match (usize::try_from(width), usize::try_from(height)) {
            (Ok(w), Ok(h)) if w.checked_mul(h).is_some_and(|n| n <= Self::MAX_TILES) => {
                Ok(Self { width: w, height: h })
            }
            _ => Err(Error::InvalidSize { width, height }),
        }
    }

    pub const fn area(self) -> usize {
        self.width * self.height
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub name: String,
    pub generator: Generator,
    pub uuid: Uuid,
    pub id: i32,
    pub bounds: Bounds,
    pub size: Size,
    pub difficulty: Difficulty,
}

const TILE_PIXELS: usize = 16;

fn pixels(tiles: usize) -> i32 {
    i32::try_from(tiles.saturating_mul(TILE_PIXELS)).unwrap_or(i32::MAX)
}

impl Header {
    pub const MAX_NAME_LEN: usize = 27;

    pub fn new(name: impl Into<String>, size: Size, difficulty: Difficulty) -> Result<Self> {
        let name = name.into();
        Self::check_name(&name)?;

        Ok(Self {
            name,
            generator: Generator { seed: String::new(), version: 0 },
            uuid: Uuid::nil(),
            id: 0,
            bounds: Bounds {
                left: 0,
                right: pixels(size.width),
                top: 0,
                bottom: pixels(size.height),
            },
            size,
            difficulty,
        })
    }

    fn check_name(name: &str) -> Result {
        if name.len() > Self::MAX_NAME_LEN {
            return Err(Error::NameTooLong { len: name.len(), max: Self::MAX_NAME_LEN });
        }
        Ok(())
    }

    pub fn read(reader: &mut Reader<'_>) -> Result<Self> {
        let name = reader.string()?;
        Self::check_name(&name)?;

        let generator = Generator { seed: reader.string()?, version: reader.u64()? };
        let uuid = reader.uuid()?;
        let id = reader.i32()?;
        let bounds = Bounds {
            left: reader.i32()?,
            right: reader.i32()?,
            top: reader.i32()?,
            bottom: reader.i32()?,
        };

        let height = reader.i32()?;
        let width = reader.i32()?;
        let size = Size::new(width, height)?;

        let difficulty = Difficulty::try_from(reader.i32()?)
            .map_err(|e| Error::UnknownDifficulty(e.number))?;

        Ok(Self { name, generator, uuid, id, bounds, size, difficulty })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_rejects_negative_and_oversized_worlds() {
        assert!(Size::new(8400, 2400).is_ok());
        assert!(Size::new(0, 0).is_ok());
        assert!(matches!(Size::new(-1, 10), Err(Error::InvalidSize { .. })));
        assert!(matches!(Size::new(10, -1), Err(Error::InvalidSize { .. })));
        assert!(matches!(Size::new(i32::MAX, i32::MAX), Err(Error::InvalidSize { .. })));
    }
}
