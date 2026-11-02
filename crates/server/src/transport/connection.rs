use std::io;
use std::mem;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use cobalt_protocol::s2c::Disconnect;
use cobalt_protocol::{Frame, NetworkText, ProtocolError, write_s2c};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time;

use super::shared::{Queue, Shared, stopped};
use crate::client::ClientId;
use crate::engine::{Admission, ClientError, Flow};
use crate::log::debug_log;

const READ_CHUNK_LEN: usize = 4096;
const WRITE_BATCH_LEN: usize = 64;
const FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Error)]
enum ConnectionError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    #[error(transparent)]
    Client(#[from] ClientError),

    #[error("handshake timed out")]
    HandshakeTimeout,
}

type Inbox = mpsc::Receiver<Arc<[u8]>>;

struct Pending {
    writer: OwnedWriteHalf,
    queue: Option<Queue>,
    inbox: Inbox,
}

enum Link {
    Pending(Pending),
    Admitted { id: ClientId, writer: JoinHandle<()> },
    Closed,
}

#[derive(Default)]
struct Inbound {
    buffer: Vec<u8>,
    start: usize,
}

impl Inbound {
    fn pending(&self) -> &[u8] {
        &self.buffer[self.start..]
    }

    const fn advance(&mut self, used: usize) {
        self.start += used;
    }

    fn compact(&mut self) {
        self.buffer.drain(..self.start);
        self.start = 0;
        self.buffer.reserve(READ_CHUNK_LEN);
    }
}

struct Connection {
    shared: Arc<Shared>,
    peer: SocketAddr,
    reader: OwnedReadHalf,
    link: Link,
}

pub(crate) async fn serve(socket: TcpStream, peer: SocketAddr, shared: Arc<Shared>) {
    debug_log!("{peer} connected");
    if let Err(error) = socket.set_nodelay(true) {
        debug_log!("{peer} could not disable nagle: {error}");
    }

    let mut shutdown = shared.shutdown_signal();
    let mut connection = Connection::new(socket, peer, shared);

    tokio::select! {
        () = stopped(&mut shutdown) => debug_log!("{peer} closed by shutdown"),
        result = connection.run() => debug_log!("{peer} ended: {result:?}"),
    }

    connection.finish().await;
}

impl Connection {
    fn new(socket: TcpStream, peer: SocketAddr, shared: Arc<Shared>) -> Self {
        let (reader, writer) = socket.into_split();
        let (queue, inbox) = mpsc::channel(shared.outbound_capacity());
        let link = Link::Pending(Pending { writer, queue: Some(queue), inbox });

        Self { shared, peer, reader, link }
    }

    async fn run(&mut self) -> Result<(), ConnectionError> {
        time::timeout(self.shared.handshake_timeout(), self.pump())
            .await
            .map_err(|_| ConnectionError::HandshakeTimeout)?
    }

    async fn pump(&mut self) -> Result<(), ConnectionError> {
        let mut inbound = Inbound::default();

        loop {
            while let Some((frame, used)) = Frame::parse(inbound.pending())? {
                let flow = self.deliver(frame).await?;
                inbound.advance(used);
                if flow == Flow::Close {
                    return Ok(());
                }
            }

            inbound.compact();
            if self.fill(&mut inbound.buffer).await? == 0 {
                return Ok(());
            }
        }
    }

    async fn deliver(&mut self, frame: Frame<'_>) -> Result<Flow, ConnectionError> {
        match &mut self.link {
            Link::Admitted { id, .. } => Ok(self.shared.receive(*id, frame)?),
            Link::Pending(pending) => match self.shared.admit(frame, &mut pending.queue)? {
                Admission::Waiting => Ok(Flow::Open),
                Admission::Admitted(id) => {
                    self.promote(id);
                    Ok(Flow::Open)
                }
                Admission::Refused(reason) => {
                    self.refuse(reason).await?;
                    Ok(Flow::Close)
                }
            },
            Link::Closed => Ok(Flow::Close),
        }
    }

    fn promote(&mut self, id: ClientId) {
        if let Link::Pending(Pending { writer, inbox, .. }) =
            mem::replace(&mut self.link, Link::Closed)
        {
            let task = tokio::spawn(write_loop(writer, inbox, self.peer));
            self.link = Link::Admitted { id, writer: task };
        }
    }

    async fn refuse(&mut self, reason: &str) -> Result<(), ConnectionError> {
        debug_log!("{} refused: {reason}", self.peer);
        let Link::Pending(pending) = &mut self.link else { return Ok(()) };

        let mut bytes = Vec::new();
        write_s2c(&mut bytes, &Disconnect { reason: NetworkText::Literal(reason) })?;
        pending.writer.write_all(&bytes).await?;
        Ok(())
    }

    async fn fill(&mut self, buffer: &mut Vec<u8>) -> Result<usize, ConnectionError> {
        let read = self.reader.read_buf(buffer);

        match &mut self.link {
            Link::Admitted { writer, .. } => tokio::select! {
                read = read => Ok(read?),
                _ = writer => Ok(0),
            },
            _ => Ok(read.await?),
        }
    }

    async fn finish(mut self) {
        if let Link::Admitted { id, mut writer } = mem::replace(&mut self.link, Link::Closed) {
            self.shared.release(id);
            if time::timeout(FLUSH_TIMEOUT, &mut writer).await.is_err() {
                debug_log!("{} flush timed out", self.peer);
                writer.abort();
            }
            debug_log!("{} left slot {id}", self.peer);
        }
    }
}

async fn write_loop(mut writer: OwnedWriteHalf, mut inbox: Inbox, peer: SocketAddr) {
    let mut chunks = Vec::with_capacity(WRITE_BATCH_LEN);
    let mut batch = Vec::new();

    while inbox.recv_many(&mut chunks, WRITE_BATCH_LEN).await > 0 {
        batch.clear();
        chunks.drain(..).for_each(|chunk| batch.extend_from_slice(&chunk));

        if let Err(error) = writer.write_all(&batch).await {
            debug_log!("{peer} write failed: {error}");
            return;
        }
    }

    if let Err(error) = writer.shutdown().await {
        debug_log!("{peer} shutdown failed: {error}");
    }
}
