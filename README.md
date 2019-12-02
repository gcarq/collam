# collam
A naive and thread safe general-purpose allocator written in Rust built with `#[no_std]`.
This project started as an experiment to get comfortable with `#[no_std]` environments and `unsafe` Rust.
This library is currently *NOT* stable and I'm sure there are plenty of bugs, be warned!

## A note on its state
Exposed POSIX functions: `malloc`, `calloc`, `realloc`, `free`, `malloc_usable_size`, `mallopt`.
It is currently stable with a lot of tested programs using `LD_PRELOAD`, however it does not implement Rusts `GlobalAlloc` yet.

## Tested platforms
[x] Linux x86_64

## Implementation details
Bookkeeping is currently done with an intrusive doubly linked list.
The overhead for each use allocated block is 16 bytes whereas only 12 bytes of them are used.

## Performance
In regards of memory usage/overhead it is comparable to dlmalloc with tested applications,
however the performance is not there yet.

## Using collam
Make sure you have Rust nightly.
Manually overwrite default allocator:
```
$ cargo build --release
$ LD_PRELOAD="$(pwd)/target/release/libcollam.so" kwrite
```
Or use the test script in the root folder:
```
$ ./test.sh kwrite
```
There are some more helper scripts, see `debug.sh`, `perf.sh` and `report.sh`.


## Execute tests
Tests are not thread safe, make sure to force 1 thread only!
```
$ cargo test -- --test-threads 1
```

## TODO:
* Proper Page handling
* mmap support
* Thread-local allocation
* Logarithmic-time complexity allocation
* Support for different architectures
* Proper logging