use std::fmt;

use cobalt_protocol::{C2s, Frame};

use super::context::Context;
use super::error::ClientError;
use super::event::GameEvent;
use crate::model::ClientId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Stop,
}

pub type Handled = Result<Verdict, ClientError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageKind {
    Authenticating,
    Loading,
    Spawning,
    Playing,
}

impl StageKind {
    pub(super) const ALL: [Self; 4] =
        [Self::Authenticating, Self::Loading, Self::Spawning, Self::Playing];

    pub(super) const fn next(self) -> Self {
        match self {
            Self::Authenticating => Self::Loading,
            Self::Loading => Self::Spawning,
            Self::Spawning | Self::Playing => Self::Playing,
        }
    }

    pub(super) const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone)]
pub struct Incoming<'data> {
    pub frame: Frame<'data>,
    pub packet: Option<C2s<'data>>,
}

#[derive(Debug, Clone)]
pub struct Subscription {
    stages: Vec<StageKind>,
    packets: Option<Vec<u8>>,
}

impl Subscription {
    pub fn playing(packets: impl IntoIterator<Item = u8>) -> Self {
        Self::packets_in([StageKind::Playing], packets)
    }

    pub fn packets_in(
        stages: impl IntoIterator<Item = StageKind>,
        packets: impl IntoIterator<Item = u8>,
    ) -> Self {
        Self { stages: stages.into_iter().collect(), packets: Some(packets.into_iter().collect()) }
    }

    pub fn every_packet_in(stages: impl IntoIterator<Item = StageKind>) -> Self {
        Self { stages: stages.into_iter().collect(), packets: None }
    }

    pub(super) fn covers(&self, stage: StageKind, kind: u8) -> bool {
        self.stages.contains(&stage)
            && self.packets.as_ref().is_none_or(|packets| packets.contains(&kind))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ModuleInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
}

impl ModuleInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: String::new(),
            description: String::new(),
            authors: Vec::new(),
        }
    }

    #[must_use]
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    #[must_use]
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.authors.push(author.into());
        self
    }

    #[must_use]
    pub fn authors<A: Into<String>>(mut self, authors: impl IntoIterator<Item = A>) -> Self {
        self.authors.extend(authors.into_iter().map(Into::into));
        self
    }
}

impl fmt::Display for ModuleInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)?;
        if !self.version.is_empty() {
            write!(f, " {}", self.version)?;
        }
        if !self.authors.is_empty() {
            write!(f, " by {}", self.authors.join(", "))?;
        }
        Ok(())
    }
}

pub trait Module: Send {
    fn name(&self) -> &str;

    fn info(&self) -> ModuleInfo {
        ModuleInfo::new(self.name())
    }

    fn priority(&self) -> i32 {
        0
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        Vec::new()
    }

    fn packet(
        &mut self,
        _cx: &mut Context<'_>,
        _from: ClientId,
        _incoming: &Incoming<'_>,
    ) -> Handled {
        Ok(Verdict::Pass)
    }

    fn event(&mut self, _cx: &mut Context<'_>, _event: &GameEvent) -> Result<(), ClientError> {
        Ok(())
    }

    fn start(&mut self, _cx: &mut Context<'_>) {}

    fn stop(&mut self, _cx: &mut Context<'_>) {}

    fn update(&mut self, _cx: &mut Context<'_>) {}

    fn sync(&mut self, _cx: &mut Context<'_>) {}
}
