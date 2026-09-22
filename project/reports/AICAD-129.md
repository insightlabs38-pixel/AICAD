# AICAD-129: prove core-vs-library boundary, learnability/inspectability, and maintained examples

## Status

Done. Third and final task of batch `S5-09`.

## Objective

Per this task's own acceptance list: audit that Stage-5 geometry
capabilities use ordinary language/`RuntimeBuiltin`/library mechanisms
(no compiler intrinsic, open native-registration boundary, or
OCCT-specific public API); prove a representative program is inspectable
`source -> typed call/value -> feature/provenance/dependency -> Geometry
IR/query -> kernel result/reference evidence`; classify/refresh ACTIVE
examples; run a bounded inspectability/tool-client fixture.

## Base / resulting commit

Base: `d388b58` (`origin/claude/aicad-stage5-dev`, `AICAD-128`).

## Core-vs-library boundary audit

- **No open registration**: `grep`ed `cad-hir`/`cad-runtime` for any
  `register_builtin`/`extern "C"`/`dyn Fn(...) -> Value` pattern — none
  found. `cad_hir::builtins::catalogue()` remains the sole, closed,
  compile-time `BuiltinFnId` table every Stage-5 curve/surface/topology/
  raw capability was added to (`AICAD-105`-`126`'s own established
  convention), matching D21.
