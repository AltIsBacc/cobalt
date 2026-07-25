use anyhow::{Context, Result};
use tokio::sync::{mpsc::{self, error::TrySendError}, oneshot};

pub struct Request<Tx, Rx> {
    pub payload: Tx,
    pub reply_to: Option<oneshot::Sender<Rx>>, 
}

impl<Tx, Rx> Request<Tx, Rx> {
    pub fn reply(self, response: Rx) -> Result<(), Rx> {
        if let Some(tx) = self.reply_to {
            tx.send(response)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone)]
pub struct CobaltSender<Tx: Clone, Rx: Clone> {
    inner: mpsc::Sender<Request<Tx, Rx>>,
}

pub struct CobaltReceiver<Tx, Rx> {
    inner: mpsc::Receiver<Request<Tx, Rx>>,
}

impl<Tx: Clone, Rx: Clone> CobaltSender<Tx, Rx> {
    pub async fn send(&self, msg: Tx) -> Result<()> {
        let req = Request {
            payload: msg,
            reply_to: None,
        };
        self.inner.send(req).await
            .map_err(|_| anyhow::anyhow!("Server channel dropped"))
    }

    pub async fn request(&self, msg: Tx) -> Result<Rx> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let req = Request {
            payload: msg,
            reply_to: Some(reply_tx),
        };
        self.inner.send(req).await
            .map_err(|_| anyhow::anyhow!("Server channel dropped"))?;
            
        reply_rx.await
            .context("Server dropped before replying")
    }

    pub fn try_send(&self, msg: Tx) -> Result<()> {
        let req = Request {
            payload: msg,
            reply_to: None,
        };
        
        self.inner.try_send(req)
            .map_err(|e| match e {
                TrySendError::Full(_) => anyhow::anyhow!("Server channel is full"),
                TrySendError::Closed(_) => anyhow::anyhow!("Server channel dropped"),
            })
    }
}

impl<Tx, Rx> CobaltReceiver<Tx, Rx> {
    pub async fn recv(&mut self) -> Result<Request<Tx, Rx>> {
        self.inner.recv().await
            .context("All senders disconnected")
    }
}

pub fn create_bi_channel<HostMsg: Clone, ServerMsg: Clone>(
    capacity: usize,
) -> (
    (CobaltSender<HostMsg, ServerMsg>, CobaltReceiver<ServerMsg, HostMsg>), // Host Side
    (CobaltSender<ServerMsg, HostMsg>, CobaltReceiver<HostMsg, ServerMsg>), // Server Side
) {
    let (tx_h2s, rx_h2s) = mpsc::channel(capacity);
    let (tx_s2h, rx_s2h) = mpsc::channel(capacity);

    ((CobaltSender { inner: tx_h2s.clone() }, CobaltReceiver { inner: rx_s2h }), 
     (CobaltSender { inner: tx_s2h }, CobaltReceiver { inner: rx_h2s }))
}

