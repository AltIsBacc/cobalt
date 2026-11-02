use num_enum::TryFromPrimitive;

use crate::{
    error::{Error, Result},
    reader::Reader,
    world::item::Item,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
enum Kind {
    TrainingDummy,
    ItemFrame,
    LogicSensor,
    DisplayDoll,
    WeaponRack,
    HatRack,
    FoodPlatter,
    TeleportationPylon,
    DeadCellsDisplayJar,
    KiteAnchor,
    CritterAnchor,
}

const fn bit(mask: u8, index: usize) -> bool {
    mask >> index & 1 != 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayDoll {
    pub pose: u8,
    pub items: [Option<Item>; 9],
    pub dyes: [Option<Item>; 9],
    pub misc: Option<Item>,
}

impl DisplayDoll {
    const EXTRA_MISC: u8 = 0b001;
    const EXTRA_NINTH_ITEM: u8 = 0b010;
    const EXTRA_NINTH_DYE: u8 = 0b100;

    fn read(reader: &mut Reader<'_>) -> Result<Self> {
        let item_mask = reader.u8()?;
        let dye_mask = reader.u8()?;
        let pose = reader.u8()?;
        let extra = reader.u8()?;

        let items = Item::read_entity_slots(reader, |index| match index {
            8 => extra & Self::EXTRA_NINTH_ITEM != 0,
            _ => bit(item_mask, index),
        })?;
        let dyes = Item::read_entity_slots(reader, |index| match index {
            8 => extra & Self::EXTRA_NINTH_DYE != 0,
            _ => bit(dye_mask, index),
        })?;
        let [misc] = Item::read_entity_slots(reader, |_| extra & Self::EXTRA_MISC != 0)?;

        Ok(Self { pose, items, dyes, misc })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HatRack {
    pub items: [Option<Item>; 2],
    pub dyes: [Option<Item>; 2],
}

impl HatRack {
    fn read(reader: &mut Reader<'_>) -> Result<Self> {
        let mask = reader.u8()?;
        let items = Item::read_entity_slots(reader, |index| bit(mask, index))?;
        let dyes = Item::read_entity_slots(reader, |index| bit(mask, index + 2))?;
        Ok(Self { items, dyes })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileEntityData {
    TrainingDummy { npc: i16 },
    ItemFrame(Item),
    LogicSensor { check: u8, enabled: bool },
    DisplayDoll(DisplayDoll),
    WeaponRack(Item),
    HatRack(HatRack),
    FoodPlatter(Item),
    TeleportationPylon,
    DeadCellsDisplayJar(Item),
    KiteAnchor { item: i16 },
    CritterAnchor { item: i16 },
}

impl TileEntityData {
    fn read(reader: &mut Reader<'_>, kind: u8) -> Result<Self> {
        let kind = Kind::try_from(kind).map_err(|e| Error::UnknownTileEntity(e.number))?;
        Ok(match kind {
            Kind::TrainingDummy => Self::TrainingDummy { npc: reader.i16()? },
            Kind::ItemFrame => Self::ItemFrame(Item::read_entity_slot(reader)?),
            Kind::LogicSensor => Self::LogicSensor { check: reader.u8()?, enabled: reader.bool()? },
            Kind::DisplayDoll => Self::DisplayDoll(DisplayDoll::read(reader)?),
            Kind::WeaponRack => Self::WeaponRack(Item::read_entity_slot(reader)?),
            Kind::HatRack => Self::HatRack(HatRack::read(reader)?),
            Kind::FoodPlatter => Self::FoodPlatter(Item::read_entity_slot(reader)?),
            Kind::TeleportationPylon => Self::TeleportationPylon,
            Kind::DeadCellsDisplayJar => Self::DeadCellsDisplayJar(Item::read_entity_slot(reader)?),
            Kind::KiteAnchor => Self::KiteAnchor { item: reader.i16()? },
            Kind::CritterAnchor => Self::CritterAnchor { item: reader.i16()? },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileEntity {
    pub id: i32,
    pub x: i16,
    pub y: i16,
    pub data: TileEntityData,
}

impl TileEntity {
    pub fn read_all(reader: &mut Reader<'_>) -> Result<Vec<Self>> {
        let count = reader.count_i32()?;
        reader.repeat(count, Self::read)
    }

    fn read(reader: &mut Reader<'_>) -> Result<Self> {
        let kind = reader.u8()?;
        let id = reader.i32()?;
        let x = reader.i16()?;
        let y = reader.i16()?;
        let data = TileEntityData::read(reader, kind)?;
        Ok(Self { id, x, y, data })
    }
}