- **No kernel-type leakage above the sanctioned boundary**: `grep`ed
  every kernel-neutral crate (`cad-ast`, `cad-hir`, `cad-geometry-api`,
  `cad-feature-graph`, `cad-types`) for `cad_occt_bridge`/`OcctContext`
  references. Every hit in `cad-hir`/`cad-geometry-api` is prose in a doc
  comment (describing what a `GeometryOp` variant maps to at the kernel
  level), never a `use` import or a type in a public signature — the
  public language/HIR/Geometry-IR layers remain kernel-neutral.
  `cad-query` (`health.rs`/`feature_lineage.rs`/`resolve.rs`) does
  import/use `cad_occt_bridge::{OcctContext, Shape}` directly in real
  (non-test) code — this is the *already-sanctioned* boundary
  (`AICAD-126`'s own report: "no OCCT-specific type crosses into
  `cad-query`'s or `cad-cli`'s own public surface beyond the existing,
  already-sanctioned `cad_occt_bridge::Shape`/`OcctContext` handles those
  crates already depended on before this batch"), not a new leak — the
  resolver's own kernel-backed evaluation layer (D23) necessarily needs
  real kernel handles to do its job, and no `.aicad`-facing type/syntax
  is affected.
- **No compiler intrinsic**: every Stage-5 curve/surface/topology/raw
  capability is an ordinary `RuntimeBuiltin` dispatched through
  `Interpreter::call`, exactly like every Stage-2 primitive — no new AST/
  HIR node kind, special-cased grammar production, or parser-level
  geometry syntax was added for any of `AICAD-109`-`126`.

## Inspectability chain

Proven by `crates/cad-cli/tests/stage5_inspectability_fixture.rs` (4
tests, all passing) through two distinct, precisely-disclosed mechanisms:

- **Layers 1 (typed call/value) and 4 (reference evidence)**: the real
  stable **JSON text contract** `cad build --json`/`cad refs check
  --json` print (`docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §2-3) —
  `BuildReport`/`RefsCheckReport::to_json()`, serialized to text and
  re-parsed with `cad_diagnostics::json::Json::parse`, walked with only
  `Json`'s own generic `get`/`as_array`/`as_str` accessors (never a
  private `Diagnostic`/`BuildReport` field). A clean build reports
  `status: "ok"`, zero diagnostics; a deliberately type-broken variant
  reports `status: "failed"` with a real diagnostic carrying stable
  `code`/`severity`/`title`/`message` string fields. `cad refs check
  --json`'s own `health` field reports real `ambiguous`/`broken` counts
  for the two ACTIVE fail-closed reference examples.
- **Layers 2 (feature/provenance/dependency) and 3 (Geometry IR/query/
  kernel result)**: this project's *public Rust library* API
  (`cad_feature_graph::FeatureGraph`, `cad_cli::ParametricBuildSession`)
  — today's real inspection mechanism for this layer.
  `docs/plan/17`'s richer `cad inspect`/`cad explain`/`cad why` commands
  remain unimplemented placeholders (`crates/cad-cli/src/cli.rs`'s own
  `Command` enum has only `Build`/`RefsCheck`) — correctly out of this
  task's own scope per `AGENTS.md`'s "no speculative future work," not a
  gap this task should close. `FeatureGraph::build`'s own typed
  `FeatureNode`s (`op`/`name`/`geometry_inputs`/`parameters`) and
  `ParametricBuildSession::shape_for_binding`'s own real kernel `Shape`
  (measured area matching the exact closed form) are stable, documented,
  crate-public types — a genuine structured surface, just not yet a wire
  protocol.

## Discovered finding: `List<Geometry>` is invisible to `geometry_inputs`

Building the Layer-2 fixture found that `is_geometry_type`/
`is_geometry_type_ref` (both `cad_feature_graph::graph` and
`cad_runtime::interp`) only recognize a bare `Geometry`-typed parameter,
not `List<Geometry>` — so `make_wire([bottom, right, top, left])`'s own
`square` node reports an **empty** `geometry_inputs`, even though
`square_face = make_face(square)` (a single-`Geometry` parameter)
correctly reports `geometry_inputs = [square.id]`. Recorded precisely in
`project/OWNER_DECISIONS.md` (non-decision item, not an architecture
ruling): this confirms only an introspection-API-level fact — it does
**not** establish whether `TraceFeatureGraph::dirty_set`'s own real
incremental-rebuild correctness is affected, since that also consults a
separate `binding_refs`/`provenance_of` resolution path this fixture does
not exercise. A future task should add the specific incremental-rebuild
regression (or extend the two `is_geometry_type*` checks to recognize
`List<Geometry>`) — not assumed fixed or assumed broken here.

## ACTIVE examples classification and coverage

`examples/README.md` now carries an explicit **Class** column
(`teaching`/`realistic`) for all 14 ACTIVE examples, and a new "Stage-5
checkpoint coverage" section recording: Checkpoint A (curves) covered by
`curves/circle_curve_basics.aicad` (teaching) +
`curves/cable_routing_path.aicad` (realistic); Checkpoint B (surfaces/
queries) by `surfaces/surface_query_basics.aicad` (teaching) +
`surfaces/pipe_clearance_check.aicad` (realistic); Checkpoint C (ordinary
topology half) by `topology/topology_construction_basics.aicad`
(teaching). No raw/lineage-tier `.aicad` example exists, by
already-established, disclosed precedent (`AICAD-119`-`126`'s own
reports: raw/query-category builtins need a real live `OcctContext`,
which the structural-only `active_examples.rs` harness does not
configure — proven instead by dedicated `stage5_raw_*.rs`/
`stage5_lineage_checkpoint.rs` files, run automatically by `cargo test`).
`AICAD-127`'s difficult-freeform corpus correctly stays out of
`examples/` per this file's own "stress/benchmark fixtures stay out of
the primary learning path" policy — noted explicitly in the README rather
than left implicit. No stale/archived example needed reclassification;
no public syntax changed this task, so no example needed updating.

## No architecture boundary bypassed

This task added no new `RuntimeBuiltin`, type, or kernel-adapter surface
— it audits and documents the existing one. No example's own syntax or
semantics changed.

## Verification

```
cargo fmt --all -- --check                                              # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings    # clean
cargo test -p cad-cli --test stage5_inspectability_fixture              # 4/4 passed
cargo test -p cad-cli --test active_examples                            # 3/3 (14 ACTIVE examples)
cargo test --workspace                                                  # 1830 passed, 0 failed
```

(1826 after `AICAD-128` + 4 new inspectability-fixture tests.)

## Limitations (honest, not hidden)

1. The `List<Geometry>`/`geometry_inputs` finding above is disclosed but
   not resolved — its real-world dirty-propagation impact (if any) is an
   open question for a future task, not silently assumed either way.
2. Layers 2/3's inspection mechanism is the Rust library API, not yet a
   JSON wire contract — `docs/plan/17`'s fuller `cad inspect`/`explain`/
   `why` vision remains unimplemented; correctly out of this task's own
   "no speculative future work" scope, but worth knowing for anyone
   expecting a CLI-only inspection story today.
3. The core-vs-library boundary audit is a targeted grep-based check
   (import sites, registration patterns, doc-comment-only vs. real usage)
   across the relevant crates, not an exhaustive line-by-line review of
   every Stage-5 diff — proportionate to this task's own scope, consistent
   with every individual `AICAD-109`-`126` task's own repeated
   self-reported "no architecture boundary bypassed" claims, now
   independently spot-checked at the batch level rather than only trusted
   per-task.

## Next dependency

`AICAD-130` (final Stage-5 owner gate packet) depends on `AICAD-129`
(satisfied) and every prior Stage-5 task.
