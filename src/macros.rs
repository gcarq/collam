#![macro_use]

#[macro_export]
macro_rules! debug_assert {
    ($($arg:tt)*) => (if cfg!(feature = "debug") { assert!($($arg)*); })
}

#[macro_export]
macro_rules! debug_assert_eq {
    ($($arg:tt)*) => (if cfg!(feature = "debug") { assert_eq!($($arg)*); })
}

#[macro_export]
macro_rules! debug_assert_ne {
    ($($arg:tt)*) => (if cfg!(feature = "debug") { assert_ne!($($arg)*); })
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        libc_eprintln!($($arg)*)
    };
}

#[macro_export]
macro_rules! dprintln {
    ($($arg:tt)*) => {
        if cfg!(feature = "debug") {
            println!($($arg)*)
        }
    };
}

#[macro_export]
macro_rules! eprintln {
    ($($arg:tt)*) => {
        println!($($arg)*)
    };
}
