# Stage-3 Batch checkpoint — Parametric Graph (AICAD-064A..069)

Prepared after `AICAD-069`, per the active scheduled-task brief's fixed
batch order (`S3-00`, `S3-01`, `S3-02`, checkpoint). This is the **first**
Stage-3 batch checkpoint — there is no earlier one to build on, so its
scope is every Stage-3 task completed since Stage 2 closed
(`DECISION_LOG.md#DL-16`): `AICAD-064A`/`AICAD-065` (`S3-00`),
`AICAD-066`/`AICAD-067` (`S3-01`), `AICAD-068`/`AICAD-069` (`S3-02`). This
is a **batch checkpoint** gating Batch `S3-03` (`AICAD-070`/`AICAD-071`),
not a Stage-3 owner gate packet (that is `AICAD-079B`'s job, per
`project/gates/README.md`'s format for `stage-<n>-gate.md`). Per
`AGENTS.md` ("Stage gates"), preparing evidence and a recommendation is
within this agent's role; this checkpoint does not itself constitute owner
approval of anything.

## 1. Exact git revision at checkpoint time

Prepared on branch `claude/aicad-stage3-dev`, HEAD `8803326` (the
`AICAD-069` commit), immediately before this document lands. Working tree
clean at the start of this checkpoint's own verification run (confirmed
via `git status --short` immediately before §4 below).

```
$ git log --oneline 0b6b0b3..HEAD
8803326 AICAD-069: Implement feature-DAG source-to-feature mapping and provenance
2958ae3 AICAD-068: Implement feature-DAG cache keys and dirty propagation
f91fe80 AICAD-066/AICAD-067: Build the Stage-3 Feature DAG node identity/dependency model and its construction from supported modeling operations
799a957 AICAD-065: Implement first-class param declarations and derived expressions
1865855 AICAD-064A: Implement/calibrate the D5 v1 cad-validation comparison profile
0a5202d Record Stage-2 owner approval (DL-16) and advance to Stage 3
```

(`0b6b0b3` = the Stage-2-closing merge commit, `project/CURRENT_STAGE.md`'s
own Stage-2/Stage-3 boundary.)

## 2. Batch scope and task reports

| Task | Title | Report |
|---|---|---|
| AICAD-064A | Implement/calibrate the D5 v1 cad-validation comparison profile | `project/reports/AICAD-064A.md` |
| AICAD-065 | Implement first-class param declarations and derived expressions | `project/reports/AICAD-065.md` |
| AICAD-066 | Build feature-DAG node identity/dependency model | `project/reports/AICAD-066.md` |
| AICAD-067 | Build feature DAG from supported modeling operations | `project/reports/AICAD-067.md` |
| AICAD-068 | Implement cache keys and dirty propagation | `project/reports/AICAD-068.md` |
| AICAD-069 | Implement source-to-feature mapping and provenance | `project/reports/AICAD-069.md` |

