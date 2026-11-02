macro_rules! debug_log {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            eprintln!("[cobalt] {}", format_args!($($arg)*));
        }
    };
}

pub(crate) use debug_log;
