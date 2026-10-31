use crate::{error::ProtocolResult, frame::Frame};

pub trait Message: Sized {
    fn decode(frame: Frame<'_>) -> ProtocolResult<Self>;

    fn encode(&self, out: &mut Vec<u8>) -> ProtocolResult;
}
