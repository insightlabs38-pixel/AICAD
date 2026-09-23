# AICAD-149 — immutable configuration overlays

## Result

New crate `cad-configurations` implements D29's "configuration is an
immutable semantic overlay" as a Rust IR: `Configuration` (an owned overlay
value) and `resolve` (a pure, borrow-only view against a base
`ComponentDefinitionRegistry`). 11 tests, all passing.

## Changes

- `crates/cad-configurations/src/id.rs`: `ConfigurationId` — stable named
  configuration identity, distinct from `cad_assemblies::ConfigurationSlotId`
  (a replacement slot *inside* the base assembly, `AICAD-133`) and from every
  other D26 identity domain.
- `crates/cad-configurations/src/overlay.rs`: `Configuration` carries two
  overlay contents — a flat `named_values: BTreeMap<String, ParameterValue>`
  (configuration-scoped options, e.g. `docs/plan/07`'s `ratio: Int = 10`) and
  `parameter_overrides: BTreeMap<OccurrencePath, BTreeMap<String,
  ParameterValue>>` (per-occurrence argument overrides). `resolve(registry,
  configuration) -> ResolvedConfiguration` borrows both immutably;
  `ResolvedConfiguration::parameter(occurrence, name)` returns the
  configuration's override if present, else the base `ChildInstance`'s own
  bound argument (looked up by walking `occurrence`'s parent segment through
  `registry`), else `None`. Because `resolve` only ever takes `&`
  references, "resolution never destructively mutates the base" is a
  compile-time fact (the borrow checker enforces it), not a convention.
- `crates/cad-configurations/Cargo.toml`: depends on `cad-assemblies`
  (one direction only — `cad-assemblies` has no dependency on this crate,
  preserving Checkpoint A/B's own dependency-cleanliness evidence) and
  `cad-diagnostics`.

## Decisions

- **Overlay content, not overlay syntax.** `DL-31`/D29 explicitly defer
  exact surface syntax; no `.aicad` `configuration { ... }` declaration
  exists yet. This crate is Rust-owned semantic IR only, the same
  relationship `cad_assemblies::mate`/`joint` already have to their own
  still-absent source syntax — consistent with the existing Stage-6
  pattern rather than a new one.
- **Overrides are keyed by `OccurrencePath`, never by definition/child-slot
  name.** A `ComponentDefinition`'s `children` list (and therefore its
  `ChildInstance::arguments`) is shared across every instantiation of that
  definition; occurrence-path keying is what lets two occurrences of the
  same shared definition receive independent overrides without touching
  (or duplicating) the base definition.
- Provenance/logical identity are preserved structurally: an override never
  removes or replaces a base `ChildInstance`; it only takes priority when
  present, and the base value stays reachable through the registry exactly
  as before (`resolving_an_overlay_never_mutates_the_base_registry` asserts
  the base argument directly, not just that resolution "looks" pure).

## Verification

- `cargo test -p cad-configurations` — 11/11 PASS.
- `cargo fmt -p cad-configurations -- --check` — PASS.
- `cargo clippy -p cad-configurations --all-targets --all-features -- -D warnings` — PASS.

## Limitations

- No `.aicad` surface syntax for declaring a configuration or its overrides
  — Rust-IR only, matching AICAD-139/140's own precedent and D29/DL-31's
  explicit deferral.
- Named values reuse `ParameterValue` (a typed engineering quantity); there
  is no enum-valued configuration option type yet (`docs/plan/07`'s
  `MotorSize`/`HousingMaterial` examples), since no `enum`-shaped D26
  identity or language surface for it is approved in this stage — adding
  one would be new public language semantics, not this task's scope.

## Next

`AICAD-150` (configuration validity rules, evaluated over `named_values`)
and `AICAD-151` (suppression, extending `Configuration` with a suppressed-
occurrence overlay) build directly on this module.
