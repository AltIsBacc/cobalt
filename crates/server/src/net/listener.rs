use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::{JoinHandle, JoinSet};
use tokio::time;

use super::session;
use crate::error::ServerResult;
use crate::handle::ServerHandle;
use crate::log::debug_log;

const ACCEPT_BACKOFF: Duration = Duration::from_millis(50);

pub(crate) struct Listener {
    local_addr: SocketAddr,
    shutdown: watch::Sender<()>,
    task: JoinHandle<()>,
}

impl Listener {
    pub(crate) fn bind(
        addr: SocketAddr,
        server: ServerHandle,
        handshake_timeout: Duration,
    ) -> ServerResult<Self> {
        let listener = std::net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let listener = TcpListener::from_std(listener)?;
        let local_addr = listener.local_addr()?;
        debug_log!("listening on {local_addr}");
        let (shutdown, signal) = watch::channel(());
        let task = tokio::spawn(accept_loop(listener, server, handshake_timeout, signal));

        Ok(Self { local_addr, shutdown, task })
    }

    pub(crate) const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub(crate) async fn stop(self) {
        let Self { shutdown, task, .. } = self;
        drop(shutdown);
        let _ = task.await;
    }
}

async fn accept_loop(
    listener: TcpListener,
    server: ServerHandle,
    handshake_timeout: Duration,
    mut shutdown: watch::Receiver<()>,
) {
    let mut sessions = JoinSet::new();

    loop {
        tokio::select! {
            _ = shutdown.changed() => break,
            accepted = listener.accept() => match accepted {
                Ok((socket, peer)) => {
                    sessions.spawn(session::serve(
                        socket,
                        peer,
                        server.clone(),
                        handshake_timeout,
                        shutdown.clone(),
                    ));
                }
                Err(error) => {
                    debug_log!("accept failed: {error}");
                    time::sleep(ACCEPT_BACKOFF).await;
                }
            },
            Some(_) = sessions.join_next(), if !sessions.is_empty() => {}
        }
    }

    while sessions.join_next().await.is_some() {}
}

#[cfg(test)]
mod tests {
    use cobalt_format::World;
    use cobalt_format::world::header::{Difficulty, Size};
    use cobalt_protocol::{C2sPacket, Frame, Message, NetworkText, PROTOCOL_VERSION, S2cPacket};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    use super::*;
    use crate::config::ServerConfig;
    use crate::error::ServerError;
    use crate::event::Event;
    use crate::server::{RunningServer, Server};

    const TIMEOUT: Duration = Duration::from_secs(5);

    fn listening(config: ServerConfig) -> ServerConfig {
        ServerConfig { listen: Some("127.0.0.1:0".parse().unwrap()), ..config }
    }

    fn start(config: ServerConfig) -> RunningServer {
        let size = Size::try_new(1, 1).unwrap();
        let world = World::try_new("net", size, Difficulty::Normal).unwrap();
        Server::spawn(world, listening(config)).unwrap()
    }

    fn encode(packet: &C2sPacket) -> Vec<u8> {
        let mut out = Vec::new();
        packet.encode(&mut out).unwrap();
        out
    }

    fn connect_bytes() -> Vec<u8> {
        encode(&C2sPacket::Connect { version: PROTOCOL_VERSION.to_owned() })
    }

    async fn dial(running: &RunningServer) -> TcpStream {
        TcpStream::connect(running.local_addr().unwrap()).await.unwrap()
    }

    async fn join(running: &RunningServer) -> TcpStream {
        let mut stream = dial(running).await;
        stream.write_all(&connect_bytes()).await.unwrap();
        stream
    }

    async fn read_packet(stream: &mut TcpStream) -> Option<S2cPacket> {
        time::timeout(TIMEOUT, async {
            let mut buffer = Vec::new();
            loop {
                if let Some((frame, _)) = Frame::parse(&buffer).unwrap() {
                    return Some(S2cPacket::decode(frame).unwrap());
                }
                match stream.read_buf(&mut buffer).await {
                    Ok(0) | Err(_) => return None,
                    Ok(_) => {}
                }
            }
        })
        .await
        .unwrap()
    }

