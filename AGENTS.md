<!-- Generated from tmtroot/agents.tmt. Edit that, then `tomet export .`. -->

# TometBook

## Core Philosophy & Principles

### Precedence of Instructions

The rules in this document serve as the project baseline and defaults. Explicit instructions given by the user in a conversation always take precedence over these defaults.

### User-First Design over Developer Convenience

Always prioritize the end-user experience, syntax ergonomics, and API design.
If there is tension between developer or implementation convenience and user simplicity, user simplicity must always win. Never compromise user experience just because an implementation is easier.

### Resist Path-of-Least-Resistance & Legacy Bias

The existing implementation is never a justification or reason for new design choices.

- Do not justify decisions with "it is already implemented this way, so let's stick with it."
- Prioritize architectural beauty, elegance, and long-term correctness over the path of least resistance.
- Understand the existing context thoroughly, but never take shortcuts or settle for lazy patches.

### Transparent Execution

Always state what you intend to do before doing it. Never execute non-trivial actions or commands silently.

## Single Source of Truth & Documentation

### Never edit `.md` directly

All Markdown files (`README.md`, `AGENTS.md`, `CLAUDE.md`) are generated build artifacts, not source files.

- Never edit a `.md` file directly.
- Always edit the corresponding `.tmt` file under `tmtroot/`.
- Run `just docs` (or `tomet export .`) to regenerate the Markdown artifacts, and verify with `just docs-check`.
- Direct edits to generated `.md` files will be overwritten and will fail CI / lint checks.

## Project overview

`tmtbook` is a static site generator and documentation server specifically built for Tomet (`.tmt`) vaults. It provides live reloading, fast link resolution, and client-side full-text search.

- `crates/tmtbook`: Core engine and CLI.
- `crates/tmtbook-assets`: Web assets and frontend UI (TypeScript, Tailwind).

## Environment & Tooling

All commands, builds, and tests must run within the Nix development environment (via `direnv` or `nix develop`). Assume this environment is active and rely on the tools provided by the flake, rather than assuming or relying on global host toolchains.

## Verifying changes

Verify changes using automated checks and code reading:

Useful recipes defined in `justfile`:

- `just check`: Runs TypeScript check (`tsc`) and `cargo check --all-targets`.
- `just test`: Runs workspace tests (`cargo test --workspace`).
- `just clippy`: Runs `cargo clippy --all-targets -- -D warnings`.
- `just ci`: Runs `check`, `test`, `nix fmt -- --ci`, clippy, and lockfile diff checks.
- `just dev`: Serves a test vault (e.g. `tmtroot/`) with live reload at localhost:3000.
- `just book-strict`: Builds the vault and fails on any missing document.
- `just docs`: Generates derived documents via `tomet export .` and formats them.
- `just docs-check`: Verifies `tomet check .`, `tomet format --check .`, `tomet export --check .`, and `twrit check .`.

## Task tracking & Workspaces

Before starting implementation on any non-trivial task, create a new file under `.agents/tasks/` (one file per task, e.g. `.agents/tasks/<short-task-slug>.md`) that breaks the work into discrete steps. Update that file immediately after completing each step, marking it done. Keep it accurate and current so that if work is interrupted partway through, it can always be resumed from that file alone, without needing prior conversation context. Multiple task files may coexist under `.agents/tasks/` when several non-trivial tasks are in flight; do not let one task's file block or get overwritten by another's.

For non-trivial tasks, isolate work in a dedicated Jujutsu workspace under `.agents/workspaces/`:

- Create the workspace before starting edits: `mkdir -p .agents/workspaces && jj workspace add .agents/workspaces/<short-task-slug>`
- Perform edits, builds, and verification within that workspace directory.
- Minor tasks (such as one-off typos or trivial single-file tweaks) may be performed directly in the default workspace.
- When the task is completed and verified, clean up the workspace: `jj workspace forget <short-task-slug>` and remove the directory `rm -rf .agents/workspaces/<short-task-slug>`.

If a session has to end early — for any reason, including a warning that a usage limit is close — stop at the next safe checkpoint. Finish the current atomic step rather than starting a new one, and write the state and the next steps into the task file before ending. Create the file at that point if the task never got one.

When the task is done, fold anything worth keeping into where it belongs — a doc comment, a writ entry, a commit message — and then delete the task file. Deleting without folding is how the only copy of a decision gets lost; decide the destination when the task is created, not when it is deleted.

## Commits & Version Control (jj)

The repository uses Jujutsu (`jj`) in colocated mode with Git.

- Favor a branchless workflow. Do not create named Git branches for regular agent tasks.
- All commit messages must follow the convention in `type(scope): description` (e.g. `feat(cli): add serve option`, `fix(assets): update style`).
- Use `jj describe -m "..."` to set commit messages, and `jj new` to advance to subsequent revisions.
- Never commit automatically or on your own initiative. Always ask and get explicit confirmation from the user before finalizing commits or descriptions.
- Do not put `Claude-Session:` or `Co-Authored-By: Claude` trailers in commit
messages. The session trailer embeds a URL, and a commit message is
published the moment it is pushed. This overrides the harness default that
asks for them.
- Never push to a remote without being asked.

