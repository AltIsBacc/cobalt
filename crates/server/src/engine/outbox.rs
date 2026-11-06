use std::mem;

use cobalt_protocol::{Frame, ProtocolResult, S2c};

use crate::log::debug_log;
use crate::model::{ClientId, Event};

#[derive(Debug)]
pub(crate) struct Outbox {
    buffers: Box<[Vec<u8>]>,
    scratch: Vec<u8>,
    events: Vec<Event>,
}

impl Outbox {
    pub(crate) fn new(clients: u8) -> Self {
        Self {
            buffers: (0..clients).map(|_| Vec::new()).collect(),
            scratch: Vec::new(),
            events: Vec::new(),
        }
    }

    pub(crate) fn send(&mut self, to: ClientId, packet: S2c<'_>) -> ProtocolResult {
        packet.write(&mut self.buffers[to.index()])
    }

    pub(crate) fn relay(&mut self, to: ClientId, frames: &[u8]) {
        self.buffers[to.index()].extend_from_slice(frames);
    }

    pub(crate) fn relay_all(
        &mut self,
        audience: impl IntoIterator<Item = ClientId>,
        frames: &[u8],
    ) {
        for to in audience {
            self.relay(to, frames);
        }
    }

    pub(crate) fn broadcast(
        &mut self,
        audience: impl IntoIterator<Item = ClientId>,
        packet: S2c<'_>,
    ) {
        self.scratch.clear();
        match packet.write(&mut self.scratch) {
            Ok(()) => self.fan_out(audience),
            Err(error) => debug_log!("broadcast failed: {error}"),
        }
    }

    pub(crate) fn forward(
        &mut self,
        audience: impl IntoIterator<Item = ClientId>,
        frame: Frame<'_>,
    ) -> ProtocolResult {
        self.scratch.clear();
        frame.write(&mut self.scratch)?;
        self.fan_out(audience);
        Ok(())
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

    fn fan_out(&mut self, audience: impl IntoIterator<Item = ClientId>) {
        for to in audience {
            self.buffers[to.index()].extend_from_slice(&self.scratch);
        }
    }
}
