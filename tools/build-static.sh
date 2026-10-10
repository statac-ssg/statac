#!/bin/sh
# Builds, tests and packages a Linux binary that needs no C library from the
# system it runs on. It is run in an Alpine container, as root, with the
# source as the working folder:
#
#     tools/build-static.sh <version> <Rust version> <target triple> <user id> <folder for the package>
#
# Alpine's compiler links musl and has the kernel's headers, which the TLS
# library reads and the musl package of other distributions leaves out.
set -eu

if [ $# -ne 5 ]; then
    echo "usage: tools/build-static.sh <version> <Rust version> <target triple> <user id> <folder for the package>" >&2
    exit 2
fi
version=$1
rust=$2
target=$3
user=$4
out=$5

if [ "$(id -u)" = 0 ]; then
    apk add --no-cache build-base cmake curl linux-headers nasm perl
    # Root may read and write anything, and the tests of what Statac says
    # about a file it may not read need a user who may not. The user has the
    # id that owns the source, so what is built can be read outside.
    adduser -D -u "$user" builder
    exec su builder -c "sh '$0' '$version' '$rust' '$target' '$user' '$out'"
fi

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs |
    sh -s -- -y --profile minimal --default-toolchain "$rust"
. "$HOME/.cargo/env"

host=$(rustc -vV | sed -n 's/^host: //p')
if [ "$host" != "$target" ]; then
    echo "this machine builds for $host, not $target" >&2
    exit 1
fi
rustc -vV
cmake --version
case $target in x86_64-*) nasm -v ;; esac

cargo build --release --locked --target "$target"
# --release reuses the C libraries the build above compiled.
cargo test --release --locked --no-fail-fast --target "$target"

binary=target/$target/release/statac
if readelf -l "$binary" | grep -q INTERP || readelf -d "$binary" | grep -q NEEDED; then
    echo "$binary asks the system for a loader or a library:" >&2
    readelf -l "$binary" | grep INTERP >&2 || true
    readelf -d "$binary" | grep NEEDED >&2 || true
    exit 1
fi

sh tools/package.sh "$version" "$target" "$out"
