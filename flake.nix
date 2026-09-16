{
  description = "Tomet Book generator (tmtbook) - A standalone book & wiki generator for Tomet";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix.url = "github:numtide/treefmt-nix";

    #[ Rust ]
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";

    #[ Dev ]
    tomet.url = "github:tomet-lang/tomet";
  };

  outputs = inputs: import ./nix inputs;
}
