# Testing and evidence

AICAD's correctness policy is evidence-first: plausible rendering is not proof.

## Standard workspace checks

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

The workspace build requires the OCCT development environment used by `cad-occt-bridge`.

## Geometry and reference evidence

Use the strongest practical evidence:

- exact validity and analytically known dimensions/volume/area/center of mass;
- semantically meaningful topology properties;
- STEP export/re-import where interchange is under test;
- source/provenance assertions for compiler/feature behavior;
- dirty/reused/recomputed evidence for incremental rebuilds;
- explicit `Resolved`/`Ambiguous`/`Broken` assertions for persistent references;
- stale raw-handle rejection across rebuild epochs.

Do not promote raw topology enumeration order into identity.

## Stage-4 semantic-reference gate

The frozen Stage-4 corpus remains under `project/benchmarks/stage4_semantic_reference/`. The production Stage-4 resolver is now implemented and the final gate demonstrated zero surviving `SILENT_WRONG` outcomes. `Resolved`, explicit `Ambiguous`, explicit `Broken`, kernel failure, and unrelated failure remain distinct result classes.

Fingerprint evidence/ranking is not automatic recovery. Every discovered silent-wrong result remains a critical regression.

## ACTIVE example invariant

Beginning with the Stage-5 transition, every ACTIVE user-facing example is an executable product surface.

`crates/cad-cli/tests/active_examples.rs` is the maintained baseline. It must at minimum parse/typecheck/build each ACTIVE example. Reference examples additionally assert their intended health outcome, and the canonical persistent-reference example is replayed through a real parameter edit/rebuild.

When a public language/modeling change lands, update affected ACTIVE examples in the same task/batch. At each major stage checkpoint, add or refresh representative examples. A stale example must be updated or explicitly archived; it must not silently remain as historical syntax in the current learning path.

Stress/benchmark fixtures belong under test/benchmark infrastructure rather than the primary user-learning tree.

## Determinism

AICAD-owned semantic/compiler output is deterministic where defined. B-rep/STEP bytes and raw topology order are not the cross-platform determinism criterion; exact geometry is checked under the accepted D5/D19 equivalence profile.

## CI and historical evidence

See [`ci-and-branch-protection.md`](ci-and-branch-protection.md) for CI layers. Historical reports/gates remain evidence from their time; current tests/source are the primary implementation truth.
