use super::module::{Module, StageKind};

const PACKET_KINDS: usize = 256;

pub(super) struct Router {
    routes: Box<[Box<[usize]>]>,
}

impl Router {
    pub(super) fn build(modules: &[Box<dyn Module>]) -> Self {
        let subscriptions: Vec<_> = modules.iter().map(|module| module.subscriptions()).collect();

        let routes = StageKind::ALL
            .into_iter()
            .flat_map(|stage| (0..=u8::MAX).map(move |kind| (stage, kind)))
            .map(|(stage, kind)| {
                subscriptions
                    .iter()
                    .enumerate()
                    .filter(|(_, owned)| owned.iter().any(|entry| entry.covers(stage, kind)))
                    .map(|(index, _)| index)
                    .collect()
            })
            .collect();

        Self { routes }
    }

    pub(super) fn route(&self, stage: StageKind, kind: u8) -> &[usize] {
        &self.routes[stage.index() * PACKET_KINDS + usize::from(kind)]
    }
}
