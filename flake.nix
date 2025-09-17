{
  inputs = {
    nixpkgs = {
      url = "github:nixos/nixpkgs/nixpkgs-unstable";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs = {
        nixpkgs = {
          follows = "nixpkgs";
        };
      };
    };
  };
  outputs =
    {
      nixpkgs,
      flake-utils,
      fenix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            fenix.packages.${system}.latest.toolchain
          ];

          shellHook = ''
            echo -e "\033[1;32m\n\noso development environment loaded"
            echo -e "System: ${system}"
            echo -e "Available tools:"
            echo -e "  - cargo: $(which cargo 2>/dev/null || echo 'not found')"
            echo -e "\n\033[0m"
          '';
        };
      }
    );
}
