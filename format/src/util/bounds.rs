use derive_more::derive::Constructor;
use getset::{CopyGetters, Getters};

#[derive(Debug, CopyGetters, Getters, Constructor, PartialEq, Eq, Clone, Copy)]
#[getset(get_copy = "pub")]
pub struct Bounds {
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
}

