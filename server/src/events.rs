use crate::commands::ClientId;

/// Things the server broadcasts *after the fact*, for anyone listening:
/// mods, loggers, chat bridges, etc. Nobody replies to these -- see
/// `CobaltServerHandle::subscribe_events`.
#[derive(Debug, Clone)]
pub enum CobaltServerEvent {
    ClientJoined { id: ClientId },
    ClientLeft { id: ClientId },
    ChatMessage { from: ClientId, text: String },
}
