#!/bin/sh
set -e

VERSION=$(cat target/release/version.txt)
if [ -z "$VERSION" ]; then
  echo "No version to publish"
  exit 0
fi

# --- Install LuaRocks and dependencies ---
apt-get update
apt-get install -y lua5.1 liblua5.1-dev wget zip unzip jq
wget -q https://luarocks.org/releases/luarocks-3.9.2.tar.gz
tar zxf luarocks-3.9.2.tar.gz
cd luarocks-3.9.2 && ./configure && make && make install
cd ..
# dkjson is required by `luarocks upload` to parse the server response
luarocks install dkjson

# --- Publish source rockspec ---
# The CLI `luarocks upload` packs and uploads a `.src.rock` automatically
# alongside the rockspec. luarocks.org accepts these via its standard
# upload endpoint.
SOURCE_URL="https://github.com/Ruddickmg/hermes.nvim/archive/refs/tags/v${VERSION}.tar.gz"
SOURCE_TARBALL=$(mktemp)
curl -fsSL "$SOURCE_URL" -o "$SOURCE_TARBALL"
SOURCE_MD5=$(md5sum "$SOURCE_TARBALL" | awk '{print $1}')
rm -f "$SOURCE_TARBALL"
SED_EXPR="s/scm-1/${VERSION}-1/g; /branch = \"main\"/d; s|git+https://github.com/Ruddickmg/hermes.nvim.git|${SOURCE_URL}|"
SOURCE_ROCKSPEC="/tmp/hermes.nvim-${VERSION}-1.rockspec"
sed "$SED_EXPR" hermes.nvim-scm-1.rockspec > "$SOURCE_ROCKSPEC"
# The GitHub-generated archive extracts into "hermes.nvim-${VERSION}" (no "v"),
# so we must tell LuaRocks the expected source directory.
sed -i "/url = \"https:.*archive.*tar\\.gz\"/a\\
  md5 = \"${SOURCE_MD5}\",\\
  dir = \"hermes.nvim-${VERSION}\"," "$SOURCE_ROCKSPEC"
luarocks upload "$SOURCE_ROCKSPEC" --api-key="${LUAROCKS_API_KEY}"
echo "Source rock published: hermes.nvim-${VERSION}-1"

# --- Look up the version_id needed for binary rock uploads ---
# luarocks.org's REST API requires the numeric version id; we fetch it via
# the check_rockspec endpoint after the source upload registered the version.
CHECK_RESPONSE=$(curl -fsSL "https://luarocks.org/api/1/${LUAROCKS_API_KEY}/check_rockspec?package=hermes.nvim&version=${VERSION}-1")
VERSION_ID=$(echo "$CHECK_RESPONSE" | jq -r '.version.id // empty')
if [ -z "$VERSION_ID" ]; then
  echo "Failed to obtain version_id from API"
  echo "Response: $CHECK_RESPONSE"
  exit 1
fi
echo "Version ID: $VERSION_ID"

# --- Publish one binary rock per platform ---
# We construct a minimal per-platform rockspec that declares the prebuilt
# binary via `build.install.lib`, run `luarocks make --pack-binary-rock`
# to produce a properly formatted `.rock` (ZIP with rock_manifest), then
# rename the file to the target platform and POST it to the LuaRocks API.
# The platform string in the filename is the only thing the server uses
# to record the rock's arch; the ZIP contents are platform-agnostic from
# LuaRocks' perspective.
for binary in target/release/libhermes-*.so target/release/libhermes-*.dylib target/release/libhermes-*.dll; do
  [ -f "$binary" ] || continue

  BINARY_NAME=$(basename "$binary")
  OUR_PLATFORM=$(echo "$BINARY_NAME" | sed 's/^libhermes-//; s/\.\(so\|dylib\|dll\)$//')

  case "$OUR_PLATFORM" in
    linux-x86_64)   LR_PLATFORM="linux-x86_64" ;;
    linux-aarch64)  LR_PLATFORM="linux-aarch64" ;;
    macos-aarch64)  LR_PLATFORM="macosx-arm64" ;;
    macos-x86_64)   LR_PLATFORM="macosx-x86_64" ;;
    windows-x86_64) LR_PLATFORM="win32-x86_64" ;;
    *)
      echo "Skipping unknown platform: $OUR_PLATFORM"
      continue
      ;;
  esac

  ROCK_DIR=$(mktemp -d)
  mkdir -p "$ROCK_DIR/lib"
  cp "$binary" "$ROCK_DIR/lib/$BINARY_NAME"

  # Generate per-platform binary rockspec. `build.install.lib` is what
  # tells `luarocks make` to include the binary in the rock's lib/ dir.
  # The key ("hermes_native") is the install module name and is
  # irrelevant: hermes.binary loads the file directly via package.loadlib
  # using the filename, which is preserved verbatim.
  cat > "$ROCK_DIR/hermes.nvim-${VERSION}-1.rockspec" <<ROCKSPEC
