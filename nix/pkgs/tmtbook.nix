{
  craneLib,
  lib,
  tailwindcss,
  esbuild,
  # Cargo features to build with, e.g. [ "embedded-search" ]. See
  # ./tmtbook-embedded-search.nix for the variant that sets this.
  features ? [ ],
}:
let
  root = ../..;

  # Only what the compiler actually reads: the cargo sources, plus the CSS, JS
  # and templates under frontend/ that `include_str!` bakes into the binary,
  # plus tailwind.config.ts (read by build.rs's `tailwindcss` invocation).
  # Everything else in the repo -- notes in tmtroot/, the justfile, the flake
  # files -- would otherwise trigger a full recompile when edited.
  src = lib.fileset.toSource {
    inherit root;
    fileset = lib.fileset.unions [
      (craneLib.fileset.commonCargoSources root)
      (root + "/crates/tmtbook-assets/frontend")
    ];
  };

  commonArgs = {
    inherit src;
    pname = "tmtbook";
    version = "0.1.0";
    # build.rs shells out to `tailwindcss` and `esbuild` to compile the utility CSS
    # and bundle JS; the sandboxed build needs them on PATH same as
    # the interactive dev shell (nix/dev.nix) already has them.
    nativeBuildInputs = [
      tailwindcss
      esbuild
    ];
    cargoExtraArgs =
      "-p tmtbook-cli"
      + lib.optionalString (features != [ ]) " --features ${lib.concatStringsSep "," features}";
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage (
  commonArgs
  // {
    inherit cargoArtifacts;

    doCheck = false;
  }
)
