use cobalt_protocol::registry::FinishedConnectingToServer;
use cobalt_protocol::{C2s, Frame};

use super::context::Context;
use super::error::ClientError;
use super::event::GameEvent;
use super::group::Group;
use super::module::{Incoming, Module, ModuleInfo, StageKind, Verdict};
use super::outbox::Outbox;
use super::state::State;
use crate::log::{debug_log, warn_log};
use crate::model::ClientId;

const MAX_EVENT_ROUNDS: usize = 16;

pub(super) struct Modules {
    root: Group,
}

impl Modules {
    pub(super) fn new(list: Vec<Box<dyn Module>>) -> Self {
        let root = Group::new(ModuleInfo::new("cobalt"), list);
        debug_log!("modules: {}", root.info());
        Self { root }
    }

    pub(super) fn dispatch(
        &mut self,
        state: &mut State,
        out: &mut Outbox,
        id: ClientId,
        frame: Frame<'_>,
    ) -> Result<(), ClientError> {
        let Some(stage) = state.stage_kind(id) else { return Ok(()) };
        if self.root.handles(stage, frame.kind) {
            let incoming = Incoming { frame, packet: C2s::decode(frame)? };
            let cx = &mut Context::new(state, out);
            if self.root.route(cx, id, stage, &incoming)? == Verdict::Stop {
                return Ok(());
            }
        }
        debug_log!("[cid {id}] unhandled packet {}", frame.kind);
        Ok(())
    }

    pub(super) fn start(&mut self, state: &mut State, out: &mut Outbox) {
        self.root.start(&mut Context::new(state, out));
    }

    pub(super) fn stop(&mut self, state: &mut State, out: &mut Outbox) {
        self.root.stop(&mut Context::new(state, out));
    }

    pub(super) fn update(&mut self, state: &mut State, out: &mut Outbox) {
        self.root.update(&mut Context::new(state, out));
    }

    pub(super) fn sync(&mut self, state: &mut State, out: &mut Outbox) {
        self.root.sync(&mut Context::new(state, out));
    }

    pub(super) fn settle(&mut self, state: &mut State, out: &mut Outbox) {
        for _ in 0..MAX_EVENT_ROUNDS {
            if state.bus.is_empty() {
                return;
            }
            for event in state.bus.take() {
                let _ = self.root.event(&mut Context::new(state, out), &event);
                if let GameEvent::PlayerEntering { id } = event {
                    finish_entry(state, out, id);
                }
            }
        }
        warn_log!("game events kept producing events; dropping the remainder");
        state.bus.clear();
    }
}

fn finish_entry(state: &mut State, out: &mut Outbox, id: ClientId) {
    if state.stage_kind(id) != Some(StageKind::Playing) {
        return;
    }
    match out.send(id, FinishedConnectingToServer {}.into()) {
        Ok(()) => state.bus.emit(GameEvent::PlayerEntered { id }),
        Err(error) => {
            debug_log!("{id} could not be admitted: {error}");
            state.close(id);
        }
    }
}
