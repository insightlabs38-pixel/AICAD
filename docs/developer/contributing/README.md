# Contributing

AICAD development is stage-gated. Contributions should extend the current approved architecture rather than treating the long-term plan as permission to pull later-stage features forward.

## Before changing code

1. Read `AGENTS.md`.
2. Read `project/CURRENT_STAGE.md` and the assigned entry in `project/TASKS.yaml`.
3. Read current developer docs and directly relevant code/spec/RFC material.
4. Check accepted owner decisions before choosing an architectural alternative.
5. Use archived reports only when a specific historical claim needs evidence.

## Core boundaries

Do not bypass these without an explicit accepted architecture change:

- source/AICAD semantic state remains authoritative;
- no OCCT-specific type above the adapter;
- engineering quantities remain typed;
- geometry operations remain value-oriented/functional in HIR and Geometry IR;
- RuntimeBuiltin functions use ordinary call/type semantics;
- raw topology selection is ephemeral, not durable identity;
- ambiguous semantic resolution must fail closed rather than choose arbitrarily;
- compiler intrinsics require the established RFC process;
- AI/automation may not bypass compiler/runtime validation.

## Keep stages separate

On this transition branch, Stage 3 is complete but Stage-4 implementation has not begun. `AICAD-080` remains todo. Do not add semantic-reference implementation as a documentation cleanup, and do not pull Stage-5+ APIs into current docs/code.

## Tests

For Rust changes, the normal verification set is:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Add focused unit/integration/property/adversarial tests appropriate to the change. Geometry work needs exact/semantic evidence, not visual confirmation alone.

## Documentation

Update `docs/user/` when the public implemented surface changes and `docs/developer/` when current architecture/extension behavior changes. Keep future proposals under `project/planning/`; do not describe a planned syntax as current merely because it appears in `docs/plan/`.

## Task evidence and commits

Completed roadmap tasks produce a focused report/evidence record according to `AGENTS.md`. Historical Stage-0..3 reports are archived but deliberately retained.

Repository commit identity is:

```text
insightlabs38-pixel <insightlabs38@gmail.com>
```

Do not bypass repository hooks, use `--no-verify`, force-push transition/development history, or add AI/session attribution metadata to commits or pull requests.
