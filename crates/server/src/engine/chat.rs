use cobalt_protocol::module::{CHAT_MODULE, ChatBroadcast, ChatRequest};
use cobalt_protocol::registry::NetModule;
use cobalt_protocol::{Color, Decode, Encode, NetworkText, Reader, Remainder, Writer};

use super::error::ClientError;
use super::{Engine, Outbox};
use crate::model::ClientId;

const SAY: &str = "Say";
const MAX_CHAT_BYTES: usize = 512;
const WHITE: Color = Color { r: 255, g: 255, b: 255 };

impl Engine {
    pub(super) fn handle_module(
        &self,
        id: ClientId,
        module: &NetModule<'_>,
        out: &mut Outbox,
    ) -> Result<(), ClientError> {
        if module.module != CHAT_MODULE {
            return Ok(());
        }
        let request = ChatRequest::decode(&mut Reader::new(module.rest.0))?;
        self.say(id, &request, out)
    }

    fn say(
        &self,
        id: ClientId,
        request: &ChatRequest<'_>,
        out: &mut Outbox,
    ) -> Result<(), ClientError> {
        let text = request.text.trim();
        if request.command != SAY
            || text.is_empty()
            || text.len() > MAX_CHAT_BYTES
            || text.starts_with('/')
        {
            return Ok(());
        }
        let Some(player) = self.player(id) else { return Ok(()) };

        let line = format!("<{}> {text}", player.name);
        let mut body = Vec::new();
        ChatBroadcast { author: id.get(), text: NetworkText::Literal(&line), color: WHITE }
            .encode(&mut Writer::new(&mut body))?;
        let module = NetModule { module: CHAT_MODULE, rest: Remainder(&body) };
        self.broadcast(module.into(), None, out);
        Ok(())
    }
}
