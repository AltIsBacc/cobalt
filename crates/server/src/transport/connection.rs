use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use cobalt_protocol::registry::{Connect, Disconnect};
use cobalt_protocol::{C2s, Frame, NetworkText, PROTOCOL_VERSION, Packet, ProtocolError, S2c};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time;

use super::hub::{Hub, Inbox, Traffic, stopped};
use crate::client::ClientId;
use crate::engine::Admission;
use crate::log::debug_log;

const READ_CHUNK_LEN: usize = 4096;
const WRITE_BATCH_LEN: usize = 64;
const FLUSH_TIMEOUT: Duration = Duration::from_secs(5);
const INCOMPATIBLE_VERSION: &str = "incompatible client version";

#[derive(Debug, Error)]
enum ConnectionError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    #[error("handshake timed out")]
    HandshakeTimeout,
}

struct Handshake {
    version_matches: bool,
    rest: Vec<u8>,
}

struct Inbound {
    reader: OwnedReadHalf,
    buffer: Vec<u8>,
}

impl Inbound {
    const fn new(reader: OwnedReadHalf) -> Self {
        Self { reader, buffer: Vec::new() }
    }

    async fn read_frames(&mut self) -> Result<Option<Vec<u8>>, ConnectionError> {
        self.buffer.reserve(READ_CHUNK_LEN);
        if self.reader.read_buf(&mut self.buffer).await? == 0 {
            return Ok(None);
        }
        Ok(Some(self.take_frames()?))
    }

    fn take_frames(&mut self) -> Result<Vec<u8>, ProtocolError> {
        let mut used = 0;
        while let Some((_, len)) = Frame::parse(&self.buffer[used..])? {
            used += len;
        }
        Ok(self.buffer.drain(..used).collect())
    }
}

pub(crate) async fn serve(socket: TcpStream, peer: SocketAddr, mut hub: Hub) {
    debug_log!("{peer} connected");
    if let Err(error) = socket.set_nodelay(true) {
        debug_log!("{peer} could not disable nagle: {error}");
    }

    let (reader, mut writer) = socket.into_split();
    let mut inbound = Inbound::new(reader);

    let handshake = match negotiate(&mut inbound, &mut hub).await {
        Ok(Some(handshake)) => handshake,
        Ok(None) => return,
        Err(error) => {
            debug_log!("{peer} ended: {error}");
            return;
        }
    };

    if !handshake.version_matches {
        refuse(&mut writer, INCOMPATIBLE_VERSION, peer).await;
        return;
    }

    let (queue, inbox) = mpsc::channel(hub.outbound_capacity);
    let (reply, admission) = oneshot::channel();
    if hub.traffic.send(Traffic::Join { queue, reply }).await.is_err() {
        return;
    }

    let id = match admission.await {
        Ok(Admission::Admitted(id)) => id,
        Ok(Admission::Refused(reason)) => {
            refuse(&mut writer, reason, peer).await;
            return;
        }
        Err(_) => return,
    };

    let mut write_task = tokio::spawn(write_loop(writer, inbox, peer));
    let result = relay(&mut inbound, id, handshake.rest, &hub, &mut write_task).await;
    debug_log!("{peer} ended: {result:?}");

    if hub.traffic.send(Traffic::Leave { id }).await.is_err() {
        debug_log!("{peer} left while the server was stopping");
    }
    if !write_task.is_finished() && time::timeout(FLUSH_TIMEOUT, &mut write_task).await.is_err() {
        debug_log!("{peer} flush timed out");
        write_task.abort();
    }
    debug_log!("{peer} left slot {id}");
}

async fn negotiate(
    inbound: &mut Inbound,
    hub: &mut Hub,
) -> Result<Option<Handshake>, ConnectionError> {
    let timeout = hub.handshake_timeout;

    tokio::select! {
        () = stopped(&mut hub.shutdown) => Ok(None),
        result = time::timeout(timeout, await_connect(inbound)) => {
            result.map_err(|_| ConnectionError::HandshakeTimeout)?
        }
    }
}

async fn await_connect(inbound: &mut Inbound) -> Result<Option<Handshake>, ConnectionError> {
    while let Some(bytes) = inbound.read_frames().await? {
        let mut used = 0;
        while let Some((frame, len)) = Frame::parse(&bytes[used..])? {
            used += len;
            if frame.kind != Connect::ID {
                continue;
            }
            if let Some(C2s::Connect(connect)) = C2s::decode(frame)? {
                return Ok(Some(Handshake {
                    version_matches: connect.version == PROTOCOL_VERSION,
                    rest: bytes[used..].to_vec(),
                }));
            }
        }
    }
    Ok(None)
}

async fn relay(
    inbound: &mut Inbound,
    id: ClientId,
    first: Vec<u8>,
    hub: &Hub,
    write_task: &mut JoinHandle<()>,
) -> Result<(), ConnectionError> {
    if !first.is_empty() && hub.traffic.send(Traffic::Input { id, frames: first }).await.is_err() {
        return Ok(());
    }

    loop {
        let read = tokio::select! {
            read = inbound.read_frames() => read?,
            _ = &mut *write_task => return Ok(()),
        };
        let Some(frames) = read else { return Ok(()) };
        if frames.is_empty() {
            continue;
        }
        if hub.traffic.send(Traffic::Input { id, frames }).await.is_err() {
            return Ok(());
        }
    }
}

async fn refuse(writer: &mut OwnedWriteHalf, reason: &str, peer: SocketAddr) {
    debug_log!("{peer} refused: {reason}");

    let packet = S2c::from(Disconnect { reason: NetworkText::Literal(reason) });
    let mut bytes = Vec::new();
    if let Err(error) = packet.write(&mut bytes) {
        debug_log!("{peer} refusal could not be encoded: {error}");
        return;
    }
    if let Err(error) = writer.write_all(&bytes).await {
        debug_log!("{peer} refusal could not be sent: {error}");
    }
}

async fn write_loop(mut writer: OwnedWriteHalf, mut inbox: Inbox, peer: SocketAddr) {
    let mut chunks = Vec::with_capacity(WRITE_BATCH_LEN);
    let mut batch = Vec::new();

    while inbox.recv_many(&mut chunks, WRITE_BATCH_LEN).await > 0 {
        let written = match chunks.as_slice() {
            [chunk] => writer.write_all(chunk).await,
            _ => {
                batch.clear();
                for chunk in &chunks {
                    batch.extend_from_slice(chunk);
                }
                writer.write_all(&batch).await
            }
        };
        chunks.clear();

        if let Err(error) = written {
            debug_log!("{peer} write failed: {error}");
            return;
        }
    }

    if let Err(error) = writer.shutdown().await {
        debug_log!("{peer} shutdown failed: {error}");
    }
}
