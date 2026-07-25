use chrono::{DateTime as ChronoDateTime, Local, NaiveDateTime, TimeZone, Utc};

use crate::io::csharp_datetime::error::{DateTimeError, DateTimeResult};

pub mod error;

const TICKS_MASK: u64 = 0x3FFF_FFFF_FFFF_FFFF;
const FLAGS_MASK: u64 = 0xC000_0000_0000_0000;

const KIND_UTC: u64         = 0x4000_0000_0000_0000;
const KIND_LOCAL: u64       = 0x8000_0000_0000_0000;
const KIND_LOCAL_DST: u64   = 0xC000_0000_0000_0000;

const DOTNET_UNIX_EPOCH_TICKS: u64 = 621_355_968_000_000_000; 
const TICKS_PER_SECOND: u64 = 10_000_000;
const DOTNET_MAX_TICKS: u64 = 3_155_378_975_999_999_999; 

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CSharpDateTime {
    Unspecified(NaiveDateTime),
    Utc(ChronoDateTime<Utc>),
    Local(ChronoDateTime<Local>),
}

impl CSharpDateTime {
    pub fn from_dotnet_date_data(raw: u64) -> DateTimeResult<Self> {
        let flags = raw & FLAGS_MASK;
        let ticks = raw & TICKS_MASK;

        if ticks < DOTNET_UNIX_EPOCH_TICKS {
            return Err(DateTimeError::BeforeUnixEpoch);
        }

        if ticks > DOTNET_MAX_TICKS {
            return Err(DateTimeError::OverCalendarCeiling);
        }

        let unix_ticks = ticks - DOTNET_UNIX_EPOCH_TICKS;
        let secs = (unix_ticks / TICKS_PER_SECOND) as i64;
        let nanos = ((unix_ticks % TICKS_PER_SECOND) * 100) as u32;

        // SAFETY: bounds check is done above
        let base_dt = ChronoDateTime::from_timestamp(secs, nanos).unwrap();

        match flags {
            KIND_UTC => Ok(CSharpDateTime::Utc(base_dt)),
            KIND_LOCAL | KIND_LOCAL_DST =>
                Local
                    .from_local_datetime(&base_dt.naive_utc())
                    .single()
                    .map(CSharpDateTime::Local)
                    .ok_or(DateTimeError::InvalidLocalTime),
            _ => Ok(CSharpDateTime::Unspecified(base_dt.naive_utc()))
        }
    }

    pub fn to_dotnet_date_data(&self) -> u64 {
        let (unix_secs, subsec_nanos, flags) = match self {
            CSharpDateTime::Utc(dt) => (dt.timestamp(), dt.timestamp_subsec_nanos(), KIND_UTC),
            CSharpDateTime::Local(dt) => (dt.timestamp(), dt.timestamp_subsec_nanos(), KIND_LOCAL),
            CSharpDateTime::Unspecified(naive) => {
                (naive.and_utc().timestamp(), naive.and_utc().timestamp_subsec_nanos(), 0)
            }
        };

        let unix_ticks = (unix_secs as u64) * TICKS_PER_SECOND + (subsec_nanos as u64) / 100;
        let ticks = unix_ticks + DOTNET_UNIX_EPOCH_TICKS;

        (ticks & TICKS_MASK) | flags
    }
}

