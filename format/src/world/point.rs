use crate::{error::Result, reader::Reader};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn read(reader: &mut Reader<'_>) -> Result<Self> {
        Ok(Self { x: reader.i32()?, y: reader.i32()? })
    }

    pub fn read_all(reader: &mut Reader<'_>) -> Result<Vec<Self>> {
        let count = reader.count_i32()?;
        reader.repeat(count, Self::read)
    }
}
