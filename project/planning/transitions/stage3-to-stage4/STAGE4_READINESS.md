# Stage-4 readiness after AICAD-079C

Status: **READY FOR OWNER REVIEW/MERGE — Stage 4 not implemented**.

AICAD-079C is the final pre-Stage-4 transition/infrastructure task. It prepares verification and repository governance so AICAD-080..100 can add semantic-reference behavior without inventing CI/test infrastructure mid-stage.

## Hard-gate contract

Stage 4's catastrophic failure mode is **silent wrong selection**. The accepted D7 outcome contract remains:

- `Resolved(exactly one intended entity)` — good;
- `Ambiguous(candidates + evidence)` — good;
- `Broken(reason/evidence)` — acceptable/expected when the intended entity no longer exists or cannot be resolved safely;
- `SILENT_WRONG` — catastrophic.

Never use arbitrary first-candidate selection, raw topology enumeration/order fallback, hidden kernel pointer identity, or silent fingerprint recovery. Fingerprint information may be diagnostic evidence, ranking input, or benchmark information only until a later explicit owner ruling supported by Stage-4 evidence authorizes more.

Every discovered silent wrong selection becomes a minimized permanent regression under `tests/semantic_refs/regressions/`.

## Infrastructure ready for AICAD-080+

- fast required CI for format/repository metadata, clippy, workspace build/unit tests, native OCCT CTest, and focused compiler/runtime/exact-geometry smoke tests;
- exact-geometry integration through source -> compiler/runtime -> Geometry IR/feature execution -> kernel-neutral adapter -> OCCT -> valid B-rep -> STEP where applicable;
- D5/D19-aware determinism checks for canonical AICAD-owned state plus engineering equivalence rather than B-rep/STEP byte identity;
- resolver-independent grading of the frozen AICAD-079A public/held-out corpus using six explicit outcome classes;
- permanent silent-misselection regression records;
- bounded parser fuzzing, native ASan/UBSan coverage, bounded deterministic property-invariant sweeps, and repeated determinism stress on scheduled/manual layers;
- explicit Tier-1 Linux validation without unsupported Windows/macOS claims;
- measurement-only performance baseline with future resolver/candidate-set extension points;
- lock/workspace dependency policy plus scheduled/manual Rust advisory audit;
- manual/tag release-build, artifact naming, and SHA256 foundation with no publication/signing credentials;
- concise failure artifact retention and documented branch-protection recommendations.

## Stage-4 queue audit

AICAD-079C is durably represented as the final transition task. AICAD-080 depends on it. IDs AICAD-080..100 are not renumbered. AICAD-092 is explicitly fingerprint evidence/ranking/benchmark support without automatic recovery, and AICAD-096 extends the already-frozen AICAD-079A corpus rather than recreating it. AICAD-100 remains the owner hard gate.

## Scope guard

AICAD-079C implements no `VertexRef`/`EdgeRef`/`WireRef`/`FaceRef`/`ShellRef`/`SolidRef` semantics, resolver, matching, lineage algorithm, automatic fingerprint fallback, or Stage-4 source API. It also implements no D21-D30 Stage-5/6 capabilities and promotes no AICAD-101+ work.

## Branch handoff

After owner acceptance and merge of the transition branch, create `claude/aicad-stage4-dev` from exact merged `main` HEAD. All sequential Stage-4 agents must synchronize to newest `origin/claude/aicad-stage4-dev`; do not recreate work independently from another `main` snapshot.

## Commit identity note

A record cannot contain the SHA of the commit that contains that same record without changing that SHA. Therefore the exact **validated content head immediately before the final evidence/governance stamp** is recorded in `project/reports/AICAD-079C.md`, while the final authoritative transition HEAD is the tip of `refs/heads/claude/aicad-stage4-transition` and is reported after publication. This avoids inventing a false self-referential commit SHA.
