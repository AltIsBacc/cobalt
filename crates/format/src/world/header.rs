use std::collections::BTreeSet;

use num_enum::TryFromPrimitive;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    reader::Reader,
    world::point::Point,
};

pub mod flags;
pub mod state;

use flags::WorldFlag;
use state::{Angler, Backgrounds, Clock, Invasion, Locations, OreTiers, Timers, Weather};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(i32)]
pub enum Difficulty {
    #[default]
    Normal = 0,
    Expert = 1,
    Master = 2,
    Creative = 3,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Generator {
    pub seed: String,
    pub version: u64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Header {
    pub name: String,
    pub generator: Generator,
    pub uuid: Uuid,
    pub id: i32,
    pub bounds: Bounds,
    pub size: Size,
    pub difficulty: Difficulty,
    pub flags: BTreeSet<WorldFlag>,
    pub created_at: i64,
    pub last_played: i64,
    pub clock: Clock,
    pub locations: Locations,
    pub backgrounds: Backgrounds,
    pub invasion: Invasion,
    pub weather: Weather,
    pub ore_tiers: OreTiers,
    pub timers: Timers,
    pub angler: Angler,
    pub shadow_orb_count: u8,
    pub altar_count: i32,
    pub killed_mobs: Vec<i32>,
    pub claimable_banners: Vec<u16>,
    pub party_npcs: Vec<i32>,
    pub team_spawns: Vec<Point>,
    pub manifest: String,
}

const TILE_PIXELS: usize = 16;

fn pixels(tiles: usize) -> i32 {
    i32::try_from(tiles.saturating_mul(TILE_PIXELS)).unwrap_or(i32::MAX)
}

fn read_flags(
    reader: &mut Reader<'_>,
    flags: &mut BTreeSet<WorldFlag>,
    order: &[WorldFlag],
) -> Result {
    for &flag in order {
        if reader.bool()? {
            flags.insert(flag);
        }
    }
    Ok(())
}

impl Header {
    pub const MAX_NAME_LEN: usize = 27;

    pub fn new(name: impl Into<String>, size: Size, difficulty: Difficulty) -> Result<Self> {
        let name = name.into();
        Self::check_name(&name)?;

        Ok(Self {
            name,
            bounds: Bounds {
                left: 0,
                right: pixels(size.width),
                top: 0,
                bottom: pixels(size.height),
            },
            size,
            difficulty,
            ..Self::default()
        })
    }

    fn check_name(name: &str) -> Result {
        if name.len() > Self::MAX_NAME_LEN {
            return Err(Error::NameTooLong { len: name.len(), max: Self::MAX_NAME_LEN });
        }
        Ok(())
    }

    pub fn read(reader: &mut Reader<'_>) -> Result<Self> {
        let mut header = Self::read_identity(reader)?;
        header.read_layout(reader)?;
        header.read_world_state(reader)?;
        header.read_population(reader)?;
        header.read_events(reader)?;
        header.read_extras(reader)?;
        Ok(header)
    }

    fn read_identity(reader: &mut Reader<'_>) -> Result<Self> {
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

        let mut flags = BTreeSet::new();
        read_flags(
            reader,
            &mut flags,
            &[
                WorldFlag::Drunk,
                WorldFlag::GetGood,
                WorldFlag::TenthAnniversary,
                WorldFlag::DontStarve,
                WorldFlag::NotTheBees,
                WorldFlag::Remix,
                WorldFlag::NoTraps,
                WorldFlag::Zenith,
                WorldFlag::Skyblock,
            ],
        )?;

        Ok(Self { name, generator, uuid, id, bounds, size, difficulty, flags, ..Self::default() })
    }

    fn read_layout(&mut self, reader: &mut Reader<'_>) -> Result {
        self.created_at = reader.i64()?;
        self.last_played = reader.i64()?;
        self.clock.moon_type = reader.u8()?;

        let backgrounds = &mut self.backgrounds;
        backgrounds.tree_x = reader.i32s()?;
        backgrounds.tree_style = reader.i32s()?;
        backgrounds.cave_back_x = reader.i32s()?;
        backgrounds.cave_back_style = reader.i32s()?;
        backgrounds.ice_back_style = reader.i32()?;
        backgrounds.jungle_back_style = reader.i32()?;
        backgrounds.hell_back_style = reader.i32()?;
        Ok(())
    }

    fn read_world_state(&mut self, reader: &mut Reader<'_>) -> Result {
        self.locations.spawn = Point::read(reader)?;
        self.locations.surface = reader.f64()?;
        self.locations.rock_layer = reader.f64()?;
        self.clock.time = reader.f64()?;
        self.read_flags(reader, &[WorldFlag::DayTime])?;
        self.clock.moon_phase = reader.i32()?;
        self.read_flags(reader, &[WorldFlag::BloodMoon, WorldFlag::Eclipse])?;
        self.locations.dungeon = Point::read(reader)?;
        self.read_flags(
            reader,
            &[
                WorldFlag::Crimson,
                WorldFlag::EyeOfCthulhu,
                WorldFlag::EaterOfWorlds,
                WorldFlag::Skeletron,
                WorldFlag::QueenBee,
                WorldFlag::Destroyer,
                WorldFlag::Twins,
                WorldFlag::SkeletronPrime,
                WorldFlag::MechBossAny,
                WorldFlag::Plantera,
                WorldFlag::Golem,
                WorldFlag::SlimeKing,
                WorldFlag::SavedGoblin,
                WorldFlag::SavedWizard,
                WorldFlag::SavedMechanic,
                WorldFlag::Goblins,
                WorldFlag::Clown,
                WorldFlag::FrostLegion,
                WorldFlag::Pirates,
                WorldFlag::ShadowOrbSmashed,
                WorldFlag::SpawnMeteor,
            ],
        )?;
        self.shadow_orb_count = reader.u8()?;
        self.altar_count = reader.i32()?;
        self.read_flags(reader, &[WorldFlag::HardMode, WorldFlag::PartyOfDoom])?;

        self.invasion.delay = reader.i32()?;
        self.invasion.size = reader.i32()?;
        self.invasion.kind = reader.i32()?;
        self.invasion.x = reader.f64()?;
        self.weather.slime_rain_time = reader.f64()?;
        self.timers.sundial = reader.u8()?;

        self.read_flags(reader, &[WorldFlag::Raining])?;
        self.weather.rain_time = reader.i32()?;
        self.weather.max_rain = reader.f32()?;
        self.ore_tiers.cobalt = reader.i32()?;
        self.ore_tiers.mythril = reader.i32()?;
        self.ore_tiers.adamantite = reader.i32()?;

        let backgrounds = &mut self.backgrounds;
        [
            backgrounds.forest,
            backgrounds.corruption,
            backgrounds.jungle,
            backgrounds.snow,
            backgrounds.hallow,
            backgrounds.crimson,
            backgrounds.desert,
            backgrounds.ocean,
        ] = reader.array::<8>()?;

        self.weather.cloud_bg_active = reader.i32()?;
        self.weather.num_clouds = reader.i16()?;
        self.weather.wind_speed = reader.f32()?;
        Ok(())
    }

    fn read_population(&mut self, reader: &mut Reader<'_>) -> Result {
        let finished = reader.count_i32()?;
        self.angler.finished_today = reader.repeat(finished, Reader::string)?;
        self.read_flags(reader, &[WorldFlag::SavedAngler])?;
        self.angler.quest = reader.i32()?;
        self.read_flags(
            reader,
            &[WorldFlag::SavedStylist, WorldFlag::SavedTaxCollector, WorldFlag::SavedGolfer],
        )?;
        self.invasion.size_start = reader.i32()?;
        self.timers.cultist_delay = reader.i32()?;

        let killed = reader.count_i16()?;
        self.killed_mobs = reader.repeat(killed, Reader::i32)?;
        let banners = reader.count_i16()?;
        self.claimable_banners = reader.repeat(banners, Reader::u16)?;

        self.read_flags(
            reader,
            &[
                WorldFlag::FastForwardToDawn,
                WorldFlag::Fishron,
                WorldFlag::Martians,
                WorldFlag::LunaticCultist,
                WorldFlag::MoonLord,
                WorldFlag::HalloweenKing,
                WorldFlag::HalloweenTree,
                WorldFlag::ChristmasQueen,
                WorldFlag::Santa,
                WorldFlag::ChristmasTree,
                WorldFlag::SolarPillar,
                WorldFlag::VortexPillar,
                WorldFlag::NebulaPillar,
                WorldFlag::StardustPillar,
                WorldFlag::SolarPillarActive,
                WorldFlag::VortexPillarActive,
                WorldFlag::NebulaPillarActive,
                WorldFlag::StardustPillarActive,
                WorldFlag::LunarApocalypse,
                WorldFlag::PartyManual,
                WorldFlag::PartyGenuine,
            ],
        )?;
        self.timers.party = reader.i32()?;
        let party = reader.count_i32()?;
        self.party_npcs = reader.repeat(party, Reader::i32)?;
        Ok(())
    }

    fn read_events(&mut self, reader: &mut Reader<'_>) -> Result {
        self.read_flags(reader, &[WorldFlag::SandstormHappening])?;
        self.weather.sandstorm.time_left = reader.i32()?;
        self.weather.sandstorm.severity = reader.f32()?;
        self.weather.sandstorm.intended_severity = reader.f32()?;
        self.read_flags(
            reader,
            &[
                WorldFlag::SavedBartender,
                WorldFlag::Dd2InvasionT1,
                WorldFlag::Dd2InvasionT2,
                WorldFlag::Dd2InvasionT3,
            ],
        )?;

        let backgrounds = &mut self.backgrounds;
        [
            backgrounds.mushroom,
            backgrounds.underworld,
            backgrounds.forest2,
            backgrounds.forest3,
            backgrounds.forest4,
        ] = reader.array::<5>()?;

        self.read_flags(reader, &[WorldFlag::CombatBookUsed])?;
        self.timers.lantern_night = reader.i32()?;
        self.read_flags(
            reader,
            &[
                WorldFlag::LanternNightGenuine,
                WorldFlag::LanternNightManual,
                WorldFlag::LanternNightNextGenuine,
            ],
        )?;

        let tree_tops = reader.count_i32()?;
        self.backgrounds.tree_tops = reader.repeat(tree_tops, Reader::i32)?;
        self.read_flags(reader, &[WorldFlag::ForceHalloween, WorldFlag::ForceXmas])?;

        self.ore_tiers.copper = reader.i32()?;
        self.ore_tiers.iron = reader.i32()?;
        self.ore_tiers.silver = reader.i32()?;
        self.ore_tiers.gold = reader.i32()?;
        Ok(())
    }

    fn read_extras(&mut self, reader: &mut Reader<'_>) -> Result {
        self.read_flags(
            reader,
            &[
                WorldFlag::BoughtCat,
                WorldFlag::BoughtDog,
                WorldFlag::BoughtBunny,
                WorldFlag::EmpressOfLight,
                WorldFlag::QueenSlime,
                WorldFlag::Deerclops,
                WorldFlag::UnlockedSlimeBlue,
                WorldFlag::UnlockedMerchant,
                WorldFlag::UnlockedDemolitionist,
                WorldFlag::UnlockedPartyGirl,
                WorldFlag::UnlockedDyeTrader,
                WorldFlag::UnlockedTruffle,
                WorldFlag::UnlockedArmsDealer,
                WorldFlag::UnlockedNurse,
                WorldFlag::UnlockedPrincess,
                WorldFlag::CombatBookTwoUsed,
                WorldFlag::PeddlersSatchelUsed,
                WorldFlag::UnlockedSlimeGreen,
                WorldFlag::UnlockedSlimeOld,
                WorldFlag::UnlockedSlimePurple,
                WorldFlag::UnlockedSlimeRainbow,
                WorldFlag::UnlockedSlimeRed,
                WorldFlag::UnlockedSlimeYellow,
                WorldFlag::UnlockedSlimeCopper,
                WorldFlag::FastForwardToDusk,
            ],
        )?;
        self.timers.moondial = reader.u8()?;
        self.read_flags(
            reader,
            &[
                WorldFlag::ForceHalloweenForever,
                WorldFlag::ForceXmasForever,
                WorldFlag::VampireSeed,
                WorldFlag::InfectedSeed,
            ],
        )?;
        self.weather.meteor_shower_count = reader.i32()?;
        self.weather.coin_rain = reader.i32()?;
        self.read_flags(reader, &[WorldFlag::TeamBasedSpawns])?;

        let spawns = usize::from(reader.u8()?);
        self.team_spawns = reader.repeat(spawns, |reader| {
            Ok(Point { x: i32::from(reader.i16()?), y: i32::from(reader.i16()?) })
        })?;

        self.read_flags(reader, &[WorldFlag::DualDungeons])?;
        self.manifest = reader.string()?;
        Ok(())
    }

    fn read_flags(&mut self, reader: &mut Reader<'_>, order: &[WorldFlag]) -> Result {
        read_flags(reader, &mut self.flags, order)
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
