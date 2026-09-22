# Contributing to AICAD

AICAD is pre-1.0 and uses stage-gated development. Contributions should preserve the current semantic contracts rather than bypassing them for a local implementation shortcut.

## Before changing code

1. Read `AGENTS.md` and the relevant current user/developer documentation.
2. Check `project/CURRENT_STAGE.md` for the currently authorized stage and execution boundary.
3. For semantic or architectural changes, read the relevant accepted RFCs and `project/OWNER_DECISIONS.md` / `project/DECISION_LOG.md`.
4. Keep kernel-specific/native identity below the adapter boundary and preserve fail-closed semantic-reference behavior.

Do not implement roadmap-stage work that is not currently authorized.

## Local validation

For Rust/product changes, the normal baseline is:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Run task-specific/native/reference/example checks required by the area you changed. Do not weaken tests or gates to make a change pass.

For documentation-only changes, at minimum check links/commands you touched and run repository documentation validation where available.

## Examples and documentation

Public syntax or modeling changes must update affected ACTIVE examples and current-facing user/developer documentation in the same change. Historical reports under `project/` are evidence records; do not rewrite them to match current prose.

Use canonical names: **AICAD**, `.aicad`, and `.aicadpkg` only where the native-artifact role actually applies. Do not present kernel pointers/indices as semantic identity.

## Reporting bugs

Use the repository's GitHub issue tracker for reproducible bugs. Include the AICAD commit/version, platform/toolchain, minimal `.aicad` input where possible, command invoked, expected behavior, and actual diagnostic/output.

For security-sensitive reports, follow [`SECURITY.md`](SECURITY.md) rather than publishing exploit details in a normal issue.
