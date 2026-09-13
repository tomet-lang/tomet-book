{
  craneLib,
  lib,
  # Cargo features to build with, e.g. [ "embedded-search" ]. See
  # ./tmtbook-embedded-search.nix for the variant that sets this.
  features ? [ ],
}:
let
  root = ../..;

  # Only what the compiler actually reads: the cargo sources, plus the CSS, JS
  # and templates under src/assets that `include_str!` bakes into the binary.
  # Everything else in the repo -- notes in tmtroot/, the justfile, the flake
  # files -- would otherwise trigger a full recompile when edited.
  src = lib.fileset.toSource {
    inherit root;
    fileset = lib.fileset.unions [
      (craneLib.fileset.commonCargoSources root)
      (root + "/src/assets")
    ];
  };

  commonArgs = {
    inherit src;
    cargoExtraArgs = lib.optionalString (
      features != [ ]
    ) "--features ${lib.concatStringsSep "," features}";
  }
  // craneLib.crateNameFromCargoToml { cargoToml = root + "/Cargo.toml"; };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage (
  commonArgs
  // {
    inherit cargoArtifacts;

    doCheck = false;
  }
)
