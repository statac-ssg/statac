#!/bin/sh
# Installs Statac on a Mac or Linux from a release's package:
#
#     curl -fsSL https://get.statac.dev | sh
#     curl -fsSL https://get.statac.dev | sh -s -- --to=~/bin --updates=on
#
# In a terminal it asks where to put Statac, ~/.local/bin unless told
# otherwise, and whether Statac keeps itself up to date, reading each
# answer from the terminal, since sh reads the script itself from its
# input. An option answers its question, so `--to` and `--updates` run it
# unattended. With no terminal it takes the answer in brackets and says
# so. A terminal with no input, as `docker run -t` without `-i` gives,
# can't be told from one with someone at it, and waits for an answer:
# there, give both options.
#
# It downloads the package for this machine and checks it against the
# release's SHA256SUMS, and SHA256SUMS against its signature where the
# release has one and the machine can check it: with ssh-keygen from
# OpenSSH 8.1 or later, as every Mac has, or else with OpenSSL 3. The
# licences that travel with the program go to ~/.local/share/statac.
# Beside the program, .statac-install records the answer and where the
# licences are, for `statac upgrade` and `statac uninstall`, which
# removes what this puts in place. It asks for no password and changes no
# file of the shell's.
set -eu

# Each release is in v<version>/ there, and latest/ names the newest.
releases=${STATAC_RELEASES:-https://get.statac.dev}

# The public half of the key a release's SHA256SUMS is signed with, from
# 0.8.0 on: Statac's own, as src/upgrade.rs holds it.
key=9ba90e0b6ec11a149baad9a54d6cf33d15a872e75c08d915f7361225d32febde

# The same key as the line of an allowed signers file that `ssh-keygen -Y
# verify` reads, the signer's name first, and the namespace a release's
# SSH signature is made in. tools/publish.sh signs with both as the tagged
# script holds them.
signer='statac ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJupDgtuwRoUm6rZpU1s8z0VqHLnXAjZFfc2EiXTL+ve'
namespace=release@statac.dev

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

# Whether this machine's ssh-keygen can check an SSH signature, which came
# with OpenSSH 8.1.
verifier() {
    command -v ssh-keygen >/dev/null 2>&1 &&
        ssh-keygen '-?' 2>&1 | grep -q -e '-Y verify'
}

# An OpenSSL that can check a signature of a whole file with an Ed25519
# key, which takes OpenSSL 3's `pkeyutl -rawin`. A Mac's own is LibreSSL,
# which can't, so Homebrew's is looked for where it isn't on the PATH.
checker() {
    for openssl in openssl /opt/homebrew/opt/openssl@3/bin/openssl \
        /usr/local/opt/openssl@3/bin/openssl; do
        if command -v "$openssl" >/dev/null 2>&1 &&
            "$openssl" version 2>/dev/null | grep -q '^OpenSSL [3-9]' &&
            "$openssl" pkeyutl -help 2>&1 | grep -q -e -rawin; then
            echo "$openssl"
            return
        fi
    done
}

# The key in DER, which OpenSSL reads: an Ed25519 public key is these 12
# bytes, then its own 32.
der() {
    hex=302a300506032b6570032100$key
    while [ -n "$hex" ]; do
        rest=${hex#??}
        # shellcheck disable=SC2059
        printf "\\$(printf %03o "0x${hex%"$rest"}")"
        hex=$rest
    done
}

# $1 with the home folder written as ~, to show.
tilde() {
    case $1 in
        "$HOME") echo "~" ;;
        "$HOME"/*) echo "~${1#"$HOME"}" ;;
        *) echo "$1" ;;
    esac
}

# $1 with a ~ at its start read as the home folder, as a shell would have.
expand() {
    # shellcheck disable=SC2088
    case $1 in
        "~") echo "$HOME" ;;
        "~/"*) echo "$HOME/${1#"~/"}" ;;
        *) echo "$1" ;;
    esac
}

# Why Statac can't be put in the folder $1, or nothing where it can: the
# nearest folder on the way that is there must be one that can be
# written in.
unusable() {
    at=$1
    while [ ! -e "$at" ] && [ ! -L "$at" ]; do
        at=$(dirname "$at")
    done
    if [ ! -d "$at" ]; then
        echo "\`$at\` is a file, not a folder."
    elif [ ! -w "$at" ]; then
        echo "You can't write in \`$at\`."
    fi
}

# The answer the marker in the folder $1 already holds, on or off, or
# nothing: the first line naming `updates` counts, as Statac reads it.
answered() {
    [ -f "$1/.statac-install" ] || return 0
    value=$(sed -n 's/^[[:space:]]*updates[[:space:]]*:[[:space:]]*//p' "$1/.statac-install" |
        head -n 1 | sed 's/[[:space:]]*$//')
    case $value in
        on | off) echo "$value" ;;
    esac
}

# ask <answer in brackets>: prints the prompt and reads an answer from
# the terminal into `answer`, the one in brackets where it is left empty.
ask() {
    printf '  [%s] > ' "$1" >/dev/tty
    IFS= read -r answer </dev/tty ||
        stop "No answer, so nothing was installed."
    answer=${answer:-$1}
}

