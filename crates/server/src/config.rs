use std::net::SocketAddr;
use std::time::Duration;

use crate::error::{ServerError, ServerResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerConfig {
    pub tick_rate: u32,
    pub max_players: u8,
    pub outbound_capacity: usize,
    pub event_capacity: usize,
    pub listen: Option<SocketAddr>,
    pub handshake_timeout: Duration,
    pub engine_core: Option<usize>,
}

impl ServerConfig {
    pub const MAX_TICK_RATE: u32 = 1000;

    pub(crate) fn validate(&self) -> ServerResult {
        validate_tick_rate(self.tick_rate)?;

        if self.max_players == 0 {
            return Err(ServerError::InvalidConfig("max_players must be at least 1"));
        }
        if self.outbound_capacity == 0 {
            return Err(ServerError::InvalidConfig("outbound_capacity must be at least 1"));
        }
        if self.event_capacity == 0 {
            return Err(ServerError::InvalidConfig("event_capacity must be at least 1"));
        }
        if self.handshake_timeout.is_zero() {
            return Err(ServerError::InvalidConfig("handshake_timeout must be non-zero"));
        }
        if let Some(core) = self.engine_core {
            validate_core(core)?;
        }

        Ok(())
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            tick_rate: 60,
            max_players: 8,
            outbound_capacity: 256,
            event_capacity: 256,
            listen: None,
            handshake_timeout: Duration::from_secs(10),
            engine_core: None,
        }
    }
}

fn validate_core(core: usize) -> ServerResult {
    let available = core_affinity::get_core_ids()
        .ok_or(ServerError::InvalidConfig("core pinning is not supported on this platform"))?;

    if available.iter().any(|candidate| candidate.id == core) {
        Ok(())
    } else {
        Err(ServerError::InvalidConfig("engine_core is not an available core"))
    }
}

pub(crate) fn validate_tick_rate(rate: u32) -> ServerResult {
    if (1..=ServerConfig::MAX_TICK_RATE).contains(&rate) {
        Ok(())
    } else {
        Err(ServerError::InvalidConfig("tick_rate must be between 1 and 1000"))
    }
}
