{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nci.url = "github:yusdacra/nix-cargo-integration";
    nci.inputs.nixpkgs.follows = "nixpkgs";
    parts.url = "github:hercules-ci/flake-parts";
    parts.inputs.nixpkgs-lib.follows = "nixpkgs";
  };

  outputs =
    inputs@{ parts, nci, ... }:
    parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      imports = [
        nci.flakeModule
        ./crates.nix
      ];
      perSystem =
        { config, ... }:
        let
          # shorthand for accessing this crate's outputs
          # you can access crate outputs under `config.nci.outputs.<crate name>` (see documentation)
          crateOutputs = config.nci.outputs."fastest-ecs-scheduler";
        in
        {
          # export the crate devshell as the default devshell
          devShells.default = crateOutputs.devShell;
          # export the release package of the crate as default package
          packages.default = crateOutputs.packages.release;
        };
    };
}
