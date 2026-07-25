use thiserror::Error;

#[derive(Debug, Error)]
pub enum Uleb128Error {
    #[error("value overflowed target integer width")]
    Overflow,

    #[error("unterminated: ran out of bytes before continuation bit cleared")]
    Unterminated,
}

pub type Uleb128Result<T = ()> = Result<T, Uleb128Error>;

