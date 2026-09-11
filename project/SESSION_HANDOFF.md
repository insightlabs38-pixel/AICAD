# Session Handoff

## Latest: `AICAD-059` COMPLETE. Batch S2-10 finished (it contained only
`AICAD-059`). Next: `AICAD-060` (Batch S2-11, "Implement HIR/runtime
geometry dispatch into Geometry IR/kernel API").

This session started from `b55dbc2` ("Batch S2-09 checkpoint:
STAGE2-C_EXECUTION.md (PASS)"), the tip of `origin/claude/aicad-stage2-dev`
at session start. The session's container had initially checked out
`branch/admiring-wozniak-jy47ie` (= `origin/main`'s tip, an older/random
branch per the campaign brief's own language); per the active campaign
brief's explicit instruction, the working branch was reset to the newest
`origin/claude/aicad-stage2-dev` before any roadmap work began (`git
checkout -B claude/aicad-stage2-dev origin/claude/aicad-stage2-dev`).
Stage-1 owner approval (`DECISION_LOG.md#DL-11`) and the D5
determinism-equivalence policy (`DECISION_LOG.md#DL-12`) were already
durably recorded from a prior session and were **not** re-recorded, per
the campaign brief's "Do not duplicate the approval on later invocations."

### What this session did

**`AICAD-059`: Create backend-independent Geometry IR** (Batch S2-10, its
own architectural boundary batch — per the fixed batch order, this
invocation did *not* start `AICAD-060`). Populated the previously-empty
`crates/cad-geometry-api` placeholder crate with the Geometry IR:

- `GeomId` — an SSA-style node index local to one `GeometryGraph`, minted
  only by the graph itself (never `cad_kernel_api::KernelId` — no kernel
  context/epoch involved at this layer).
- `Quantity { magnitude: f64, ty: cad_units::OperandType }` — typed
  engineering quantities for every dimensioned operation parameter, reusing
  `cad_units`'s existing vocabulary directly rather than inventing a
  parallel one (smooths the `AICAD-060` conversion from a runtime
  `NumberValue`).
- `GeometryOp` (18 variants: `Box`, `Cylinder`, `ImportStep`, `LineEdge`,
  `CircleWire`, `WireFromEdges`, `MakeFace`, `Extrude`, `Revolve`, `Sweep`,
  `Loft`, `Union`, `Cut`, `Intersect`, `Fillet`, `Chamfer`, `Shell`,
  `Offset`, `Transform`) and `GeometryQuery` (8 variants: `IsValid`,
  `Volume`, `Area`, `BoundingBox`, `CenterOfMass`, `Validate`,
  `Tessellate`, `ExportStep`) — every variant maps one-to-one onto an
  existing `cad-occt-bridge` capability (`DECISION_LOG.md#DL-5`'s
  capability-driven minimal surface); no new kernel capability invented.
- `GeometryGraph` — an append-only, functional/SSA (`DL-2`) node builder
  (`push_op`/`push_query`) enforcing purely structural,
  backend-independent invariants at construction time: every `GeomId`
  operand must already exist in *this* graph and produce geometry (not a
  query result); every `Quantity` must carry the dimension its slot
  requires; operand lists documented as non-empty (`WireFromEdges.edges`,
  `Loft.sections`, `Fillet.edges`, `Chamfer.edges`) must be.
- `GeometryIrError` activates the `"GEOM"` diagnostic family
  `cad_diagnostics::DIAGNOSTIC_FAMILIES` reserved since `AICAD-038` and
  left unused until now (`GEOM-E001`..`GEOM-E004`), mirroring exactly how
  `AICAD-058` activated `"BUDGET"`.
- `EdgeIndex`/`FaceIndex` — raw index-based edge/face selectors for
  `Fillet`/`Chamfer`/`Shell`, matching the *only* selection mechanism
  `cad-occt-bridge` exposes today. Documented prominently (module doc
  comment + both newtypes' own doc comments) as a known, deliberate
  limitation — raw/epoch-bound per `AGENTS.md`, not a durable semantic
  reference, to be superseded by the Stage-4 semantic-reference layer
  (`DECISION_LOG.md#DL-9`).

`cad-geometry-api` depends only on `cad-ast` (`Span`/`LineIndex`),
`cad-diagnostics`, `cad-kernel-api` (for its already kernel-neutral
`Point3`/`Vector3`/`Direction3`/`Axis3`/`Transform` math primitives only —
never a `Kernel*` handle/error type), `cad-types`, `cad-units`. **No**
dependency on `cad-hir`, `cad-runtime`, or `cad-occt-bridge` — this crate
builds and validates the IR only; it makes zero kernel calls and
constructs zero kernel contexts (confirmed both by `Cargo.toml` inspection
and `cargo tree -p cad-geometry-api`'s full dependency closure, reproduced
in `project/reports/AICAD-059.md`). Dispatching a `GeometryGraph` into
actual kernel calls is `AICAD-060`'s job, not crossed by this task.

16 new tests in `crates/cad-geometry-api/src/ir.rs` (0 -> 16 for this
crate). Full detail, including the scope-cut rationale (why no low-level
topology-exploration ops were added) and all four material design
decisions, in `project/reports/AICAD-059.md`.

**Zero escalations filed this session.** No `AGENTS.md` escalation
condition was triggered: no public syntax changed, no kernel-specific type
crossed the Geometry IR boundary, no stage gate/test was weakened, no
ambiguous reference was resolved by arbitrary fallback (the module's own
`OperandIsNotGeometry`/`InvalidOperand` checks fail closed instead), and no
later roadmap stage was entered.

## Current state / next action

- **Active stage**: Stage 2. Stage 1 closed (DL-11); D5 policy-shape closed
  (DL-12); D16 closed (DL-13); D17 closed (DL-14).
- **Batch S2-10 is COMPLETE**: `AICAD-059` done (it was this batch's only
  task, per the fixed batch order).
- **THE NEXT INVOCATION MUST START `AICAD-060`** ("Implement HIR/runtime
  geometry dispatch into Geometry IR/kernel API", `project/TASKS.yaml`,
  Batch S2-11 — its own dependency, `AICAD-059`, is now satisfied). Per the
  fixed batch order: Batch S2-11 is `AICAD-060 -> AICAD-061` — do not start
  `AICAD-062` in the same invocation that completes both.
  - Before writing any dispatch code, read `crates/cad-geometry-api/src/
    ir.rs`'s own module doc comment (this session's own new code — do not
    re-derive its design from memory) plus `crates/cad-runtime/src/
    interp.rs` (the `Interpreter`/`Value`/`NumberValue` shapes `AICAD-060`
    must bridge from) and `crates/cad-occt-bridge/src/lib.rs`
    (`OcctContext`/`Shape`'s exact operation signatures the dispatcher
    calls into).
  - `AICAD-060`'s own likely first real design question: how does the
    runtime's ordinary value/expression evaluation (numbers, functions,
    control flow — all already implemented) actually *produce* a
    `GeometryGraph`/dispatch into it? No Stage-2 task through `AICAD-058`
    added any geometry-producing syntax or builtin to the language at all
    (`cad-hir`'s prelude has no `box`/`cylinder`/etc. today) — confirm
    this by inspection before assuming a mechanism; if giving a `.aicad`
    program a way to actually invoke a geometry operation requires new
    public syntax or a new binding mechanism beyond ordinary `fn`/`call`
    resolution, that is an `AGENTS.md` escalation trigger ("change public
    language syntax or semantics beyond an approved RFC") the same way
    `AICAD-056`'s `for`-loop collection gap was (`D16`, `DECISION_LOG.md
    #DL-13`) — do not silently invent a mechanism; escalate to
    `project/OWNER_DECISIONS.md` if the coverage gap is real, following
    the `D16`/`D17` precedent exactly (audit first, then escalate a
    concrete, scoped question, not an open-ended one).
  - `AICAD-061` ("Create cad-cli build command with human and JSON
    diagnostics") depends on `AICAD-060`'s dispatcher existing first, per
    the batch's own required order.
- **Exact recent state**: this session's own fresh runs (most recent
  first, all from a clean working tree at this session's own final
  commit): `cargo test --workspace` — every crate "test result: ok", 0
  failures anywhere (`cad-hir` 187, `cad-runtime` 83, `cad-units` 84,
  `cad-geometry-api` 16 [new], `cad-occt-bridge`'s three test binaries 9+6+3,
  all others unchanged from the `AICAD-058` session's own baseline);
  `cargo build --workspace --all-targets` clean; `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` clean; `cargo fmt --all --
  --check` clean (after one `cargo fmt --all` pass on the new
  `ir.rs` — recorded, not hidden, in `project/reports/AICAD-059.md`).
- **No open regressions.**
- **No escalations filed this session.**
- **Unresolved owner decisions**: unchanged from prior sessions — D17
  resolved (DL-14). Open/partial: D3, D5 (concrete tolerance constants
  only), D10, D11, D12, D15.
- **D5 status/evidence**: unchanged from the `AICAD-058` session's own
  `STAGE2-C_EXECUTION.md` §5 cross-check (still the freshest for the
  execution phase). This session's own `GeometryGraph` builder is a pure,
  ordinary `Vec`-backed Rust data structure with no iteration-order-
  dependent collection, no concurrency, and no filesystem/environment
  dependence — consistent with `DECISION_LOG.md#DL-12` Level 1, though no
  new determinism-specific test was needed (nothing in this module is
  nondeterminism-prone; that becomes meaningful once `AICAD-060`
  introduces actual kernel dispatch). Concrete D5 tolerance constants (the
  still-open half of D5) remain undetermined, unchanged.
- **Pre-existing `TASKS.yaml` staleness** (unchanged, not this batch's
  scope): `AICAD-001` through `AICAD-037` still show `status: todo` despite
  being long complete.
- **Recommended next action**: start `AICAD-060` (Batch S2-11) per
  `project/TASKS.yaml`'s own entry and the notes above.

## Environment

Unchanged from prior sessions (reconfirmed at session start): Rust 1.98.1
(auto-installed via `rustup`, matching `rust-toolchain.toml` — the
toolchain does not persist across sessions in this container), edition
2024. This session added one new intra-workspace dependency edge set:
`cad-geometry-api` now depends on `cad-ast`, `cad-diagnostics`,
`cad-kernel-api`, `cad-types`, `cad-units` (previously depended on
nothing, per its `AICAD-002`/`AICAD-003` placeholder state). Zero new
third-party (crates.io) dependencies. No `specs/language/grammar.ebnf`
change (no new syntax — `AICAD-059` is a pure new-crate-content IR
addition with no language-surface change). No native/OCCT work was
touched; `crates/cad-occt-bridge`, `native/occt_bridge` are unchanged by
this session.

## Git identity

This container's global git config is `Claude <noreply@anthropic.com>`
(with `user.useConfigOnly=true` and `core.hooksPath` pointed at the repo's
identity-enforcing hooks) — matching every prior session's own finding, and
**not modified** by this session (per `CLAUDE.md`/`AGENTS.md`: "NEVER
update the git config"). This session's commit sets `GIT_AUTHOR_NAME`/
`GIT_AUTHOR_EMAIL`/`GIT_COMMITTER_NAME`/`GIT_COMMITTER_EMAIL` to
`insightlabs38-pixel`/`insightlabs38@gmail.com` as process-local
environment variables for the `git commit` invocation only — the repo's
`prepare-commit-msg` hook passed without needing any config change. No
hook was bypassed or modified; `core.hooksPath` was left untouched;
`--no-verify` was never used.