# Everything runs from here, called on the last line, so a download cut
# short runs nothing.
main() {
    version=
    to=
    updates=
    while [ $# -gt 0 ]; do
        option=$1
        case $option in
            --version=* | --to=* | --updates=*)
                value=${option#*=}
                option=${option%%=*}
                shift
                ;;
            --version | --to | --updates)
                [ $# -ge 2 ] || stop "Missing value: \`$1\` needs one after it."
                value=$2
                shift 2
                ;;
            *)
                stop "Unknown option: \`$1\`" \
                    "The options are \`--to=<folder>\`, \`--updates=on\` or \`off\`, and \`--version=<version>\`."
                ;;
        esac
        [ -n "$value" ] || stop "Missing value: \`$option\` needs one after it."
        case $option in
            --version) [ -z "$version" ] || stop "Option given twice: \`$option\`" "Keep one of them." ;;
            --to) [ -z "$to" ] || stop "Option given twice: \`$option\`" "Keep one of them." ;;
            --updates) [ -z "$updates" ] || stop "Option given twice: \`$option\`" "Keep one of them." ;;
        esac
        case $option in
            --version) version=${value#v} ;;
            --to) to=$(expand "$value") ;;
            --updates)
                case $value in
                    on | off) updates=$value ;;
                    *) stop "Wrong kind of value: \`--updates\` is \`on\` or \`off\`, not \`$value\`" ;;
                esac
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
    trap 'exit 1' HUP INT TERM

    if [ -n "$version" ]; then
        sums=$releases/v$version/SHA256SUMS
        fetch "$sums" "$work/SHA256SUMS" ||
            stop "Can't download $sums" \
                "Is $version a release? They are listed in https://git.statac.dev/statac/statac/src/branch/main/CHANGELOG.md"
    else
        sums=$releases/latest/SHA256SUMS
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
    # Keeping up to date, and the signature, came with 0.8.0.
    case $version in
        0.[0-7].*) keeps=false ;;
        *) keeps=true ;;
    esac
    if ! $keeps && [ "$updates" = on ]; then
        stop "Statac $version can't keep itself up to date: that came in 0.8.0." \
            "Leave out \`--updates=on\`, or install a later release."
    fi

    # Fetched from v<version>/, which never changes, so a cache holding an
    # older latest/ can't pair it with a newer signature. A release is
    # signed once it is published, so a release tried before then has none.
    # SHA256SUMS.sshsig, the SSH signature, came after SHA256SUMS.sig, and
    # where this machine can't check it, SHA256SUMS.sig is tried.
    sshsig=false
    sig=false
    checked=
    if $keeps && fetch "$releases/v$version/SHA256SUMS.sshsig" "$work/SHA256SUMS.sshsig" 2>/dev/null; then
        sshsig=true
        if verifier; then
            echo "$signer" >"$work/allowed_signers"
            ssh-keygen -Y verify -f "$work/allowed_signers" -I "${signer%% *}" -n "$namespace" \
                -s "$work/SHA256SUMS.sshsig" <"$work/SHA256SUMS" >/dev/null 2>&1 ||
                stop "The release's SHA256SUMS isn't signed with Statac's key, so nothing was installed."
            checked=ssh-keygen
        fi
    fi
    if $keeps && [ -z "$checked" ] && fetch "$releases/v$version/SHA256SUMS.sig" "$work/SHA256SUMS.sig" 2>/dev/null; then
        sig=true
        openssl=$(checker)
        if [ -n "$openssl" ]; then
            der >"$work/key.der"
            "$openssl" pkeyutl -verify -rawin -pubin -keyform DER -inkey "$work/key.der" \
                -in "$work/SHA256SUMS" -sigfile "$work/SHA256SUMS.sig" >/dev/null 2>&1 ||
                stop "The release's SHA256SUMS isn't signed with Statac's key, so nothing was installed."
            checked=openssl
        fi
    fi

    echo "Installing Statac $version ($target)."

    asking=false
    if { [ -z "$to" ] || { $keeps && [ -z "$updates" ]; }; } && (: </dev/tty) 2>/dev/null; then
        asking=true
        echo "Enter takes the answer in brackets." >/dev/tty
        echo >/dev/tty
    fi
    usual=$HOME/.local/bin
    taken=
    if [ -z "$to" ] && $asking; then
        echo "Where should Statac go?" >/dev/tty
        case :$PATH: in
            *:"$usual":*) ;;
            *) echo "  \`$(tilde "$usual")\` isn't on your PATH: the end says how to add it." >/dev/tty ;;
        esac
        while [ -z "$to" ]; do
            ask "$(tilde "$usual")"
            to=$(expand "$answer")
            why=$(unusable "$to")
            if [ -n "$why" ]; then
                echo "  $why" >/dev/tty
                to=
            fi
        done
    elif [ -z "$to" ]; then
        to=$usual
        taken="goes in $(tilde "$usual")"
    fi
    why=$(unusable "$to")
    [ -z "$why" ] || stop "Can't install Statac in $to: $why"

    # Where this folder has an install already, its answer is the one in
    # brackets.
    before=$(answered "$to")
    given=$updates
    if $keeps && [ -z "$updates" ] && $asking; then
        echo "Should Statac keep itself up to date?" >/dev/tty
        echo "  If yes, \`statac serve\` checks for a fix each time it starts, installs it" >/dev/tty
        echo "  and tells you. Turn it off later with \`statac upgrade --updates=off\`." >/dev/tty
        case $before in
            on) usual=yes ;;
            *) usual=no ;;
        esac
        while [ -z "$updates" ]; do
            ask "$usual"
            case $answer in
                yes) updates=on ;;
                no) updates=off ;;
                *) echo "  \`$answer\` isn't an answer here. Write \`yes\` or \`no\`." >/dev/tty ;;
            esac
        done
    elif $keeps && [ -z "$updates" ]; then
        updates=${before:-off}
        case $updates,$before in
            on,*) said="keeps itself up to date, as it did before" ;;
            off,off) said="doesn't keep itself up to date, as before" ;;
            *) said="doesn't keep itself up to date" ;;
        esac
        taken=${taken:+$taken, and }$said
    fi
    updates=${updates:-off}
    if [ -n "$taken" ]; then
        # The answer given, or the other one to the answer taken.
        case $given,$updates in
            ,on) other=off ;;
            ,*) other=on ;;
            *) other=$given ;;
        esac
        choose="--to=<folder>"
        ! $keeps || choose="$choose --updates=$other"
        echo "No terminal to ask in, so Statac $taken."
        echo "  To choose: curl -fsSL https://get.statac.dev | sh -s -- $choose"
    fi
    if $asking; then
        echo >/dev/tty
    fi

    echo "Downloading $package"
    address=$releases/v$version/$package
    fetch "$address" "$work/$package" || stop "Can't download $address"

    found=$(sha256 "$work/$package")
    [ "$found" = "$expected" ] ||
        stop "The package doesn't match the release's SHA256SUMS, so nothing was installed." \
            "Expected $expected, and the download is $found."
    if [ "$checked" = ssh-keygen ]; then
        echo "Checked the package against the release's SHA256SUMS, and SHA256SUMS against"
        echo "  its signature with \`ssh-keygen\`."
    elif [ "$checked" = openssl ]; then
        echo "Checked the package against the release's SHA256SUMS, and SHA256SUMS against"
        echo "  its signature with OpenSSL 3."
    elif $sshsig; then
        echo "Checked the package against the release's SHA256SUMS, but not its signature:"
        echo "  that needs \`ssh-keygen\` from OpenSSH 8.1 or later, or OpenSSL 3, and this"
        echo "  machine has neither."
    elif $sig; then
        echo "Checked the package against the release's SHA256SUMS, but not its signature:"
        echo "  that needs OpenSSL 3, which this machine doesn't have."
    elif $keeps; then
        echo "Checked the package against the release's SHA256SUMS alone: its signature"
        echo "  couldn't be downloaded."
    else
        echo "Checked the package against the release's SHA256SUMS. Releases before 0.8.0 aren't signed."
    fi

    tar -xzf "$work/$package" -C "$work" ||
        stop "Can't unpack $package, so nothing was installed."
    [ -f "$work/$name/statac" ] ||
        stop "The package holds no \`$name/statac\`, so nothing was installed."

    mkdir -p "$to" || stop "Can't make the folder $to"
    to=$(cd "$to" && pwd)
    # Written beside its place and moved in, so a `statac` that is running
    # is replaced whole. The names are the ones `statac upgrade` and
    # `statac uninstall` know.
    cp "$work/$name/statac" "$to/.statac-new-$$" || stop "Can't write to $to"
    chmod 755 "$to/.statac-new-$$"
    mv -f "$to/.statac-new-$$" "$to/statac"

    notices=${XDG_DATA_HOME:-$HOME/.local/share}/statac
    mkdir -p "$notices" || stop "Can't make the folder $notices"
    notices=$(cd "$notices" && pwd)
    cp "$work/$name/LICENSE-MIT" "$work/$name/LICENSE-APACHE" \
        "$work/$name/THIRD-PARTY.md" "$work/$name/THIRD-PARTY-GRAMMARS.md" \
        "$notices/" || stop "Can't write to $notices"

    {
        echo "# Written by Statac's install script. statac upgrade and statac uninstall read it."
        echo "updates: $updates"
        echo "licences: $notices"
    } >"$to/.statac-install-$$" || stop "Can't write to $to"
    mv -f "$to/.statac-install-$$" "$to/.statac-install"

    said=$("$to/statac" --version 2>&1) ||
        stop "Installed $to/statac, and it doesn't run on this machine:" "$said"
    echo "Installed $said at $to/statac"
    echo "Its licences are in $notices"
    if $keeps; then
        case $updates in
            on)
                echo "Statac keeps itself up to date: \`statac serve\` installs each fix as it starts."
                echo "Turn this off with \`statac upgrade --updates=off\`."
                ;;
            *)
                echo "Statac doesn't keep itself up to date."
                echo "Turn this on with \`statac upgrade --updates=on\`."
                ;;
        esac
    fi

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
