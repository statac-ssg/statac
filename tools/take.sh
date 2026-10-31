#!/bin/sh
# Takes the packages the release workflow staged for one commit into a
# folder, and checks them as a set before anyone publishes them:
#
#     tools/take.sh <version> <commit> <folder>
#
# Run with the Bunny login of the person releasing. It wants every
# platform's package, checks each against its SHA-256, writes SHA256SUMS
# and signs it, and installs this machine's package with the install
# script, as a reader would. The folder then holds what tools/publish.sh
# publishes.
#
# It signs SHA256SUMS with the release key, whose public half the commit's
# Statac carries in src/upgrade.rs, and which it reads from
# ~/.config/statac-release/signing-key.pem or the file STATAC_SIGNING_KEY
# names, as SHA256SUMS.sig. It signs it a second way too, as an SSH
# signature made with the same key, with the signer and namespace the
# commit's tools/install.sh holds, as SHA256SUMS.sshsig.
set -eu

zone=statac-staging
key=${STATAC_SIGNING_KEY:-$HOME/.config/statac-release/signing-key.pem}
targets="aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu
aarch64-unknown-linux-musl x86_64-unknown-linux-gnu x86_64-unknown-linux-musl
x86_64-pc-windows-msvc"

stop() {
    echo "$1" >&2
    [ $# -lt 2 ] || echo "  $2" >&2
    exit 1
}

if [ $# -ne 3 ]; then
    echo "usage: tools/take.sh <version> <commit> <folder>" >&2
    exit 2
fi
version=$1
commit=$2
folder=$3
root=$(cd "$(dirname "$0")/.." && pwd)

command -v bunny >/dev/null 2>&1 ||
    stop "Can't reach the staged packages: \`bunny\` isn't installed."
command -v ssh-keygen >/dev/null 2>&1 || stop "Can't sign the release: \`ssh-keygen\` isn't installed."
[ -f "$key" ] || stop "Can't sign the release: there's no key at $key." \
    "Set STATAC_SIGNING_KEY to the release key's file."
key=$(cd "$(dirname "$key")" && pwd)/$(basename "$key")
case $commit in
    *[!0-9a-f]* | "") stop "\`$commit\` isn't a commit: give the whole hash the release run built." ;;
esac
[ ${#commit} -eq 40 ] ||
    stop "\`$commit\` isn't a whole commit hash." "The staged folder is named by all 40 characters."
git -C "$root" cat-file -e "$commit^{commit}" 2>/dev/null ||
    git -C "$root" fetch --quiet origin main || stop "Can't fetch main from origin."
git -C "$root" cat-file -e "$commit^{commit}" 2>/dev/null ||
    stop "The packages' commit, $commit, isn't in this checkout."
if [ -e "$folder" ] && [ -n "$(ls -A "$folder")" ]; then
    stop "$folder isn't empty." "Take the packages into a new folder."
fi
mkdir -p "$folder"

staged=v$version/$commit
for target in $targets; do
    case $target in
        *-windows-*) package=statac-$version-$target.zip ;;
        *) package=statac-$version-$target.tar.gz ;;
    esac
    for file in "$package" "$package.sha256"; do
        bunny storage files download "$staged/$file" -z "$zone" --out "$folder/$file" >/dev/null ||
            stop "Nothing staged at $staged/$file" \
                "Did the release run for $version at this commit finish for $target?"
    done
done

cd "$folder"
cat ./*.sha256 >SHA256SUMS
rm ./*.sha256
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum --check --quiet SHA256SUMS
else
    shasum -a 256 --check --quiet SHA256SUMS
fi ||
    stop "A package doesn't match its SHA-256." "Run the release again; publish nothing from this folder."
echo "$commit" >COMMIT

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
trap 'exit 1' HUP INT TERM

# Signed with the key whose public half this release carries, and checked,
# so no release goes out that its own upgrade would refuse. The signature
# is Ed25519's 64 bytes as they are, which needs OpenSSL 3, not LibreSSL.
public=$work/public.pem
openssl pkey -in "$key" -pubout -out "$public" 2>/dev/null ||
    stop "Can't read the key at $key." "It needs OpenSSL 3: \`openssl version\` says $(openssl version)."
carried=$(openssl pkey -pubin -in "$public" -outform DER | tail -c 32 | od -An -tx1 | tr -d ' \n')
git -C "$root" show "$commit:src/upgrade.rs" | grep -qF "hex(\"$carried\")" ||
    stop "The key at $key isn't the one $version carries in src/upgrade.rs."
openssl pkeyutl -sign -rawin -inkey "$key" -in SHA256SUMS -out SHA256SUMS.sig ||
    stop "Can't sign $folder/SHA256SUMS."
openssl pkeyutl -verify -rawin -pubin -inkey "$public" -in SHA256SUMS -sigfile SHA256SUMS.sig >/dev/null ||
    stop "The signature on $folder/SHA256SUMS doesn't check."

# The SSH signature, which ssh-keygen makes only with a key in OpenSSH's own
# format and can't read from the PEM file. Unencrypted, that format is
# fixed fields round the public key and the private key's 32 bytes, laid
# out as OpenSSH's PROTOCOL.key gives, so it is written here for this run
# alone, and checked as the release's install script will check it.
script=$(git -C "$root" show "$commit:tools/install.sh")
signer=$(echo "$script" | sed -n "s/^signer='\(.*\)'\$/\1/p")
namespace=$(echo "$script" | sed -n 's/^namespace=//p')
[ -n "$signer" ] && [ -n "$namespace" ] ||
    stop "Can't read the SSH signer and namespace from tools/install.sh in $version."
openssl pkey -pubin -in "$public" -outform DER | tail -c 32 >"$work/public.raw"
ed25519() {
    printf '\000\000\000\013ssh-ed25519\000\000\000\040'
}
(
    umask 077
    {
        echo "-----BEGIN OPENSSH PRIVATE KEY-----"
        {
            printf 'openssh-key-v1\000\000\000\000\004none\000\000\000\004none\000\000\000\000\000\000\000\001'
            printf '\000\000\000\063'
            ed25519
            cat "$work/public.raw"
            # Two equal check numbers, the key, an empty comment, and
            # padding to a whole number of 8 bytes.
            printf '\000\000\000\210\000\000\000\000\000\000\000\000'
            ed25519
            cat "$work/public.raw"
            printf '\000\000\000\100'
            openssl pkey -in "$key" -outform DER | tail -c 32
            cat "$work/public.raw"
            printf '\000\000\000\000\001\002\003\004\005'
        } | openssl base64
        echo "-----END OPENSSH PRIVATE KEY-----"
    } >"$work/key"
) || stop "Can't write the key in OpenSSH's format."
[ "$(ssh-keygen -y -f "$work/key" | cut -d ' ' -f 1,2)" = "${signer#* }" ] ||
    stop "The key at $key isn't the one $version carries in tools/install.sh."
ssh-keygen -Y sign -q -f "$work/key" -n "$namespace" <SHA256SUMS >SHA256SUMS.sshsig ||
    stop "Can't make the SSH signature of $folder/SHA256SUMS."
echo "$signer" >"$work/allowed_signers"
ssh-keygen -Y verify -f "$work/allowed_signers" -I "${signer%% *}" -n "$namespace" \
    -s SHA256SUMS.sshsig <SHA256SUMS >/dev/null ||
    stop "The SSH signature on $folder/SHA256SUMS doesn't check."

# The install script, given these packages laid out as a release's.
tried=$work/tried
mkdir -p "$tried/releases/latest" "$tried/releases/v$version"
cp SHA256SUMS "$tried/releases/latest/"
cp SHA256SUMS SHA256SUMS.sig SHA256SUMS.sshsig statac-* "$tried/releases/v$version/"
STATAC_RELEASES="file://$tried/releases" XDG_DATA_HOME="$tried/share" \
    sh "$root/tools/install.sh" --version="$version" --to="$tried/installed" --updates=off >/dev/null
said=$("$tried/installed/statac" --version)
[ "$said" = "Statac $version" ] ||
    stop "The installed package says \`$said\`, not \`Statac $version\`."

echo "Took $version, built from $commit, into $folder:"
cat SHA256SUMS
