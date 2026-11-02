use crate::{error::Result, reader::Reader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub net_id: i32,
    pub prefix: u8,
    pub stack: i16,
}

impl Item {
    pub fn read_chest_slot(reader: &mut Reader<'_>) -> Result<Option<Self>> {
        let stack = reader.i16()?;
        if stack <= 0 {
            return Ok(None);
        }
        Ok(Some(Self { net_id: reader.i32()?, prefix: reader.u8()?, stack }))
    }

    pub fn read_entity_slot(reader: &mut Reader<'_>) -> Result<Self> {
        Ok(Self { net_id: i32::from(reader.i16()?), prefix: reader.u8()?, stack: reader.i16()? })
    }

    pub fn read_entity_slots<const N: usize>(
        reader: &mut Reader<'_>,
        present: impl Fn(usize) -> bool,
    ) -> Result<[Option<Self>; N]> {
        let mut slots = [None; N];
        for (index, slot) in slots.iter_mut().enumerate() {
            if present(index) {
                *slot = Some(Self::read_entity_slot(reader)?);
            }
        }
        Ok(slots)
    }
}
