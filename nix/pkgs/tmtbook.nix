{
  craneLib,
  lib,
  tailwindcss,
  # Cargo features to build with, e.g. [ "embedded-search" ]. See
  # ./tmtbook-embedded-search.nix for the variant that sets this.
  features ? [ ],
}:
let
  root = ../..;

  # Only what the compiler actually reads: the cargo sources, plus the CSS, JS
  # and templates under src/assets that `include_str!` bakes into the binary,
  # plus tailwind.config.js (read by build.rs's `tailwindcss` invocation).
  # Everything else in the repo -- notes in tmtroot/, the justfile, the flake
  # files -- would otherwise trigger a full recompile when edited.
  src = lib.fileset.toSource {
    inherit root;
    fileset = lib.fileset.unions [
      (craneLib.fileset.commonCargoSources root)
      (root + "/src/assets")
      (root + "/tailwind.config.js")
    ];
  };

  commonArgs = {
    inherit src;
    # build.rs shells out to `tailwindcss` to compile the utility CSS used
    # by the Jinja templates; the sandboxed build needs it on PATH same as
    # the interactive dev shell (nix/dev.nix) already has it.
    nativeBuildInputs = [ tailwindcss ];
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
