use crate::error::Result;
use crate::reader::Reader;
use crate::world::point::Point;

#[derive(Debug, Clone, PartialEq)]
pub struct TownNpc {
    pub id: i32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub homeless: bool,
    pub home: Point,
    pub variation: Option<i32>,
    pub homeless_despawn: bool,
}

impl TownNpc {
    fn read(reader: &mut Reader<'_>) -> Result<Self> {
        let id = reader.i32()?;
        let name = reader.string()?;
        let x = reader.f32()?;
        let y = reader.f32()?;
        let homeless = reader.bool()?;
        let home = Point::read(reader)?;
        let variation = if reader.u8()? & 1 != 0 { Some(reader.i32()?) } else { None };
        let homeless_despawn = reader.bool()?;
        Ok(Self { id, name, x, y, homeless, home, variation, homeless_despawn })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mob {
    pub id: i32,
    pub x: f32,
    pub y: f32,
}

impl Mob {
    fn read(reader: &mut Reader<'_>) -> Result<Self> {
        Ok(Self { id: reader.i32()?, x: reader.f32()?, y: reader.f32()? })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Npcs {
    pub shimmered: Vec<i32>,
    pub town: Vec<TownNpc>,
    pub mobs: Vec<Mob>,
}

impl Npcs {
    pub fn read(reader: &mut Reader<'_>) -> Result<Self> {
        let shimmered_count = reader.count_i32()?;
        let shimmered = reader.repeat(shimmered_count, Reader::i32)?;
        let town = Self::read_flagged(reader, TownNpc::read)?;
        let mobs = Self::read_flagged(reader, Mob::read)?;
        Ok(Self { shimmered, town, mobs })
    }

    fn read_flagged<T>(
        reader: &mut Reader<'_>,
        read: impl Fn(&mut Reader<'_>) -> Result<T>,
    ) -> Result<Vec<T>> {
        let mut items = Vec::new();
        while reader.bool()? {
            items.push(read(reader)?);
        }
        Ok(items)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Room {
    pub npc_id: i32,
    pub home: Point,
}

impl Room {
    pub fn read_all(reader: &mut Reader<'_>) -> Result<Vec<Self>> {
        let count = reader.count_i32()?;
        reader
            .repeat(count, |reader| Ok(Self { npc_id: reader.i32()?, home: Point::read(reader)? }))
    }
}
