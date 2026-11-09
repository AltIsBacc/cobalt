use cobalt_protocol::registry::{
    Emoji, HealEffect, ManaEffect, PlayerActive, PlayerControls, PlayerDodge, PlayerHp, PlayerInfo,
    PlayerInventorySlot, PlayerItemAnimation, PlayerMana, PlayerTeam, PlayerTeleportPortal,
    PlayerZone, SetPlayerStealth, SpawnPlayer, SyncTilePicking, TogglePvp, UpdatePlayerBuff,
};
use cobalt_protocol::{C2s, Frame, HEADER_LEN, Packet, ProtocolResult, S2c};

use super::LATE;
use crate::config::ServerConfig;
use crate::engine::world_info::narrow;
use crate::engine::{
    Audience, ClientError, Context, GameEvent, Handled, Incoming, Module, StageKind, Subscription,
    Verdict,
};
use crate::model::{ClientId, Item, Player};

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

pub(super) struct RosterModule {
    spawns: Box<[Vec<u8>]>,
    scratch: Vec<u8>,
}

impl RosterModule {
    pub(super) fn new(config: &ServerConfig) -> Self {
        Self { spawns: (0..config.max_players).map(|_| Vec::new()).collect(), scratch: Vec::new() }
    }

    fn enter(
        &mut self,
        cx: &mut Context<'_>,
        id: ClientId,
        frame: Frame<'_>,
    ) -> Result<(), ClientError> {
        let spawn = &mut self.spawns[id.index()];
        spawn.clear();
        write_attributed(frame, id, spawn)?;
        cx.advance(id);
        cx.emit(GameEvent::PlayerEntering { id });
        Ok(())
    }

    fn play(
        &mut self,
        cx: &mut Context<'_>,
        id: ClientId,
        incoming: &Incoming<'_>,
    ) -> Result<Verdict, ClientError> {
        if let (Some(packet), Some(player)) = (&incoming.packet, cx.player_mut(id)) {
            remember(player, incoming.frame, packet)?;
        }
        if !RELAYED.contains(&incoming.frame.kind) {
            return Ok(Verdict::Pass);
        }

        self.scratch.clear();
        write_attributed(incoming.frame, id, &mut self.scratch)?;
        cx.relay_to(Audience::Except(id), &self.scratch);
        Ok(Verdict::Stop)
    }

    fn introduce(&mut self, cx: &mut Context<'_>, id: ClientId) -> Result<(), ClientError> {
        self.scratch.clear();
        for (other, player) in cx.players().filter(|&(other, _)| other != id) {
            describe(other, player, &mut self.scratch)?;
        }
        cx.relay(id, &self.scratch);
        Ok(())
    }

    fn announce(&mut self, cx: &mut Context<'_>, id: ClientId) -> Result<(), ClientError> {
        let Some(player) = cx.player(id) else { return Ok(()) };
        self.scratch.clear();
        describe(id, player, &mut self.scratch)?;
        self.scratch.extend_from_slice(&self.spawns[id.index()]);
        cx.relay_to(Audience::Except(id), &self.scratch);
        Ok(())
    }
}

impl Module for RosterModule {
    fn name(&self) -> &'static str {
        "roster"
    }

    fn priority(&self) -> i32 {
        LATE
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        vec![
            Subscription::packets_in([StageKind::Spawning], [SpawnPlayer::ID]),
            Subscription::every_packet_in([StageKind::Playing]),
        ]
    }

    fn packet(&mut self, cx: &mut Context<'_>, from: ClientId, incoming: &Incoming<'_>) -> Handled {
        match cx.stage(from) {
            Some(StageKind::Spawning) => {
                self.enter(cx, from, incoming.frame)?;
                Ok(Verdict::Stop)
            }
            Some(StageKind::Playing) => self.play(cx, from, incoming),
            _ => Ok(Verdict::Pass),
        }
    }

    fn event(&mut self, cx: &mut Context<'_>, event: &GameEvent) -> Result<(), ClientError> {
        match event {
            GameEvent::PlayerEntering { id } => self.introduce(cx, *id)?,
            GameEvent::PlayerEntered { id } => self.announce(cx, *id)?,
            GameEvent::PlayerLeft { id } => {
                let gone = PlayerActive { player_id: id.get(), active: 0 };
                cx.broadcast(Audience::Everyone, gone.into());
            }
            _ => {}
        }
        Ok(())
    }
}

pub(super) fn remember(
    player: &mut Player,
    frame: Frame<'_>,
    packet: &C2s<'_>,
) -> Result<(), ClientError> {
    match packet {
        C2s::PlayerInfo(info) => {
            player.name = info.name.to_owned();
            player.appearance = frame.payload.to_vec();
        }
        C2s::ClientUuid(client) => player.uuid = client.uuid.to_owned(),
        C2s::PlayerHp(hp) => (player.hp, player.max_hp) = (hp.hp, hp.max_hp),
        C2s::PlayerMana(mana) => (player.mana, player.max_mana) = (mana.mana, mana.max_mana),
        C2s::PlayerTeam(team) => player.team = team.team,
        C2s::UpdatePlayerBuff(buffs) => player.buffs = buffs.buffs,
        C2s::PlayerInventorySlot(slot) => {
            let item = usize::try_from(slot.slot)
                .ok()
                .and_then(|index| player.inventory.get_mut(index))
                .ok_or(ClientError::InvalidSlot(slot.slot))?;
            *item = Item { stack: slot.stack, prefix: slot.prefix, net_id: slot.item_net_id };
        }
        _ => {}
    }
    Ok(())
}

fn write_attributed(frame: Frame<'_>, from: ClientId, out: &mut Vec<u8>) -> ProtocolResult {
    let start = out.len();
    frame.write(out)?;
    if let Some(owner) = out.get_mut(start + HEADER_LEN) {
        *owner = from.get();
    }
    Ok(())
}

fn describe(owner: ClientId, player: &Player, out: &mut Vec<u8>) -> ProtocolResult {
    let player_id = owner.get();

    S2c::from(PlayerActive { player_id, active: 1 }).write(out)?;
    if !player.appearance.is_empty() {
        write_attributed(Frame { kind: PlayerInfo::ID, payload: &player.appearance }, owner, out)?;
    }
    S2c::from(PlayerHp { player_id, hp: player.hp, max_hp: player.max_hp }).write(out)?;
    S2c::from(PlayerMana { player_id, mana: player.mana, max_mana: player.max_mana }).write(out)?;
    S2c::from(PlayerTeam { player_id, team: player.team }).write(out)?;
    S2c::from(UpdatePlayerBuff { player_id, buffs: player.buffs }).write(out)?;

    let held = player.inventory.iter().enumerate().filter(|(_, item)| **item != Item::default());
    for (slot, item) in held {
        let slot = PlayerInventorySlot {
            player_id,
            slot: narrow(slot),
            stack: item.stack,
            prefix: item.prefix,
            item_net_id: item.net_id,
        };
        S2c::from(slot).write(out)?;
    }
    Ok(())
}
