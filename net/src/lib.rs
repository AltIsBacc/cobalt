use std::{io, net::SocketAddr, thread};

use cobalt_server::{CobaltServerHandle, commands::{ClientId, CobaltCommand, CobaltResponse, CobaltSuccessResponseData}};
use derive_more::derive::Constructor;
use tokio::{net::{TcpListener, TcpStream}, sync::mpsc};

use crate::error::{CobaltNetworkError, CobaltNetworkResult};

pub mod error;

pub struct CobaltNetworkHandle {
    pub shutdown: mpsc::Sender<()>,

    accept_task: tokio::JoinHandle<()>,
    // events_task: thread::JoinHandle<()>,
}

#[derive(Constructor)]
pub struct CobaltNetworkServer {
    bind_addr: SocketAddr,
    server: CobaltServerHandle,
}

impl CobaltNetworkServer {
    pub async fn spawn(self) -> CobaltNetworkResult<CobaltNetworkHandle> {
        let mut listener = TcpListener::bind(self.bind_addr).await?;
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let server = self.server;
        let accept_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;

                    _ = shutdown_rx.recv() => break,

                    accepted = listener.accept() => {
                        match accepted {
                            Ok((socket, addr)) => {
                                tokio::spawn(Self::handle_connection(
                                    socket, addr, server.clone()
                                ));
                            }
                            Err(e) => eprintln!("[cobalt-net] accept error: {e}"),
                        }
                    }
                }
            }
        });

        Ok(CobaltNetworkHandle {
            shutdown: shutdown_tx,
            accept_task,
        })
    }

    pub async fn handle_connection(
        socket: TcpStream,
        addr: SocketAddr,
        server: CobaltServerHandle,
    ) -> CobaltNetworkResult {
        let id = 0;

        let CobaltResponse::Success {
            response: CobaltSuccessResponseData::ClientJoined { id }
        } = server.commands.request(CobaltCommand::ClientJoin).await? else {
            return Err(CobaltNetworkError::FailedToJoin);
        };

        Ok(())
    }
}

