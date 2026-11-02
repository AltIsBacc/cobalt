use std::mem;

use cobalt_protocol::{ProtocolResult, S2c};

use crate::model::{ClientId, Event};

#[derive(Debug)]
pub(crate) struct Outbox {
    buffers: Box<[Vec<u8>]>,
    events: Vec<Event>,
}

impl Outbox {
    pub(crate) fn new(clients: u8) -> Self {
        Self { buffers: (0..clients).map(|_| Vec::new()).collect(), events: Vec::new() }
    }

    pub(crate) fn send(&mut self, to: ClientId, packet: S2c<'_>) -> ProtocolResult {
        packet.write(&mut self.buffers[to.index()])
    }

    pub(crate) fn relay(&mut self, to: ClientId, frames: &[u8]) {
        self.buffers[to.index()].extend_from_slice(frames);
    }

    pub(crate) fn emit(&mut self, event: Event) {
        self.events.push(event);
    }

    pub(crate) fn drain(
        &mut self,
        mut deliver: impl FnMut(ClientId, Vec<u8>),
        mut publish: impl FnMut(Event),
    ) {
        for (raw, buffer) in (0u8..).zip(self.buffers.iter_mut()) {
            if !buffer.is_empty() {
                deliver(ClientId::new(raw), mem::take(buffer));
            }
        }
        for event in self.events.drain(..) {
            publish(event);
        }
    }
}
