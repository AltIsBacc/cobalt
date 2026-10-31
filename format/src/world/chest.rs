use crate::{error::Result, reader::Reader, world::item::Item};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chest {
    pub x: i32,
    pub y: i32,
    pub name: String,
    pub items: Vec<Option<Item>>,
}

impl Chest {
    pub fn read_all(reader: &mut Reader<'_>) -> Result<Vec<Self>> {
        let count = reader.count_i16()?;
        reader.repeat(count, Self::read)
    }

    fn read(reader: &mut Reader<'_>) -> Result<Self> {
        let x = reader.i32()?;
        let y = reader.i32()?;
        let name = reader.string()?;
        let slots = reader.count_i32()?;
        let items = reader.repeat(slots, Item::read_chest_slot)?;
        Ok(Self { x, y, name, items })
    }
}
