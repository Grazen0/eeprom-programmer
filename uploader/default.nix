{
  lib,
  rustPlatform,

  pkg-config,
  udev,
}:
let
  manifest = (lib.importTOML ./Cargo.toml).package;
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = manifest.name;
  inherit (manifest) version;

  src = ./.;
  cargoLock.lockFile = "${finalAttrs.src}/Cargo.lock";

  nativeBuildInputs = [
    pkg-config
    udev
  ];
  env = {
    PKG_CONFIG_PATH = "${udev.dev}/lib/pkgconfig";
  };

  meta = with lib; {
    inherit (manifest) description homepage;
    license = licenses.mit;
  };
})