rockspec_format = "3.0"
package = "hermes.nvim"
version = "${VERSION}-1"
source = {
  url = "${SOURCE_URL}",
  md5 = "${SOURCE_MD5}",
  dir = "hermes.nvim-${VERSION}"
}
description = {
  summary = "ACP (Agent Client Protocol) client for Neovim",
  detailed = [[
    Hermes is an interface between Neovim and ACP (Agent Client Protocol),
    enabling AI assistant integration directly within Neovim.
  ]],
  homepage = "https://github.com/Ruddickmg/hermes.nvim",
  license = "MIT",
  maintainer = "Ruddickmg"
}
dependencies = {
  "lua >= 5.1"
}
build = {
  type = "builtin",
  modules = {
    ["hermes"] = "lua/hermes/init.lua",
    ["hermes.binary"] = "lua/hermes/binary.lua",
    ["hermes.config"] = "lua/hermes/config.lua",
    ["hermes.download"] = "lua/hermes/download.lua",
    ["hermes.health"] = "lua/hermes/health.lua",
    ["hermes.logging"] = "lua/hermes/logging.lua",
    ["hermes.platform"] = "lua/hermes/platform.lua",
    ["hermes.queue"] = "lua/hermes/queue.lua",
    ["hermes.version"] = "lua/hermes/version.lua"
  },
  install = {
    lib = {
      ["hermes_native"] = "lib/${BINARY_NAME}"
    }
  },
  copy_directories = {
    "plugin",
    "doc"
  }
}
ROCKSPEC

  # Stage the rest of the source tree so `luarocks make` can find it
  mkdir -p "$ROCK_DIR/lua/hermes" "$ROCK_DIR/plugin" "$ROCK_DIR/doc"
  cp lua/hermes/*.lua "$ROCK_DIR/lua/hermes/"
  cp plugin/hermes.lua "$ROCK_DIR/plugin/"
  cp doc/hermes.txt "$ROCK_DIR/doc/"

  # Build the rock. The runner is Linux x86_64 so the output filename
  # is always `*.linux-x86_64.rock` regardless of the target platform.
  (
    cd "$ROCK_DIR"
    luarocks make --pack-binary-rock "hermes.nvim-${VERSION}-1.rockspec"
  )

  # The build runner is Linux x86_64; skip the rename when the target
  # platform is the same, otherwise `mv` errors on identical paths.
  ROCK_FILE="$ROCK_DIR/hermes.nvim-${VERSION}-1.${LR_PLATFORM}.rock"
  if [ "$LR_PLATFORM" != "linux-x86_64" ]; then
    mv "$ROCK_DIR/hermes.nvim-${VERSION}-1.linux-x86_64.rock" "$ROCK_FILE"
  fi

  # Upload the binary rock via the LuaRocks REST API and report any
  # non-2xx response body for diagnostics.
  UPLOAD_BODY=$(mktemp)
  HTTP_CODE=$(curl -sS -o "$UPLOAD_BODY" -w "%{http_code}" -X POST \
    "https://luarocks.org/api/1/${LUAROCKS_API_KEY}/upload_rock/${VERSION_ID}" \
    -F "rock_file=@${ROCK_FILE}")
  if [ "$HTTP_CODE" != "200" ]; then
    echo "Binary rock upload failed for ${LR_PLATFORM} (HTTP ${HTTP_CODE}):"
    cat "$UPLOAD_BODY"
    rm -f "$UPLOAD_BODY"
    rm -rf "$ROCK_DIR"
    exit 1
  fi
  rm -f "$UPLOAD_BODY"

  echo "Binary rock published: hermes.nvim-${VERSION}-1.${LR_PLATFORM}.rock"
  rm -rf "$ROCK_DIR"
done
