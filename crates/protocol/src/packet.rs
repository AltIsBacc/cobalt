pub const PROTOCOL_VERSION: &str = "Terraria326";

pub trait Packet {
    const ID: u8;
}
