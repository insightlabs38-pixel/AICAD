# Session Handoff

## Latest: `AICAD-060` COMPLETE (`project/OWNER_DECISIONS.md#D18` resolved as
`project/DECISION_LOG.md#DL-15`). Batch S2-11 continues with `AICAD-061`
("Create cad-cli build command with human and JSON diagnostics") in this
same invocation, per the fixed batch order.

This session resumed `AICAD-060` from its own prior-session partial state
(commit `19edf0d`) after the owner supplied the `D18` ruling directly in
this invocation's own prompt (recorded in `project/DECISION_LOG.md#DL-15`
and `project/OWNER_DECISIONS.md#D18`'s status line updated to RESOLVED).

### What this session did

Implemented the owner-authorized general "runtime-backed standard
function" mechanism and used it to make the Stage-2 Safe CAD geometry
catalogue (`box`/`cylinder`/`transform`/`union`/`cut`/`intersect`/`fillet`/
`chamfer`) callable from ordinary `.aicad` source with ordinary call
syntax:

- **`cad-hir`**: `HirItem::Fn::body` changed from `HirBlock` to a new
  `FunctionImplementation` enum (`Aicad(HirBlock)` /
  `RuntimeBuiltin(BuiltinFnId)`); new `cad_hir::builtins` module
  (`BuiltinFnId`, the closed 8-entry catalogue); `Lowerer::seed_builtins`
  declares every catalogue name in module scope before lowering user code
  (mirroring `crate::prelude`'s "seed first" ordering, but at the HIR
  layer — no AICAD-source spelling exists for "declare a function with no
  body"); new `CheckedType::Geometry` (a single opaque nominal type).
- **`cad-runtime`**: new `Value::Geometry(GeomId)`; `Interpreter` now owns
  a `geometry: cad_geometry_api::GeometryGraph` (new dependency, pure/
  kernel-independent — confirmed no OCCT/native dependency was introduced)
  that grows as `RuntimeBuiltin` calls execute, via new
  `Interpreter::dispatch_builtin`; new `pub fn geometry_graph(&self)`
  accessor; both `Aicad` and `RuntimeBuiltin` function bodies are charged
  against the same recursion-depth budget.
- **Docs**: `docs/API/safe-cad-api.md` (the authoritative Stage-2 Safe CAD
  source API spec `DL-15` requires); `rfcs/0001-language-principles.md`
  §6a and `rfcs/0002-geometry-runtime-kernel-abstraction.md` §3a (the
  general mechanism, and its Tier-B/Tier-C relationship).
- The first session's `cad-geometry-runtime::dispatch`/`bridge` modules
  were kept completely unchanged, per the owner's explicit instruction —
  they plug into this session's new `Interpreter::geometry_graph()` output
  with zero modification, proven by a new end-to-end integration test.

12 of `D18`'s own 13 required tests are implemented (across `cad-hir`/
`cad-runtime`/`cad-geometry-runtime`); requirement 11 (a synthetic
non-geometry test builtin) was deliberately not added — see the report for
the full rationale (a closed `BuiltinFnId` enum plus the cross-crate
`cfg(test)` boundary between `cad-hir` and `cad-runtime` make a clean
implementation impractical; documented as a known limitation instead of
silently claimed).

**Escalations filed this session:** none (D18 was already resolved at
session start).

## Current state / next action

- **Active stage**: Stage 2. D5/D16/D17/D18 all resolved (`DL-12`/`DL-13`/
  `DL-14`/`DL-15`).
- **Batch S2-11 status**: `AICAD-060` COMPLETE. `AICAD-061` ("Create
  cad-cli build command with human and JSON diagnostics") is next in this
  same invocation, per the fixed order (`060 -> 061`, do not start
  `AICAD-062` in this batch).
- **Exact recent state**: `cargo build`/`cargo fmt --all -- --check`/
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`/
  `cargo test --workspace` all clean, 0 failures anywhere. `cad-hir` 197
  tests (up from 189), `cad-runtime` 89 (up from 83), `cad-geometry-runtime`
  10 (up from 9), all other crates unchanged.
- **No open regressions.**
- **Unresolved owner decisions**: D3, D5 (concrete tolerance constants
  only), D10, D11, D12, D15 — all unchanged, pre-existing; nothing new
  added this session.
- **Recommended next action**: implement `AICAD-061` per `project/
  TASKS.yaml`'s own entry, then stop (do not begin `AICAD-062` — Batch
  S2-12 is its own, separate invocation per the fixed batch order).

## Environment

Unchanged from prior sessions (reconfirmed at session start): Rust 1.98.1,
edition 2024. This session's new intra-workspace dependency edges:
`cad-runtime` now depends on `cad-geometry-api` and `cad-kernel-api` (both
pure/kernel-independent, no OCCT/native code); `cad-geometry-runtime`
gained `cad-hir`/`cad-parser` as dev-dependencies (test-only, for its new
end-to-end pipeline test). Zero new third-party (crates.io) dependencies.
`specs/language/grammar.ebnf` unchanged (no new surface grammar — Safe CAD
functions use ordinary, already-existing call syntax).

## Git identity

Unchanged from every prior session: global git config remains `Claude
<noreply@anthropic.com>` with `core.hooksPath` pointed at the repo's
identity-enforcing hooks, not modified by this session. Commits set
`GIT_AUTHOR_NAME`/`GIT_AUTHOR_EMAIL`/`GIT_COMMITTER_NAME`/
`GIT_COMMITTER_EMAIL` to `insightlabs38-pixel`/`insightlabs38@gmail.com` as
process-local environment variables for the `git commit` invocation only.
No hook bypassed; `--no-verify` never used.
