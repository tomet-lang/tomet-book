{
  pkgs,
  mkShell,
  fenix,

  tmtbook,
  tomet,
  twrit,
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
    twrit
    pagefind

    #[ Rust ]
    rust-toolchain
    cargo-edit
    cargo-nextest

    #[ Misc ]
    just
    pkg-config
  ];

  shellHook = ''
    echo "📖 tmtbook - Rust dev shell ready"
  '';
}
