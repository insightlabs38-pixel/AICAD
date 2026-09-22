# AICAD-126: Checkpoint C — topology, raw tier, adoption, lineage, and reference-integrity gate

## Status

Done. Second and final task of batch S5-08. Recommendation only — the
agent does not approve stage/checkpoint progression (`AGENTS.md`).

## Objective

Per `project/DECISION_LOG.md#DL-8`/`DL-24`/`DL-27`, `project/gates/
stage-4-gate.md`: verify the actual integrated Stage-5 topology/raw-tier/
adoption/lineage production path (`AICAD-119`-`125`) — no architecture
boundary bypassed, examples current, required regression surfaces green —
and record material limitations honestly before the realistic/adversarial
campaign (`S5-09`) proceeds.

## Base / resulting commit

Base: this batch's own `AICAD-125` commit (`origin/claude/aicad-stage5-dev`).

## Integrated vertical-slice proof

`crates/cad-cli/tests/stage5_lineage_checkpoint.rs` (new): one real
`ParametricBuildSession` chain — construct (`box`) → inspect
(`topology_face_at`) → construct (`make_shell`) → sew → heal → inspect
(`raw_topology_kind_of`) → raw-edit (`remove_face`) → explicit adopt →
persistent reference (`generated_by`/`modified_by`, resolved through the
real resolver, fail-closed `Broken(NoMatch)` since `remove_face` creates
no new face). Every step's own real kernel property is checked (volume,
validity, face count, classified kind), not merely "it built." Each
individual capability was already proven in isolation by its own task's
own dedicated file (`stage5_sewing_healing.rs`/`stage5_topology_
inspection.rs`/`stage5_raw_editing.rs`/`stage5_raw_adoption.rs`/
`stage5_raw_lineage.rs`) — this file's own job is proving the whole chain
composes through one session without a silent architecture-boundary
break, which it does.

## No architecture boundary bypassed

