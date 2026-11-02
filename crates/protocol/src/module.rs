use crate::codec::{Reader, Writer};
use crate::error::ProtocolResult;
use crate::field::{Color, Decode, Encode};
use crate::text::NetworkText;

pub const CHAT_MODULE: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRequest<'data> {
    pub command: &'data str,
    pub text: &'data str,
}

impl<'data> Decode<'data> for ChatRequest<'data> {
    fn decode(reader: &mut Reader<'data>) -> ProtocolResult<Self> {
        Ok(Self { command: reader.string()?, text: reader.string()? })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatBroadcast<'data> {
    pub author: u8,
    pub text: NetworkText<'data>,
    pub color: Color,
}

impl Encode for ChatBroadcast<'_> {
    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
        self.author.encode(writer)?;
        self.text.encode(writer)?;
        self.color.encode(writer)
    }
}
