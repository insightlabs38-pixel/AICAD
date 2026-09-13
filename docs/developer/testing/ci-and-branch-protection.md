# CI layers and branch-protection recommendations

AICAD uses layered CI so routine pull requests receive fast feedback while expensive hardening remains scheduled/manual.

## Required pull-request checks

Configure branch protection on `main` and, while active, `claude/aicad-stage4-dev` to require these stable job names:

- `CI / fmt and repository metadata`
- `CI / clippy`
- `CI / workspace build and unit tests`
- `CI / native OCCT bridge`
- `CI / compiler/runtime/exact-geometry smoke`
- `Integration and determinism / exact geometry and STEP`
- `Integration and determinism / deterministic AICAD-owned state`

When a change touches the semantic-reference corpus/harness paths, also require:

- `Stage-4 semantic-reference harness / corpus and harness contract`
- `Stage-4 semantic-reference harness / frozen corpus exact geometry`

GitHub branch protection itself is an owner/repository setting. AICAD-079C only documents the recommendation; it does not claim to configure it.

## Optional/heavy checks

The following are intentionally not routine blockers:

- `Supported platforms` — current Tier-1 Linux validation;
- `Performance baseline` — measurement artifact, no noisy threshold;
- `Dependency and supply-chain checks / Rust advisory audit` — networked advisory database check on schedule/manual runs;
- `Nightly hardening` — ASan, UBSan, bounded parser fuzzing, bounded property invariants, and determinism repetition;
- `Release foundation` — manual/tag build/checksum artifact only, never a publication step.

## Current platform policy

Current repository installation and CI evidence is Linux-first and specifically uses Debian/Ubuntu OCCT packages. Therefore **Linux on GitHub-hosted Ubuntu is Tier 1 full build/test**. AICAD-079C does not claim Windows or macOS support that the repository has not yet evidenced. Those platforms should be added as Tier 2 compile/smoke or promoted to Tier 1 only after their OCCT bootstrap, native bridge, workspace build, and representative exact-geometry tests are reproducible.

Do not weaken Linux native/exact-geometry validation merely to make a future cross-platform matrix uniform.

## Failure artifacts

Integration jobs retain concise logs and useful generated STEP evidence on failure where available. Performance runs retain benchmark JSON. Semantic-reference result evidence should be uploaded by future resolver jobs once a real resolver adapter exists. Never upload secrets, dependency caches, or whole build trees as failure artifacts.

## Cost/control policy

Routine PR checks use parallel jobs and dependency/build caches where appropriate. Fuzzing, sanitizers, repeated determinism stress, performance measurements, platform verification, and networked advisory audits stay scheduled/manual. Cache keys include OS plus the Rust toolchain, lockfile, and native bridge build definition so incompatible native/toolchain state is not silently reused.
