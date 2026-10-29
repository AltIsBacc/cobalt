/// A client's identity from the server's point of view. The networking
/// layer mints these when a connection is accepted.
pub type ClientId = u8;

/// Placeholder for the real packet type. `cobalt-protocol` doesn't define
pub type ClientPacket = Vec<u8>;

/// One reply is expected per command (see `CobaltResponse`).
#[derive(Debug, Clone)]
pub enum CobaltCommand {
    ClientJoin,

    /// A client's connection has closed.
    ClientLeave { id: ClientId, },

    /// A raw packet arrived from a client, still needing to be interpreted.
    ClientPacket {
        id: ClientId,
        packet: ClientPacket,
    },
}

#[derive(Debug, Clone)]
pub enum CobaltResponse {
    Success { response: CobaltSuccessResponseData },
    Failed { response: CobaltFailedResponseData },
}

#[derive(Debug, Clone)]
pub enum CobaltSuccessResponseData {
    Acknowledged,

    ClientJoined { id: ClientId },
}

#[derive(Debug, Clone)]
pub enum CobaltFailedResponseData {
    Reason(String),
}

