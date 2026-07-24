#!/bin/sh
set -e

VERSION=$(cat target/release/version.txt)
if [ -z "$VERSION" ]; then
  echo "No version to publish"
  exit 0
fi

echo "Generating sources.json for v${VERSION}"

echo "{" > target/release/sources.json
echo "  \"version\": \"$VERSION\"," >> target/release/sources.json

for binary in target/release/libhermes-*; do
  [ -f "$binary" ] || continue
  BINARY_NAME=$(basename "$binary")
  PLATFORM=$(echo "$BINARY_NAME" | sed 's/^libhermes-//; s/\.\(so\|dylib\|dll\)$//')

  case "$PLATFORM" in
    linux-x86_64)   KEY="x86_64-linux" ;;
    linux-aarch64)  KEY="aarch64-linux" ;;
    macos-x86_64)   KEY="x86_64-darwin" ;;
    macos-aarch64)  KEY="aarch64-darwin" ;;
    *) echo "Skipping unknown platform: $PLATFORM"; continue ;;
  esac

  HASH_B16=$(sha256sum "$binary" | awk '{print $1}')
  HASH_B64=$(printf "$(echo "$HASH_B16" | sed 's/\([0-9a-f][0-9a-f]\)/\\x\1/g')" | base64 -w0)
  echo "  \"${KEY}\": { \"hash\": \"sha256-${HASH_B64}\" }," >> target/release/sources.json
done

echo "}" >> target/release/sources.json

echo "Generated sources.json:"
cat target/release/sources.json
