# Testing and evidence

AICAD's correctness policy is evidence-first: a geometry operation is not considered correct because a render looks plausible.

## Standard workspace checks

From the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

The workspace build invokes the native OCCT bridge through `cad-occt-bridge/build.rs`, so an appropriate OCCT development installation is required.

## Native bridge checks

The bridge also supports direct CMake/CTest execution:

```sh
cmake -S native/occt_bridge -B native/occt_bridge/build
cmake --build native/occt_bridge/build
ctest --test-dir native/occt_bridge/build --output-on-failure
```

Native lifecycle/ABI tests cover context/handle safety in addition to geometric operations.

## Geometry evidence

Tests should use the strongest practical evidence for the operation under test:

- validity plus analytically known dimensions/volume/area/center of mass;
- topology counts/classes only when they are semantically meaningful;
- independent STEP file-structure/re-import checks where interchange is the feature;
- source/provenance assertions for compiler/feature behavior;
- explicit dirty/reused/recomputed evidence for incremental rebuilds.

Do not turn kernel edge/face enumeration order into an identity assertion unless the test is deliberately characterizing that raw selector behavior.

## Determinism

AICAD-owned compiler/semantic output should be deterministic for identical inputs. B-rep bytes, STEP text bytes, and raw topology enumeration order are not the cross-platform determinism criterion; exact geometry is compared under D5/D19's accepted semantic/numerical equivalence policy.

`crates/cad-cli/tests/stage4_determinism_foundation.rs` provides the Stage-4-ready baseline: canonical AICAD-owned build/diagnostic report serialization is repeated exactly, while repeated exact geometry is compared through the D5 v1 engineering profile instead of byte equality.

## Stage-4 semantic-reference hard gate

The AICAD-079A corpus remains frozen under `project/benchmarks/stage4_semantic_reference/`. AICAD-079C adds resolver-independent plumbing at `tests/semantic_refs/` and `scripts/ci/semantic_ref_harness.py`; it does **not** implement a semantic reference or resolver.

The harness recognizes `RESOLVED_CORRECT`, `AMBIGUOUS`, `BROKEN`, `SILENT_WRONG`, `KERNEL_FAILURE`, and `UNRELATED_FAILURE`. D7 remains fail-closed: one intended entity resolving correctly is good; explicit ambiguity is good; explicit breakage is acceptable where necessary; silent wrong selection is catastrophic. Fingerprints are evidence/ranking/benchmark data only in the first Stage-4 implementation unless a later owner decision authorizes automatic recovery.

Every discovered `SILENT_WRONG` result must become a minimized permanent regression under `tests/semantic_refs/regressions/`.

## Property/adversarial testing

Prefer bounded invariant sweeps over large volumes of low-value random cases. The Stage-4 readiness foundation includes deterministic spatial direction/frame property sweeps in `cad-kernel-api`; future reference cardinality/identity properties are added only after real Stage-4 interfaces exist.

## CI layers

See [`ci-and-branch-protection.md`](ci-and-branch-protection.md) for required PR checks, scheduled/manual hardening, the current Linux support tier, failure artifacts, and the branch-protection check names the owner should configure after merge.

## Stage gates and historical evidence

Completed Stage-0..3 task reports and checkpoint/gate packets are archived under `project/reports/archive/` and `project/gates/archive/`. Use them when validating a historical implementation claim, but keep current tests/source as the primary source of truth.
