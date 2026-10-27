#!/bin/sh
# Publishes a release's packages, taken and checked by tools/take.sh, to
# get.statac.dev:
#
#     tools/publish.sh [--dry-run] <folder>
#
# A person's step, run with the Bunny login of the person releasing, which
# the release workflow never has. It publishes only packages built from the
# commit the tag v<version> names on main, and never replaces a version
# already published. The version's files go to v<version>/, then
# latest/SHA256SUMS names it as the newest release, and the install script
# as tagged goes to the top, as index.html.
#
# --dry-run makes every check and says each upload it would make, uploading
# nothing, so a practice release can reach this step. It accepts a version
# with no tag, the packages' commit still having to be on main.
set -eu

zone=statac-get
served=https://statac-get.b-cdn.net

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

# Asked of the zone, not the CDN, which might keep its answer.
present=$(bunny storage files list "v$version" -z "$zone" -o json) ||
    stop "Can't ask $zone what it holds."
case $present in
    *objectName*) stop "$version is published already." "A published version is never replaced: release a new one." ;;
esac

if $dry; then
    for file in statac-* SHA256SUMS; do
        echo "would publish v$version/$file"
    done
    echo "would make latest/ name $version"
    git -C "$root" show "$source:tools/install.sh" >"$folder/install.sh"
    echo "would publish install.sh from $source at the top"
    echo "Checked $version, built from $commit: nothing was uploaded."
    exit 0
fi

for file in statac-* SHA256SUMS; do
    bunny storage files upload "$file" -z "$zone" --to "v$version/" --checksum >/dev/null ||
        stop "Couldn't upload $file." "v$version/ may be part-published: finish it by hand before latest/ names it."
    echo "published v$version/$file"
done
bunny storage files upload SHA256SUMS -z "$zone" --to "latest/" --checksum >/dev/null ||
    stop "Couldn't update latest/SHA256SUMS." "v$version/ is published, but latest/ still names the release before."
echo "latest/ names $version"

# The script that installs it, which get.statac.dev serves at its top:
# a request for / is answered from index.html, as text so a browser
# shows it.
git -C "$root" show "v$version:tools/install.sh" >"$folder/install.sh"
bunny storage files upload "$folder/install.sh" -z "$zone" --to index.html \
    --content-type 'text/plain; charset=utf-8' --checksum >/dev/null ||
    stop "Couldn't upload install.sh." "v$version/ is published and latest/ names it."
echo "published install.sh"

curl -fsS "$served/v$version/SHA256SUMS" | cmp -s - SHA256SUMS ||
    stop "What $served serves for v$version/SHA256SUMS isn't what was published."
# The edge keeps / for 60 seconds.
tries=0
until curl -fsS "$served/" | cmp -s - "$folder/install.sh"; do
    tries=$((tries + 1))
    [ "$tries" -lt 15 ] ||
        stop "What $served serves at / isn't the install script as tagged."
    sleep 5
done
echo "Published $version, built from $commit."
