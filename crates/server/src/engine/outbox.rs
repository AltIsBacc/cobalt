use std::ops::Range;

use cobalt_protocol::{Encode, ProtocolResult, S2c, write_s2c};

use crate::client::ClientId;
use crate::event::Event;

#[derive(Debug, Default)]
pub(crate) struct Outbox {
    bytes: Vec<u8>,
    sends: Vec<(ClientId, Range<usize>)>,
    events: Vec<Event>,
}

impl Outbox {
    pub(crate) fn send<P: S2c + Encode>(&mut self, to: ClientId, packet: &P) -> ProtocolResult {
        let start = self.bytes.len();
        write_s2c(&mut self.bytes, packet)?;
        self.sends.push((to, start..self.bytes.len()));
        Ok(())
    }

    pub(crate) fn emit(&mut self, event: Event) {
        self.events.push(event);
    }

    pub(crate) fn drain(
        &mut self,
        mut deliver: impl FnMut(ClientId, &[u8]),
        publish: impl FnMut(Event),
    ) {
        for (to, range) in self.sends.drain(..) {
            deliver(to, &self.bytes[range]);
        }
        self.bytes.clear();
        self.events.drain(..).for_each(publish);
    }
}
