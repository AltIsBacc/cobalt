use cobalt_format::world::tile::block::{Block, BlockShape};
use cobalt_format::world::tile::paint::Paint;
use cobalt_format::world::tile::wall::Wall;
use cobalt_protocol::registry::{DoorToggle, ModifyTile};
use cobalt_protocol::{C2s, Frame, Packet};

use crate::engine::{
    Audience, ClientError, Context, GameEvent, Handled, Incoming, Module, Subscription, Verdict,
};
use crate::model::ClientId;

const KILL_TILE: u8 = 0;
const PLACE_TILE: u8 = 1;
const KILL_WALL: u8 = 2;
const PLACE_WALL: u8 = 3;
const KILL_TILE_QUIET: u8 = 4;
const FAILED: i16 = 1;

pub(super) struct TilesModule;

impl Module for TilesModule {
    fn name(&self) -> &'static str {
        "tiles"
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        vec![Subscription::playing([DoorToggle::ID, ModifyTile::ID])]
    }

    fn packet(&mut self, cx: &mut Context<'_>, from: ClientId, incoming: &Incoming<'_>) -> Handled {
        match &incoming.packet {
            Some(C2s::DoorToggle(door)) => toggle_door(cx, from, door, incoming.frame)?,
            Some(C2s::ModifyTile(change)) => modify_tile(cx, from, change, incoming.frame)?,
            _ => return Ok(Verdict::Pass),
        }
        Ok(Verdict::Stop)
    }
}

fn toggle_door(
    cx: &mut Context<'_>,
    id: ClientId,
    door: &DoorToggle,
    frame: Frame<'_>,
) -> Result<(), ClientError> {
    let in_world =
        coordinates(door.x, door.y).is_some_and(|(x, y)| cx.world().tiles.get(x, y).is_some());
    if in_world {
        cx.forward(Audience::Except(id), frame)?;
    }
    Ok(())
}

fn modify_tile(
    cx: &mut Context<'_>,
    id: ClientId,
    change: &ModifyTile,
    frame: Frame<'_>,
) -> Result<(), ClientError> {
    let Some((x, y)) = coordinates(change.x, change.y) else { return Ok(()) };
    let world = cx.world_mut();
    let frame_important = &world.frame_important;
    let Some(tile) = world.tiles.get_mut(x, y) else { return Ok(()) };
    let previous = *tile;
    let mut broken = None;

    match change.action {
        KILL_TILE | KILL_TILE_QUIET if change.flags1 != FAILED => {
            if let Some(block) = tile.block.filter(|block| !frame_important.contains(block.id)) {
                tile.block = None;
                broken = (change.action == KILL_TILE).then_some(block.id);
            }
        }
        PLACE_TILE if tile.block.is_none() => {
            tile.block = u16::try_from(change.flags1)
                .ok()
                .filter(|&block| !frame_important.contains(block))
                .map(plain_block);
        }
        KILL_WALL if change.flags1 != FAILED => tile.wall = None,
        PLACE_WALL if tile.wall.is_none() => {
            tile.wall = u16::try_from(change.flags1).ok().filter(|&wall| wall != 0).map(plain_wall);
        }
        _ => {}
    }
    let changed = *tile != previous;

    if changed {
        let (x, y) = (i32::from(change.x), i32::from(change.y));
        cx.emit(GameEvent::TileChanged { x, y });
    }
    cx.forward(Audience::Except(id), frame)?;
    if let Some(block) = broken {
        cx.emit(GameEvent::TileBroken { by: id, block, x: change.x, y: change.y });
    }
    Ok(())
}

fn coordinates(x: i16, y: i16) -> Option<(usize, usize)> {
    Some((usize::try_from(x).ok()?, usize::try_from(y).ok()?))
}

fn plain_block(id: u16) -> Block {
    Block {
        id,
        frame: None,
        shape: BlockShape::Normal,
        is_actuated: false,
        is_illuminant: false,
        is_echo: false,
        paint: Paint::NONE,
    }
}

fn plain_wall(id: u16) -> Wall {
    Wall { id, is_illuminant: false, is_echo: false, paint: Paint::NONE }
}
