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

AICAD-owned compiler/semantic output should be deterministic for identical inputs. B-rep bytes are not the cross-platform determinism criterion; exact geometry is compared under the accepted semantic/numerical equivalence policy.

## Stage gates and historical evidence

Completed Stage-0..3 task reports and checkpoint/gate packets are archived under `project/reports/archive/` and `project/gates/archive/`. Use them when validating a historical implementation claim, but keep current tests/source as the primary source of truth.

The Stage-4 semantic-reference benchmark corpus is already frozen to prevent tuning against held-out cases, but Stage-4 resolver implementation has not started on this transition branch.
