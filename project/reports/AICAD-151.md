# AICAD-151 — suppression with stable logical identity

## Result

`Configuration` (`overlay.rs`) gains a `suppressed: BTreeSet<OccurrencePath>`
overlay field; `crates/cad-configurations/src/suppression.rs` derives
active/inactive occurrence, mate, and quantity views from it. Suppression
is membership in this set, never removal from any base structure. 34/34
crate tests pass (11 new).

## Changes

- `Configuration::suppress`/`unsuppress`/`is_directly_suppressed`/
  `suppressed_paths` — insert/remove/query `OccurrencePath` membership
  only; no other field of `Configuration` or of the base
  `ComponentDefinitionRegistry`/`Vec<Occurrence>` is ever touched.
- `suppression::is_active(resolved, path)` — false if `path` or any strict
  ancestor is suppressed (a suppressed subassembly's children are inactive
  too, since a child's world pose is only meaningful composed under its
  parent).
- `active_occurrences`/`suppressed_occurrences` — the base occurrence set
  partitioned by `is_active`, still the same `Occurrence` values.
- `classify_mates(resolved, mates) -> MateActivation` — a `Mate` is active
  only if *both* `Mate::subjects()` occurrences are active; otherwise it is
  reported (not dropped) as `inactive`.
- `active_definition_counts` — a minimal deterministic per-definition
  active-occurrence tally, explicitly documented as *not* the BOM engine
  (`AICAD-154`'s job).

## Decisions / evidence

- **References compose with suppression for free.** No suppression-aware
  code was added to `cad_assemblies::reference::resolve`. Passing
  `active_occurrences`'s filtered slice into that existing function is
  enough: a reference into a suppressed occurrence already reproduces
  `AssemblyBrokenReason::OccurrenceNotFound` (`AICAD-137`'s own
  fail-closed check), and an active occurrence with no wired resolver
  produces the differently-attributed `Entity(InsufficientEvidence)` —
  `a_reference_into_a_suppressed_occurrence_fails_closed_as_occurrence_not_found`
  and `a_reference_into_an_active_occurrence_reaches_ordinary_entity_resolution`
  assert both outcomes are distinguishable.
- **Reactivation identity.** `reactivation_restores_the_identical_logical_subject`
  suppresses then unsuppresses the same `OccurrencePath` and asserts it is
  still `==` the path independently rebuilt from the (never-touched) base
  occurrence tree — reactivation is structurally guaranteed, not merely
  tested for the happy path, since `suppress`/`unsuppress` never construct
  or mutate an `OccurrencePath` at all.
- **Suppressed subjects remain traceable.** They stay present, at their own
  stable path, in the base `Vec<Occurrence>` and in `suppressed_occurrences`
  — never deleted, only excluded from the active view.

## Verification

- `cargo test -p cad-configurations` — 34/34 PASS.
- `cargo fmt --all -- --check` — PASS.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS.
- `cargo test --workspace` — 104/104 test binaries PASS (no regressions in
  `cad-assemblies`/`cad-assembly-solver`; `cargo tree -p cad-assemblies`
  still shows no dependency on this crate).

## Limitations

- No `.aicad` `@suppress_if(...)`/`if` surface syntax (`docs/plan/07` §12)
  — Rust-IR only, matching `AICAD-149`/`150`.
- `active_definition_counts` is a minimal proof aggregate, not real BOM
  semantics (classification identity, purchasing equivalence, nested
  quantity multiplication) — that is `AICAD-154`'s own scope.

## Next

`S6-07` is complete. `AICAD-152` (batch `S6-08`, replacement/variant
selection through stable logical slots) is next, building on
`ConfigurationSlotId` (`AICAD-133`) and this batch's overlay/suppression
shape.
