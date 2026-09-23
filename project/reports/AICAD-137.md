# AICAD-137 — assembly-level semantic-reference addressing

## Result

`cad-assemblies::reference` resolves an `OccurrenceTopologyRef` (occurrence path + Stage-4 `AnyRef` recipe) against a live occurrence tree by first checking the addressed occurrence still exists, then delegating entity resolution to Stage-4's own fail-closed `cad_query::resolve_reference` — reusing the existing D7 `Resolved`/`Ambiguous`/`Broken` mechanism rather than inventing a second one.

## Changes

- `reference.rs` (new): `AssemblyPartResolver<'ctx>` (supplies a per-`ComponentDefinitionId` `cad_query::ResolverContext`); `resolve(occurrences, reference, parts) -> Result<AssemblyResolutionOutcome<'ctx>, ResolveError>`; `resolve_with_durability` pairs the outcome with the recipe's static `DurabilityLevel` (mirrors `cad_query::resolve_reference_with_durability`).
- `AssemblyBrokenReason::{OccurrenceNotFound(OccurrencePath), Entity(cad_query::BrokenReason)}` — a new `ASM-E003` diagnostic for the former (a genuinely new assembly-level failure mode); the latter reuses Stage-4's already-established `REF-E101 BROKEN_REFERENCE` diagnostic unchanged.
- `cad-assemblies` now depends on `cad-query` (no existing cycle: `cad-query` does not depend on `cad-assemblies`) and, as a dev-dependency, `cad-occt-bridge` for real (non-mocked) box-geometry test fixtures, matching `cad_query::resolve`'s own test precedent.
- Occurrence lookup is by `OccurrencePath` equality only; since `LogicalInstanceId` (and therefore every path segment) embeds its own `ComponentDefinitionId`, any edit that changes which definition an occurrence names structurally changes its path — the lookup can never silently rebind a reference to a renamed/replaced slot.

## Verification

- `cargo fmt --all -- --check` — PASS
- `cargo clippy -p cad-assemblies --all-targets --all-features -- -D warnings` — PASS
- `cargo test -p cad-assemblies` — 82/82 PASS (61 pre-existing + 8 new `reference::tests`, plus the four existing integration suites unchanged)
- `cargo build --workspace` / `cargo test --workspace` — PASS, no regressions

## Limitations

- `AssemblyPartResolver` is keyed by `ComponentDefinitionId` only (one evidence source per definition, shared by every occurrence of it) — correct for the current geometry-less `ComponentDefinition` IR, since no per-occurrence realized geometry exists yet; wiring real per-occurrence compiled parts is out of this task's scope.
- No diagnostic exists yet for `AssemblyBrokenReason::Entity` beyond reusing the unscoped Stage-4 `REF-E101` shape (no assembly-specific context, e.g. the occurrence path, is embedded in that diagnostic's own fields) — acceptable since Stage-4's diagnostic already carries `entity`, and richer assembly-aware diagnostics are not required by this task's acceptance criteria.

## Next

`AICAD-138` (reusable mechanical-interface semantics, batch `S6-03`) depends on `AICAD-132` and this task.
