#!/bin/sh
# Installs Statac on a Mac or Linux from a release's package:
#
#     curl -fsSL https://get.statac.dev | sh
#     curl -fsSL https://get.statac.dev | sh -s -- --version 0.1.0 --to ~/bin
#
# It downloads the package for this machine, checks it against the release's
# SHA256SUMS, and puts `statac` in ~/.local/bin, or in the folder --to names.
# The licences that travel with the program go to ~/.local/share/statac.
# It asks for no password and changes no file of the shell's.
set -eu

releases=${STATAC_RELEASES:-https://github.com/statac-ssg/statac/releases}

stop() {
    echo "$1" >&2
    [ $# -lt 2 ] || echo "  $2" >&2
    exit 1
}

# fetch <address> <file>
fetch() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    else
        wget -q -O "$2" "$1"
    fi
}

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1"
    else
        shasum -a 256 "$1"
    fi | cut -d ' ' -f 1
}

# Everything runs from here, called on the last line, so a download cut
# short runs nothing.
main() {
    version=
    to=
    while [ $# -gt 0 ]; do
        case $1 in
            --version | --to)
                [ $# -ge 2 ] || stop "Missing value: \`$1\` needs one after it."
                case $1 in
                    --version) version=${2#v} ;;
                    --to) to=$2 ;;
                esac
                shift 2
                ;;
            *)
                stop "Unknown option: \`$1\`" "The options are \`--version\` and \`--to\`."
                ;;
        esac
    done

    system=$(uname -s)
    processor=$(uname -m)
    case $processor in
        x86_64 | amd64) processor=x86_64 ;;
        aarch64 | arm64) processor=aarch64 ;;
        *) processor= ;;
    esac
    case $system in
        Darwin)
            # A shell running under Rosetta says x86_64 on Apple silicon.
            if [ "$(sysctl -n hw.optional.arm64 2>/dev/null)" = 1 ]; then
                processor=aarch64
            fi
            target=$processor-apple-darwin
            ;;
        # The static package, which runs on any distribution.
        Linux) target=$processor-unknown-linux-musl ;;
        *) processor= ;;
    esac
    [ -n "$processor" ] ||
        stop "No package for this machine: $(uname -s) on $(uname -m)" \
            "Other ways to install: https://statac.dev/install/"

    command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1 ||
        stop "Can't download Statac: this machine has neither \`curl\` nor \`wget\`." \
            "Install one, then run this again."
    command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1 ||
        stop "Can't check the package: this machine has neither \`sha256sum\` nor \`shasum\`." \
            "Install one, then run this again."

    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT

    if [ -n "$version" ]; then
        sums=$releases/download/v$version/SHA256SUMS
        fetch "$sums" "$work/SHA256SUMS" ||
            stop "Can't download $sums" \
                "Is $version a release? They are listed at $releases"
    else
        sums=$releases/latest/download/SHA256SUMS
        fetch "$sums" "$work/SHA256SUMS" || stop "Can't download $sums"
    fi

    # A line is the hash, a space, a space or an asterisk, and the name.
    line=$(grep -e "-$target\\.tar\\.gz\$" "$work/SHA256SUMS" || true)
    [ -n "$line" ] ||
        stop "No package for this machine in the release: nothing in $sums ends \`$target.tar.gz\`." \
            "Other ways to install: https://statac.dev/install/"
    expected=${line%% *}
    package=${line##*[ \*]}
    name=${package%.tar.gz}
    version=${name#statac-}
    version=${version%-"$target"}

    echo "Downloading Statac $version ($target)"
    address=$releases/download/v$version/$package
    fetch "$address" "$work/$package" || stop "Can't download $address"

    found=$(sha256 "$work/$package")
    [ "$found" = "$expected" ] ||
        stop "The package doesn't match the release's SHA256SUMS, so nothing was installed." \
            "Expected $expected, and the download is $found."

    tar -xzf "$work/$package" -C "$work" ||
        stop "Can't unpack $package, so nothing was installed."
    [ -f "$work/$name/statac" ] ||
        stop "The package holds no \`$name/statac\`, so nothing was installed."

    to=${to:-$HOME/.local/bin}
    mkdir -p "$to" || stop "Can't make the folder $to"
    to=$(cd "$to" && pwd)
    # Written beside its place and moved in, so a `statac` that is running
    # is replaced whole.
    cp "$work/$name/statac" "$to/.statac-new" || stop "Can't write to $to"
    chmod 755 "$to/.statac-new"
    mv -f "$to/.statac-new" "$to/statac"

    notices=${XDG_DATA_HOME:-$HOME/.local/share}/statac
    mkdir -p "$notices" || stop "Can't make the folder $notices"
    cp "$work/$name/LICENSE-MIT" "$work/$name/LICENSE-APACHE" \
        "$work/$name/THIRD-PARTY.md" "$work/$name/THIRD-PARTY-GRAMMARS.md" \
        "$notices/" || stop "Can't write to $notices"

    said=$("$to/statac" --version 2>&1) ||
        stop "Installed $to/statac, and it doesn't run on this machine:" "$said"
    echo "Installed $said at $to/statac"
    echo "Its licences are in $notices"

    case :$PATH: in
        *:"$to":*)
            first=$(command -v statac || true)
            if [ "$first" != "$to/statac" ]; then
                echo
                echo "Another \`statac\` comes first on your PATH: $first"
                echo "  Until it is removed, \`statac\` runs that one."
            fi
            ;;
        *)
            echo
            echo "$to isn't on your PATH, so \`statac\` alone won't be found."
            case ${SHELL##*/} in
                fish) echo "  Add it: fish_add_path \"$to\"" ;;
                zsh) echo "  Add this line to ~/.zshrc: export PATH=\"$to:\$PATH\"" ;;
                bash) echo "  Add this line to ~/.bashrc: export PATH=\"$to:\$PATH\"" ;;
                *) echo "  Add this line to your shell's startup file: export PATH=\"$to:\$PATH\"" ;;
            esac
            ;;
    esac
}

main "$@"
