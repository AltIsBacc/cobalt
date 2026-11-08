macro_rules! debug_log {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            eprintln!("[cobalt] {}", format_args!($($arg)*))
        }
    };
}

macro_rules! warn_log {
    ($($arg:tt)*) => {
        eprintln!("[cobalt] warn: {}", format_args!($($arg)*))
    };
}

pub(crate) use debug_log;
pub(crate) use warn_log;
