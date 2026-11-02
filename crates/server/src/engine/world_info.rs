use std::collections::BTreeSet;

use cobalt_format::World;
use cobalt_format::world::header::Header;
use cobalt_format::world::header::flags::WorldFlag;
use cobalt_format::world::header::state::OreTiers;
use cobalt_format::world::point::Point;
use cobalt_protocol::registry::WorldInfo;
use cobalt_protocol::{SpawnPoint, SpawnPoints};

const WORLD_FLAG_BYTES: usize = 11;
const TREE_TOP_COUNT: usize = 13;

#[derive(Debug, Clone, Copy)]
struct FlagBit {
    byte: usize,
    bit: u8,
}

impl FlagBit {
    const fn at(byte: usize, bit: u8) -> Self {
        Self { byte, bit }
    }

    fn set(self, bytes: &mut [u8; WORLD_FLAG_BYTES]) {
        bytes[self.byte] |= 1 << self.bit;
    }
}

const CLOUD_BACKGROUND: FlagBit = FlagBit::at(1, 4);
const SLIME_RAIN: FlagBit = FlagBit::at(2, 2);

pub(super) fn build(world: &World) -> WorldInfo<'_> {
    let header = &world.header;
    let backgrounds = &header.backgrounds;
    let weather = &header.weather;

    WorldInfo {
        time: header.clock.time as i32,
        day_flags: day_flags(&header.flags),
        moon_phase: narrow(header.clock.moon_phase),
        max_tiles_x: narrow(header.size.width),
        max_tiles_y: narrow(header.size.height),
        spawn_x: narrow(header.locations.spawn.x),
        spawn_y: narrow(header.locations.spawn.y),
        surface: header.locations.surface as i16,
        rock_layer: header.locations.rock_layer as i16,
        world_id: header.id,
        name: &header.name,
        game_mode: header.difficulty as u8,
        uuid: header.uuid.to_bytes_le(),
        generator_version: header.generator.version,
        moon_type: header.clock.moon_type,
        backgrounds: [
            backgrounds.forest,
            backgrounds.forest2,
            backgrounds.forest3,
            backgrounds.forest4,
            backgrounds.corruption,
            backgrounds.jungle,
            backgrounds.snow,
            backgrounds.hallow,
            backgrounds.crimson,
            backgrounds.desert,
            backgrounds.ocean,
            backgrounds.mushroom,
            backgrounds.underworld,
        ],
        ice_back_style: narrow(backgrounds.ice_back_style),
        jungle_back_style: narrow(backgrounds.jungle_back_style),
        hell_back_style: narrow(backgrounds.hell_back_style),
        wind_speed_target: weather.wind_speed,
        num_clouds: narrow(weather.num_clouds),
        tree_x: backgrounds.tree_x,
        tree_style: narrow_all(backgrounds.tree_style),
        cave_back_x: backgrounds.cave_back_x,
        cave_back_style: narrow_all(backgrounds.cave_back_style),
        tree_tops: tree_tops(&backgrounds.tree_tops),
        max_rain: weather.max_rain,
        flags: world_flags(header),
        sundial_cooldown: header.timers.sundial,
        moondial_cooldown: header.timers.moondial,
        ore_tiers: ore_tiers(&header.ore_tiers),
        invasion_type: narrow(header.invasion.kind),
        lobby_id: 0,
        sandstorm_severity: weather.sandstorm.severity,
        spawn_points: spawn_points(&header.team_spawns),
        dungeon_x: narrow(header.locations.dungeon.x),
        dungeon_y: narrow(header.locations.dungeon.y),
    }
}

pub(super) fn narrow<T, U: TryFrom<T> + Default>(value: T) -> U {
    U::try_from(value).unwrap_or_default()
}

fn narrow_all<T: Copy, U: TryFrom<T> + Default, const N: usize>(values: [T; N]) -> [U; N] {
    values.map(narrow)
}

fn day_flags(flags: &BTreeSet<WorldFlag>) -> u8 {
    u8::from(flags.contains(&WorldFlag::DayTime))
        | u8::from(flags.contains(&WorldFlag::BloodMoon)) << 1
        | u8::from(flags.contains(&WorldFlag::Eclipse)) << 2
}

fn tree_tops(values: &[i32]) -> [u8; TREE_TOP_COUNT] {
    let mut tops = [0; TREE_TOP_COUNT];
    for (top, &value) in tops.iter_mut().zip(values) {
        *top = narrow(value);
    }
    tops
}

