# Stage-2 Owner Gate Packet

Prepared by `AICAD-064`, per `project/gates/README.md`, `AGENTS.md` ("Stage
gates": "The agent may prepare gate evidence and recommend pass/do-not-pass.
The agent may not approve a roadmap stage. Stage progression is an owner
decision."), and the active scheduled-task brief's Batch S2-14 instructions
("Perform no new roadmap feature development in this batch. Audit the
complete Stage-2 implementation against actual code and tests. Produce an
explicit recommendation: PASS / PASS WITH CONDITIONS / DO NOT PASS. The
recommendation is advisory only."). **This packet recommends; it does not
approve.** Nothing in this document treats Stage 2 as passed until the
owner records that decision in `project/DECISION_LOG.md`. Per the active
scheduled-task brief, roadmap advancement STOPS here: `AICAD-065` and all
Stage-3 work are forbidden until that owner decision exists.

This audit re-runs the full workspace verification suite and independently
re-inspects source (not just report claims) rather than only citing prior
task reports.

## 1. Exact git commit/revision

```text
7c7bc6b730ab5019c17728ef8a11b8d4269c1d0e
```

Branch `claude/aicad-stage2-dev` (the canonical Stage-2 development
lineage), exactly `origin/claude/aicad-stage2-dev` at the start of this
invocation. Working tree was clean before this invocation's own doc/gate
edits. All of Batches S2-01 through S2-13 (`AICAD-038` through `AICAD-063`)
are on this commit already; this packet adds no roadmap feature code, only
`project/OWNER_DECISIONS.md`, this gate file, `project/reports/AICAD-064.md`,
`project/TASKS.yaml` status, and `project/SESSION_HANDOFF.md`.

## 2. Stage-2 exit gate and evidence

**Exit gate** (`project/CURRENT_STAGE.md`): "A `.aicad` source program
exercising parameters, engineering units, derived expressions, functions,
and ordinary control flow, and geometry operations compiles end-to-end
(lexer/parser -> binding -> units/type checking -> typed HIR -> execution
-> Geometry IR -> Stage-1 kernel API) into a valid exact B-rep part,
verified by B-rep validity, bounds, dimensions, volume, center of mass,
solid count, topology sanity, and STEP verification (`AICAD-063`), with no
demo-specific interpreter shortcut."

### 2.1 The end-to-end proof itself (`AICAD-063`)

`examples/brackets/stage2_mounting_plate.aicad`: 9 `param`s with
engineering units, 5 functions (one calling the other four), `if`/`while`/
`for`/`match`/`return` control flow, and `box`/`cylinder`/`transform`/
`union`/`cut`/`fillet`/`chamfer` geometry calls — every one an ordinary
`RuntimeBuiltin` standard function (`DL-15`), not a compiler intrinsic.
`crates/cad-cli/tests/stage2_end_to_end.rs` drives
`cad_cli::build::build_source` — the literal, unmodified `cad build`
pipeline — and re-imports the resulting STEP through a second, independent
`OcctContext`, asserting: `is_valid()`, a closed-form volume matched to
~13 significant figures, a bounding box, an exact mirror-symmetry
center-of-mass invariant, an exact solid count (`MANIFOLD_SOLID_BREP(`
occurs exactly once in the exported STEP text), and non-exact
(deliberately loose) topology counts. A second test proves the build is
deterministic across 3 independent runs; a third proves a dimensionally
broken variant is rejected before any execution (`UNIT-E104`), with no
artifact written.

**Demo-shortcut check (independently re-verified this audit, not just
cited from the report):**
```
git diff --stat cf21946..7c7bc6b -- crates/
 crates/cad-cli/tests/stage2_end_to_end.rs | 292 ++++++++++++++++++++++++++++++
 crates/cad-parser/tests/shared_corpus.rs  |  61 +++++++
 2 files changed, 353 insertions(+)
```
`AICAD-062`+`AICAD-063` combined touched only test files — zero production
crate source changed to make the proof pass. A workspace-wide grep for
`063|demo|hardcode|special.?case|hack` across `crates/*/src` turned up only
comments *documenting the absence* of such shortcuts (e.g.
`crates/cad-cli/src/lib.rs`: "a full bracket-shaped fixture... it is given,
not demonstrate that fixture itself"), never an actual special case. No
special AST node, hidden host callback, hard-coded bracket generator, or
interpreter shortcut exists.

### 2.2 Kernel/Geometry-IR boundary (AGENTS.md non-negotiable: "Public
language/API must not expose OCCT-specific classes")

Independently re-checked this audit via dependency-graph and source grep,
not cited from reports:
- `crates/cad-kernel-api/Cargo.toml` has **zero** dependencies — the
  kernel-neutral vocabulary crate cannot physically reference OCCT types.
- `crates/cad-geometry-api` (Geometry IR) depends only on
  `cad-ast`/`cad-diagnostics`/`cad-kernel-api`/`cad-types`/`cad-units` —
  never `cad-occt-bridge`.
- `crates/cad-hir` depends only on `cad-ast`/`cad-diagnostics`/
  `cad-parser`/`cad-types`/`cad-units` — no kernel/geometry dependency at
  all; HIR does not know geometry or kernel types exist.
- `crates/cad-runtime` depends on `cad-geometry-api`/`cad-kernel-api` but
  not `cad-occt-bridge` directly — kernel dispatch happens one layer
  further out (`cad-geometry-runtime`), keeping `cad-runtime` itself
  backend-agnostic.
- Only `crates/cad-cli` (the top-level composition point) and
  `crates/cad-geometry-runtime`/`cad-occt-bridge` depend on OCCT.
- A grep for `occt|opencascade|TopoDS|BRep|gp_Pnt` across
  `cad-geometry-api/src`, `cad-hir/src`, `cad-kernel-api/src`,
  `cad-ast/src`, `cad-types/src` matches only doc comments that name OCCT
  functions/types for *design-rationale* traceability (e.g. "no OCCT type
  ... appears anywhere in this file") — never an actual OCCT type,
  dependency, or enum value used in code.

### 2.3 Typed units (non-negotiable: "Units are typed engineering
quantities, not untyped floats")

`crates/cad-units::dimension_vector::DimensionVector` (structural
dimension-exponent vector) and `crates/cad-units::arithmetic` (typed
arithmetic with `DimensionalArithmeticError` for illegal combinations, e.g.
`mm + s`) back every quantity the type checker and runtime carry;
`AICAD-046`/`AICAD-047`/`AICAD-048`/`AICAD-049`'s reports and
`gates/STAGE2-B_TYPES_HIR.md` §3-6 already exercised `mm+in`,
`mm+s`-rejection, derived-dimension multiplication/division, and affine
absolute/delta temperature rules; this audit did not re-derive those (the
B-checkpoint's own re-verification stands), only confirmed the underlying
types still exist unchanged in current source.

### 2.4 Checkpoint gates already on this branch

| Gate | Verdict | Batches covered |
|---|---|---|
| `STAGE2-A_FRONTEND.md` | PASS | S2-01..S2-03 (`AICAD-038`-`045`) |
| `STAGE2-B_TYPES_HIR.md` | PASS | S2-04..S2-07 (`AICAD-046`-`053`) |
| `STAGE2-C_EXECUTION.md` | PASS | S2-08..S2-09 (`AICAD-054`-`058`) |

All three explicitly re-verified their own claims against current source
at their own HEAD rather than only citing task reports, and all three
explicitly note (per `AGENTS.md`) that they recommend, not approve. No
checkpoint required weakening a test or gate to pass.

## 3. Test/benchmark commands and results (re-run this audit, not cited)

```
cargo fmt --all -- --check
```
Clean, zero diffs.

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
Clean, zero warnings, all 27 crates.

```
cargo test --workspace
```
0 failures across the entire workspace. Per-crate unit/integration test
counts (unchanged from `SESSION_HANDOFF.md`'s own record, reconfirmed by
this run): `cad-ast` 7+19, `cad-lexer` 29, `cad-diagnostics` 15,
`cad-kernel-api` 23, `cad-types` 14, `cad-units` 75, `cad-parser` 119
(includes `shared_corpus.rs`'s tree-sitter/production-parser corpus
cross-check), `cad-hir` 197, `cad-compiler` 20+10 (loader+binder),
`cad-runtime` 89, `cad-occt-bridge` 84 (includes the full native/OCCT
Stage-1 regression suite), `cad-geometry-api` 17, `cad-geometry-runtime`
9+6 (includes the 7.32s determinism/dispatch suite), `cad-cli` 49+3
(`stage2_end_to_end.rs`). All 12 remaining crates (`cad-agent-tools`,
`cad-assemblies`, `cad-configurations`, `cad-constraints`,
`cad-feature-graph`, `cad-interchange`, `cad-lsp`, `cad-packages`,
`cad-provenance`, `cad-query`, `cad-references`, `cad-requirements`) are
still the unmodified `AICAD-002` placeholder stubs (0 tests, 6-line
`lib.rs` each) — confirmed by direct line-count inspection this audit, not
scope creep (see §4).

```
cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1
```
3/3 passing, re-run serially this audit for a clean determinism trace (3
independent STEP exports, `774` entities each run).

## 4. Scope-creep / speculative-work audit

`AGENTS.md` "No speculative future work" and `CURRENT_STAGE.md` "Not
allowed yet" were checked against every crate directory, not just the
fixed-batch task list:
- The 12 stub crates above (plus `cad-provenance`/`cad-references`/etc.)
  are exactly the `AICAD-002` monorepo skeleton (commit `b46303a`,
  predates Stage 2 entirely) — placeholder namespacing for future stages,
  not implemented Stage-3+ features. `git log --follow` on each confirms
  no commit since `b46303a` touched them.
- `crates/cad-compiler` (1549 lines: `loader.rs` 562, `binder.rs` 969) is
  legitimately `AICAD-044` (module/import loader) and `AICAD-050` (name
  binding) — both in-scope Stage-2 tasks, organized as a "thin composition
  layer" crate per its own module doc comment rather than embedded in
  `cad-parser`/`cad-hir`, not new unscoped functionality.
- No crate implements sketch/constraint solving, a semantic-reference
  layer, assemblies, or a general AI/agent tool layer — `cad-constraints`,
  `cad-references`, `cad-assemblies`, `cad-agent-tools` are all still
  6-line stubs, consistent with `CURRENT_STAGE.md`'s explicit "Not allowed
  yet" list.
- The Tree-sitter grammar (`AICAD-062`) implements exactly
  `crates/cad-parser::Parser::parse_item`'s current subset (not the full
  aspirational `specs/language/grammar.ebnf`), per its own report's
  documented scope decision — it does not become a second independent
  language specification.

## 5. Known limitations / open items

- **D19 (new, this audit)**: `DECISION_LOG.md#DL-12` assigned Stage 2 to
  derive and document the D5 comparison profile's concrete v1 numeric
  tolerance constants and named `AICAD-064` as the re-audit point; no
  Stage-2 task actually did this (`crates/cad-validation` is still an
  unmodified stub). This audit produced the measurements and an
  evidence-supported partial recommendation (`linear_abs = 1e-4`,
  `volume_rel = 1e-3`, both directly evidenced from
  `project/reports/AICAD-034.md`) and explicitly flagged `linear_rel`,
  `area_abs`/`area_rel`, and `volume_abs` as *not* evidence-supported,
  escalating them in `project/OWNER_DECISIONS.md#D19` rather than
  guessing, per `AGENTS.md`. **Non-blocking for Stage-2 exit** (no
  `AICAD-038`..`063` acceptance criterion required these constants), but
  `DL-12` ties it to "before the Stage 8/13 determinism benchmarks
  mature" — recommend ruling before any task first implements
  `crates/cad-validation`.
- **Pre-existing open owner decisions carried forward unchanged**: `D3`
  (sketch entity model), `D10` (diagnostic-schema stability policy), `D11`
  (constraint IR/solver independence), `D12` (trusted native plugin
  boundary), `D15` (package plugin runtime) — all explicitly Stage-3+
  scope per `CURRENT_STAGE.md`'s "Not allowed yet" list; none block Stage
  2's own exit gate.
- **Stage-2 `transform` is translate-only** (`AICAD-063`'s report,
  documented in both the fixture's header comment and that report): the
  proof fixture is a flat plate + boss rather than an L-bracket for this
  reason, since rotation has no unambiguous source-level syntax yet. Not
  a defect — a disclosed, deliberate scope boundary consistent with "no
  speculative future work."
- No new regressions found by this audit; no existing test/gate was
  weakened to produce this recommendation.

## 6. Representative artifacts

- `examples/brackets/stage2_mounting_plate.aicad` (the end-to-end proof
  source).
- `/tmp/aicad_063_determinism_{0,1,2}.step` (this audit's own re-run of
  the determinism test; ephemeral, not committed, matching
  `AICAD-063`'s own original evidence of 774 STEP entities per run).
- `docs/API/safe-cad-api.md` (`DL-15`'s required Safe CAD source API
  specification).

## 7. Recommendation

**PASS WITH CONDITIONS.**

Every checklist item in `CURRENT_STAGE.md`'s Stage-2 exit gate is met with
direct, checkable, independently re-verified evidence (§2): the complete
`.aicad` source -> lexer/parser -> binding -> units/type checking -> typed
HIR -> execution -> Geometry IR -> Stage-1 kernel API -> exact bracket ->
valid STEP pipeline is proven end-to-end with no demo-specific interpreter
shortcut (re-confirmed by this audit's own diff/grep, not only cited);
`cargo fmt`/`clippy`/the full workspace test suite are all clean with zero
failures; the kernel/Geometry-IR boundary is structurally clean by
dependency graph, not just by convention; typed units and the fixed
S2-01..S2-13 batch order were followed without reordering, combining, or
skipping; no scope creep into Stage-3+ territory was found anywhere in the
27-crate workspace; and all three prior checkpoint gates (A/B/C)
independently PASSed with no weakened test or gate.

The **one condition**: `DECISION_LOG.md#DL-12` explicitly assigned Stage 2
to derive the D5 comparison-profile's concrete v1 tolerance constants and
named this exact task (`AICAD-064`) as the re-audit point, and that
derivation was never done by any batch task. This audit closes the gap by
producing the measurements and a partial, evidence-supported
recommendation, escalating the unsupported remainder as
`OWNER_DECISIONS.md#D19` rather than guessing. This does not block Stage-2
exit (no exit-gate criterion required it), but the owner should rule on
`D19` before any future task first implements `crates/cad-validation`, per
`DL-12`'s own "before the Stage 8/13 determinism benchmarks mature"
framing.

**This recommendation is not an approval.** Per `AGENTS.md` and
`CURRENT_STAGE.md` ("Owner approval required to advance: Yes"), Stage 3
work (`AICAD-065` onward) must not begin until the owner records a Stage-2
pass decision in `project/DECISION_LOG.md`, following the same pattern as
`DL-10`/`DL-11`. Per the active scheduled-task brief, future invocations on
this branch switch into Stage-2 hardening mode (rerunning full compiler/
runtime tests, expanding parser/typechecker adversarial cases, running
determinism comparisons, fuzzing parser/lexer inputs under bounds, auditing
HIR invariants and Geometry-IR backend neutrality, reproducing the final
bracket, rerunning STEP verification, minimizing regressions, fixing clear
internal defects) rather than beginning `AICAD-065` or any Stage-3 scope,
until that owner decision is recorded.
