#!/bin/sh
set -e

VERSION=$(cat target/release/version.txt)
if [ -z "$VERSION" ]; then
  echo "No version to publish"
  exit 0
fi

echo "Publishing nix package for v${VERSION}"

# Update hashes
nix run nixpkgs#nix-update -- hermes-nvim --flake --version v"${VERSION}"

# Commit with [skip ci]
git add nix/packaging/default.nix
git commit -m "nix: update hermes-nvim to v${VERSION} [skip ci]"
git push origin HEAD
