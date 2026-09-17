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

    #[ Develop ]
    just
    ##[ UI ]
    tailwindcss
    esbuild
    ##[ Rust ]
    rust-toolchain
    cargo-edit
    cargo-nextest

    #[ Runtime ]
    pkg-config
    pagefind
  ];

  shellHook = ''
    echo "📖 rust tomet"
  '';
}
