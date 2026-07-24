{
  lib,
  stdenv,
  fetchFromGitHub,
  fetchurl,
  vimUtils,
}:

let
  version = "0.10.1";

  sourcesJson = fetchurl {
    url = "https://github.com/Ruddickmg/hermes.nvim/releases/download/v${version}/sources.json";
    hash = "sha256-YUWOdtn4RjKLjFkKanA/yw2BYVKVV9saOOduwjE+8WI=";
  };
  sources = builtins.fromJSON (builtins.readFile sourcesJson);

  # Map nix kernel name to hermes os name
  os = {
    "linux" = "linux";
    "darwin" = "macos";
  }.${stdenv.hostPlatform.kernel.name};

  arch = stdenv.hostPlatform.qemuArch or stdenv.hostPlatform.arch;
  arch' = {
    "x86_64" = "x86_64";
    "aarch64" = "aarch64";
  }.${arch} or arch;

  ext = stdenv.hostPlatform.extensions.sharedLibrary;
  binaryName = "libhermes-${os}-${arch'}.${ext}";

  hermes-binary = fetchurl {
    url = "https://github.com/Ruddickmg/hermes.nvim/releases/download/v${version}/${binaryName}";
    hash = sources.${stdenv.hostPlatform.system}.hash;
  };
in
vimUtils.buildVimPlugin {
  pname = "hermes-nvim";
  inherit version;

  src = fetchFromGitHub {
    owner = "Ruddickmg";
    repo = "hermes.nvim";
    tag = "v${version}";
    hash = sources.source.hash;
  };

  # Skip require() check for the native module — the Rust shared library
  # is not available in the build sandbox, so require("hermes") would fail.
  nvimSkipModules = [ "hermes" ];

  # Place the pre-built binary where binary.lua's get_rock_binary_path() looks:
  #   <plugin_root>/lib/libhermes-<os>-<arch>.<ext>
  postInstall = ''
    mkdir -p $out/lib
    cp ${hermes-binary} $out/lib/${binaryName}
  '';

  passthru = {
    inherit hermes-binary;
  };

  meta = {
    description = "ACP (Agent Client Protocol) client for Neovim";
    homepage = "https://github.com/Ruddickmg/hermes.nvim";
    license = lib.licenses.mit;
  };
}
