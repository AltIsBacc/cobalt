use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Status {
    pub world_name: String,
    pub players: usize,
    pub max_players: usize,
    pub tick_rate: u32,
    pub ticks: u64,
    pub uptime: Duration,
}
