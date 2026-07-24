{
  description = "A program to interact with AT28C EEPROM chips";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    nixpkgs-stable.url = "github:nixos/nixpkgs?ref=nixos-25.05";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
  };

  outputs =
    inputs@{
      self,
      flake-parts,
      nixpkgs-stable,
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

          stablePkgs = import nixpkgs-stable { inherit system; };
          avrPkgs = stablePkgs.pkgsCross.avr;
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
              avrdude
              glibc_multi.dev
              platformio
              rust-analyzer
              xxd
            ];
          };
        };
    };
}
