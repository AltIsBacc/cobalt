use super::Outbox;
use super::clock::ClockSystem;
use super::state::State;

pub(super) trait System {
    fn update(&self, _: &mut State) {}

    fn sync(&self, _: &mut State, _: &mut Outbox) {}
}

const SYSTEMS: &[&dyn System] = &[&ClockSystem];

pub(super) fn run(state: &mut State, out: &mut Outbox) {
    for system in SYSTEMS {
        system.update(state);
    }
    for system in SYSTEMS {
        system.sync(state, out);
    }
}