`AICAD-066`/`AICAD-067` share one commit/report pair (see
`project/reports/AICAD-066.md`'s own "Why one commit" section) but remain
two distinct `project/TASKS.yaml` entries, both `status: done`.

Crates in scope: `crates/cad-validation` (`AICAD-064A`), `crates/
cad-runtime/src/params.rs` (`AICAD-065`), and `crates/cad-feature-graph`
in full (`AICAD-066`-`069` — this crate held only an unfilled `AICAD-002`/
`AICAD-003` placeholder before `AICAD-066`). `git diff 0b6b0b3..HEAD --stat
-- crates/` confirms no crate outside this list changed.

## 3. Checklist

Checklist items follow `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md`
§9-10 (the feature-DAG node-field list and incremental-invalidation
algorithm this batch exists to build) plus `DECISION_LOG.md#DL-12`/`#DL-17`
(D5/D19, closed by `AICAD-064A`) and `#DL-15` (D18, the runtime-backed
standard-function mechanism `AICAD-066`/`AICAD-067` build the feature DAG
on top of).

### 3.1 D5 v1 comparison-profile calibration (D19)

`crates/cad-validation::profile::ComparisonProfile::v1` implements the
complete versioned comparison profile `DECISION_LOG.md#DL-12` froze the
*shape* of (`linear`/`area`/`volume`/`center-of-mass`, each `max(abs,
rel * S^k)`), with all seven numeric constants now filled in:
`linear_abs = 0.0001mm`/`center_of_mass_abs = linear_abs`/`volume_rel =
0.001` (owner-accepted directly from Stage-1 evidence, `DECISION_LOG.md
#DL-17`) and `linear_rel = 0.0`/`area_abs = 1e-6`/`area_rel = 1e-3`/
`volume_abs = 1e-6` (derived by `AICAD-064A`'s own bounded multi-scale
calibration corpus, per that report). `v1_matches_the_owner_accepted_dl17_
constants` (re-run, §4) pins all seven values against the decision-log
text directly, so a future accidental edit to the profile would fail this
checkpoint's own re-run, not just `AICAD-064A`'s original test. No
escalation was needed (`OWNER_DECISIONS.md#D19` marked `RESOLVED`).
**PASS.**

### 3.2 First-class `param` declarations and derived expressions

`crates/cad-runtime::params::ParamModel` gives every top-level `param` a
stable `ParamId` (wrapping its own `BindingId`, not a parallel identity),
computes `depends_on` edges from each `default` expression's own
referenced params (never implicit source-position order), and produces a
deterministic topological `evaluation_order` (Kahn's algorithm, ties
broken by original declaration order — `evaluation_order_is_deterministic_
across_rebuilds` re-run, §4). A cyclic dependency
(`direct_cycle_is_detected_not_silently_ordered`/
`self_referential_param_is_a_cycle`) is a structured `ParamModelError`,
never an arbitrary evaluation order, per `AGENTS.md`'s "ambiguity is an
error, never an arbitrary selection." `ParamOverrides` (a plain `HashMap<
ParamId, Value>`) is the edit/rebuild primitive `AICAD-068`'s own
`dirty_set` is the feature-DAG-layer analogue of. **PASS.**

### 3.3 Feature-DAG node identity and dependency edges (D18-consistent)

`crates/cad-feature-graph::graph::FeatureGraph::build` walks an
already-lowered `HirProgram`'s top-level `let`/`const` items and gives
every call to one of the eight closed `BuiltinFnId` supported modeling
operations (`box`/`cylinder`/`transform`/`union`/`cut`/`intersect`/
`fillet`/`chamfer` — `DECISION_LOG.md#DL-15`'s own closed catalogue, not a
second parallel operation-kind enum) its own stable `FeatureId`, minted in
strictly increasing build order (never reused from `BindingId`, since an
anonymous nested argument expression is not itself declared — see
`graph.rs`'s own "Node identity: minted, not reused from `BindingId`").
`geometry_inputs` dependency edges reuse an already-built node for a
repeated name reference rather than duplicating one
(`shared_named_input_is_reused_not_duplicated`,
`alias_reference_shares_the_same_feature_id`) — the dependency structure
is a genuine DAG over shared substructure, not a tree. A `Geometry`-typed
parameter slot that does not resolve to a recognized feature shape is a
structured `GEOM-E005`/`GEOM-E006` diagnostic, never a panic or a silent
skip (`dangling_geometry_argument_is_a_structured_error`,
`forward_reference_is_unresolved_not_silently_reordered`). **PASS.**

### 3.4 Cache keys (`docs/plan/06...` §9's "cache key" field)

`crates/cad-feature-graph::cache::CacheKey` is a purely structural content
hash — operation name, every `geometry_inputs` node's own already-computed
key (so an upstream structural edit propagates into every downstream
node's key automatically), and the node's own `parameters` *expressions*
(never an evaluated value — this crate has no interpreter, consistent with
`FeatureNode::parameters`' own pre-existing "borrowed expression, not a
computed value" design). Re-verified directly against source for this
checkpoint: identical source produces identical keys across two
independent lowering passes (`identical_source_produces_identical_cache_
keys`); editing one literal changes that node's own key and every
downstream consumer's key while leaving an untouched sibling's key
unchanged (`changing_one_literal_changes_only_its_own_and_downstream_
cache_keys`); span position never affects the key
(`cache_key_ignores_span_position`). **PASS.**

### 3.5 Dirty propagation (`docs/plan/06...` §10, "Incremental invalidation")

`FeatureGraph::dirty_set` implements §10 steps 1-3 exactly: a node is
dirty if one of its own `binding_refs` is in the caller-supplied changed
set (step 1, "find parameter dependents"), or transitively if any of its
own `geometry_inputs` is itself dirty (step 2, "mark affected feature
nodes dirty" propagated downstream); every other node is left out of the
returned set entirely (step 3, "preserve unaffected cached nodes"). Proven
directly: a feature referencing a changed `param` is marked dirty, a
downstream consumer of that feature is transitively dirty, an unrelated
sibling and a structurally-disconnected node are both left clean
(`dirty_set_marks_direct_and_transitive_dependents_only`); an empty
changed set yields an empty dirty set
(`dirty_set_is_empty_when_nothing_changed`). Correctness of the
single-linear-pass implementation (no explicit topological-sort/queue
machinery) rests on `FeatureGraph::nodes()` already being
dependency-respecting build order — re-confirmed by inspection of
`Builder::resolve_geometry_expr` (every `geometry_inputs` entry is fully
built and pushed onto `self.nodes` before the node containing it). **PASS.**

### 3.6 Source-to-feature mapping and provenance (`docs/plan/06...` §9's
remaining "source_span"/"provenance" fields)

`FeatureGraph::feature_at(position: u32) -> Option<FeatureId>` resolves
the innermost feature node whose own span contains a byte offset — proven
against a genuinely nested case (`cut(base, cylinder(...))`): a position
inside the nested `cylinder(...)` call resolves to that inner feature, not
the enclosing `cut(...)` (`feature_at_resolves_to_the_innermost_
containing_span`); a position outside every span is `None`
(`feature_at_outside_every_span_is_none`). `crate::provenance::Provenance`
records `declared_as` (`Let`/`Const`/`Anonymous`, first-name-wins,
mirroring the pre-existing `FeatureNode::name` convention —
`top_level_let_gets_declaration_let`/`top_level_const_gets_declaration_
const`/`anonymous_nested_call_gets_declaration_anonymous`) and
`transitive_bindings` (the full top-level-binding dependency closure,
merging a node's own `binding_refs` with every `geometry_inputs`
ancestor's own already-computed closure —
`transitive_bindings_include_upstream_features_own_param_refs` proves a
downstream node inherits an upstream feature's own referenced `param` even
though its own parameters never mention it directly). Deliberately
excludes `docs/plan/14_COLLABORATION_PROVENANCE_SECURITY.md`'s much larger
Git/AI-governance "feature-level provenance" concept (actor identity,
commit metadata, review/approval state) — that is WP-14, a separate,
later work package with no Stage-3 task and no supporting infrastructure
anywhere in this codebase; see `provenance.rs`'s own "What this
deliberately does not do" for the full boundary. **PASS.**

### 3.7 Deterministic execution/hashing (D5/DL-12 Level 1)

Specifically re-examined for this checkpoint's own new code
(`crates/cad-feature-graph`), per the campaign brief's standing instruction
to check every batch for nondeterminism entering through iteration order,
hashing, concurrency, or environment-dependent behavior:

- **Hashing.** `crate::cache::CacheKey` deliberately does **not** use
  `std::collections::hash_map::DefaultHasher` — that type's own
  documentation states its algorithm "is not specified, and so it and its
  hashes should not be relied upon over releases," which would violate
  `DECISION_LOG.md#DL-12` Level 1's byte-identical canonical-serialization
  requirement for identical compiler inputs/version. `cache::StableHasher`
  is a small, fully-specified FNV-1a 64-bit accumulator instead (`grep -n
  "DefaultHasher" crates/cad-feature-graph/src/` finds only this module's
  own doc-comment explanation of why it is not used, never an actual use).
- **Iteration order.** `FeatureGraph`'s two `HashMap`/`HashSet`-shaped
  fields (`named: HashMap<BindingId, FeatureId>` and every `HashSet` a
  caller passes into/receives from `dirty_set`) are used exclusively for
  point lookups/membership tests (`grep -n "\.iter()\|\.values()\|\.keys()"`
  against `graph.rs` finds no call against `named` — every hit is either a
  `Vec`/slice iteration, a test assertion, or a fresh
  `HashSet::new()`/`.contains()` call) — never iterated to produce
  ordered output. `dirty_set`'s own return type is a `HashSet<FeatureId>`
  (membership, not an ordered report); nothing in this codebase yet
  consumes it for display/serialization, so no ordering concern currently
  exists downstream, but any future consumer that needs a stable order
  must sort explicitly rather than rely on `HashSet` iteration.
- **Concurrency/environment.** No `std::thread`/`std::time`/`env::var`/
  `SystemTime` anywhere in `crates/cad-feature-graph` (confirmed by direct
  grep, zero matches) or in `crates/cad-runtime::params`/`crates/
  cad-validation::profile` (unchanged from those tasks' own reports).

**No D5 Level-1 violation found** in any task from `AICAD-064A` through
`AICAD-069` inclusive.

## 4. Fresh verification run at checkpoint time

```
$ git status --short
(empty)

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0)

$ cargo test -p cad-validation -p cad-runtime -p cad-feature-graph -p cad-hir \
    -p cad-ast -p cad-parser -p cad-lexer -p cad-compiler -p cad-diagnostics
test result: ok. 7 passed (cad-ast lib)
test result: ok. 19 passed (cad-ast printer_round_trip)
test result: ok. 49 passed (cad-compiler)
test result: ok. 20 passed (cad-diagnostics lib)
test result: ok. 10 passed (cad-diagnostics schema_conformance)
test result: ok. 34 passed (cad-feature-graph — this checkpoint's own
  primary new crate; 0 before AICAD-066, 11 after AICAD-066/067, 24 after
  AICAD-068, 34 after AICAD-069)
test result: ok. 197 passed (cad-hir)
test result: ok. 29 passed (cad-lexer)
test result: ok. 119 passed (cad-parser)
test result: ok. 2 passed (cad-parser shared_corpus)
test result: ok. 101 passed (cad-runtime — up from Stage-2's 83, reflecting
  AICAD-065's params.rs module)
test result: ok. 5 passed (cad-validation lib)
test result: ok. 2 passed (cad-validation calibration)
Total: 594 passed, 0 failed.

$ cargo test --workspace
853 tests total passed, 0 failed, 0 ignored, across every crate with tests
(includes the full native/OCCT Stage-0/1 Rust suite and every other
Stage-2 crate, unaffected by this batch, included only because
`--workspace` runs everything).

$ cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1
test result: ok. 3 passed; 0 failed (Stage-2 gate proof unaffected by this
  batch's own crates).
```

Environment: Rust 1.98.1, edition 2024 (`rust-toolchain.toml`), unchanged
from Stage 2.

## 5. Known limitations (carried forward, not blocking Batch S3-03)

- `cad_runtime::params::ParamModel` and `cad_feature_graph::FeatureGraph`
  are not yet wired together — `FeatureGraph::dirty_set` takes a plain
  caller-supplied `HashSet<BindingId>`, not a `ParamOverrides` edit
  directly; that integration is future work for whichever task first
  needs an end-to-end "edit a param, rebuild only the affected features"
  pipeline (not scoped to any completed Stage-3 task so far — `AICAD-070`/
  `AICAD-071` build the safe language-facing geometry types and `Part`
  concept next, not this integration).
- The feature DAG only recognizes a direct call to one of the eight closed
  `BuiltinFnId` operations, or a bare name reference to an already-built
  node — a call to an ordinary AICAD-source `fn` that itself constructs
  geometry is not inlined/flattened into the graph, and a
  `Geometry`-typed `if`/`match` expression is not modeled at all (both
  deliberate, documented scope boundaries from `AICAD-066`/`AICAD-067`,
  unchanged since).
- `FeatureGraph::feature_at` is a linear scan over every node; acceptable
  for every current Stage-3 fixture, flagged in `AICAD-069`'s own report
  as a straightforward future optimization if graph size ever makes it
  matter.
- No `cad`-CLI-facing surface exists yet for any of this batch's data
  (no "explain feature"/"what does this depend on" command) — this batch
  supplies the underlying model only, per each task's own documented scope
  boundary.
- Every unresolved `OWNER_DECISIONS.md` item carried into Stage 3
  (`D7`/`D8`/`D12`/`D15`) remains exactly as it was at Stage-2 exit; this
  batch closed none and opened none.

## 6. Recommendation

**PASS — Batch S3-03 (`AICAD-070`/`AICAD-071`, safe language-facing
geometry types and the `Part` concept) may begin.** All six checklist
items in §3 are met, re-verified directly against current source at this
exact HEAD rather than only cited from individual task reports; the D5
cross-check (§3.7) found no nondeterminism entering through iteration
order, hashing, concurrency, or environment-dependent behavior anywhere in
this batch's own crates. `AICAD-064A` closed `OWNER_DECISIONS.md#D19`
completely (no remaining open sub-item); no other `OWNER_DECISIONS.md`
item is newly required by this checkpoint itself. This recommendation
does not itself constitute Stage-3 owner approval of anything — Stage 3
as a whole still requires the owner-recorded decision `AICAD-079B` will
seek, per `project/CURRENT_STAGE.md`.
