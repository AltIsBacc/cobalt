use enumflags2::{BitFlags, bitflags};

#[bitflags]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MechanicalState {
    HasRedWire,
    HasGreenWire,
    HasBlueWire,
    HasYellowWire,
    HasActuator,
}

pub type Mechanicals = BitFlags<MechanicalState>;
