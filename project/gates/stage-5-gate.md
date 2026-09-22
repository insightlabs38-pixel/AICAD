# Stage-5 Owner Gate Packet

Prepared by `AICAD-130`, the sole task of Batch `S5-10`, per `project/
gates/README.md` and `AGENTS.md` ("Stage gates": "The agent may prepare
gate evidence and recommend pass/do-not-pass. The agent may not approve a
roadmap stage.") and this task's own `project/TASKS.yaml` acceptance
criteria (independent re-audit of every `AICAD-101..129` acceptance item;
proof of `D21`-`D25`(-`D27`) compatibility; realistic/adversarial evidence;
known limitations; exactly one advisory recommendation; no Stage-6
approval/promotion/ID-assignment/implementation).

**This packet recommends; it does not approve.** Nothing here treats
Stage 5 as passed until the owner records that decision in `project/
DECISION_LOG.md`, following the `DL-10`/`DL-11`/`DL-16`/`DL-34` pattern.
Per the campaign brief's final stop rule, roadmap development STOPS after
this packet lands: Stage 6 (global IDs, implementation, or provisional-
queue activation) is forbidden until a separate, later, explicit owner
approval exists.

This audit re-runs the full workspace verification suite, the native OCCT
CTest suite, the semantic-reference harness, and the Stage-5
corpus/adversarial/example test binaries directly against current source
at this exact HEAD, rather than only citing prior batch reports' own
claims.

## 1. Exact git commit/revision

```text
1052a5e9d1b6a8b3397add649799dccace40e102 (AICAD-129, origin/claude/aicad-stage5-dev HEAD at the start of this packet)
```

Branch `claude/aicad-stage5-dev`, created from the owner-approved merged
head `df9334634e09a9793191ea5de40f0aad12032589` (`DL-35`). Working tree
was clean before this invocation's own gate-packet edit. Batches `S5-00`
through `S5-09` (`AICAD-101` through `AICAD-129`, including the
owner-requested `AICAD-104A` remediation) are all on this commit already;
this packet adds no roadmap feature code, only this gate file, `project/
reports/AICAD-130.md`, and routine `project/TASKS.yaml`/`CURRENT_STAGE.md`/
`SESSION_HANDOFF.md` state updates. 34 commits, 132 files, +35916/-3518
lines since the branch point (`git diff --stat
df933463..HEAD`) — a touched-directory audit (§8) confirms this stays
inside the expected geometry/runtime/query/feature-graph/units/kernel-api/
occt-bridge/references/cli crates plus `project/`/`examples`/`native`; no
assemblies/configurations/requirements/packages/lsp/artifact/interchange/
provenance/diagnostics/constraints crate was touched, consistent with the
brief's "no speculative future work" rule.

## 2. Stage-5 batch-by-batch summary (independently re-confirmed this audit)

| Batch | Tasks | Result |
|---|---|---|
| S5-00 | AICAD-101..104, 104A | done — nested `part`-in-`part` now recurses to unbounded depth (`DL-36`); `Area` unit-literal spellings added; remaining Stage-4 query vocabulary (`nearest_to`/`farthest_from`/etc.) lowered to real `.aicad` syntax; production-path query regressions added; part-body `param`s wired into `ParamModel` (`DL-37`, owner-requested) |
| S5-01 | AICAD-105..106 | done — `RuntimeBuiltin` catalogue scaled per `D21`/`DL-23`; kernel-backed `is_valid`/`volume`/`area` queries added under `D23`/`DL-25`; `ConstructionTolerance`/`ApproximationTolerance` domains added under `D24`/`DL-26` |
| S5-02 | AICAD-107..108 | done — feature identity/dependency/provenance preserved through user functions/branches/loops (`D25`/`DL-27`); kernel-neutral `Curve`/`Surface`/query-result value families added |
| S5-03 | AICAD-109..112 | done — analytic + Bezier/B-spline/NURBS curves, curve operations; **Checkpoint A: PASS** (§below) |
| S5-04 | AICAD-113..116 | done — analytic + Bezier/B-spline/NURBS surfaces, trimmed-surface model, evaluation/derivatives/offsets |
| S5-05 | AICAD-117..118 | done — multi-solution intersection/projection/distance queries; **Checkpoint B: PASS** |
| S5-06 | AICAD-119..121 | done — general topology construction + exact validity evidence, sewing/healing, deterministic traversal/inspection |
| S5-07 | AICAD-122..124 | done — epoch-bound raw/unsafe geometry tier, functional raw editing with lineage evidence, explicit raw-to-safe adoption |
| S5-08 | AICAD-125..126 | done — lineage/reference integrity integrated across Stage-5 topology changes; **Checkpoint C: PASS** |
| S5-09 | AICAD-127..129 | done — frozen difficult freeform corpus (7/7), 8-case adversarial campaign (8/8, no reproducible defect), core-vs-library boundary + inspectability audit (4/4) |
| S5-10 | AICAD-130 | **this packet** |

Each batch's own task reports (`project/reports/AICAD-101.md`..`AICAD-
129.md`) record `Status: Done` with their own acceptance-criterion
evidence; this audit re-ran the aggregate test/CI suite (§4) rather than
trusting those claims alone, and directly re-read the three checkpoint
reports (`AICAD-112`/`118`/`126`) and every batch's disclosed limitations
section (§5) rather than only their status lines.

## 3. Invariant compatibility (re-verified this audit)

### 3.1 D21 — closed first-party `RuntimeBuiltin` catalogue (`DL-23`)

No open registration, native callback, or plugin-injected builtin/type
exists anywhere in the Stage-5 diff — confirmed by grep across
`crates/cad-hir`, `crates/cad-runtime`, `crates/cad-geometry-runtime` for
`NativeFn|host_fn|plugin.*register|extern.*callback` (zero structural
matches; `AICAD-129`'s own equivalent audit is the origin of this check,
independently repeated here). The catalogue grew through Batches
`S5-01`/`03`/`04`/`05` entirely via ordinary closed rows — new
`BuiltinCategory::Value` entries for curves/surfaces, new `Query` entries
for kernel-backed operations — never a new binding kind beyond the
`D18`/`DL-15` mechanism Stage 2 already froze.

### 3.2 D22 — safe/raw/reference separation (`DL-24`)

`Value::Raw` (epoch-bound, `cad-runtime`-owned) never itself becomes a
`FeatureAnchor`-addressed reference; only an explicitly `adopt`ed `GeomId`
does (`AICAD-122`-`124`). Raw handles remain opaque at the `.aicad`
source/HIR boundary and are rejected across a build-context/epoch
boundary — `AICAD-122`-`124`'s stale-handle/wrong-context/dropped-owner
regressions are unchanged and still pass (§4). Raw-to-safe adoption is
explicit and evidence-producing (`AdoptionEvidence`, `AICAD-124`); no code
path silently reinterprets a raw object as validated safe geometry.

### 3.3 D23 — kernel-backed queries during ordinary evaluation (`DL-25`)

`is_valid`/`volume`/`area` (Stage 5's own kernel-backed queries) remain
demand-materialized through `KernelQueryExecutor` with query-budget
accounting (`AICAD-105`); results are ordinary AICAD values participating
in normal dependency/provenance behavior. Curve/surface construction and
evaluation are deliberately **not** D23 queries (`BuiltinCategory::Value`,
closed-form math, no kernel call) — `AICAD-112`'s own
`curve_construction_and_evaluation_never_touch_the_kernel` and `AICAD-118`'s
`surface_surface_intersection_and_projection_never_touch_the_kernel`
production-path tests are direct, re-run evidence (§4) this boundary is
real, not merely documented.

### 3.4 D24 — distinct tolerance domains (`DL-26`)

`ConstructionTolerance` (domain 2) and `ApproximationTolerance` (domain 3)
remain compile-time-distinct (`cad-units::tolerance`, no `From`/`Into`
between them). `AICAD-111`'s `interpolate_curve` is the first curve
consumer of `ApproximationTolerance`; `AICAD-115`'s trim-loop membership
test reuses `ConstructionTolerance`'s canonical magnitude as a
dimensionless parameter-space epsilon (disclosed, not a domain collapse —
Stage 5 has no separate parameter-space domain and none of the active
tasks required adding one). No global epsilon was introduced anywhere in
the Stage-5 diff.

### 3.5 D25 — feature identity/provenance through abstraction (`DL-27`)

`AICAD-107` extended dirty-propagation/provenance capture through user
functions, part/helpers, and supported control flow; `AICAD-112`'s
`curve_construction_stays_invisible_to_the_feature_trace_alongside_real_
geometry` re-confirms the generic `is_geometry_type_ref` gate applies to
curve values without curve-specific logic. **Disclosed gap (not a
regression):** `is_geometry_type`/`is_geometry_type_ref` does not
recognize `List<Geometry>` parameters (`make_wire`/`make_shell`/`compound`/
`sew`), so those calls' list *elements* are invisible to `geometry_inputs`
in both `FeatureGraph` and `TraceFeatureGraph` — found by `AICAD-129`'s own
inspectability fixture, recorded in `project/OWNER_DECISIONS.md`'s
"Non-decision items" section. Each element's reference is still captured
via the separate `parameters`/`binding_refs`/`provenance_of` path; whether
`TraceFeatureGraph::dirty_set`'s real incremental-rebuild behavior is
affected for this case is **not yet verified either way** — an explicit
open item, not assumed safe or unsafe (§6).

### 3.6 D6 — kernel-neutral public semantics (`DL-5`)

No `cad_occt_bridge`-specific type crosses into `cad-hir`/`cad-runtime`/
`cad-query`/`cad-cli` public surfaces beyond the already-sanctioned
`cad_occt_bridge::Shape`/`OcctContext` handles those crates depended on
before Stage 5 (re-confirmed by `AICAD-126`'s own grep, repeated by this
audit: zero new leaked-type matches). `Curve`/`Surface` remain two single
opaque nominal types; every new Stage-5 struct type (`RawLineageChain`,
`ClassifiedShape`, `GeomId`, `OperationReport`, ...) is
`cad_kernel_api`/`cad_geometry_api`-owned.

### 3.7 D7 — fail-closed semantic references (`DL-8`)

`cad_query::resolve`'s `Resolved`/`Ambiguous`/`Broken` contract and its
unconditional `GeometricFingerprint -> Broken` mapping are unchanged by
Stage 5 (re-read directly this audit, not cited from a prior report).
`semantic_ref_harness.py validate`/`self-test` both report `ok` at this
HEAD (§4) — the frozen 10-case corpus (7 public / 3 held out) and its
silent-wrong self-test gate are exercised and pass, unchanged in policy.
No new `SILENT_WRONG` outcome exists anywhere in the Stage-5 diff.
`AICAD-125`'s own report records the one real bug this stage *did* find
and fix pre-merge (a cross-dispatch identity mismatch that would have
produced spuriously "generated" raw-adoption evidence) — caught by direct
experiment during `S5-08`, not shipped and later regressed. **Zero
surviving silent-wrong regressions.** Automatic fingerprint recovery
remains disabled (`D7` still only PARTIALLY RESOLVED — the "whether/when
to enable it later" sub-question remains explicitly open, unchanged and
untouched by Stage 5).

### 3.8 Lineage for Stage-5 topology-changing operations (`D25`/`AICAD-125`)

Sewing, healing, raw editing (`remove_face`/`replace_face`/`merge_faces`/
`split_edge`), and raw-to-safe adoption all emit resolver-consumable
lineage through the same `TraceFeatureGraph`/`capture_named_feature_
lineage` path ordinary construction ops already use (`AICAD-125`).
`AICAD-126`'s integrated vertical-slice test (construct → inspect →
construct → sew → heal → inspect → raw-edit → adopt → resolve) is re-run
this audit (§4, `stage5_lineage_checkpoint`, 1/1) and still passes.
**Disclosed, pre-existing gaps** (unchanged since `AICAD-123`/`125`, not
new this packet): `replace_face`'s `Modified` evidence has no known
constructible *valid* `adopt`-through fixture (every geometrically
compatible replacement tried fails kernel validity — a geometric-
compatibility gap in `replace_face` itself, not in the lineage-capture
code, which is proven correct independently via `Sew`'s own relabeled-edge
path exercising the identical consumer code); per-entity `Heal` lineage
remains unavailable; `merge_faces`/`split_edge`'s `List<Raw>` results
cannot be fed into `adopt` from `.aicad` source (no list-element-selection
syntax exists).

## 4. Test/CI status (re-run this audit)

```
$ cargo fmt --all -- --check
(clean)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.86s
(zero warnings)

$ cargo test --workspace
1830 passed; 0 failed; 0 ignored — 95 test binaries (unit + integration + doc tests)

$ python3 scripts/ci/semantic_ref_harness.py validate
{"cases": 10, "splits": {"held_out": 3, "public": 7}, "status": "ok"}

$ python3 scripts/ci/semantic_ref_harness.py self-test
{"status": "ok", "self_test": "silent-wrong gate exercised"}

$ cmake -S native/occt_bridge -B native/occt_bridge/build -DCMAKE_BUILD_TYPE=RelWithDebInfo
$ cmake --build native/occt_bridge/build -j$(nproc)
$ ctest --test-dir native/occt_bridge/build --output-on-failure
100% tests passed, 0 tests failed out of 18 (occt_probe, abi_boundary, lifecycle,
box_cylinder, transform, curve_edge_wire, face, extrude_revolve, sweep_loft,
boolean, fillet_chamfer, shell_offset, topology, measurement, validation,
tessellation, step_export, step_import)
```

Stage-5 corpus/adversarial/example binaries (all included in, and re-run
as part of, the `cargo test --workspace` run above — individually
extracted from that run's own output, not run separately/differently):

```
tests/active_examples.rs ................ 3 passed  (14 ACTIVE examples + 3 reference-health fixtures)
tests/stage5_adversarial_campaign.rs ..... 8 passed  (near-degenerate/multi-scale, tangent spheres, periodic
                                                       wraparound, sew tolerance boundary, self-intersecting
                                                       B-spline closest-point, 64-edge polygon, deterministic
                                                       repeat-build, malformed knot vector — no reproducible defect)
tests/stage5_freeform_corpus.rs .......... 7 passed  (4 public + 3 held-out difficult freeform fixtures)
tests/stage5_curves_checkpoint.rs ........ 2 passed
tests/stage5_queries_checkpoint.rs ....... 3 passed
tests/stage5_lineage_checkpoint.rs ....... 1 passed
tests/stage5_inspectability_fixture.rs ... 4 passed
tests/stage5_kernel_backed_queries.rs, stage5_query_production_path.rs,
tests/stage5_nested_part_references.rs, stage5_part_scoped_param_rebuild.rs,
tests/stage5_raw_adoption.rs, stage5_raw_editing.rs, stage5_raw_geometry.rs,
tests/stage5_raw_lineage.rs, stage5_sewing_healing.rs,
tests/stage5_topology_construction.rs, stage5_topology_inspection.rs ..... all passed, 0 failed
```

`stage4_resolver_execution.rs` (Stage-4 production-path regression corpus,
11 tests) and `cad-query`'s own unit suite (103 tests, grown from
`AICAD-118`'s 98 as Stage-5 query/lineage work added coverage) both pass
unchanged, re-confirmed in the same `cargo test --workspace` run — no
Stage-4 regression anywhere in Stage 5.

`grep -rl "#\[ignore\]"` across every `crates/*/tests/*.rs`/`crates/*/src/
*.rs` in the workspace returns zero matches — no test anywhere is ignored,
skipped, disabled, or weakened to reach this packet's conclusions.

## 5. Known limitations / open items (compiled, not hidden)

None of the below represents a hidden regression, a weakened gate, or a
silent-wrong result — each is an honestly disclosed, pre-existing scope
boundary from its own originating task report, compiled here for a single
independently-reviewable view per this packet's own acceptance criterion.

**Structural (most significant):**

1. **`make_edge`/`make_face_on_surface` only bridge the original Stage-2
   analytic curve/surface families into real kernel B-rep topology.** A
   Bezier/B-spline curve (`AICAD-110`), Bezier/B-spline surface
   (`AICAD-114`), or any `trim_surface` result (`AICAD-115`) is rejected
   with an explicit `UNSUPPORTED_TOPOLOGY_CONSTRUCTION` diagnostic —
   never silently. `AICAD-110`/`114`/`115` gave `.aicad` real freeform
   curve/surface **values** (construction/evaluation/trim/intersection/
   projection/distance, all kernel-free); `AICAD-119`'s topology dispatch
   was never extended to accept them. Found building `AICAD-127`'s
   freeform corpus; recorded in `project/OWNER_DECISIONS.md`. Closing
   this is ordinary future implementation work (wiring `curve_to_edge_op`/
   `surface_to_surface_spec` to OCCT's matching `Geom_BSplineCurve`/
   `Geom_BSplineSurface`/pcurve-trim constructors), not an architecture
   decision.
2. **`List<Geometry>` builtin parameters are invisible to `geometry_inputs`**
   in both `FeatureGraph` and `TraceFeatureGraph` (§3.5). Whether real
   incremental-rebuild dirty-propagation is affected is unverified either
   way — an explicit follow-up (a `make_wire`/`make_shell`/`compound`/
   `sew` dirty-and-rebuild regression, or extending `is_geometry_type`/
   `is_geometry_type_ref`) is recorded, not assumed.
3. **`intersect_surfaces` is exact only for `Plane`-`Plane`/`Plane`-
   `Sphere`/`Sphere`-`Sphere`**; every other pair (notably the common
   `Cylinder`-`Cylinder`/`Plane`-`Cylinder`) is `Unsupported` — a
   structural gap (no space-curve value type exists yet to hold a general
   quadric/quadric intersection), not a narrowed effort scope
   (`AICAD-117`).
4. **`replace_face`'s `Modified` lineage evidence has no known
   constructible valid `adopt`-through fixture** (§3.8) — a geometric-
   compatibility gap in `AICAD-123`'s `replace_face`, substituted with an
   equivalent `Sew`-based production-path proof exercising the same
   consumer code.

**Narrower, individually disclosed in each task's own report (not
repeated in full here):** Arc construction rejects wraparound-through-zero
(`AICAD-109`); periodic/closed B-splines are rejected, not implemented
(`AICAD-110`/`114`); `offset_curve` is exact only for Line/Circle/Arc
(`AICAD-111`); composite multi-segment trim loops are unsupported
(`AICAD-115`); `Line`-vs-`Cone`/`Torus`/`Bezier`/`BSpline` distance and any
`Trimmed`-surface query are `Unsupported` (`AICAD-117`); `make_shell`
cannot sew coincident-but-distinct (non-shared-handle) topology and
`make_face_on_surface` can silently produce a degenerate trim region on a
3D-coincident-but-unparametrized wire (`AICAD-119`, both disclosed, not
crashing/silent-wrong); `sew`/`heal`'s richer `SewReport`/`HealReport`
detail, `AdoptionEvidence`'s `checks_performed`, and `OperationReport`'s
`created`/`split`/`merged` breakdowns all remain Rust-only, not
source-exposed (`AICAD-120`/`124`/`125`, consistent precedent); per-entity
`Heal` lineage is unavailable (`AICAD-120`); `topology_wire_at` is not
source-exposed and `is_forward_oriented` collapses 4 orientation values to
a bool (`AICAD-121`); raw handles wrap a whole classified shape only, no
per-entity raw handle (`AICAD-122`); `merge_faces`'s partial-merge pairing
and `replace_face`'s compatibility checking are narrower than the kernel's
own full capability (`AICAD-123`); adoption always runs exactly two fixed
checks, no configurable `ValidationLevel` (`AICAD-124`).

`part`-in-`part` nesting (resolved by `AICAD-101`, `DL-36`) and the
Stage-4 `D31` `part`-scoping question (resolved by `AICAD-100A`) both
remain resolved — re-confirmed unchanged this audit, not reopened by
Stage 5.

## 6. Unresolved owner decisions (enumerated)

- **`D7` fingerprint-recovery sub-question** (`project/OWNER_DECISIONS.md`
  §D7): whether/when to enable automatic geometry-fingerprint-based
  reference recovery remains open, gated on future benchmark evidence.
  Unchanged by Stage 5; automatic recovery stays disabled.
- **§5 item 1** (freeform-to-topology bridging gap) and **§5 item 2**
  (`List<Geometry>` `geometry_inputs` visibility / unverified dirty-set
  interaction) are both recorded as non-decision disclosed limitations in
  `project/OWNER_DECISIONS.md`'s "Non-decision items carried forward"
  section — ordinary future implementation work, not an architecture
  question requiring a ruling, per each finding's own framing. Listed here
  for visibility per this packet's own acceptance criterion, not as a
  blocker.
- No new `D32`+ entry was opened during Stage 5 (`grep "^## D3[2-9]"
  project/OWNER_DECISIONS.md` returns nothing beyond the pre-existing
  `D31`) — the campaign ran `AICAD-101` through `AICAD-129` without
  requiring a new owner ruling beyond the two already-recorded, narrow
  ones (`DL-36` nested-`part` depth, `DL-37` part-body `param`s).

## 7. Realistic / adversarial campaign results (`AICAD-127`-`129`, re-run this audit)

- **`AICAD-127`** froze `project/benchmarks/stage5_freeform_corpus/` (4
  `public` + 3 `held_out` fixtures: spline-hook, twisted-variable-section
  solid, twisted-blade surface, trimmed freeform patch, out-of-domain trim
  rejection, near-degenerate extreme twist, plus the freeform-topology
  rejection cases) — every fixture checked against exact/closed-form
  values, `stage5_freeform_corpus.rs` 7/7 this audit.
- **`AICAD-128`** ran 8 adversarial cases (`stage5_adversarial_campaign.rs`,
  8/8 this audit): near-degenerate + multi-scale circular faces (1 nm/1 km
  radius, area exact to ~1e-16 relative error), tangent spheres
  (`intersect_surfaces` fails the whole build explicitly with
  `GeometricQueryFailed`, never an empty `List`), periodic circle
  wraparound, a real sew-tolerance boundary (7/6 edges/vertices inside
  tolerance vs. 8/8 outside), a self-intersecting B-spline's closest-point
  query, a 64-edge polygon (~30 ms, exact area — the campaign's own
  performance/resource evidence point), deterministic repeat-build
  (bit-identical), and a malformed B-spline knot vector (rejected
  explicitly). **No reproducible defect found.**
- **`AICAD-129`** audited the core-vs-library boundary (§3.1) and proved
  the inspectability chain (`stage5_inspectability_fixture.rs`, 4/4 this
  audit): typed-call diagnostics and reference evidence via the real `cad
  build --json`/`cad refs check --json` JSON contract; feature/provenance/
  dependency and Geometry-IR/kernel-result layers via the public
  `FeatureGraph`/`ParametricBuildSession` Rust API. Discovered the
  `List<Geometry>` `geometry_inputs` gap (§3.5/§5 item 2) in the course of
  building this fixture. Also added an explicit teaching/realistic
  **Class** column and Stage-5 checkpoint coverage section to `examples/
  README.md`.

## 8. Scope-creep audit (whole Stage-5 diff, this audit)

`git diff --stat df933463..HEAD`, categorized by top-level touched
directory: `project/` (36 files — reports/TASKS.yaml/state, expected),
`crates/cad-cli` (14), `crates/cad-geometry-api` (10), `crates/cad-runtime`
(8), `crates/cad-geometry-runtime` (7), `examples/` (5, all Stage-5
teaching/realistic examples), `crates/cad-feature-graph` (5),
`crates/cad-validation`/`cad-query`/`cad-hir` (4 each), `native/` (3,
`occt_bridge` C++ additions for the new topology/raw-tier kernel
operations), `crates/cad-units` (3), `crates/cad-occt-bridge`/`cad-kernel-
api` (2 each), `crates/cad-references` (1), plus `project/benchmarks/
stage5_freeform_corpus/` fixtures and `Cargo.lock`. No `cad-assemblies`/
`cad-configurations`/`cad-requirements`/`cad-artifact`/`cad-interchange`/
`cad-packages`/`cad-lsp`/`cad-provenance`/`cad-diagnostics`/`cad-
constraints` crate was touched anywhere in Stage 5 — consistent with the
campaign brief's "no speculative future work" list (no assembly/mate/
configuration/BOM/requirement/packaging/plugin-ABI scaffolding exists
beyond what Stage 4 already had).

## 9. Maintained examples

14 ACTIVE examples registered in `crates/cad-cli/tests/active_examples.rs`
(up from Stage-4's count), all building cleanly this audit
(`active_examples.rs`, 3/3 including reference-health fixtures). Five new
Stage-5 examples were added across the batches per the Examples Policy
(update in the same batch a public capability changes; add representative
examples at each checkpoint): `examples/curves/circle_curve_basics.aicad`,
`examples/curves/cable_routing_path.aicad` (`AICAD-112`/Checkpoint A),
`examples/surfaces/surface_query_basics.aicad`, `examples/surfaces/
pipe_clearance_check.aicad` (`AICAD-118`/Checkpoint B), and `examples/
topology/topology_construction_basics.aicad` (`AICAD-119`, unchanged since,
re-confirmed current at Checkpoint C). No raw-tier `.aicad` example exists
by deliberate, consistently-applied convention (every raw/query-category
builtin needs a live `OcctContext`, demonstrated instead through dedicated
`crates/cad-cli/tests/stage5_raw_*.rs`/`stage5_lineage_checkpoint.rs`
files that `cargo test` runs automatically). No stale example was found;
`examples/README.md`'s Class column and Stage-5 checkpoint-coverage
section (`AICAD-129`) remain current.

## 10. Performance / resource evidence

Bounded by what the active tasks actually collected (no dedicated Stage-5
performance-benchmark task exists in the fixed queue): `AICAD-128`'s
64-edge polygon adversarial case built and validated in ~30 ms with exact
area to closed-form precision — the only explicit timing evidence
recorded, and it shows no resource-budget concern at that scale. No
broader performance/scaling benchmark was run or is required by
`AICAD-130`'s own acceptance criteria beyond this.

## 11. Recommendation

**PASS.** Every `AICAD-101`..`129` acceptance item is independently
re-confirmed against current source and a fresh full-suite run (§2-§4),
not merely cited from prior reports. `D21`-`D25`/`D27` compatibility holds
(§3.1-3.5), kernel neutrality holds (§3.6), `D7` fail-closed references
show zero surviving silent-wrong regressions (§3.7), tolerance domains
remain separated (§3.4), raw-handle epoch/non-identity rules are unchanged
and still enforced (§3.2), and topology-changing operations emit
resolver-consumable lineage (§3.8). The three internal checkpoints
(`AICAD-112`/`118`/`126`) each independently recommended PASS with no
architecture-boundary bypass found, and this audit found nothing at the
final gate that contradicts any of them. The realistic/adversarial
campaign (§7) found no reproducible defect across 15 corpus/adversarial
cases. Every required check is green (§4): `cargo fmt`/`clippy` clean,
`cargo test --workspace` 1830/1830, native CTest 18/18, semantic-reference
harness `ok`/`ok`, 14 ACTIVE examples building cleanly.

The disclosed limitations (§5) are real and should inform Stage-6
planning — most significantly, item 1 (freeform curves/surfaces cannot yet
become real topology) means any provisional Stage-6 assumption that
Stage-5 delivers full B-rep construction from every Stage-5 curve/surface
family is **not yet evidenced** and should be reconciled, not silently
carried forward, before Stage-6 promotion (`project/planning/transitions/
stage4-to-stage5/STAGE_PROMOTION_POLICY.md`). None of the disclosed items
represents a hidden regression, a weakened gate, or a silent-wrong result;
each is an honestly-reported, individually-evidenced scope boundary. This
is a recommendation only — Stage-5 approval, Stage-6 authorization, and
Stage-6 AICAD-ID assignment all remain separate, later owner decisions.

## 12. Next dependency

Per the campaign brief's **FINAL STOP RULE**: **STOP ROADMAP DEVELOPMENT.**
No future invocation may begin Stage-6 implementation, finalize/activate
the provisional Stage-6/7 queues as executable roadmap work, or assign
final global Stage-6 AICAD IDs without a separate, later, explicit owner
approval recorded in `project/DECISION_LOG.md`. Only the owner may review
this packet, approve Stage 5, and authorize the lightweight Stage-5 ->
Stage-6 reconciliation and Stage-6 itself.
