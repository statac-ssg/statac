#!/bin/sh
# Publishes a release's packages, taken and checked by tools/take.sh, to
# get.statac.dev:
#
#     tools/publish.sh [--dry-run] <folder>
#
# A person's step, run with the Bunny login of the person releasing, which
# the release workflow never has. It publishes only packages built from the
# commit the tag v<version> names on main, and never replaces a version
# already published. It signs SHA256SUMS with the release key, whose public
# half the tagged Statac carries in src/upgrade.rs, and which it reads from
# ~/.config/statac-release/signing-key.pem or the file STATAC_SIGNING_KEY
# names. It signs it a second way too, as an SSH signature made with the
# same key, with the signer and namespace tools/install.sh holds. The
# version's files go to v<version>/, the signatures beside SHA256SUMS as
# SHA256SUMS.sig and SHA256SUMS.sshsig, then latest-<major>.<minor>/SHA256SUMS names
# it as the newest of its series and, unless a later release is out,
# latest/SHA256SUMS as the newest of all, and the install script as tagged
# goes to the top, as index.html, with the one for Windows beside it as
# install.ps1.
#
# --dry-run makes every check and says each upload it would make, uploading
# nothing, so a practice release can reach this step. It accepts a version
# with no tag, the packages' commit still having to be on main.
set -eu

zone=statac-get
served=https://statac-get.b-cdn.net
key=${STATAC_SIGNING_KEY:-$HOME/.config/statac-release/signing-key.pem}

