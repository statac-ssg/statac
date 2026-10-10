# What the toolchain adds

A binary for Linux that asks nothing of the system's C library has two
pieces of Rust's toolchain linked in beside the standard library: a C
library and an unwinder. They come with the standard library for
`x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`, in its
`self-contained` folder, and are read for Rust 1.99.0 from
https://github.com/rust-lang/rust/tree/1.99.0: `src/ci/docker/scripts/musl.sh`
and the commit of `src/llvm-project`. The texts beside this file are
copies of theirs. No other binary links either: on macOS and on Linux
with glibc both are the system's own, and a binary for Windows links
Microsoft's C runtime, which its terms let a program carry and which
asks for no notice.

- musl 1.2.5, the C library of a static Linux binary: MIT. Copyright (c) 2005-2020 Rich Felker, et al. https://musl.libc.org. `LICENSE-musl.txt`
- libunwind, from LLVM as Rust 1.99.0 builds it, the unwinder of a static Linux binary: Apache-2.0 WITH LLVM-exception. Copyright (c) 2009-2019 by the contributors listed in LLVM's `CREDITS.TXT`. https://github.com/rust-lang/llvm-project. `LICENSE-libunwind.txt`