fn ore_tiers(tiers: &OreTiers) -> [i16; 7] {
    [
        tiers.copper,
        tiers.iron,
        tiers.silver,
        tiers.gold,
        tiers.cobalt,
        tiers.mythril,
        tiers.adamantite,
    ]
    .map(narrow)
}

fn spawn_points(points: &[Point]) -> SpawnPoints {
    SpawnPoints(
        points.iter().map(|point| SpawnPoint { x: narrow(point.x), y: narrow(point.y) }).collect(),
    )
}

fn world_flags(header: &Header) -> [u8; WORLD_FLAG_BYTES] {
    let mut bytes = [0; WORLD_FLAG_BYTES];
    for &flag in &header.flags {
        if let Some(position) = flag_bit(flag) {
            position.set(&mut bytes);
        }
    }
    if header.weather.cloud_bg_active != 0 {
        CLOUD_BACKGROUND.set(&mut bytes);
    }
    if header.weather.slime_rain_time > 0.0 {
        SLIME_RAIN.set(&mut bytes);
    }
    bytes
}

fn flag_bit(flag: WorldFlag) -> Option<FlagBit> {
    use WorldFlag::*;

    let position = match flag {
        ShadowOrbSmashed => FlagBit::at(0, 0),
        EyeOfCthulhu => FlagBit::at(0, 1),
        EaterOfWorlds => FlagBit::at(0, 2),
        Skeletron => FlagBit::at(0, 3),
        HardMode => FlagBit::at(0, 4),
        Clown => FlagBit::at(0, 5),
        Plantera => FlagBit::at(0, 7),
        Destroyer => FlagBit::at(1, 0),
        Twins => FlagBit::at(1, 1),
        SkeletronPrime => FlagBit::at(1, 2),
        MechBossAny => FlagBit::at(1, 3),
        Crimson => FlagBit::at(1, 5),
        FastForwardToDawn => FlagBit::at(2, 1),
        SlimeKing => FlagBit::at(2, 3),
        QueenBee => FlagBit::at(2, 4),
        Fishron => FlagBit::at(2, 5),
        Martians => FlagBit::at(2, 6),
        LunaticCultist => FlagBit::at(2, 7),
        MoonLord => FlagBit::at(3, 0),
        HalloweenKing => FlagBit::at(3, 1),
        HalloweenTree => FlagBit::at(3, 2),
        ChristmasQueen => FlagBit::at(3, 3),
        Santa => FlagBit::at(3, 4),
        ChristmasTree => FlagBit::at(3, 5),
        Golem => FlagBit::at(3, 6),
        PartyManual | PartyGenuine => FlagBit::at(3, 7),
        Pirates => FlagBit::at(4, 0),
        FrostLegion => FlagBit::at(4, 1),
        Goblins => FlagBit::at(4, 2),
        SandstormHappening => FlagBit::at(4, 3),
        Dd2InvasionT1 => FlagBit::at(4, 5),
        Dd2InvasionT2 => FlagBit::at(4, 6),
        Dd2InvasionT3 => FlagBit::at(4, 7),
        CombatBookUsed => FlagBit::at(5, 0),
        LanternNightGenuine | LanternNightManual => FlagBit::at(5, 1),
        SolarPillar => FlagBit::at(5, 2),
        VortexPillar => FlagBit::at(5, 3),
        NebulaPillar => FlagBit::at(5, 4),
        StardustPillar => FlagBit::at(5, 5),
        ForceHalloween | ForceHalloweenForever => FlagBit::at(5, 6),
        ForceXmas | ForceXmasForever => FlagBit::at(5, 7),
        BoughtCat => FlagBit::at(6, 0),
        BoughtDog => FlagBit::at(6, 1),
        BoughtBunny => FlagBit::at(6, 2),
        Drunk => FlagBit::at(6, 4),
        EmpressOfLight => FlagBit::at(6, 5),
        QueenSlime => FlagBit::at(6, 6),
        GetGood => FlagBit::at(6, 7),
        TenthAnniversary => FlagBit::at(7, 0),
        DontStarve => FlagBit::at(7, 1),
        Deerclops => FlagBit::at(7, 2),
        NotTheBees => FlagBit::at(7, 3),
        Remix => FlagBit::at(7, 4),
        UnlockedSlimeBlue => FlagBit::at(7, 5),
        CombatBookTwoUsed => FlagBit::at(7, 6),
        _ => return None,
    };
    Some(position)
}
