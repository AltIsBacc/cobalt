use enumflags2::bitflags;

#[bitflags]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u64)]
pub enum SaveFlags {
    FAVORITE,
}

