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

# Type-check TypeScript without producing output
check-ui:
    tsc --project ui/tsconfig.json --noEmit

# Type-check without producing a binary
check: check-ui
    cargo check --all-targets

# Lint. The tree is warning-free; keep it that way.
clippy:
    cargo clippy --all-targets -- -D warnings

# Format every tracked file (rust, nix, toml, shell, tomet)
fmt:
    nix fmt

# Everything a CI job would run. Formatting is verified, not applied.
ci: check test
    nix fmt -- --ci
    cargo clippy --all-targets -- -D warnings
    # nix build reads Cargo.lock to vendor the tomet crates; a lock that drifted
    # from what cargo resolves here would send it to the network for them.
    git diff --exit-code Cargo.lock

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
