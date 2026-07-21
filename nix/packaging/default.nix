{
  lib,
  stdenv,
  fetchFromGitHub,
  fetchurl,
  vimUtils,
  nix-update-script,
}:

let
  version = "0.0.0-development";

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
    sha256 = lib.fakeHash;
  };
in
vimUtils.buildVimPlugin {
  pname = "hermes-nvim";
  inherit version;

  src = fetchFromGitHub {
    owner = "Ruddickmg";
    repo = "hermes.nvim";
    tag = "v${version}";
    hash = lib.fakeHash;
  };

  # Place the pre-built binary where binary.lua's get_rock_binary_path() looks:
  #   <plugin_root>/lib/libhermes-<os>-<arch>.<ext>
  postInstall = ''
    mkdir -p $out/lib
    cp ${hermes-binary} $out/lib/${binaryName}
  '';

  passthru = {
    updateScript = nix-update-script {
      attrPath = "vimPlugins.hermes-nvim";
    };
    inherit hermes-binary;
  };

  meta = {
    description = "ACP (Agent Client Protocol) client for Neovim";
    homepage = "https://github.com/Ruddickmg/hermes.nvim";
    license = lib.licenses.mit;
  };
}
