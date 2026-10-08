<!-- Generated from tmtroot/readme.tmt. Edit that, then `tomet export .`. -->

# TometBook

Fast static site generator and documentation server specifically built for Tomet (`.tmt`) vaults.

## Overview

`tmtbook` provides a modern documentation site experience for Tomet vaults. It features:

- **Blazing fast builds**: Written in Rust, optimized for large-scale knowledge bases.
- **Instant link resolution**: First-class support for Tomet intra-vault link resolution and references.
- **Client-side search**: Full-text client-side indexing and instant search interface.
- **Live reloading development server**: Fast feedback cycle during documentation authoring.

## Workspace Layout

- `crates/tmtbook` (`tmtbook`) -- Core SSG engine, routing, markdown rendering pipeline, and CLI server.
- `crates/tmtbook-assets` (`tmtbook-assets`) -- Frontend web assets, theme styling, and client scripts.

## Building and Running

### Build workspace

```bash
cargo build --workspace
```

### Run tests

```bash
cargo test --workspace
```

### Serve a vault locally

```bash
cargo run -p tmtbook -- serve tmtroot/
```

