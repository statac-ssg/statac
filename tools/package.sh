#!/bin/sh
# Packages a built binary with the licences that must travel with it:
#
#     tools/package.sh <version> <target triple> <folder for the package>
#
# It reads the binary `cargo build --release --target <target triple>` left
# under CARGO_TARGET_DIR (or `target`), and writes
# statac-<version>-<target triple>.tar.gz (.zip for Windows) and, beside it,
# the same name with .sha256 added.
set -eu

if [ $# -ne 3 ]; then
    echo "usage: tools/package.sh <version> <target triple> <folder for the package>" >&2
    exit 2
fi
version=$1
target=$2
out=$3

root=$(cd "$(dirname "$0")/.." && pwd)
built=${CARGO_TARGET_DIR:-$root/target}/$target/release

case $target in
    *-windows-*) binary=statac.exe ;;
    *) binary=statac ;;
esac
if [ ! -f "$built/$binary" ]; then
    echo "no binary at $built/$binary" >&2
    exit 1
fi

# The binary names its own version, so a package can't be given another's.
# One built for another processor can't be asked and is taken at its word.
if said=$("$built/$binary" --version 2>/dev/null); then
    if [ "$said" != "Statac $version" ]; then
        echo "the binary says \`$said\`, not \`Statac $version\`" >&2
        exit 1
    fi
fi

name=statac-$version-$target
mkdir -p "$out"
out=$(cd "$out" && pwd)
staging=$(mktemp -d)
trap 'rm -rf "$staging"' EXIT
mkdir "$staging/$name"
cp "$built/$binary" "$staging/$name/"
cp "$root/LICENSE-MIT" "$root/LICENSE-APACHE" "$root/THIRD-PARTY.md" \
    "$root/THIRD-PARTY-GRAMMARS.md" "$staging/$name/"

case $target in
    *-apple-*)
        strip "$staging/$name/$binary"
        # Stripping changes the bytes the linker signed, and an Apple silicon
        # Mac runs nothing whose signature doesn't match.
        codesign --force --sign - "$staging/$name/$binary"
        ;;
    *-linux-*)
        strip "$staging/$name/$binary"
        ;;
esac

cd "$staging"
case $target in
    *-windows-*)
        package=$name.zip
        # The shell that comes with Git for Windows has no zip.
        powershell.exe -NoProfile -Command \
            "Compress-Archive -Path '$name' -DestinationPath '$package'"
        ;;
    *)
        package=$name.tar.gz
        # Otherwise the tar of macOS adds a ._ file beside each file that has
        # extended attributes.
        COPYFILE_DISABLE=1 tar -czf "$package" "$name"
        ;;
esac

if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$package" >"$package.sha256"
else
    shasum -a 256 "$package" >"$package.sha256"
fi
mv "$package" "$package.sha256" "$out/"
echo "$out/$package"
cat "$out/$package.sha256"
