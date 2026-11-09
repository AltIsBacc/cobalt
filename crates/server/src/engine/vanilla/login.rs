use cobalt_protocol::Packet;
use cobalt_protocol::registry::{RequestEssentialTiles, RequestWorldData};

use super::{LATE, roster};
use crate::engine::{
    ClientError, Context, Handled, Incoming, Module, StageKind, Subscription, Verdict, world_info,
};
use crate::log::debug_log;
use crate::model::{ClientId, Event};

const INVALID_NAME: &str = "invalid player name";

pub(super) struct LoginModule;

impl Module for LoginModule {
    fn name(&self) -> &'static str {
        "login"
    }

    fn priority(&self) -> i32 {
        LATE
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        vec![
            Subscription::every_packet_in([StageKind::Authenticating]),
            Subscription::packets_in([StageKind::Loading], [RequestEssentialTiles::ID]),
        ]
    }

    fn packet(&mut self, cx: &mut Context<'_>, from: ClientId, incoming: &Incoming<'_>) -> Handled {
        match cx.stage(from) {
            Some(StageKind::Authenticating) => authenticate(cx, from, incoming)?,
            Some(StageKind::Loading) => {
                cx.advance(from);
            }
            _ => return Ok(Verdict::Pass),
        }
        Ok(Verdict::Stop)
    }
}

fn authenticate(
    cx: &mut Context<'_>,
    id: ClientId,
    incoming: &Incoming<'_>,
) -> Result<(), ClientError> {
    let Some(player) = cx.player_mut(id) else { return Ok(()) };

    if incoming.frame.kind != RequestWorldData::ID {
        if let Some(packet) = &incoming.packet {
            roster::remember(player, incoming.frame, packet)?;
        }
        return Ok(());
    }
    if !player.has_valid_name() {
        return cx.disconnect(id, INVALID_NAME);
    }

    debug_log!("{id} logged in as {}", player.name);
    cx.advance(id);
    cx.publish(Event::ClientLoggedIn { id });
    cx.send_with_world(id, |world| world_info::build(world).into())
}
