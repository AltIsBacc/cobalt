use thiserror::Error;


#[derive(Debug, Error)]
pub enum DateTimeError {
    #[error("ticks value predates the Unix epoch, cannot represent")]
    BeforeUnixEpoch,
    #[error("ticks value out of chrono's representable range")]
    OverCalendarCeiling,
    #[error("the converted local wall-clock coordinates map into an unresolvable or missing runtime timezone gap")]
    InvalidLocalTime,
}

pub type DateTimeResult<T = ()> = Result<T, DateTimeError>;

