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

    #[ Develop ]
    tailwindcss
    just
    ##[ Rust ]
    rust-toolchain
    cargo-edit
    cargo-nextest

    #[ Runtime ]
    pkg-config
  ];

  shellHook = ''
    echo "📖 rust tomet"
  '';
}
