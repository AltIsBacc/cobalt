use std::mem;

use cobalt_protocol::registry::{
    Disconnect, Emoji, HealEffect, ManaEffect, PlayerControls, PlayerDodge, PlayerHp, PlayerInfo,
    PlayerInventorySlot, PlayerItemAnimation, PlayerMana, PlayerTeam, PlayerTeleportPortal,
    PlayerZone, RequestEssentialTiles, SetPlayerStealth, SpawnPlayer, SyncTilePicking, TogglePvp,
    UpdatePlayerBuff,
};
use cobalt_protocol::{C2s, Frame, Frames, NetworkText, Packet};

use super::error::ClientError;
use super::presence::attributed;
use super::stage::{Absorbed, Stage, remember};
use super::{Engine, Flow, Outbox, State, section, world_info};
use crate::log::debug_log;
use crate::model::{ClientId, Event};

const RELAYED: &[u8] = &[
    PlayerInfo::ID,
    PlayerInventorySlot::ID,
    SpawnPlayer::ID,
    PlayerControls::ID,
    PlayerHp::ID,
    PlayerMana::ID,
    PlayerTeam::ID,
    UpdatePlayerBuff::ID,
    PlayerItemAnimation::ID,
    TogglePvp::ID,
    HealEffect::ID,
    ManaEffect::ID,
    PlayerDodge::ID,
    SetPlayerStealth::ID,
    PlayerZone::ID,
    Emoji::ID,
    SyncTilePicking::ID,
    PlayerTeleportPortal::ID,
];

impl Engine {
    pub(crate) fn receive(
        &mut self,
        id: ClientId,
        bytes: &[u8],
        out: &mut Outbox,
    ) -> Result<Flow, ClientError> {
        for frame in Frames::new(bytes) {
            if self.state.dispatch(id, frame?, out)? == Flow::Close {
                return Ok(Flow::Close);
            }
        }
        Ok(Flow::Open)
    }
}

impl State {
    fn dispatch(
        &mut self,
        id: ClientId,
        frame: Frame<'_>,
        out: &mut Outbox,
    ) -> Result<Flow, ClientError> {
        debug_log!("[cid {id}] {}", frame.kind);
        let Some(stage) = self.slots.get_mut(id) else { return Ok(Flow::Close) };

        match mem::replace(stage, Stage::Closing) {
            Stage::Authenticating(mut authenticating) => match authenticating.absorb(frame)? {
                Absorbed::Pending => {
                    *stage = Stage::Authenticating(authenticating);
                    Ok(Flow::Open)
                }
                Absorbed::Complete => match authenticating.finish() {
                    Ok(player) => {
                        debug_log!("{id} logged in as {}", player.name);
                        out.emit(Event::ClientLoggedIn { id });
                        out.send(id, world_info::build(&self.world).into())?;
                        *stage = Stage::Loading(player);
                        Ok(Flow::Open)
                    }
                    Err(reason) => {
                        out.send(id, refusal(reason).into())?;
                        Ok(Flow::Close)
                    }
                },
            },
            Stage::Loading(player) => {
                if frame.kind != RequestEssentialTiles::ID {
                    *stage = Stage::Loading(player);
                    return Ok(Flow::Open);
                }
                section::send_spawn(&self.world, &mut self.streams[id.index()], id, out)?;
                *stage = Stage::Spawning(player);
                Ok(Flow::Open)
            }
            Stage::Spawning(player) => {
                if frame.kind != SpawnPlayer::ID {
                    *stage = Stage::Spawning(player);
                    return Ok(Flow::Open);
                }
                *stage = Stage::Playing(player);
                self.admit(id, frame, out)?;
                Ok(Flow::Open)
            }
            Stage::Playing(mut player) => {
                let packet = C2s::decode(frame)?;
                if let Some(packet) = &packet {
                    remember(&mut player, frame, packet)?;
                }
                *stage = Stage::Playing(player);
                self.play(id, frame, packet, out)?;
                Ok(Flow::Open)
            }
            Stage::Closing => Ok(Flow::Close),
        }
    }

    fn play(
        &mut self,
        id: ClientId,
        frame: Frame<'_>,
        packet: Option<C2s<'_>>,
        out: &mut Outbox,
    ) -> Result<(), ClientError> {
        let relayed = RELAYED.contains(&frame.kind);
        if relayed {
            out.relay_all(self.audience(Some(id)), &attributed(frame, id)?);
        }

        match packet {
            Some(C2s::PlayerControls(controls)) => self.stream_sections(id, &controls, out),
            Some(C2s::NetModule(module)) => self.handle_module(id, &module, out),
            Some(C2s::OpenChest(request)) => self.open_chest(id, &request, out),
            Some(C2s::ActiveChest(active)) => {
                self.close_chest(id, &active, out);
                Ok(())
            }
            Some(C2s::UpdateChestItem(update)) => self.update_chest_item(id, &update, out),
            Some(C2s::DoorToggle(door)) => self.toggle_door(id, &door, frame, out),
            Some(C2s::ModifyTile(change)) => self.modify_tile(id, &change, frame, out),
            Some(C2s::RequestSign(request)) => self.read_sign(id, &request, out),
            Some(C2s::UpdateSign(update)) => self.edit_sign(id, &update, out),
            _ if relayed => Ok(()),
            _ => {
                debug_log!("[cid {id}] unhandled packet {}", frame.kind);
                Ok(())
            }
        }
    }
}

fn refusal(reason: &'static str) -> Disconnect<'static> {
    Disconnect { reason: NetworkText::Literal(reason) }
}
