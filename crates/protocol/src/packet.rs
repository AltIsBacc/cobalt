pub const PROTOCOL_VERSION: &str = "Terraria326";

pub trait Packet {
    const ID: u8;
}

pub trait C2s: Packet {}

pub trait S2c: Packet {}
