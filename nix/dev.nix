{
  inputs,
  pkgs,
  stdenv,
  mkShell,
  fenix,

  tmtbook,
  tomet,
  ...
}:
let
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