    async fn assert_closed(stream: &mut TcpStream) {
        while read_packet(stream).await.is_some() {}
    }

    async fn wait_for_players(server: &ServerHandle, expected: usize) {
        time::timeout(TIMEOUT, async {
            while server.status().await.unwrap().players != expected {
                time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }

    fn disconnect(reason: &str) -> Option<S2cPacket> {
        Some(S2cPacket::Disconnect { reason: NetworkText::literal(reason) })
    }

    #[tokio::test]
    async fn connect_joins_and_hanging_up_leaves() {
        let running = start(ServerConfig::default());
        let handle = running.handle().clone();

        let stream = join(&running).await;
        wait_for_players(&handle, 1).await;

        drop(stream);
        wait_for_players(&handle, 0).await;

        running.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn fragmented_and_unknown_frames_are_handled() {
        let running = start(ServerConfig::default());
        let handle = running.handle().clone();

        let mut stream = dial(&running).await;
        let mut bytes = vec![3, 0, 200];
        bytes.extend(connect_bytes());
        stream.write_all(&bytes[..6]).await.unwrap();
        time::sleep(Duration::from_millis(20)).await;
        stream.write_all(&bytes[6..]).await.unwrap();
        wait_for_players(&handle, 1).await;

        running.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn connections_beyond_capacity_are_refused() {
        let running = start(ServerConfig { max_players: 1, ..ServerConfig::default() });
        let handle = running.handle().clone();

        let first = join(&running).await;
        wait_for_players(&handle, 1).await;

        let mut second = join(&running).await;
        assert_eq!(read_packet(&mut second).await, disconnect("the server is full"));
        assert_eq!(read_packet(&mut second).await, None);
        assert_eq!(handle.status().await.unwrap().players, 1);

        drop(first);
        running.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn mismatched_versions_are_refused() {
        let running = start(ServerConfig::default());

        let mut stream = dial(&running).await;
        let bytes = encode(&C2sPacket::Connect { version: "Terraria1".to_owned() });
        stream.write_all(&bytes).await.unwrap();

        assert_eq!(read_packet(&mut stream).await, disconnect("incompatible client version"));
        assert_eq!(running.handle().status().await.unwrap().players, 0);

        running.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn silent_connections_time_out_without_taking_a_slot() {
        let running = start(ServerConfig {
            handshake_timeout: Duration::from_millis(50),
            ..ServerConfig::default()
        });

        let mut stream = dial(&running).await;
        assert_eq!(read_packet(&mut stream).await, None);
        assert_eq!(running.handle().status().await.unwrap().players, 0);

        running.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn shutdown_releases_clients_before_the_server_stops() {
        let running = start(ServerConfig::default());
        let handle = running.handle().clone();
        let mut events = handle.subscribe();

        let mut stream = join(&running).await;
        wait_for_players(&handle, 1).await;

        running.shutdown().await.unwrap();

        assert!(matches!(events.recv().await.unwrap(), Event::ClientJoined { .. }));
        assert!(matches!(events.recv().await.unwrap(), Event::ClientLeft { .. }));
        assert_eq!(events.recv().await.unwrap(), Event::Stopped);
        assert_closed(&mut stream).await;
    }

    #[tokio::test]
    async fn join_returns_after_an_external_shutdown_and_closes_clients() {
        let running = start(ServerConfig::default());
        let handle = running.handle().clone();

        let mut stream = join(&running).await;
        wait_for_players(&handle, 1).await;

        handle.shutdown().await.unwrap();
        running.join().await.unwrap();

        assert_closed(&mut stream).await;
    }

    #[tokio::test]
    async fn binding_an_occupied_address_fails() {
        let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let size = Size::try_new(1, 1).unwrap();
        let world = World::try_new("net", size, Difficulty::Normal).unwrap();
        let config = ServerConfig { listen: occupied.local_addr().ok(), ..ServerConfig::default() };

        assert!(matches!(Server::spawn(world, config), Err(ServerError::Io(_))));
    }
}
