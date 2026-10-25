#!/bin/sh
# Uploads a release's packages to the private storage zone they are
# published from, at the address in STAGE, with the zone's key in
# STAGING_KEY:
#
#     tools/stage.sh <file>...
#
# Each upload carries the file's SHA-256, so the zone refuses one that
# arrives damaged.
set -eu

if [ $# -eq 0 ]; then
    echo "usage: tools/stage.sh <file>..." >&2
    exit 2
fi
if [ -z "${STAGE:-}" ]; then
    echo "STAGE is empty: give the staging address to upload to" >&2
    exit 1
fi
if [ -z "${STAGING_KEY:-}" ]; then
    echo "STAGING_KEY is empty: is the STATAC_STAGING_KEY secret set?" >&2
    exit 1
fi

for file in "$@"; do
    if [ ! -f "$file" ]; then
        echo "no file at $file" >&2
        exit 1
    fi
    if command -v sha256sum >/dev/null 2>&1; then
        sum=$(sha256sum "$file" | cut -d ' ' -f 1)
    else
        sum=$(shasum -a 256 "$file" | cut -d ' ' -f 1)
    fi
    name=$(basename "$file")
    curl --fail --silent --show-error --upload-file "$file" \
        --header "AccessKey: $STAGING_KEY" \
        --header "Checksum: $(printf '%s' "$sum" | tr '[:lower:]' '[:upper:]')" \
        "$STAGE/$name" >/dev/null
    echo "staged $name"
done
