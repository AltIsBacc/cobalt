use cobalt_protocol::registry::{RequestSign, UpdateSign};

use super::error::ClientError;
use super::world_info::narrow;
use super::{Outbox, State};
use crate::model::ClientId;

const MAX_SIGN_BYTES: usize = 1_000;

impl State {
    pub(super) fn read_sign(
        &self,
        id: ClientId,
        request: &RequestSign,
        out: &mut Outbox,
    ) -> Result<(), ClientError> {
        let (x, y) = (i32::from(request.x), i32::from(request.y));
        let Some(index) = self.world.signs.iter().position(|sign| sign.x == x && sign.y == y)
        else {
            return Ok(());
        };

        let sign = UpdateSign {
            sign_id: narrow(index),
            x: request.x,
            y: request.y,
            text: &self.world.signs[index].text,
            player_id: id.get(),
            flags: 0,
        };
        out.send(id, sign.into())?;
        Ok(())
    }

    pub(super) fn edit_sign(
        &mut self,
        id: ClientId,
        update: &UpdateSign<'_>,
        out: &mut Outbox,
    ) -> Result<(), ClientError> {
        if update.text.len() > MAX_SIGN_BYTES {
            return Ok(());
        }
        let (x, y) = (i32::from(update.x), i32::from(update.y));
        let Some(sign) = self.world.signs.iter_mut().find(|sign| sign.x == x && sign.y == y)
        else {
            return Ok(());
        };
        sign.text = update.text.to_owned();
        self.invalidate_section(x, y);

        let relayed = UpdateSign { player_id: id.get(), ..update.clone() };
        out.broadcast(self.audience(Some(id)), relayed.into());
        Ok(())
    }
}
