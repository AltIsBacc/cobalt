use super::Outbox;
use super::state::State;

pub(super) trait System {
    fn run(&mut self, state: &mut State, out: &mut Outbox);
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Phase {
    Update,
    Sync,
}

impl Phase {
    const COUNT: usize = Self::Sync as usize + 1;
}

#[derive(Default)]
pub(super) struct Schedule {
    phases: [Vec<Box<dyn System>>; Phase::COUNT],
}

impl Schedule {
    pub(super) fn with(mut self, phase: Phase, system: impl System + 'static) -> Self {
        self.phases[phase as usize].push(Box::new(system));
        self
    }

    pub(super) fn run(&mut self, state: &mut State, out: &mut Outbox) {
        for system in self.phases.iter_mut().flatten() {
            system.run(state, out);
        }
    }
}
