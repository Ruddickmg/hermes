#!/bin/sh
set -e

VERSION=$(cat target/release/version.txt)
if [ -z "$VERSION" ]; then
  echo "No version to publish"
  exit 0
fi

echo "Committing nix/sources.json for v${VERSION}"

cp target/release/sources.json nix/sources.json

# Commit with [skip ci]
git config user.email "ci@hermes.nvim"
git config user.name "Hermes CI"
git remote set-url origin "https://x-access-token:${GITHUB_WRITE_TOKEN}@github.com/Ruddickmg/hermes.nvim.git"
git add nix/sources.json

if git diff --cached --quiet; then
  echo "No hash changes to commit"
  exit 0
fi

git commit -m "nix: update hashes for v${VERSION} [skip ci]"
git push origin "HEAD:${CIRCLE_BRANCH}"
