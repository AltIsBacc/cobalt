use super::context::Context;
use super::error::ClientError;
use super::event::GameEvent;
use super::module::{Handled, Incoming, Module, ModuleInfo, StageKind, Subscription, Verdict};
use super::router::Router;
use crate::log::debug_log;
use crate::model::ClientId;

pub struct Group {
    info: ModuleInfo,
    priority: i32,
    modules: Vec<Box<dyn Module>>,
    router: Router,
}

impl Group {
    #[must_use]
    pub fn new(info: ModuleInfo, mut modules: Vec<Box<dyn Module>>) -> Self {
        modules.sort_by_key(|module| module.priority());
        let router = Router::build(&modules);
        Self { info, priority: 0, modules, router }
    }

    #[must_use]
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub(super) fn handles(&self, stage: StageKind, kind: u8) -> bool {
        !self.router.route(stage, kind).is_empty()
    }

    pub(super) fn route(
        &mut self,
        cx: &mut Context<'_>,
        from: ClientId,
        stage: StageKind,
        incoming: &Incoming<'_>,
    ) -> Handled {
        for &index in self.router.route(stage, incoming.frame.kind) {
            if self.modules[index].packet(cx, from, incoming)? == Verdict::Stop {
                return Ok(Verdict::Stop);
            }
        }
        Ok(Verdict::Pass)
    }
}

impl Module for Group {
    fn name(&self) -> &str {
        &self.info.name
    }

    fn info(&self) -> ModuleInfo {
        self.info.clone()
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        self.modules.iter().flat_map(|module| module.subscriptions()).collect()
    }

    fn packet(&mut self, cx: &mut Context<'_>, from: ClientId, incoming: &Incoming<'_>) -> Handled {
        let Some(stage) = cx.stage(from) else { return Ok(Verdict::Pass) };
        self.route(cx, from, stage, incoming)
    }

    fn event(&mut self, cx: &mut Context<'_>, event: &GameEvent) -> Result<(), ClientError> {
        for module in &mut self.modules {
            if let Err(error) = module.event(cx, event) {
                debug_log!("{} failed on {event:?}: {error}", module.name());
            }
        }
        Ok(())
    }

    fn start(&mut self, cx: &mut Context<'_>) {
        for module in &mut self.modules {
            module.start(cx);
        }
    }

    fn stop(&mut self, cx: &mut Context<'_>) {
        for module in self.modules.iter_mut().rev() {
            module.stop(cx);
        }
    }

    fn update(&mut self, cx: &mut Context<'_>) {
        for module in &mut self.modules {
            module.update(cx);
        }
    }

    fn sync(&mut self, cx: &mut Context<'_>) {
        for module in &mut self.modules {
            module.sync(cx);
        }
    }
}
