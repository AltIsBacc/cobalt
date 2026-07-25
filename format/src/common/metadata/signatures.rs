use strum::{AsRefStr, Display, EnumString};

#[derive(Debug, Display, EnumString, AsRefStr, PartialEq, Eq, Clone, Copy)]
#[strum(serialize_all = "lowercase")]
pub enum SaveSignature {
    Relogic,
    Xindong,
}

