#!/bin/sh
set -e

VERSION=$(cat target/release/version.txt)
if [ -z "$VERSION" ]; then
  echo "No version to publish"
  exit 0
fi

echo "Updating hashes for v${VERSION}"

REPO="Ruddickmg/hermes.nvim"
BASE_URL="https://github.com/${REPO}/releases/download/v${VERSION}"

# Binary names keyed by nix system name
BINARIES="x86_64-linux:libhermes-linux-x86_64.so
aarch64-linux:libhermes-linux-aarch64.so
x86_64-darwin:libhermes-macos-x86_64.dylib
aarch64-darwin:libhermes-macos-aarch64.dylib
x86_64-windows:libhermes-windows-x86_64.dll"

# Temporary files
CHECKSUMS_TMP=$(mktemp)
SOURCES_TMP=$(mktemp)
trap 'rm -f "$CHECKSUMS_TMP" "$SOURCES_TMP"' EXIT

# Start sources.json
echo '{"version": "'"$VERSION"'"}' > "$SOURCES_TMP"

echo "Generating checksums.txt and sources.json..."
for entry in $BINARIES; do
  system="${entry%%:*}"
  binary="${entry##*:}"

  echo "  Downloading $binary..."
  curl -sL -o "/tmp/$binary" "${BASE_URL}/${binary}"

  HEX=$(sha256sum "/tmp/$binary" | awk '{print $1}')
  SRI=$(nix hash to-sri --type sha256 "$HEX")

  echo "${HEX}  ${binary}" >> "$CHECKSUMS_TMP"
  echo "  $system: $SRI"

  # Add to sources.json using jq
  SOURCES_TMP_NEW=$(mktemp)
  jq --arg sys "$system" --arg hash "$SRI" \
    '. + {($sys): {"hash": $hash}}' "$SOURCES_TMP" > "$SOURCES_TMP_NEW"
  mv "$SOURCES_TMP_NEW" "$SOURCES_TMP"
done

echo ""
echo "checksums.txt:"
cat "$CHECKSUMS_TMP"
echo ""
echo "sources.json:"
cat "$SOURCES_TMP"

# Upload checksums.txt as release asset
RELEASE_ID=$(curl -sS \
  -H "Authorization: token ${GITHUB_TOKEN}" \
  -H "Accept: application/vnd.github+json" \
  "https://api.github.com/repos/${REPO}/releases/tags/v${VERSION}" \
  | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>process.stdout.write(String(JSON.parse(s).id||"")))')

if [ -n "$RELEASE_ID" ]; then
  echo ""
  echo "Uploading checksums.txt to release..."
  curl -sS -X POST \
    -H "Authorization: token ${GITHUB_TOKEN}" \
    -H "Content-Type: text/plain" \
    "https://uploads.github.com/repos/${REPO}/releases/${RELEASE_ID}/assets?name=checksums.txt" \
    --data-binary @"$CHECKSUMS_TMP"
else
  echo "WARNING: Could not resolve release id, skipping checksums.txt upload"
fi

# Update nix/sources.json in repo
cp "$SOURCES_TMP" nix/sources.json

# Commit with [skip ci]
git config user.email "ci@hermes.nvim"
git config user.name "Hermes CI"
git add nix/sources.json
git commit -m "nix: update hashes for v${VERSION} [skip ci]"
git push origin "HEAD:${CIRCLE_BRANCH}"
