#!/bin/sh
# Takes the packages the release workflow staged for one commit into a
# folder, and checks them as a set before anyone publishes them:
#
#     tools/take.sh <version> <commit> <folder>
#
# Run with the Bunny login of the person releasing. It wants every
# platform's package, checks each against its SHA-256, writes SHA256SUMS,
# and installs this machine's package with the install script, as a reader
# would. The folder then holds what tools/publish.sh publishes.
set -eu

zone=statac-staging
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
case $commit in
    *[!0-9a-f]* | "") stop "\`$commit\` isn't a commit: give the whole hash the release run built." ;;
esac
[ ${#commit} -eq 40 ] ||
    stop "\`$commit\` isn't a whole commit hash." "The staged folder is named by all 40 characters."
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

# The install script, given these packages laid out as a release's.
tried=$(mktemp -d)
trap 'rm -rf "$tried"' EXIT
mkdir -p "$tried/releases/latest" "$tried/releases/v$version"
cp SHA256SUMS "$tried/releases/latest/"
cp SHA256SUMS statac-* "$tried/releases/v$version/"
STATAC_RELEASES="file://$tried/releases" XDG_DATA_HOME="$tried/share" \
    sh "$root/tools/install.sh" --to="$tried/installed" --updates=off >/dev/null
said=$("$tried/installed/statac" --version)
[ "$said" = "Statac $version" ] ||
    stop "The installed package says \`$said\`, not \`Statac $version\`."

echo "Took $version, built from $commit, into $folder:"
cat SHA256SUMS
