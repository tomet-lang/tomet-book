{
  pkgs,
  mkShell,
  fenix,

  tmtbook,
  tomet,
  tomet-lsp,
  twrit,
  ...
}:
let
  rustToolchain = fenix.combine [
    (fenix.stable.withComponents [
      "cargo"
      "clippy"
      "rustc"
      "rust-src"
      "rustfmt"
      "rust-analyzer"
    ])
  ];
in
mkShell rec {
  buildInputs = with pkgs; [
    #= Develop
    tmtbook
    tomet
    tomet-lsp
    twrit
    just
    #== Build
    pkg-config
    pagefind
    #== Rust
    rustToolchain
    cargo-edit
    cargo-nextest
    #== UI
    typescript
    esbuild
    tailwindcss_4
  ];

  shellHook = ''
    echo "📖 Rust Tomet Book"
  '';
}
