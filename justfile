# tmtbook development tasks.
# Run `just` to see everything available.

# Vault used by the `dev` and `book` recipes when none is given.
vault := "tmtroot"

default:
    @just --list

#[ Build ]

# Debug build
build:
    cargo build

# Optimized build
release:
    cargo build --release

# Build the Nix package (what CI and installs use)
package:
    nix build

#[ Test & lint ]

# Run the test suite
test:
    cargo test

# Type-check without producing a binary
check:
    cargo check --all-targets

# Lint. Add `-- -D warnings` once the pre-existing warnings are cleared.
clippy:
    cargo clippy --all-targets

# Format every tracked file (rust, nix, toml, shell, tomet)
fmt:
    nix fmt

# Everything a CI job would run. Formatting is verified, not applied.
ci: check test
    nix fmt -- --ci
    cargo clippy --all-targets

#[ Run ]

# Serve a vault with live reload. Bound to localhost; pass host=0.0.0.0 to expose it.
dev dir=vault port="3000" host="127.0.0.1":
    cargo run -- serve {{ dir }} --port {{ port }} --host {{ host }}

# Build a vault into its dist directory
book dir=vault:
    cargo run -- build {{ dir }}

# Build a vault and fail if any document was left out
book-strict dir=vault:
    cargo run -- build {{ dir }} --strict

#[ Housekeeping ]

# Remove build artifacts and generated books
clean:
    cargo clean
    rm -rf {{ vault }}/dist result
