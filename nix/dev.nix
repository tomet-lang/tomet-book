{
  inputs,
  pkgs,
  stdenv,
  mkShell,

  tmtbook,
  tomet,
  ...
}:
let
  fenix = inputs.fenix.packages.${stdenv.hostPlatform.system};
  rust-toolchain = fenix.combine [
    (fenix.stable.withComponents [
      "cargo"
      "clippy"
      "rustc"
      "rust-src"
    ])
  ];
in
mkShell rec {
  buildInputs = with pkgs; [
    tmtbook
    tomet
    pagefind

    #[ Rust ]
    rust-toolchain
    cargo-edit
    cargo-nextest

    #[ Misc ]
    pkg-config
  ];

  shellHook = ''
    echo "📖 tmtbook - Rust dev shell ready"
  '';
}
