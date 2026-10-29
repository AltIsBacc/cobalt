use crate::error::{ServerError, ServerResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerConfig {
    pub tick_rate: u32,
    pub max_players: u8,
    pub command_capacity: usize,
    pub event_capacity: usize,
}

impl ServerConfig {
    pub const MAX_TICK_RATE: u32 = 1000;

    pub(crate) fn validate(&self) -> ServerResult {
        validate_tick_rate(self.tick_rate)?;

        if self.max_players == 0 {
            return Err(ServerError::InvalidConfig("max_players must be at least 1"));
        }
        if self.command_capacity == 0 {
            return Err(ServerError::InvalidConfig("command_capacity must be at least 1"));
        }
        if self.event_capacity == 0 {
            return Err(ServerError::InvalidConfig("event_capacity must be at least 1"));
        }

        Ok(())
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { tick_rate: 60, max_players: 8, command_capacity: 256, event_capacity: 256 }
    }
}

pub(crate) fn validate_tick_rate(rate: u32) -> ServerResult {
    if (1..=ServerConfig::MAX_TICK_RATE).contains(&rate) {
        Ok(())
    } else {
        Err(ServerError::InvalidConfig("tick_rate must be between 1 and 1000"))
    }
}
