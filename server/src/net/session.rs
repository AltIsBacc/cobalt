use std::{net::SocketAddr, time::Duration};

use cobalt_protocol::{
    C2sPacket, Frame, Message, NetworkText, PROTOCOL_VERSION, ProtocolError, S2cPacket,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::watch,
    time,
};

use super::{
    error::{SessionError, SessionResult},
    profile,
};
use crate::{
    client::ClientId, error::ServerError, handle::ServerHandle, log::debug_log, player::Player,
};

const READ_CHUNK_LEN: usize = 4096;

pub(crate) async fn serve(
    socket: TcpStream,
    peer: SocketAddr,
    server: ServerHandle,
    handshake_timeout: Duration,
    mut shutdown: watch::Receiver<()>,
) {
    debug_log!("{peer} connected");
    let mut connection = Connection::new(socket, peer);

    let handshake = tokio::select! {
        _ = shutdown.changed() => return,
        result = time::timeout(handshake_timeout, connection.accept_connect()) => result,
    };
    match handshake {
        Ok(Ok(true)) => {}
        Ok(Ok(false)) => {
            debug_log!("{peer} closed during handshake");
            return;
        }
        Ok(Err(error)) => {
            debug_log!("{peer} handshake failed: {error}");
            return;
        }
        Err(_) => {
            debug_log!("{peer} handshake timed out");
            return;
        }
    }

    let id = match server.join().await {
        Ok(id) => id,
        Err(ServerError::Full) => {
            let _ = connection.refuse("the server is full").await;
            return;
        }
        Err(error) => {
            debug_log!("{peer} could not join: {error}");
            return;
        }
    };
    debug_log!("{peer} joined as slot {id}");

    tokio::select! {
        _ = shutdown.changed() => debug_log!("{peer} closed by shutdown"),
        result = connection.run(id, &server, handshake_timeout) => debug_log!("{peer} session ended: {result:?}"),
    }

    let _ = server.leave(id).await;
    debug_log!("{peer} left slot {id}");
}

struct Connection {
    socket: TcpStream,
    peer: SocketAddr,
    inbound: Vec<u8>,
    consumed: usize,
    outbound: Vec<u8>,
}

impl Connection {
    fn new(socket: TcpStream, peer: SocketAddr) -> Self {
        Self { socket, peer, inbound: Vec::new(), consumed: 0, outbound: Vec::new() }
    }

    async fn accept_connect(&mut self) -> SessionResult<bool> {
        match self.next_packet().await? {
            Some(C2sPacket::Connect { version }) if version == PROTOCOL_VERSION => Ok(true),
            Some(C2sPacket::Connect { version }) => {
                self.refuse(&format!("incompatible client version: {version}")).await?;
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    async fn run(
        &mut self,
        id: ClientId,
        server: &ServerHandle,
        timeout: Duration,
    ) -> SessionResult {
        self.send(&S2cPacket::SetUserSlot { player_id: id.get() }).await?;

        let collected = time::timeout(timeout, self.collect_player())
            .await
            .map_err(|_| SessionError::HandshakeTimeout)??;
        let Some(player) = collected else { return Ok(()) };

        if !player.has_valid_name() {
            return self.refuse("invalid player name").await;
        }

        server.login(id, player).await?;
        self.refuse("world data is not available yet").await
    }

    async fn collect_player(&mut self) -> SessionResult<Option<Player>> {
        let mut player = Player::default();
        loop {
            match self.next_packet().await? {
                None => return Ok(None),
                Some(C2sPacket::RequestWorldData) => return Ok(Some(player)),
                Some(packet) => profile::absorb(&mut player, packet)?,
            }
        }
    }

    async fn refuse(&mut self, reason: &str) -> SessionResult {
        debug_log!("{} refused: {reason}", self.peer);
        self.send(&S2cPacket::Disconnect { reason: NetworkText::literal(reason) }).await
    }

    async fn send(&mut self, packet: &S2cPacket) -> SessionResult {
        debug_log!("{} -> {packet:?}", self.peer);
        self.outbound.clear();
        packet.encode(&mut self.outbound)?;
        self.socket.write_all(&self.outbound).await?;
        Ok(())
    }

    async fn next_packet(&mut self) -> SessionResult<Option<C2sPacket>> {
        loop {
            if let Some(packet) = self.decode_buffered()? {
                return Ok(Some(packet));
            }

            self.inbound.drain(..self.consumed);
            self.consumed = 0;
            self.inbound.reserve(READ_CHUNK_LEN);
            if self.socket.read_buf(&mut self.inbound).await? == 0 {
                return Ok(None);
            }
        }
    }

    fn decode_buffered(&mut self) -> SessionResult<Option<C2sPacket>> {
        while let Some((frame, used)) = Frame::parse(&self.inbound[self.consumed..])? {
            self.consumed += used;
            match C2sPacket::decode(frame) {
                Ok(packet) => {
                    debug_log!("{} <- {packet:?}", self.peer);
                    return Ok(Some(packet));
                }
                Err(ProtocolError::UnknownMessage(kind)) => {
                    debug_log!("{} <- unknown message {kind} ({used} bytes)", self.peer);
                }
                Err(error) => {
                    debug_log!("{} <- invalid packet: {error}", self.peer);
                    return Err(error.into());
                }
            }
        }
        Ok(None)
    }
}
