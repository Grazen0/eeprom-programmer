{
  description = "A program to interact with AT28C EEPROM chips";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
  };

  outputs =
    inputs@{
      self,
      flake-parts,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;

      perSystem =
        {
          self',
          pkgs,
          system,
          ...
        }:
        let
          inherit (pkgs) lib;
        in
        {
          packages = {
            default = self'.packages.eeprom-uploader;

            eeprom-uploader = pkgs.rustPlatform.buildRustPackage (
              finalAttrs:
              let
                manifest = (lib.importTOML (finalAttrs.src + /Cargo.toml)).package;
              in
              {
                pname = manifest.name;
                inherit (manifest) version;

                src = lib.cleanSource ./uploader;
                cargoLock.lockFile = finalAttrs.src + /Cargo.lock;

                nativeBuildInputs = with pkgs; [
                  pkg-config
                  udev.dev
                ];

                env = {
                  PKG_CONFIG_PATH = "${pkgs.udev.dev}/lib/pkgconfig";
                };

                meta = with lib; {
                  inherit (manifest) description homepage;
                  license = licenses.mit;
                };
              }
            );
          };

          devShells.default = pkgs.mkShell {
            inputsFrom = lib.attrValues self'.packages;

            packages = with pkgs; [
              glibc_multi.dev
              platformio
              xxd
            ];
          };
        };
    };
}
