#![macro_use]

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            libc_eprintln!($($arg)*)
        }
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        libc_eprintln!($($arg)*)
    };
}
