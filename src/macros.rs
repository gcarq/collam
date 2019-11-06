#![macro_use]

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            libc_eprintln!($($arg)*)
        }
    };
}
