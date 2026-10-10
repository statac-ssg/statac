# What Rust's standard library carries

A binary links the standard library of the Rust that builds it. This is
read for Rust 1.99.0 from https://github.com/rust-lang/rust/tree/1.99.0:
its `COPYRIGHT`, `REUSE.toml`, `library/Cargo.lock` and
`library/std/Cargo.toml`, and the crates those name. Neither the
repository nor any crate here has a `NOTICE` file. The texts beside this
file are copies of theirs. A binary for Windows built with Microsoft's
tools links, of the crates from crates.io, only `cfg-if`, `hashbrown` and
`rustc-demangle`.

- Rust's standard library 1.99.0, which is `std`, `core`, `alloc` and the Rust project's other crates built with them: MIT OR Apache-2.0, taken under Apache-2.0. Copyright (c) The Rust Project Contributors. https://github.com/rust-lang/rust.
- In `core`, the tables of Unicode's character properties, `library/core/src/unicode/unicode_data.rs`: Unicode-3.0. Copyright 1991-2024 Unicode, Inc. `LICENSE-Unicode-3.0.txt`
- In `std`, the backtrace crate, `library/backtrace`: MIT OR Apache-2.0, taken under Apache-2.0. Copyright 2014 Alex Crichton.
- In `std`, the channels of `library/std/src/sync/mpmc`, from crossbeam: MIT OR Apache-2.0, taken under Apache-2.0. Copyright 2019 The Crossbeam Project Developers.
- `compiler_builtins` 0.1.160, in `library/compiler-builtins`: MIT AND Apache-2.0 WITH LLVM-exception AND (MIT OR Apache-2.0). `LICENSE-compiler-builtins.txt`
- In `compiler_builtins`, the `math` module, from the libm crate: MIT. `LICENSE-libm.txt`
- `addr2line` 0.27.1, `adler2` 2.0.1, `cfg-if` 1.0.4, `gimli` 0.34.0, `hashbrown` 0.17.1, `libc` 0.2.189, `miniz_oxide` 0.9.1, `object` 0.39.1 and `rustc-demangle` 0.1.28, from crates.io: each offers Apache-2.0, taken under Apache-2.0.
- `memchr` 2.8.3, from crates.io: Unlicense OR MIT, taken under MIT. `COPYING-memchr` `LICENSE-MIT-memchr`
