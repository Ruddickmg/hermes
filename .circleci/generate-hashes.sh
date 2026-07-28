#!/bin/sh
set -e

VERSION=$(cat target/release/version.txt)
if [ -z "$VERSION" ]; then
  echo "No version to publish"
  exit 0
fi

echo "Generating hashes for v${VERSION}"

# Binary names keyed by nix system name
BINARIES="x86_64-linux:libhermes-linux-x86_64.so
aarch64-linux:libhermes-linux-aarch64.so
x86_64-darwin:libhermes-macos-x86_64.dylib
aarch64-darwin:libhermes-macos-aarch64.dylib
x86_64-windows:libhermes-windows-x86_64.dll"

# Temporary file for sources.json
SOURCES_TMP=$(mktemp)
trap 'rm -f "$SOURCES_TMP"' EXIT

# Start sources.json
echo '{"version": "'"$VERSION"'"}' > "$SOURCES_TMP"

# Start checksums.txt
CHECKSUMS_FILE="target/release/checksums.txt"
> "$CHECKSUMS_FILE"

echo "Generating checksums.txt and sources.json..."
for entry in $BINARIES; do
  system="${entry%%:*}"
  binary="${entry##*:}"

  HEX=$(sha256sum "target/release/$binary" | awk '{print $1}')
  SRI=$(nix --extra-experimental-features 'nix-command' hash convert --hash-algo sha256 "$HEX")

  echo "${HEX}  ${binary}" >> "$CHECKSUMS_FILE"
  echo "  $system: $SRI"

  # Add to sources.json using jq
  SOURCES_TMP_NEW=$(mktemp)
  jq --arg sys "$system" --arg hash "$SRI" \
    '. + {($sys): {"hash": $hash}}' "$SOURCES_TMP" > "$SOURCES_TMP_NEW"
  mv "$SOURCES_TMP_NEW" "$SOURCES_TMP"
done

echo ""
echo "checksums.txt:"
cat "$CHECKSUMS_FILE"
echo ""
echo "sources.json:"
cat "$SOURCES_TMP"

cp "$SOURCES_TMP" target/release/sources.json
