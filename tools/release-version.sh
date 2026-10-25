#!/bin/sh
# Stops a release run asked for a version other than Cargo.toml's:
#
#     tools/release-version.sh <version>
set -eu

if [ $# -ne 1 ]; then
    echo "usage: tools/release-version.sh <version>" >&2
    exit 2
fi

root=$(cd "$(dirname "$0")/.." && pwd)
have=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | sed -n 1p)
if [ "$have" != "$1" ]; then
    echo "::error::Cargo.toml's version is $have, not $1"
    exit 1
fi
