use crate::codec::{Reader, Writer};
use crate::error::{ProtocolError, ProtocolResult};
use crate::field::{Decode, Encode};
use crate::packet::{C2s, Packet, S2c};

pub const HEADER_LEN: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame<'data> {
    pub kind: u8,
    pub payload: &'data [u8],
}

impl<'data> Frame<'data> {
    pub fn parse(buffer: &'data [u8]) -> ProtocolResult<Option<(Self, usize)>> {
        let Some(&[low, high]) = buffer.first_chunk::<2>() else { return Ok(None) };
        let len = usize::from(u16::from_le_bytes([low, high]));
        if len < HEADER_LEN {
            return Err(ProtocolError::FrameTooShort(len));
        }
        let Some(frame) = buffer.get(..len) else { return Ok(None) };

        Ok(Some((Self { kind: frame[2], payload: &frame[HEADER_LEN..] }, len)))
    }

    pub fn decode_c2s<P: C2s + Decode<'data>>(&self) -> ProtocolResult<P> {
        self.decode()
    }

    pub fn decode_s2c<P: S2c + Decode<'data>>(&self) -> ProtocolResult<P> {
        self.decode()
    }

    fn decode<P: Packet + Decode<'data>>(&self) -> ProtocolResult<P> {
        if self.kind != P::ID {
            return Err(ProtocolError::UnknownMessage(self.kind));
        }
        P::decode(&mut Reader::new(self.payload))
    }
}

pub fn write_c2s<P: C2s + Encode>(out: &mut Vec<u8>, packet: &P) -> ProtocolResult {
    write(out, packet)
}

pub fn write_s2c<P: S2c + Encode>(out: &mut Vec<u8>, packet: &P) -> ProtocolResult {
    write(out, packet)
}

fn write<P: Packet + Encode>(out: &mut Vec<u8>, packet: &P) -> ProtocolResult {
    let start = out.len();
    out.extend_from_slice(&[0, 0, P::ID]);

    let written = packet.encode(&mut Writer::new(out));
    let len = out.len() - start;

    match written.and_then(|()| u16::try_from(len).map_err(|_| ProtocolError::PacketTooLarge(len)))
    {
        Ok(len) => {
            out[start..start + 2].copy_from_slice(&len.to_le_bytes());
            Ok(())
        }
        Err(error) => {
            out.truncate(start);
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::s2c::Disconnect;
    use crate::text::NetworkText;

    #[test]
    fn waits_for_a_complete_frame() {
        let partial: [&[u8]; 5] = [&[], &[5], &[5, 0], &[5, 0, 1], &[5, 0, 1, 9]];
        for buffer in partial {
            assert!(Frame::parse(buffer).unwrap().is_none());
        }
    }

    #[test]
    fn parses_one_frame_and_reports_its_length() {
        let (frame, used) = Frame::parse(&[4, 0, 9, 7, 3, 0, 1]).unwrap().unwrap();
        assert_eq!(frame, Frame { kind: 9, payload: &[7] });
        assert_eq!(used, 4);
    }

    #[test]
    fn rejects_lengths_below_the_header() {
        assert!(matches!(Frame::parse(&[2, 0]), Err(ProtocolError::FrameTooShort(2))));
    }

    #[test]
    fn failed_encodes_leave_the_buffer_untouched() {
        let mut out = vec![1, 2, 3];

        let args = vec![NetworkText::Literal(""); 256];
        let reason = NetworkText::Formatted { text: "", args: Cow::Owned(args) };
        assert!(matches!(
            write_s2c(&mut out, &Disconnect { reason }),
            Err(ProtocolError::TooManySubstitutions(256))
        ));
        assert_eq!(out, [1, 2, 3]);

        let long = "a".repeat(usize::from(u16::MAX));
        let reason = NetworkText::Literal(&long);
        assert!(matches!(
            write_s2c(&mut out, &Disconnect { reason }),
            Err(ProtocolError::PacketTooLarge(_))
        ));
        assert_eq!(out, [1, 2, 3]);
    }
}
