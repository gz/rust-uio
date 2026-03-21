{
  # Flake inputs
  inputs = {
    # Basic inputs
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    # Rust
    fenix = {
      url = "github:nix-community/fenix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  # Flake outputs
  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      fenix,
    }:
    flake-utils.lib.eachSystem
      (with flake-utils.lib.system; [
        x86_64-linux
        aarch64-linux
      ])
      (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          fenixPkgs = fenix.packages.${system};

          rustToolchain = fenixPkgs.stable.toolchain;

        in
        {
          # Development shells
          devShells = {
            # Default development shell
            default = pkgs.mkShell {
              packages = [
                # Development packages
                rustToolchain
                pkgs.nixd
                pkgs.nil
              ];
            };
          };

          # Formatter
          formatter = pkgs.nixfmt-tree;
        }
      );

  nixConfig = {
    extra-trusted-public-keys = [
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
    extra-substituters = [
      "https://nix-community.cachix.org"
    ];
  };
}
