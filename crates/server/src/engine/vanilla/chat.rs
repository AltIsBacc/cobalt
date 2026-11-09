use cobalt_protocol::module::{CHAT_MODULE, ChatBroadcast, ChatRequest};
use cobalt_protocol::registry::NetModule;
use cobalt_protocol::{C2s, Color, Decode, Encode, NetworkText, Packet, Reader, Remainder, Writer};

use crate::engine::{
    Audience, ClientError, Context, Handled, Incoming, Module, Subscription, Verdict,
};
use crate::model::ClientId;

const SAY: &str = "Say";
const MAX_CHAT_BYTES: usize = 512;
const WHITE: Color = Color { r: 255, g: 255, b: 255 };

pub(super) struct ChatModule;

impl Module for ChatModule {
    fn name(&self) -> &'static str {
        "chat"
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        vec![Subscription::playing([NetModule::ID])]
    }

    fn packet(&mut self, cx: &mut Context<'_>, from: ClientId, incoming: &Incoming<'_>) -> Handled {
        let Some(C2s::NetModule(module)) = &incoming.packet else { return Ok(Verdict::Pass) };
        if module.module != CHAT_MODULE {
            return Ok(Verdict::Pass);
        }
        let request = ChatRequest::decode(&mut Reader::new(module.rest.0))?;
        say(cx, from, &request)?;
        Ok(Verdict::Stop)
    }
}

fn say(cx: &mut Context<'_>, id: ClientId, request: &ChatRequest<'_>) -> Result<(), ClientError> {
    let text = request.text.trim();
    if request.command != SAY
        || text.is_empty()
        || text.len() > MAX_CHAT_BYTES
        || text.starts_with('/')
    {
        return Ok(());
    }

    let Some(_) = cx.player(id) else { return Ok(()) }; // would be impossible to reach (?)
                                                        // TODO: a server /say command

    let mut body = Vec::new();
    ChatBroadcast { author: id.get(), text: NetworkText::Literal(text), color: WHITE }
        .encode(&mut Writer::new(&mut body))?;
    let module = NetModule { module: CHAT_MODULE, rest: Remainder(&body) };
    cx.broadcast(Audience::Everyone, module.into());
    Ok(())
}
