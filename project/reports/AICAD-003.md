# AICAD-003 — Create Rust workspace/toolchain policy and baseline ignore/editor config

## Objective
Turn the AICAD-002 directory skeleton into a real, buildable Rust workspace
with a pinned toolchain policy, baseline `.gitignore`/`.editorconfig`, and
formatter policy, per `project/TASKS.yaml` (AICAD-003).

## Dependencies checked
AICAD-002 (monorepo directory skeleton, 28 `crates/*` directories with
README ownership docs) — complete, see `project/reports/AICAD-002.md`.

## Decisions made and why

1. **Workspace member list = the exact 28 crates from
   `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §1** (already created as
   directories in AICAD-002). No crate was added or removed.
2. **`edition = "2024"`, `rust-version = "1.98"`.** The installed toolchain
   is `rustc 1.98.1`, whose `cargo new` default is already edition 2024
   (verified: see Verification section), so this is the toolchain's own
   modern default rather than a speculative choice. Pinned as
   `[workspace.package]` fields inherited by every crate
   (`edition.workspace = true`, etc.) so there is one place to change it.
3. **`rust-toolchain.toml` pins `channel = "1.98.1"` exactly** (not just
   `"stable"`), because `docs/plan/14_COLLABORATION_PROVENANCE_SECURITY.md`
   §8 lists "compiler version" as part of deterministic build identity, and
   `docs/plan/01_SYSTEM_ARCHITECTURE.md` §6 lists "compiler version" as a
   cache-key component. Pinning the exact patch version is a direct,
   uncontroversial implementation of that requirement and does not touch
   the still-open D5 question (what "equivalent geometry" means across
   platforms) in `project/OWNER_DECISIONS.md`.
4. **`publish = false` and `version = "0.0.0"` workspace-wide.** Nothing in
   any crate is implemented yet (Stage 0 explicitly disallows production
   feature work); `0.0.0`/unpublishable signals "not yet functional" rather
   than implying a real pre-1.0 release line has started.
5. **Per-crate `Cargo.toml` uses workspace inheritance** (`version.workspace`,
   `edition.workspace`, `rust-version.workspace`, `license.workspace`,
   `publish.workspace`) with no crate-specific overrides, keeping all 28
   manifests mechanically identical and easy to audit.
6. **`src/lib.rs` per crate is a doc-comment-only placeholder** (no code),
   pointing to the crate's own `README.md` (ownership/plan references, from
   AICAD-002) and `project/TASKS.yaml`. This is intentionally the smallest
   possible change that makes the workspace buildable — no placeholder
   types/functions were invented, since Stage 0 disallows production
   feature work and inventing placeholder APIs now would risk presupposing
   the still-open D1-D4 syntax/type decisions.
7. **`.gitignore`** excludes `/target/` (build output), `.aicad-cache/`
   (explicitly a disposable cache per `docs/plan/01_SYSTEM_ARCHITECTURE.md`
   §6, never canonical), native build directories, and common editor/OS/
   secret-file artifacts. **`Cargo.lock` is committed, not ignored** — this
   is a workspace that will produce binaries (`cad-cli`) and the plan's
   determinism invariant (`docs/plan/00_PRINCIPLES_AND_SCOPE.md` §3
   invariant 9; `docs/plan/14_COLLABORATION_PROVENANCE_SECURITY.md` §9)
   requires reproducible builds from source + lockfile, so the lockfile
   must be version-controlled.
8. **`.editorconfig`** sets UTF-8/LF/final-newline/trim-trailing-whitespace
   defaults, 4-space indent for Rust, 2-space for YAML/JSON, and disables
   trailing-whitespace trimming for Markdown (to preserve the two-trailing-
   space hard-break convention). Not otherwise opinionated.
9. **`rustfmt.toml`** pins `edition = "2024"` to match `Cargo.toml`; no
   other style overrides — default `rustfmt` style is used.

No escalation condition was triggered: this is toolchain/CI scaffolding,
explicitly allowed at Stage 0, and introduces no public language
syntax/semantics, no kernel-type exposure, and no reference-resolution
behavior.

## Files changed
- Added: `Cargo.toml` (workspace), `rust-toolchain.toml`, `rustfmt.toml`,
  `.gitignore`, `.editorconfig`, `Cargo.lock`.
- Added: `crates/<name>/Cargo.toml` and `crates/<name>/src/lib.rs` for all
  28 crates listed in `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §1.

## Verification (exact commands/results)
```
$ rustc --version && cargo --version && cargo clippy --version && cargo fmt --version
rustc 1.98.1 (48a229cea 2026-09-01)
cargo 1.98.1 (797e8a9bc 2026-08-05)
clippy 0.1.98 (48a229ceae 2026-09-01)
rustfmt 1.9.0-stable (48a229ceae 2026-09-01)

$ cargo new /tmp/test_new_crate --lib   # confirm this toolchain's own default edition
edition = "2024"                        # (used to justify decision 2 above; test crate discarded)

$ cargo build --workspace
   Compiling cad-cli v0.0.0 (...)
   ... (28 crates)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.23s

$ cargo fmt --all -- --check
(exit code 0, no output)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Checking cad-cli v0.0.0 (...)
   ... (28 crates)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.35s
(exit code 0, zero warnings)

$ cargo test --workspace
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
(repeated per crate; all "ok", 0 tests — none exist yet, this only confirms
the harness runs cleanly)

$ git check-ignore -v target/
.gitignore:3:**/target/	target/
```

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS** (now applicable; Rust workspace
  exists as of this task).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**, zero warnings across all 28 crates.
- Task-specific check: `cargo build --workspace` and
  `cargo test --workspace` both succeed cleanly (see above).

## Limitations / follow-up
- No crate has any real implementation yet — every `src/lib.rs` is a
  doc-comment placeholder. Populating them starts with AICAD-004 (CI) and
  the Stage 0 RFC/task work (AICAD-007+).
- No `CMakeLists.txt`/`native/occt_bridge` build wiring yet — that begins
  at Stage 1 (AICAD-015, "Add OCCT discovery/probe CMake target").
- No `package.json`/IDE tooling yet — that begins at Stage 9 (WP-14/WP-15).
- `Cargo.lock` currently has no third-party dependency entries (no crate
  declares any dependency yet); it will grow as real crates start
  depending on external crates.
