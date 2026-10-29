use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::{
    codec::{Reader, Writer},
    error::{ProtocolError, ProtocolResult},
    frame::{self, Frame},
};

pub const PROTOCOL_VERSION: &str = "Terraria326";

const MAX_TEXT_DEPTH: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum MessageType {
    Connect = 1,
    Disconnect = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
enum TextMode {
    Literal = 0,
    Formatted = 1,
    Localized = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkText {
    Literal(String),
    Formatted { text: String, args: Vec<NetworkText> },
    Localized { key: String, args: Vec<NetworkText> },
}

impl NetworkText {
    pub fn literal(text: impl Into<String>) -> Self {
        Self::Literal(text.into())
    }

    fn read(reader: &mut Reader<'_>, depth: u8) -> ProtocolResult<Self> {
        if depth > MAX_TEXT_DEPTH {
            return Err(ProtocolError::TextTooDeep);
        }

        let mode = TextMode::try_from(reader.u8()?)
            .map_err(|e| ProtocolError::UnknownTextMode(e.number))?;
        let text = reader.string()?.to_owned();

        match mode {
            TextMode::Literal => Ok(Self::Literal(text)),
            TextMode::Formatted => {
                Ok(Self::Formatted { text, args: Self::read_args(reader, depth)? })
            }
            TextMode::Localized => {
                Ok(Self::Localized { key: text, args: Self::read_args(reader, depth)? })
            }
        }
    }

    fn read_args(reader: &mut Reader<'_>, depth: u8) -> ProtocolResult<Vec<Self>> {
        let count = usize::from(reader.u8()?);
        (0..count).map(|_| Self::read(reader, depth + 1)).collect()
    }

    fn write(&self, writer: &mut Writer<'_>) -> ProtocolResult {
        match self {
            Self::Literal(text) => {
                writer.u8(TextMode::Literal.into());
                writer.string(text)
            }
            Self::Formatted { text, args } => {
                Self::write_with_args(writer, TextMode::Formatted, text, args)
            }
            Self::Localized { key, args } => {
                Self::write_with_args(writer, TextMode::Localized, key, args)
            }
        }
    }

    fn write_with_args(
        writer: &mut Writer<'_>,
        mode: TextMode,
        text: &str,
        args: &[Self],
    ) -> ProtocolResult {
        let count = u8::try_from(args.len())
            .map_err(|_| ProtocolError::TooManySubstitutions(args.len()))?;

        writer.u8(mode.into());
        writer.string(text)?;
        writer.u8(count);
        args.iter().try_for_each(|arg| arg.write(writer))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    Connect { version: String },
    Disconnect { reason: NetworkText },
}

impl Packet {
    pub const fn message_type(&self) -> MessageType {
        match self {
            Self::Connect { .. } => MessageType::Connect,
            Self::Disconnect { .. } => MessageType::Disconnect,
        }
    }

    pub fn decode(frame: Frame<'_>) -> ProtocolResult<Self> {
        let kind = MessageType::try_from(frame.kind)
            .map_err(|e| ProtocolError::UnknownMessage(e.number))?;
        let mut reader = Reader::new(frame.payload);

        Ok(match kind {
            MessageType::Connect => Self::Connect { version: reader.string()?.to_owned() },
            MessageType::Disconnect => {
                Self::Disconnect { reason: NetworkText::read(&mut reader, 0)? }
            }
        })
    }

    pub fn encode(&self, out: &mut Vec<u8>) -> ProtocolResult {
        frame::encode(self.message_type().into(), out, |writer| match self {
            Self::Connect { version } => writer.string(version),
            Self::Disconnect { reason } => reason.write(writer),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(packet: &Packet) -> Vec<u8> {
        let mut out = Vec::new();
        packet.encode(&mut out).unwrap();
        let (frame, used) = Frame::parse(&out).unwrap().unwrap();
        assert_eq!(used, out.len());
        assert_eq!(&Packet::decode(frame).unwrap(), packet);
        out
    }

    #[test]
    fn connect_round_trips() {
        let out = round_trip(&Packet::Connect { version: PROTOCOL_VERSION.to_owned() });
        assert_eq!(out.len(), 15);
        assert_eq!(&out[..4], &[15, 0, 1, 11]);
    }

    #[test]
    fn literal_disconnect_matches_the_wire_format() {
        let out = round_trip(&Packet::Disconnect { reason: NetworkText::literal("full") });
        assert_eq!(out, [9, 0, 2, 0, 4, b'f', b'u', b'l', b'l']);
    }

    #[test]
    fn nested_disconnect_round_trips() {
        let reason = NetworkText::Localized {
            key: "Net.CheatingInvalid".to_owned(),
            args: vec![
                NetworkText::literal("a"),
                NetworkText::Formatted {
                    text: "{0}".to_owned(),
                    args: vec![NetworkText::literal("b")],
                },
            ],
        };
        round_trip(&Packet::Disconnect { reason });
    }

    #[test]
    fn unknown_messages_are_reported() {
        let frame = Frame { kind: 200, payload: &[] };
        assert!(matches!(Packet::decode(frame), Err(ProtocolError::UnknownMessage(200))));
    }

    #[test]
    fn truncated_payloads_are_rejected() {
        let frame = Frame { kind: 1, payload: &[11, b'T'] };
        assert!(matches!(Packet::decode(frame), Err(ProtocolError::UnexpectedEof)));
    }

    #[test]
    fn unknown_text_modes_are_rejected() {
        let frame = Frame { kind: 2, payload: &[9, 0] };
        assert!(matches!(Packet::decode(frame), Err(ProtocolError::UnknownTextMode(9))));
    }

    #[test]
    fn deeply_nested_text_is_rejected() {
        let mut payload: Vec<u8> = [1, 0, 1].repeat(usize::from(MAX_TEXT_DEPTH) + 2);
        payload.extend([0, 0]);
        let frame = Frame { kind: 2, payload: &payload };
        assert!(matches!(Packet::decode(frame), Err(ProtocolError::TextTooDeep)));
    }

    #[test]
    fn too_many_substitutions_are_rejected() {
        let reason = NetworkText::Formatted {
            text: String::new(),
            args: vec![NetworkText::literal(""); 256],
        };
        let mut out = Vec::new();
        assert!(matches!(
            Packet::Disconnect { reason }.encode(&mut out),
            Err(ProtocolError::TooManySubstitutions(256))
        ));
        assert!(out.is_empty());
    }
}
