mod error;

use std::{net::SocketAddr, time::Duration};

use cobalt_server::ServerHandle;
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinHandle,
    time,
};

pub use error::{NetError, NetResult};

const ACCEPT_BACKOFF: Duration = Duration::from_millis(50);
const READ_BUFFER_LEN: usize = 4096;

pub struct NetServer {
    local_addr: SocketAddr,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl NetServer {
    pub async fn bind(addr: SocketAddr, server: ServerHandle) -> NetResult<Self> {
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        let (shutdown, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(accept_loop(listener, server, shutdown_rx));

        Ok(Self { local_addr, shutdown, task })
    }

    pub const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn shutdown(self) {
        let _ = self.shutdown.send(());
        let _ = self.task.await;
    }
}

async fn accept_loop(
    listener: TcpListener,
    server: ServerHandle,
    mut shutdown: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            accepted = listener.accept() => match accepted {
                Ok((socket, _)) => {
                    tokio::spawn(serve_connection(socket, server.clone()));
                }
                Err(_) => time::sleep(ACCEPT_BACKOFF).await,
            },
        }
    }
}

async fn serve_connection(mut socket: TcpStream, server: ServerHandle) {
    let Ok(id) = server.join().await else { return };

    let mut buffer = [0u8; READ_BUFFER_LEN];
    while matches!(socket.read(&mut buffer).await, Ok(n) if n > 0) {}

    let _ = server.leave(id).await;
}

#[cfg(test)]
mod tests {
    use cobalt_format::{
        World,
        world::header::{Difficulty, Size},
    };
    use cobalt_server::{Server, ServerConfig};

    use super::*;

    async fn wait_for_players(server: &ServerHandle, expected: usize) {
        time::timeout(Duration::from_secs(5), async {
            while server.status().await.unwrap().players != expected {
                time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn connections_join_and_leave_the_server() {
        let world = World::new("net", Size::new(1, 1).unwrap(), Difficulty::Normal).unwrap();
        let running = Server::spawn(world, ServerConfig::default()).unwrap();
        let handle = running.handle().clone();
        let net = NetServer::bind("127.0.0.1:0".parse().unwrap(), handle.clone()).await.unwrap();

        let stream = TcpStream::connect(net.local_addr()).await.unwrap();
        wait_for_players(&handle, 1).await;

        drop(stream);
        wait_for_players(&handle, 0).await;

        net.shutdown().await;
        running.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn connections_beyond_capacity_are_dropped() {
        let world = World::new("net", Size::new(1, 1).unwrap(), Difficulty::Normal).unwrap();
        let config = ServerConfig { max_players: 1, ..ServerConfig::default() };
        let running = Server::spawn(world, config).unwrap();
        let handle = running.handle().clone();
        let net = NetServer::bind("127.0.0.1:0".parse().unwrap(), handle.clone()).await.unwrap();

        let first = TcpStream::connect(net.local_addr()).await.unwrap();
        wait_for_players(&handle, 1).await;

        let mut second = TcpStream::connect(net.local_addr()).await.unwrap();
        let mut buffer = [0u8; 1];
        let read = time::timeout(Duration::from_secs(5), second.read(&mut buffer)).await;
        assert!(matches!(read, Ok(Ok(0)) | Ok(Err(_))));
        assert_eq!(handle.status().await.unwrap().players, 1);

        drop(first);
        net.shutdown().await;
        running.shutdown().await.unwrap();
    }
}
