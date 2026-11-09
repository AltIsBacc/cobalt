use crate::model::ClientId;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GameEvent {
    PlayerEntering { id: ClientId },
    PlayerEntered { id: ClientId },
    PlayerLeft { id: ClientId },
    TileChanged { x: i32, y: i32 },
    TileBroken { by: ClientId, block: u16, x: i16, y: i16 },
}

#[derive(Debug, Default)]
pub(crate) struct Bus {
    queue: Vec<GameEvent>,
}

impl Bus {
    pub(crate) fn emit(&mut self, event: GameEvent) {
        self.queue.push(event);
    }

    pub(crate) fn take(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.queue)
    }

    pub(crate) fn clear(&mut self) {
        self.queue.clear();
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
