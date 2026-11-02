use crate::codec::Writer;
use crate::error::{ProtocolError, ProtocolResult};
use crate::field::Encode;
use crate::packet::Packet;

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
}

impl Frame<'_> {
    pub fn write(&self, out: &mut Vec<u8>) -> ProtocolResult {
        let len = HEADER_LEN + self.payload.len();
        let wire_len = u16::try_from(len).map_err(|_| ProtocolError::PacketTooLarge(len))?;
        out.extend_from_slice(&wire_len.to_le_bytes());
        out.push(self.kind);
        out.extend_from_slice(self.payload);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Frames<'data> {
    data: &'data [u8],
}

impl<'data> Frames<'data> {
    pub const fn new(data: &'data [u8]) -> Self {
        Self { data }
    }
}

impl<'data> Iterator for Frames<'data> {
    type Item = ProtocolResult<Frame<'data>>;

    fn next(&mut self) -> Option<Self::Item> {
        match Frame::parse(self.data) {
            Ok(Some((frame, used))) => {
                self.data = &self.data[used..];
                Some(Ok(frame))
            }
            Ok(None) => None,
            Err(error) => {
                self.data = &[];
                Some(Err(error))
            }
        }
    }
}

pub(crate) fn write<P: Packet + Encode>(out: &mut Vec<u8>, packet: &P) -> ProtocolResult {
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
    use crate::registry::{Disconnect, S2c};
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
            S2c::from(Disconnect { reason }).write(&mut out),
            Err(ProtocolError::TooManySubstitutions(256))
        ));
        assert_eq!(out, [1, 2, 3]);

        let long = "a".repeat(usize::from(u16::MAX));
        let reason = NetworkText::Literal(&long);
        assert!(matches!(
            S2c::from(Disconnect { reason }).write(&mut out),
            Err(ProtocolError::PacketTooLarge(_))
        ));
        assert_eq!(out, [1, 2, 3]);
    }
}
