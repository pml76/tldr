{
  description = "A very basic flake for Rust development";

  inputs.std.url = "github:divnix/std";
  inputs.std.inputs.nixpkgs.follows = "nixpkgs";

  inputs.fenix.url = "github:nix-community/fenix";
  inputs.crane.url = "github:ipetkov/crane";
  inputs.crane.inputs.nixpkgs.follows = "nixpkgs";
  inputs.crane.inputs.flake-compat.follows = "";
  inputs.crane.inputs.rust-overlay.follows = "";

  inputs.nixpkgs.follows = "fenix/nixpkgs";

  inputs.std.inputs.devshell.url = "github:numtide/devshell";
  inputs.std.inputs.devshell.inputs.nixpkgs.follows = "nixpkgs";
    
  outputs = inputs @ {
    self,
    std,
    ...
  }:
    std.growOn {
      inherit inputs;
      systems = ["x86_64-linux"];
      cellsFrom = ./nix;
      cellBlocks = with std.blockTypes; [
        (installables "packages")
        # Contribution Environment
        (devshells "shells")
        (pkgs "rust")
      ];
    } {
      devShells = std.harvest self ["repo" "shells"];
      packages = std.harvest self ["hello" "packages"];
    };

  nixConfig = {
    extra-substituters = [
      "https://nix-community.cachix.org"
    ];
    extra-trusted-public-keys = [
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };
}
