use cobalt_protocol::registry::{RequestSign, UpdateSign};
use cobalt_protocol::{C2s, Packet};

use crate::engine::world_info::narrow;
use crate::engine::{
    Audience, ClientError, Context, GameEvent, Handled, Incoming, Module, Subscription, Verdict,
};
use crate::model::ClientId;

const MAX_SIGN_BYTES: usize = 1_000;

pub(super) struct SignModule;

impl Module for SignModule {
    fn name(&self) -> &'static str {
        "signs"
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        vec![Subscription::playing([RequestSign::ID, UpdateSign::ID])]
    }

    fn packet(&mut self, cx: &mut Context<'_>, from: ClientId, incoming: &Incoming<'_>) -> Handled {
        match &incoming.packet {
            Some(C2s::RequestSign(request)) => read(cx, from, request)?,
            Some(C2s::UpdateSign(update)) => edit(cx, from, update),
            _ => return Ok(Verdict::Pass),
        }
        Ok(Verdict::Stop)
    }
}

fn read(cx: &mut Context<'_>, id: ClientId, request: &RequestSign) -> Result<(), ClientError> {
    let (x, y) = (i32::from(request.x), i32::from(request.y));
    let Some(index) = cx.world().signs.iter().position(|sign| sign.x == x && sign.y == y) else {
        return Ok(());
    };

    let text = cx.world().signs[index].text.clone();
    let sign = UpdateSign {
        sign_id: narrow(index),
        x: request.x,
        y: request.y,
        text: &text,
        player_id: id.get(),
        flags: 0,
    };
    cx.send(id, sign.into())
}

fn edit(cx: &mut Context<'_>, id: ClientId, update: &UpdateSign<'_>) {
    if update.text.len() > MAX_SIGN_BYTES {
        return;
    }
    let (x, y) = (i32::from(update.x), i32::from(update.y));
    let Some(sign) = cx.world_mut().signs.iter_mut().find(|sign| sign.x == x && sign.y == y) else {
        return;
    };
    sign.text = update.text.to_owned();
    cx.emit(GameEvent::TileChanged { x, y });

    let relayed = UpdateSign { player_id: id.get(), ..update.clone() };
    cx.broadcast(Audience::Except(id), relayed.into());
}