- **Kernel neutrality**: every new/changed type in `AICAD-125`
  (`RawLineageChain`, `classify_raw_edit_lineage`'s inputs/outputs) is
  `cad_kernel_api`/`cad_geometry_api`-owned (`ClassifiedShape`, `GeomId`,
  `OperationReport`) — no `cad_occt_bridge`/OCCT-specific type crosses
  into `cad-query`'s or `cad-cli`'s own public surface beyond the existing,
  already-sanctioned `cad_occt_bridge::Shape`/`OcctContext` handles those
  crates already depended on before this batch.
- **Safe/raw/reference separation (D22)**: unchanged. A raw handle
  (`Value::Raw`) never itself becomes a `FeatureAnchor`-addressed
  reference; only an *adopted* `GeomId` does, through the same
  `TraceFeatureGraph`/`capture_named_feature_lineage` path every ordinary
  construction op already uses. `RawLineageIndex` is purely an internal
  evidence-accumulation aid, never itself an identity or reference type.
- **Fail-closed resolution (D7/`DL-8`)**: `cad_query::resolve`'s own
  `Resolved`/`Ambiguous`/`Broken` contract and its `GeometricFingerprint`-
  always-`Broken` rule are untouched by this batch. No new fallback,
  arbitrary-selection, or fingerprint auto-recovery path was added.
- **RuntimeBuiltin closure (D21)**: no new builtin, intrinsic, or native
  callback was added in `S5-08` — this batch is internal plumbing only.

## Kernel-neutrality / raw-handle epoch evidence

Unchanged from `AICAD-122`-`124`'s own already-established regressions
(stale-handle, wrong-context, dropped-and-rebuilt-owner rejection for
`enter_raw`/every raw-edit builtin/`adopt`), all still passing. This
batch's own new `raw_lineage` mechanism adds no new handle-identity
surface: it is keyed by `KernelShape` (already `KernelId`-generation-
scoped) and cleared every round alongside `RawShapeRegistry`, so a raw
lineage chain can never outlive, or be confused across, an epoch boundary.

## Semantic-reference adversarial evidence

`python3 scripts/ci/semantic_ref_harness.py validate`/`self-test` both
report `ok` — the harness's own silent-wrong gate is exercised and passes,
unchanged in policy by this batch (10 cases, 3 held out). No new
`SILENT_WRONG` outcome was discovered or introduced by `S5-06`-`S5-08`;
`AICAD-125`'s own report records the one real bug this batch *did* find
and fix (a cross-dispatch identity mismatch that would otherwise have
produced spuriously "generated" raw-adoption evidence) — caught before
merge by direct experiment, not shipped and later regressed. Split/merge/
delete/raw-edit/adopt adversarial coverage is recorded in `AICAD-125.md`'s
own "Adversarial evidence" section; not repeated here.

## Active examples

`examples/topology/topology_construction_basics.aicad` remains the
maintained realistic-topology teaching example (unchanged this batch — no
public syntax/behavior changed). Matching `AICAD-119`-`124`'s own
established, repeatedly-applied precedent, no raw-tier `.aicad` example
was added: every raw/query-category builtin needs a real, live
`OcctContext` to demonstrate meaningfully, so this codebase's own
convention proves those through a dedicated `crates/cad-cli/tests/`
file instead of the structurally-only-checked ACTIVE registry. This
checkpoint's own "one realistic topology/raw-adoption example execute
automatically" is satisfied by that same pairing:
`topology_construction_basics.aicad` (ACTIVE, ordinary topology) plus
`stage5_raw_adoption.rs`/`stage5_raw_lineage.rs`/`stage5_lineage_
checkpoint.rs` (raw-adoption, automatically run by `cargo test`).

## Verification

```
cargo fmt --all -- --check                                            # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings  # clean
cargo test --workspace                                                # 1811 passed, 0 failed
python3 scripts/ci/semantic_ref_harness.py validate                   # ok
python3 scripts/ci/semantic_ref_harness.py self-test                  # ok
cmake -S native/occt_bridge -B native/occt_bridge/build -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build native/occt_bridge/build -j$(nproc)
ctest --test-dir native/occt_bridge/build --output-on-failure          # 18/18 passed
cargo test -p cad-cli --test active_examples                          # 3/3 (14 ACTIVE examples)
cargo test -p cad-cli --test stage5_lineage_checkpoint                # 1/1, new this task
```

## Material limitations (honest, not hidden in a generic section)

1. **`replace_face`'s own `Modified` evidence has no known constructible
   *valid* `adopt`-through fixture** — every replacement tried (an
   independently-built congruent face, a self-duplicate of the same face)
   fails `BRepCheck_Analyzer` validity or the native `ReShape` call
   itself. This is `AICAD-123`'s own disclosed geometric-compatibility gap
   surfacing, not a new one. `Modified`/`Merged` evidence propagation is
   proven correct independently (`cad-query` unit level, built from a
   real `merge_faces` kernel call) and the production-path `Modified`
   proof uses `Sew`'s own real edge-relabeling instead, exercising the
   identical consumer code.
2. **`merge_faces`/`split_edge` results (`List<Raw>`) cannot be fed into
   `adopt` from `.aicad` source** — no list-element-selection syntax
   exists yet. Not this batch's own scope to add (a language-surface
   change, outside `S5-08`'s "integrate existing evidence" charter); noted
   here so a future task does not rediscover it from scratch.
3. Per-entity `Heal` lineage remains unavailable (`AICAD-120`).
   `AdoptionEvidence`/`OperationReport`'s own richer fields remain
   Rust-only, matching every prior raw-tier task's precedent.

None of these weaken a gate, test, or the fail-closed reference contract —
each is an honestly-reported absence of a specific positive-path fixture
or source-syntax capability, not a silently-accepted wrong result.

## Recommendation

**Recommend PASS.** The integrated production path is proven end to end,
no architecture boundary was bypassed, the semantic-reference adversarial
harness reports clean, and every required regression surface (workspace
tests, native CTest, ACTIVE examples) is green. The disclosed limitations
above are pre-existing (largely `AICAD-123`'s own) and do not represent a
regression introduced by `S5-08`. This is a recommendation only — stage
progression remains an owner decision.

## Next dependency

`S5-09` (`AICAD-127`-`129`, realistic/adversarial/example campaign)
depends on this checkpoint (satisfied).
