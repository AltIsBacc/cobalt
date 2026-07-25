use enumflags2::{BitFlags, bitflags};

#[bitflags]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum MechanicalState {
    HasRedWire,
    HasGreenWire,
    HasBlueWire,
    HasYellowWire,
    HasActuator,
}

pub type Mechanicals = BitFlags<MechanicalState, u8>;

