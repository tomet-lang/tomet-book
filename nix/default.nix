{
  flake-parts,
  ...
}@inputs:
flake-parts.lib.mkFlake { inherit inputs; } {
  systems = [
    "x86_64-linux"
    "aarch64-linux"
    "aarch64-darwin"
  ];
  imports = [
    inputs.treefmt-nix.flakeModule
  ];

  perSystem =
    { pkgs, ... }:
    let
      craneLib = inputs.crane.mkLib pkgs;
    in
    {
      packages = rec {
        default = tmtbook;
        tmtbook = pkgs.callPackage ./pkgs/tmtbook.nix { inherit craneLib; };
      };

      devShells.default = pkgs.callPackage ./dev.nix {
        inherit inputs craneLib;
        tmtbook = pkgs.callPackage ./pkgs/tmtbook.nix { inherit craneLib; };
        tomet = inputs.tomet.packages.${pkgs.system}.tomet;
      };

      treefmt = import ./formatter.nix;
    };
}
