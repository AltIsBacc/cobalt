use cobalt_format::world::tile::block::{Block, BlockShape};
use cobalt_format::world::tile::paint::Paint;
use cobalt_format::world::tile::wall::Wall;
use cobalt_protocol::Frame;
use cobalt_protocol::registry::{DoorToggle, ModifyTile};

use super::error::ClientError;
use super::{Engine, Outbox};
use crate::model::ClientId;

const KILL_TILE: u8 = 0;
const PLACE_TILE: u8 = 1;
const KILL_WALL: u8 = 2;
const PLACE_WALL: u8 = 3;
const KILL_TILE_QUIET: u8 = 4;
const FAILED: i16 = 1;

impl Engine {
    pub(super) fn toggle_door(
        &self,
        id: ClientId,
        door: &DoorToggle,
        frame: Frame<'_>,
        out: &mut Outbox,
    ) -> Result<(), ClientError> {
        let in_world = coordinates(door.x, door.y)
            .is_some_and(|(x, y)| self.world.tiles.get(x, y).is_some());
        if in_world {
            self.forward(frame, id, out)?;
        }
        Ok(())
    }

    pub(super) fn modify_tile(
        &mut self,
        id: ClientId,
        change: &ModifyTile,
        frame: Frame<'_>,
        out: &mut Outbox,
    ) -> Result<(), ClientError> {
        let Some((x, y)) = coordinates(change.x, change.y) else { return Ok(()) };
        let frame_important = &self.world.frame_important;
        let Some(tile) = self.world.tiles.get_mut(x, y) else { return Ok(()) };

        match change.action {
            KILL_TILE | KILL_TILE_QUIET if change.flags1 != FAILED => {
                tile.block = tile.block.filter(|block| frame_important.contains(block.id));
            }
            PLACE_TILE if tile.block.is_none() => {
                tile.block = u16::try_from(change.flags1)
                    .ok()
                    .filter(|&block| !frame_important.contains(block))
                    .map(plain_block);
            }
            KILL_WALL if change.flags1 != FAILED => tile.wall = None,
            PLACE_WALL if tile.wall.is_none() => {
                tile.wall =
                    u16::try_from(change.flags1).ok().filter(|&wall| wall != 0).map(plain_wall);
            }
            _ => {}
        }

        self.forward(frame, id, out)
    }
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
