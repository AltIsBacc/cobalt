use crate::{
    codec::Writer,
    error::{ProtocolError, ProtocolResult},
};

pub const HEADER_LEN: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame<'a> {
    pub kind: u8,
    pub payload: &'a [u8],
}

impl<'a> Frame<'a> {
    pub fn parse(buf: &'a [u8]) -> ProtocolResult<Option<(Self, usize)>> {
        let Some(&[low, high]) = buf.first_chunk::<2>() else { return Ok(None) };
        let len = usize::from(u16::from_le_bytes([low, high]));
        if len < HEADER_LEN {
            return Err(ProtocolError::FrameTooShort(len));
        }
        let Some(frame) = buf.get(..len) else { return Ok(None) };

        Ok(Some((Self { kind: frame[2], payload: &frame[HEADER_LEN..] }, len)))
    }
}

pub(crate) fn encode(
    kind: u8,
    out: &mut Vec<u8>,
    body: impl FnOnce(&mut Writer<'_>) -> ProtocolResult,
) -> ProtocolResult {
    let start = out.len();
    out.extend_from_slice(&[0, 0, kind]);

    let written = body(&mut Writer::new(out));
    let len = out.len() - start;

    let framed = written.and_then(|()| {
        u16::try_from(len).map_err(|_| ProtocolError::PacketTooLarge(len))
    });

    match framed {
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
    use super::*;

    #[test]
    fn waits_for_a_complete_frame() {
        let partial: [&[u8]; 5] = [&[], &[5], &[5, 0], &[5, 0, 1], &[5, 0, 1, 9]];
        for buf in partial {
            assert!(Frame::parse(buf).unwrap().is_none());
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
        let result = encode(1, &mut out, |writer| {
            writer.u8(0);
            Err(ProtocolError::UnexpectedEof)
        });
        assert!(result.is_err());
        assert_eq!(out, [1, 2, 3]);

        let result = encode(1, &mut out, |writer| {
            (0..=u16::MAX).for_each(|_| writer.u8(0));
            Ok(())
        });
        assert!(matches!(result, Err(ProtocolError::PacketTooLarge(_))));
        assert_eq!(out, [1, 2, 3]);
    }
}
