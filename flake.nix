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
        {
          packages = {
            eeprom-uploader = pkgs.callPackage ./uploader { };
            default = self'.packages.eeprom-uploader;
          };

          devShells.default = pkgs.callPackage ./shell.nix {
            inherit (self'.packages) eeprom-uploader;
          };
        };
    };
}
