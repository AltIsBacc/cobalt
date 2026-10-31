use crate::{error::Result, reader::Reader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sign {
    pub x: i32,
    pub y: i32,
    pub text: String,
}

impl Sign {
    pub fn read_all(reader: &mut Reader<'_>) -> Result<Vec<Self>> {
        let count = reader.count_i16()?;
        reader.repeat(count, Self::read)
    }

    fn read(reader: &mut Reader<'_>) -> Result<Self> {
        let text = reader.string()?;
        Ok(Self { x: reader.i32()?, y: reader.i32()?, text })
    }
}