stop() {
    echo "$1" >&2
    [ $# -lt 2 ] || echo "  $2" >&2
    exit 1
}

dry=false
if [ $# -eq 2 ] && [ "$1" = --dry-run ]; then
    dry=true
    shift
fi
if [ $# -ne 1 ]; then
    echo "usage: tools/publish.sh [--dry-run] <folder>" >&2
    exit 2
fi
folder=$(cd "$1" && pwd)
root=$(cd "$(dirname "$0")/.." && pwd)

command -v bunny >/dev/null 2>&1 || stop "Can't publish: \`bunny\` isn't installed."
command -v ssh-keygen >/dev/null 2>&1 || stop "Can't sign the release: \`ssh-keygen\` isn't installed."
[ -f "$key" ] || stop "Can't sign the release: there's no key at $key." \
    "Set STATAC_SIGNING_KEY to the release key's file."
[ -f "$folder/SHA256SUMS" ] && [ -f "$folder/COMMIT" ] ||
    stop "$folder holds no taken release." "Take one first with tools/take.sh."

cd "$folder"
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum --check --quiet SHA256SUMS
else
    shasum -a 256 --check --quiet SHA256SUMS
fi || stop "A package in $folder no longer matches SHA256SUMS."
commit=$(cat COMMIT)
version=$(sed -n '1s/.*statac-\([0-9][^-]*\)-.*/\1/p' SHA256SUMS)
[ -n "$version" ] || stop "Can't read the version from $folder/SHA256SUMS."
series=${version%.*}

# The tag, on main, at the commit the packages were built from. A dry run
# without one reads the install script from the commit instead.
source=v$version
if git -C "$root" fetch --quiet origin main "+refs/tags/v$version:refs/tags/v$version" 2>/dev/null; then
    tagged=$(git -C "$root" rev-parse "v$version^{commit}")
    [ "$tagged" = "$commit" ] ||
        stop "v$version names $tagged, but the packages were built from $commit."
elif $dry; then
    git -C "$root" fetch --quiet origin main || stop "Can't fetch main from origin."
    git -C "$root" cat-file -e "$commit^{commit}" 2>/dev/null ||
        stop "The packages' commit, $commit, isn't in this checkout."
    echo "no tag v$version: a real run stops here until $commit is tagged"
    source=$commit
else
    stop "There's no tag v$version on origin." \
        "Tag the commit the release was built from first: git tag v$version $commit && git push origin v$version"
fi
git -C "$root" merge-base --is-ancestor "$commit" origin/main ||
    stop "v$version's commit, $commit, isn't on main."

# Signed with the key whose public half this release carries, and checked,
# so no release goes out that its own upgrade would refuse. The signature
# is Ed25519's 64 bytes as they are, which needs OpenSSL 3, not LibreSSL.
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
trap 'exit 1' HUP INT TERM
public=$work/public.pem
openssl pkey -in "$key" -pubout -out "$public" 2>/dev/null ||
    stop "Can't read the key at $key." "It needs OpenSSL 3: \`openssl version\` says $(openssl version)."
carried=$(openssl pkey -pubin -in "$public" -outform DER | tail -c 32 | od -An -tx1 | tr -d ' \n')
git -C "$root" show "$source:src/upgrade.rs" | grep -qF "hex(\"$carried\")" ||
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
script=$(git -C "$root" show "$source:tools/install.sh")
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

# latest/ moves only to a newer release, so a fix to an older series names
# only its own series.
newer() {
    old=$1
    new=$2
    while [ -n "$old$new" ]; do
        a=${old%%.*}
        b=${new%%.*}
        [ "${a:-0}" -eq "${b:-0}" ] || { [ "${b:-0}" -gt "${a:-0}" ]; return; }
        case $old in *.*) old=${old#*.} ;; *) old= ;; esac
        case $new in *.*) new=${new#*.} ;; *) new= ;; esac
    done
    return 1
}
latest=$(curl -fsS "$served/latest/SHA256SUMS" 2>/dev/null | sed -n '1s/.*statac-\([0-9][^-]*\)-.*/\1/p') ||
    latest=
moves=true
if [ -n "$latest" ] && ! newer "$latest" "$version"; then
    moves=false
fi

# Asked of the zone, not the CDN, which might keep its answer.
present=$(bunny storage files list "v$version" -z "$zone" -o json) ||
    stop "Can't ask $zone what it holds."
case $present in
    *objectName*) stop "$version is published already." "A published version is never replaced: release a new one." ;;
esac

if $dry; then
    for file in statac-* SHA256SUMS.sig SHA256SUMS.sshsig SHA256SUMS; do
        echo "would publish v$version/$file"
    done
    echo "would make latest-$series/ name $version"
    if $moves; then
        echo "would make latest/ name $version"
    else
        echo "would leave latest/ naming $latest"
    fi
    git -C "$root" show "$source:tools/install.sh" >"$folder/install.sh"
    echo "would publish install.sh from $source at the top"
    git -C "$root" show "$source:tools/install.ps1" >"$folder/install.ps1"
    echo "would publish install.ps1 from $source at the top"
    echo "Checked $version, built from $commit: nothing was uploaded."
    exit 0
fi

# The signatures before SHA256SUMS, so a release whose SHA256SUMS can be
# fetched always has them.
for file in statac-* SHA256SUMS.sig SHA256SUMS.sshsig SHA256SUMS; do
    bunny storage files upload "$file" -z "$zone" --to "v$version/" --checksum >/dev/null ||
        stop "Couldn't upload $file." "v$version/ may be part-published: finish it by hand before latest/ names it."
    echo "published v$version/$file"
done
bunny storage files upload SHA256SUMS -z "$zone" --to "latest-$series/" --checksum >/dev/null ||
    stop "Couldn't update latest-$series/SHA256SUMS." "v$version/ is published, but latest-$series/ still names the release before."
echo "latest-$series/ names $version"
if $moves; then
    bunny storage files upload SHA256SUMS -z "$zone" --to "latest/" --checksum >/dev/null ||
        stop "Couldn't update latest/SHA256SUMS." "v$version/ is published, but latest/ still names the release before."
    echo "latest/ names $version"
else
    echo "latest/ still names $latest, which is newer"
fi

# The script that installs it, which get.statac.dev serves at its top:
# a request for / is answered from index.html, as text so a browser
# shows it.
git -C "$root" show "v$version:tools/install.sh" >"$folder/install.sh"
bunny storage files upload "$folder/install.sh" -z "$zone" --to index.html \
    --content-type 'text/plain; charset=utf-8' --checksum >/dev/null ||
    stop "Couldn't upload install.sh." "v$version/ is published and latest/ names it."
echo "published install.sh"
git -C "$root" show "v$version:tools/install.ps1" >"$folder/install.ps1"
bunny storage files upload "$folder/install.ps1" -z "$zone" --to install.ps1 \
    --content-type 'text/plain; charset=utf-8' --checksum >/dev/null ||
    stop "Couldn't upload install.ps1." "v$version/ is published, latest/ names it and install.sh is at the top."
echo "published install.ps1"

curl -fsS "$served/v$version/SHA256SUMS" | cmp -s - SHA256SUMS ||
    stop "What $served serves for v$version/SHA256SUMS isn't what was published."
# The edge keeps / and /install.ps1 for 60 seconds.
tries=0
until curl -fsS "$served/" | cmp -s - "$folder/install.sh"; do
    tries=$((tries + 1))
    [ "$tries" -lt 15 ] ||
        stop "What $served serves at / isn't the install script as tagged."
    sleep 5
done
tries=0
until curl -fsS "$served/install.ps1" | cmp -s - "$folder/install.ps1"; do
    tries=$((tries + 1))
    [ "$tries" -lt 15 ] ||
        stop "What $served serves at /install.ps1 isn't the install script for Windows as tagged."
    sleep 5
done
echo "Published $version, built from $commit."
