use std::borrow::Cow;

use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::codec::{Reader, Writer};
use crate::error::{ProtocolError, ProtocolResult};
use crate::field::{Decode, Encode};

const MAX_TEXT_DEPTH: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
enum TextMode {
    Literal = 0,
    Formatted = 1,
    Localized = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkText<'a> {
    Literal(&'a str),
    Formatted { text: &'a str, args: Cow<'a, [NetworkText<'a>]> },
    Localized { key: &'a str, args: Cow<'a, [NetworkText<'a>]> },
}

impl<'a> NetworkText<'a> {
    fn read(reader: &mut Reader<'a>, depth: u8) -> ProtocolResult<Self> {
        if depth > MAX_TEXT_DEPTH {
            return Err(ProtocolError::TextTooDeep);
        }

        let mode = TextMode::try_from(reader.u8()?)
            .map_err(|error| ProtocolError::UnknownTextMode(error.number))?;
        let text = reader.string()?;

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

    fn read_args(reader: &mut Reader<'a>, depth: u8) -> ProtocolResult<Cow<'a, [Self]>> {
        let count = usize::from(reader.u8()?);
        (0..count)
            .map(|_| Self::read(reader, depth + 1))
            .collect::<Result<Vec<_>, _>>()
            .map(Cow::Owned)
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

impl<'a> Decode<'a> for NetworkText<'a> {
    fn decode(reader: &mut Reader<'a>) -> ProtocolResult<Self> {
        Self::read(reader, 0)
    }
}

impl Encode for NetworkText<'_> {
    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
        self.write(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c2s::Connect;
    use crate::frame::{Frame, write_c2s, write_s2c};
    use crate::packet::PROTOCOL_VERSION;
    use crate::s2c::Disconnect;

    #[test]
    fn connect_round_trips() {
        let packet = Connect { version: PROTOCOL_VERSION };
        let mut out = Vec::new();
        write_c2s(&mut out, &packet).unwrap();

        let (frame, used) = Frame::parse(&out).unwrap().unwrap();
        assert_eq!(used, out.len());
        assert_eq!(frame.decode_c2s::<Connect>().unwrap(), packet);
        assert_eq!(out.len(), 15);
        assert_eq!(&out[..4], &[15, 0, 1, 11]);
    }

    #[test]
    fn literal_disconnect_matches_the_wire_format() {
        let packet = Disconnect { reason: NetworkText::Literal("full") };
        let mut out = Vec::new();
        write_s2c(&mut out, &packet).unwrap();

        assert_eq!(out, [9, 0, 2, 0, 4, b'f', b'u', b'l', b'l']);
        let (frame, _) = Frame::parse(&out).unwrap().unwrap();
        assert_eq!(frame.decode_s2c::<Disconnect>().unwrap(), packet);
    }

    #[test]
    fn nested_disconnect_round_trips() {
        let inner = [NetworkText::Literal("b")];
        let args = [
            NetworkText::Literal("a"),
            NetworkText::Formatted { text: "{0}", args: Cow::Borrowed(&inner) },
        ];
        let packet = Disconnect {
            reason: NetworkText::Localized {
                key: "Net.CheatingInvalid",
                args: Cow::Borrowed(&args),
            },
        };
        let mut out = Vec::new();
        write_s2c(&mut out, &packet).unwrap();

        let (frame, _) = Frame::parse(&out).unwrap().unwrap();
        assert_eq!(frame.decode_s2c::<Disconnect>().unwrap(), packet);
    }

    #[test]
    fn unknown_messages_are_reported() {
        let frame = Frame { kind: 200, payload: &[] };
        assert!(matches!(frame.decode_c2s::<Connect>(), Err(ProtocolError::UnknownMessage(200))));
    }

    #[test]
    fn truncated_payloads_are_rejected() {
        let frame = Frame { kind: 1, payload: &[11, b'T'] };
        assert!(matches!(frame.decode_c2s::<Connect>(), Err(ProtocolError::UnexpectedEof)));
    }

    #[test]
    fn unknown_text_modes_are_rejected() {
        let frame = Frame { kind: 2, payload: &[9, 0] };
        assert!(matches!(frame.decode_s2c::<Disconnect>(), Err(ProtocolError::UnknownTextMode(9))));
    }

    #[test]
    fn deeply_nested_text_is_rejected() {
        let mut payload: Vec<u8> = [1, 0, 1].repeat(usize::from(MAX_TEXT_DEPTH) + 2);
        payload.extend([0, 0]);
        let frame = Frame { kind: 2, payload: &payload };
        assert!(matches!(frame.decode_s2c::<Disconnect>(), Err(ProtocolError::TextTooDeep)));
    }
}
