# AICAD-079C — Stage-4 CI/CD infrastructure expansion and transition finalization

Status: **COMPLETE — READY FOR OWNER REVIEW/MERGE; Stage 4 is not implemented**

## Scope and authority

AICAD-079C is the final Stage-3 -> Stage-4 transition/infrastructure task. It expands the repository's validation, hardening, benchmark plumbing, and transition governance without implementing Stage-4 semantic-reference behavior.

- canonical branch: `claude/aicad-stage4-transition`
- AICAD-079C base: `9e2db88cbbfe9097364fcbfc3ab8115b07b63efd`
- executable/readiness validation head: `0e51b78e4f555c6ef5a5faa4b55289d66cd690fb`
- required commit identity: `insightlabs38-pixel <insightlabs38@gmail.com>`

This task does **not** authorize or implement AICAD-080. It does not add persistent `VertexRef` / `EdgeRef` / `WireRef` / `FaceRef` / `ShellRef` / `SolidRef` behavior, a semantic-reference resolver, persistent topology matching, lineage resolution, authoritative fingerprint recovery, Stage-4 source syntax, Stage-5 behavior, or AICAD-101+ work.

## Delivered infrastructure

### Required CI

`.github/workflows/ci.yml` now provides bounded, read-only required feedback for the transition, `main`, and the future Stage-4 development branch:

- `cargo fmt --all -- --check`;
- Python CI-helper syntax validation;
- Stage-4 task metadata audit;
- transition-wide `git diff --check` against the AICAD-079C base;
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
- `cargo build --workspace --all-targets --locked`;
- full `cargo test --workspace --locked`;
- native OCCT bridge configure/build/CTest;
- focused Stage-2 source-to-kernel, Stage-3 incremental-build, and frozen Stage-4 corpus smoke tests.

### Exact geometry, STEP, and determinism

`.github/workflows/integration.yml` separates exact-geometry evidence from AICAD-owned determinism. The exact-geometry lane covers Stage-2 end-to-end behavior, Stage-3 ordinary parts, Stage-3 incremental rebuild, the D5/D19 validation profile, the frozen Stage-4 corpus, and a representative STEP build. The determinism lane covers the dedicated Stage-4 determinism foundation, feature/dependency determinism, and incremental-state determinism.

The determinism contract intentionally does **not** define byte-identical STEP/B-rep output or kernel topology enumeration order as AICAD identity. AICAD-owned state is compared deterministically; exact geometry is evaluated under the accepted D5/D19 semantic/numerical equivalence policy.

### Semantic-reference benchmark harness

The frozen AICAD-079A corpus remains unchanged as the pre-implementation ground truth. `scripts/ci/semantic_ref_harness.py` and `tests/semantic_refs/` add resolver-independent benchmark plumbing without inventing a resolver API.

The grader distinguishes:

- `RESOLVED_CORRECT`;
- `AMBIGUOUS`;
- `BROKEN`;
- `SILENT_WRONG`;
- `KERNEL_FAILURE`;
- `UNRELATED_FAILURE`.

It validates the corpus structure, expected classifications, held-out manifest/checksums, and regression-record policy. It can grade future structured resolver results against frozen ground truth, but it does not synthesize or substitute resolution behavior.

Every future `SILENT_WRONG` result is catastrophic: it must be minimized and preserved permanently under `tests/semantic_refs/regressions/` rather than weakening expected behavior or silently accepting a different target.

`.github/workflows/semantic-refs.yml` runs the corpus/harness contract and the frozen exact-geometry fixture test separately, with explicit job timeouts. D7 remains authoritative: fingerprints may support evidence, ranking, benchmarks, and experiments, but may not automatically convert `Ambiguous` or `Broken` into `Resolved` in the first Stage-4 implementation.

### Scheduled/manual hardening

The transition adds bounded foundations for later hardening without claiming unsupported coverage:

- `nightly.yml`: native ASan/UBSan, bounded parser fuzzing, spatial/property invariants, and determinism repetition;
- `fuzz/`: parser-only fuzz target; no untrusted geometry execution is introduced;
- `platforms.yml`: Tier-1 Linux/Ubuntu validation; Windows/macOS support is not claimed without evidence;
- `performance.yml` plus `scripts/ci/performance_baseline.py`: controlled implemented-capability measurements and future resolver extension points;
- `security.yml`: lock/workspace dependency policy and scheduled/manual Rust advisory checks;
- `release.yml`: build/package/checksum artifact foundation only, with no publishing, signing, or release creation.

Branch-protection recommendations are documented in `docs/developer/testing/ci-and-branch-protection.md`. Repository branch protection is an owner-side setting and is **not** claimed configured by AICAD-079C.

## Stage-4 queue and governance corrections

`project/TASKS.yaml` now records AICAD-079C as the final transition task. The existing Stage-4 IDs remain stable.

- AICAD-080 remains `todo` and depends on AICAD-079C.
- AICAD-092 is explicitly fingerprint evidence/ranking/benchmark support only; automatic authoritative recovery remains prohibited by D7/DL-8 unless a later owner ruling changes that policy.
- AICAD-096 extends/consumes the already-frozen AICAD-079A corpus instead of recreating or rewriting the baseline.
- AICAD-100 remains the Stage-4 owner hard gate.
- No AICAD-101+ task is promoted.

