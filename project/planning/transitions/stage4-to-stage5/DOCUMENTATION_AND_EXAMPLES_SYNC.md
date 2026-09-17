# Stage-4 -> Stage-5 documentation and examples synchronization

Status: complete on the Stage-5 transition branch.

## Scope

This pass synchronized current public/developer documentation and the user-facing example set with the owner-approved Stage-4 implementation. It did not implement any Stage-5 task.

## Documentation audited and corrected

Current-facing material was updated to describe:

- Stage 4 as complete and owner-approved;
- source-declared `query name : EntityKind in scope { ... }` references;
- explicit scope and fail-closed `Resolved` / `Ambiguous` / `Broken` outcomes;
- `cad refs check` consuming real source-declared references;
- AICAD-owned feature/provenance identity and kernel lineage as evidence;
- `ParamModel` + `FeatureGraph` incremental rebuild and reference replay;
- raw-index/raw-handle lifetime boundaries;
- automatic fingerprint recovery remaining disabled;
- the Stage-5 queue as **PLANNED / NOT YET IMPLEMENTED**.

Stale current-doc claims that semantic references were future Stage-4 work or that `cad build` was the only CLI command were removed.

## Example classification

### ACTIVE

- `examples/getting_started/simple_box.aicad`
- `examples/parametric/derived_plate.aicad`
- `examples/brackets/stage2_mounting_plate.aicad`
- `examples/brackets/stage3_l_bracket.aicad`
- `examples/plates/stage3_bearing_mount.aicad`
- `examples/enclosures/stage3_enclosure.aicad`
- `examples/references/hole_wall_reference.aicad`
- `examples/references/ambiguous_reference.aicad`
- `examples/references/broken_reference.aicad`

The historical stage-numbered filenames are retained because accepted tests/evidence reference them; their syntax remains current and they are explicitly registered as ACTIVE.

### UPDATE

`stage3_l_bracket.aicad` remains active but its comment that persistent references were future Stage-4 work was corrected. Current learning indexes were rewritten around the maintained set.

### ARCHIVE

The non-compilable Stage-0 assembly paper example and roadmap-only freeform/verification category placeholders moved under `project/archive/examples/`. No historical evidence was rewritten.

## Automated example baseline

`crates/cad-cli/tests/active_examples.rs` establishes the maintained baseline:

- every ACTIVE example builds through the real compiler/runtime path;
- the three reference fixtures assert expected resolved/ambiguous/broken health;
- the canonical source reference is resolved, its `hole_diameter` parameter is edited, the model is rebuilt through `ParametricBuildSession`, and the same recipe is re-resolved.

Existing ordinary-part integration tests continue to provide stronger STEP export/re-import and exact-B-rep validity coverage for representative mechanical examples.

CI/integration push filters now include the Stage-5 transition branch and run the active-example test.

## Deliberate current limitations documented

- broad unscoped whole-session resolution remains a low-level operation and can be ambiguous;
- source-declared references require explicit scope;
- automatic fingerprint recovery is disabled;
- nested `part`, Area/spatial source-value, and remaining source-query vocabulary gaps are Stage-5-prelude work;
- Stage-5 advanced curves/surfaces, multi-solution geometry queries, topology construction/healing, raw editing/adoption, and advanced lineage work are not implemented;
- assemblies/configurations/verification remain later-stage capabilities.

## Validation

Required validation for this change is:

```sh
git diff --check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test -p cad-cli --test active_examples --locked -- --test-threads=1
```

The transition-branch GitHub Actions runs are the authoritative OCCT-enabled validation environment. Exact run results should be recorded here by the committing invocation if available; a pending external run must not be represented as passed.

## Deferred documentation gaps

Historical reports/gates and frozen `docs/plan/` material intentionally retain their original stage-era wording. They are evidence/history, not current user documentation.
