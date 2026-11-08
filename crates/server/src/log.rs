use std::fmt::{self, Write as _};
use std::io::{self, Write as _};

macro_rules! debug_log {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            $crate::log::emit("", format_args!($($arg)*))
        }
    };
}

macro_rules! warn_log {
    ($($arg:tt)*) => {
        $crate::log::emit("warn: ", format_args!($($arg)*))
    };
}

pub(crate) use debug_log;
pub(crate) use warn_log;

pub(crate) fn emit(label: &str, args: fmt::Arguments<'_>) {
    let mut line = String::with_capacity(128);
    let _ = writeln!(line, "[cobalt] {label}{args}");
    let _ = io::stderr().lock().write_all(line.as_bytes());
}
