#[macro_use]
mod macros;

pub mod c2s;
pub mod s2c;
pub mod shared;

mod codec;
mod error;
mod field;
mod frame;
mod packet;
mod text;

pub use codec::{Reader, Writer};
pub use error::{ProtocolError, ProtocolResult};
pub use field::{Buff, Color, Decode, Encode};
pub use frame::{Frame, HEADER_LEN, write_c2s, write_s2c};
pub use packet::{C2s, PROTOCOL_VERSION, Packet, S2c};
pub use text::NetworkText;
