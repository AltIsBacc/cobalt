use crate::{
    error::{Error, Result},
    reader::Reader,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Sections {
    pub header: usize,
    pub tiles: usize,
    pub chests: usize,
    pub signs: usize,
    pub npcs: usize,
    pub tile_entities: usize,
    pub pressure_plates: usize,
    pub rooms: usize,
    pub bestiary: usize,
    pub journey: usize,
    pub footer: usize,
}

impl Sections {
    pub const COUNT: usize = 11;

    pub fn read(reader: &mut Reader<'_>) -> Result<Self> {
        let found = usize::from(reader.u16()?);
        if found != Self::COUNT {
            return Err(Error::SectionCount { expected: Self::COUNT, found });
        }

        let mut pointers = [0usize; Self::COUNT];
        for slot in &mut pointers {
            let raw = reader.i32()?;
            *slot = usize::try_from(raw).map_err(|_| Error::NegativeSection(raw))?;
        }

        let [
            header,
            tiles,
            chests,
            signs,
            npcs,
            tile_entities,
            pressure_plates,
            rooms,
            bestiary,
            journey,
            footer,
        ] = pointers;

        Ok(Self {
            header,
            tiles,
            chests,
            signs,
            npcs,
            tile_entities,
            pressure_plates,
            rooms,
            bestiary,
            journey,
            footer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(count: u16, pointers: &[i32]) -> Vec<u8> {
        let mut bytes = count.to_le_bytes().to_vec();
        for pointer in pointers {
            bytes.extend(pointer.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn reads_pointers_in_order() {
        let pointers: Vec<i32> = (1..=11).collect();
        let sections = Sections::read(&mut Reader::new(&bytes(11, &pointers))).unwrap();

        assert_eq!(sections.header, 1);
        assert_eq!(sections.tiles, 2);
        assert_eq!(sections.footer, 11);
    }

    #[test]
    fn rejects_wrong_count() {
        let pointers: Vec<i32> = (1..=10).collect();
        assert!(matches!(
            Sections::read(&mut Reader::new(&bytes(10, &pointers))),
            Err(Error::SectionCount { expected: 11, found: 10 })
        ));
    }

    #[test]
    fn rejects_negative_pointers() {
        let mut pointers: Vec<i32> = (1..=11).collect();
        pointers[4] = -8;
        assert!(matches!(
            Sections::read(&mut Reader::new(&bytes(11, &pointers))),
            Err(Error::NegativeSection(-8))
        ));
    }
}
