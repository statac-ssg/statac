#!/bin/sh
# Publishes a release's packages, taken and checked by tools/take.sh, to
# get.statac.dev:
#
#     tools/publish.sh <folder>
#
# A person's step, run with the Bunny login of the person releasing, which
# the release workflow never has. It publishes only packages built from the
# commit the tag v<version> names on main, and never replaces a version
# already published. The version's files go to v<version>/, then
# latest/SHA256SUMS names it as the newest release, and the install script
# as tagged goes to the top.
set -eu

zone=statac-get
served=https://statac-get.b-cdn.net

stop() {
    echo "$1" >&2
    [ $# -lt 2 ] || echo "  $2" >&2
    exit 1
}

if [ $# -ne 1 ]; then
    echo "usage: tools/publish.sh <folder>" >&2
    exit 2
fi
folder=$(cd "$1" && pwd)
root=$(cd "$(dirname "$0")/.." && pwd)

command -v bunny >/dev/null 2>&1 || stop "Can't publish: \`bunny\` isn't installed."
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

# The tag, on main, at the commit the packages were built from.
git -C "$root" fetch --quiet origin main "+refs/tags/v$version:refs/tags/v$version" ||
    stop "There's no tag v$version on origin." \
        "Tag the commit the release was built from first: git tag v$version $commit && git push origin v$version"
tagged=$(git -C "$root" rev-parse "v$version^{commit}")
[ "$tagged" = "$commit" ] ||
    stop "v$version names $tagged, but the packages were built from $commit."
git -C "$root" merge-base --is-ancestor "$commit" origin/main ||
    stop "v$version's commit, $commit, isn't on main."

# Asked of the zone, not the CDN, which might keep its answer.
present=$(bunny storage files list "v$version" -z "$zone" -o json) ||
    stop "Can't ask $zone what it holds."
case $present in
    *objectName*) stop "$version is published already." "A published version is never replaced: release a new one." ;;
esac

for file in statac-* SHA256SUMS; do
    bunny storage files upload "$file" -z "$zone" --to "v$version/" --checksum >/dev/null ||
        stop "Couldn't upload $file." "v$version/ may be part-published: finish it by hand before latest/ names it."
    echo "published v$version/$file"
done
bunny storage files upload SHA256SUMS -z "$zone" --to "latest/" --checksum >/dev/null ||
    stop "Couldn't update latest/SHA256SUMS." "v$version/ is published, but latest/ still names the release before."
echo "latest/ names $version"

# The script that installs it, which get.statac.dev serves at its top.
git -C "$root" show "v$version:tools/install.sh" >"$folder/install.sh"
bunny storage files upload "$folder/install.sh" -z "$zone" --to "/" --checksum >/dev/null ||
    stop "Couldn't upload install.sh." "v$version/ is published and latest/ names it."
echo "published install.sh"

curl -fsS "$served/v$version/SHA256SUMS" | cmp -s - SHA256SUMS ||
    stop "What $served serves for v$version/SHA256SUMS isn't what was published."
echo "Published $version, built from $commit."
