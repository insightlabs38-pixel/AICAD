# AICAD-141 — Checkpoint A: semantic assembly model without a numerical solver

## Result

`crates/cad-assemblies/tests/checkpoint_a.rs` proves the full Stage-6 semantic assembly model — definitions, repeated-definition instances, nested occurrences, frame composition, cross-instance semantic references, mechanical interfaces, mates, and joints — resolves and validates end to end with **no numerical solver present at all**. `cad-assemblies` has no dependency on any solver/adapter crate (`AICAD-142`+ do not exist yet), so this is structural, not just a claim: the checkpoint fixture cannot possibly depend on one.

## Fixture and coverage

A two-link pivoting-arm fixture: an `Arm` root with two children (`link_a`, `link_b`) sharing one `Link` component definition (repeated-definition instantiation, `AICAD-134`/`136`). It exercises:

- `expand` over the registry, proving `link_a`/`link_b` resolve to two distinct `OccurrencePath`s with a shared leaf definition (D26).
- `resolve` (`AICAD-137`) against a real `cad-occt-bridge` box shape (the same "genuine shape, not a hand-picked mock" precedent `reference.rs`'s own tests use) — a pivot-face reference resolves with no solver anywhere in the call graph.
- `MechanicalInterface`/`MechanicalInterfaceInstance` (`AICAD-138`) bound over each link's own resolved `WorldPose`, with `check_conformance` and `check_compatibility` both passing.
- A `Coincident` and a `Distance` `Mate` (`AICAD-139`) between the two links' pivot-face subjects.
- A `Revolute` `Joint` (`AICAD-140`) with explicit ±90° limits between the two links; `validate_coordinate` accepts an in-range angle and rejects an out-of-range one (fail-closed, not clamped).
- Deterministic serialized observation: `run_checkpoint_pipeline` is called twice independently and its `(occurrences, relations, interfaces)` JSON rendering is asserted byte-identical.
- No topology-index/solver-native identity leak: the rendered relation JSON is asserted to contain every declared source name verbatim (`pivot_contact`, `pivot_gap`, `elbow`, `link_a`, `link_b`, mate-kind labels) — identity survives rendering as source-derived text, never an opaque index/handle.

3 tests, all passing.

## Verification

- `cargo fmt --all -- --check` — PASS
- `cargo clippy -p cad-assemblies --all-targets --all-features -- -D warnings` — PASS
- `cargo test -p cad-assemblies` — 111/111 PASS (94 unit + `checkpoint_a` 3 + 4 existing integration suites)
- `cargo test --workspace` — PASS, no regressions

## Architecture verification

- No solver/adapter dependency exists in `cad-assemblies` — Checkpoint A's own "meaningful without `AICAD-144` or any numerical backend" requirement holds structurally, confirmed by `cargo tree -p cad-assemblies` carrying only `cad-diagnostics`/`cad-kernel-api`/`cad-query`/`cad-references`/`cad-types`/`cad-units` (plus the dev-only `cad-occt-bridge` fixture dependency, already present since `AICAD-137`).
- D26 identity domains stayed distinct throughout: `link_a`/`link_b` share `ComponentDefinitionId::named("Link")` but never collapse to one `OccurrencePath`, `MateId`/`JointId`, or `OccurrenceTopologyRef`.
- Pose changes never entered mate/joint/reference identity — `OccurrenceTopologyRef`/`MateId`/`JointId` carry no pose field (structural, not conventional, per `AICAD-135`/`137`'s own established invariant).

## Limitations

- The fixture is a single two-link mechanism, not a large/deeply nested corpus — that breadth is `AICAD-157`'s job (realistic assembly corpus), not Checkpoint A's.
- No `.aicad` source syntax exercises this pipeline yet; the checkpoint runs entirely through the public Rust API, consistent with every Stage-6 IR module to date (no assembly grammar exists).

## Next

Batch `S6-04` (`AICAD-139`/`140`/`141`) is complete. `AICAD-142` (numerical assembly-solver adapter contract, batch `S6-05`) is the next executable task.
