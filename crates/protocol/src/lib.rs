#[macro_use]
mod macros;

pub mod registry;

mod codec;
mod error;
mod field;
mod frame;
mod packet;
mod text;

pub use codec::{Reader, Writer};
pub use error::{ProtocolError, ProtocolResult};
pub use field::{Buff, BuffList, Color, Decode, Encode, MAX_BUFFS, SpawnPoint, SpawnPoints};
pub use frame::{Frame, Frames, HEADER_LEN};
pub use packet::{PROTOCOL_VERSION, Packet};
pub use registry::{C2s, S2c};
pub use text::NetworkText;