`project/CURRENT_STAGE.md`, `project/SESSION_HANDOFF.md`, and the Stage-3 -> Stage-4 transition records now state the intended terminal transition state: **transition complete pending owner review/merge; Stage 4 READY, NOT IMPLEMENTED**.

After owner acceptance, the required flow is:

1. merge `claude/aicad-stage4-transition` to `main`;
2. create `claude/aicad-stage4-dev` from the exact merged `main` HEAD;
3. synchronize sequential Stage-4 work on that branch;
4. explicitly authorize/start AICAD-080 there.

AICAD-079C does not create that branch and does not begin AICAD-080.

## Fresh executable validation

All final executable validation below was run by GitHub Actions on the same readiness head:

`0e51b78e4f555c6ef5a5faa4b55289d66cd690fb`

### CI — run 34788644325 — PASS

Workflow conclusion: **success**.

Successful required commands/checks included:

```text
cargo fmt --all -- --check
python -m py_compile scripts/ci/*.py
python scripts/ci/stage4_task_audit.py --check
git diff --check 9e2db88cbbfe9097364fcbfc3ab8115b07b63efd HEAD
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build --workspace --all-targets --locked
cargo test --workspace --locked
cmake -S native/occt_bridge -B native/occt_bridge/build -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build native/occt_bridge/build --parallel 2
ctest --test-dir native/occt_bridge/build --output-on-failure
cargo test -p cad-cli --test stage2_end_to_end --locked -- --test-threads=1
cargo test -p cad-cli --test stage3_parametric_incremental_rebuild --locked -- --test-threads=1
cargo test -p cad-cli --test stage4_reference_benchmark_fixtures --locked -- --test-threads=1
```

All five CI jobs completed successfully: repository metadata/formatting, clippy, workspace build/tests, native OCCT bridge, and compiler/runtime/exact-geometry smoke.

### Integration and determinism — run 34788644330 — PASS

Workflow conclusion: **success**.

Successful validation included:

```text
cargo test -p cad-cli --test stage2_end_to_end --locked -- --test-threads=1
cargo test -p cad-cli --test stage3_ordinary_parts --locked -- --test-threads=1
cargo test -p cad-cli --test stage3_parametric_incremental_rebuild --locked -- --test-threads=1
cargo test -p cad-validation --locked
cargo test -p cad-cli --test stage4_reference_benchmark_fixtures --locked -- --test-threads=1
cargo run -p cad-cli --locked -- build examples/brackets/stage3_l_bracket.aicad --output artifacts/stage3_l_bracket.step --name LBracket.body --json
cargo test -p cad-cli --test stage4_determinism_foundation --locked -- --test-threads=1
cargo test -p cad-feature-graph --locked
```

Both integration jobs completed successfully: exact geometry/STEP and deterministic AICAD-owned state.

### Stage-4 semantic-reference harness — run 34788644399 — PASS

Workflow conclusion: **success**.

Successful checks included:

```text
python scripts/ci/semantic_ref_harness.py validate
python scripts/ci/semantic_ref_harness.py self-test
cargo test -p cad-cli --test stage4_reference_benchmark_fixtures --locked -- --test-threads=1
```

Both semantic-reference jobs completed successfully: corpus/harness contract and frozen-corpus exact geometry.

## Failures found and remediated during the pass

AICAD-079C did not waive failing gates.

1. An early infrastructure head exposed a lockfile inconsistency after the new test-only dependency was introduced. `Cargo.lock` was corrected before `--locked` validation was accepted.
2. Readiness head `dcd41ee42d13522be4979bdd0b3f434fa645e902` failed `cargo fmt --all -- --check`. The repository's pinned formatter was run, producing formatting-only commit `7cef92fafd97e8a2a75c6a107d7067c59ca82117` affecting only the two new Rust test files. The temporary write-enabled formatter workflow used for that remediation was then removed. Final formatting validation is green.
3. The first dedicated semantic-reference workflow run on initial infrastructure commit `3ea07e607c84ca8a8c87333aebd65afa2c56cdc7` had a failing frozen-fixture Cargo lane on the pre-correction tree, although its Python corpus/harness lane passed. That run is not used as final evidence. A fresh dedicated run on `0e51b78e4f555c6ef5a5faa4b55289d66cd690fb` is fully green.

## Scope audit

The AICAD-079C delta is limited to CI/workflow infrastructure, CI helper scripts, fuzz/test foundations, testing/performance documentation, task metadata, transition/readiness governance, a `cad-cli` dev-dependency/lock update, and this evidence report.

No production semantic-reference/resolver implementation was introduced. No Stage-5 implementation was introduced. No post-100 task was promoted. No future Stage-4 development branch was created. No force push or automatic merge was performed.

## Final disposition

**AICAD-079C: COMPLETE.**

The Stage-3 -> Stage-4 transition is **ready for owner review and merge**. Stage 4 is **READY, NOT IMPLEMENTED**. AICAD-080 remains TODO and must not begin until the transition is merged, `claude/aicad-stage4-dev` is created from the exact merged `main` HEAD, and the owner explicitly opens Stage-4 implementation.
