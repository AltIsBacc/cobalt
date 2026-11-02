use crate::world::point::Point;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Clock {
    pub time: f64,
    pub moon_phase: i32,
    pub moon_type: u8,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Locations {
    pub spawn: Point,
    pub dungeon: Point,
    pub surface: f64,
    pub rock_layer: f64,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Backgrounds {
    pub tree_x: [i32; 3],
    pub tree_style: [i32; 4],
    pub cave_back_x: [i32; 3],
    pub cave_back_style: [i32; 4],
    pub ice_back_style: i32,
    pub jungle_back_style: i32,
    pub hell_back_style: i32,
    pub forest: u8,
    pub corruption: u8,
    pub jungle: u8,
    pub snow: u8,
    pub hallow: u8,
    pub crimson: u8,
    pub desert: u8,
    pub ocean: u8,
    pub mushroom: u8,
    pub underworld: u8,
    pub forest2: u8,
    pub forest3: u8,
    pub forest4: u8,
    pub tree_tops: Vec<i32>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Invasion {
    pub delay: i32,
    pub size: i32,
    pub size_start: i32,
    pub kind: i32,
    pub x: f64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Sandstorm {
    pub time_left: i32,
    pub severity: f32,
    pub intended_severity: f32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Weather {
    pub rain_time: i32,
    pub max_rain: f32,
    pub slime_rain_time: f64,
    pub cloud_bg_active: i32,
    pub num_clouds: i16,
    pub wind_speed: f32,
    pub sandstorm: Sandstorm,
    pub meteor_shower_count: i32,
    pub coin_rain: i32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OreTiers {
    pub copper: i32,
    pub iron: i32,
    pub silver: i32,
    pub gold: i32,
    pub cobalt: i32,
    pub mythril: i32,
    pub adamantite: i32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Timers {
    pub sundial: u8,
    pub moondial: u8,
    pub lantern_night: i32,
    pub party: i32,
    pub cultist_delay: i32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Angler {
    pub quest: i32,
    pub finished_today: Vec<String>,
}
