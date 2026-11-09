use super::module::StageKind;
use crate::model::Player;

pub(super) struct Session {
    pub(super) stage: StageKind,
    pub(super) closing: bool,
    pub(super) player: Player,
}

impl Session {
    pub(super) fn new() -> Self {
        Self { stage: StageKind::Authenticating, closing: false, player: Player::default() }
    }

    pub(super) fn is_playing(&self) -> bool {
        self.stage == StageKind::Playing && !self.closing
    }
}
