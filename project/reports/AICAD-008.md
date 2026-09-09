# AICAD-008 — Draft RFC-0002 geometry/runtime and kernel abstraction

## Objective
Draft RFC-0002 (geometry/runtime and kernel abstraction), covering the
Stage-0 build items "safe vs raw topology model", "source/canonical-state
rules", and "kernel abstraction contract"
(`docs/plan/15_IMPLEMENTATION_ROADMAP.md` Stage 0), per `project/TASKS.yaml`
(AICAD-008).

## Dependencies checked
AICAD-007 (RFC-0001) — complete, see `project/reports/AICAD-007.md`; this
RFC's worked example assumes RFC-0001's syntax/mutation rules. Owner
rulings DL-5 and DL-6 were recorded before this batch began (see
`project/DECISION_LOG.md`).

## What was done

Wrote `rfcs/0002-geometry-runtime-kernel-abstraction.md`:
1. Froze the architecture-layer diagram and crate-to-service mapping from
   `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2, §5, cross-checked against
   every affected crate's own `README.md` (from AICAD-002) so the mapping
   in the RFC and the crate ownership docs agree.
2. Encoded DL-5/D6 (kernel abstraction boundary): kernel-neutral,
   capability-driven `cad-kernel-api`; no OCCT type crosses
   `cad-occt-bridge`; per-operation lineage evidence only (no persistent
   lineage storage at the kernel layer); kernel handles are epoch-local;
   durable identity is explicitly assigned to `cad-references` (RFC-0003),
   not the kernel adapter.
3. Froze the raw-topology safety model verbatim from
   `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §8 and
   `docs/plan/02_LANGUAGE_AND_COMPILER.md` §16 (epochs, `unsafe geometry`
   blocks, validated promotion, no raw-handle export from public APIs).
4. Froze the canonical-state/cache model from `01` §6-7 (source+lockfile+
   immutable-assets canonical; `.aicad-cache/` never canonical; cache-key
   components including compiler/kernel version — ties back to
   `rust-toolchain.toml` from AICAD-003), and the incremental-build
   contract, while explicitly *not* resolving the still-open D5
   determinism-tolerance question.
5. Froze the four execution capability tiers (A-D) from `01` §9 as-is.
6. Encoded DL-6/D13's development-phase OCCT policy (external dependency,
   dynamic linkage, preserved notices, no vendoring, independently
   implemented standards content) while explicitly flagging the remaining
   open formal-distribution-review item as unresolved.
7. Listed alternatives considered per ruling and explicitly deferred the
   D8 (OCAF) question to RFC-0003, since it concerns durable reference
   identity rather than the kernel boundary itself.

No escalation condition was triggered: this RFC only fixes *how* the
already-adopted kernel-independence contract and raw-topology safety model
are enforced at the crate-boundary level, using owner rulings already on
record; it introduces no new public syntax/semantics, does not touch a
stage gate/benchmark, and does not resolve D5, D8, or D13's remaining open
item.

## Files changed
- Added: `rfcs/0002-geometry-runtime-kernel-abstraction.md`.
- Edited: `rfcs/README.md` status table is unchanged (already listed RFC-0002
  as "Draft (Stage 0)" from AICAD-007's index; no edit needed).

## Verification (exact commands/results)
```
$ grep -c "^## " rfcs/0002-geometry-runtime-kernel-abstraction.md
10   # confirms all 10 planned sections are present

$ cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
(both exit 0 — no Rust source touched by this task)
```

Task-specific check: the crate-to-service table in RFC-0002 §2 was checked
against `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §1's crate list and each
listed crate's own `README.md` (AICAD-002/003) to confirm no crate is
misattributed to the wrong architectural service.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific check: crate/service mapping cross-checked, as above.

## Limitations / follow-up
- The exact Stage-1 bridge operation set (§3) remains "add as required by
  the low-level API" per the plan itself — RFC-0002 does not enumerate a
  frozen final operation list, only the starting set and the
  capability-driven growth rule.
- D5, D8 (extent of internal OCAF usage), and D13's formal
  distribution-review item remain open, as intended.
