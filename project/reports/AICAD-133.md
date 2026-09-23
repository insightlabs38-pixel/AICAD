# AICAD-133 — assembly semantic identity primitives

## Result

Populated the previously-empty `cad-assemblies` crate with the seven D26 identity-domain primitives as distinct Rust newtypes: component-definition, logical-instance, nested-occurrence/path, configuration-slot, external-asset, semantic-topology, and BOM-classification identity. No assembly definition/instance/nesting IR exists yet (`AICAD-134` onward) — this task fixes only the identity *shape*: each domain is a separate type, deterministically constructed from stable source-level data, never from a topology index, solver variable, native handle, filesystem path, or BOM row.

## Identity domains

- `ComponentDefinitionId` — wraps a stable declared source name (no module system exists yet, so today's identity is that name; a future multi-module namespace would qualify it, which is new scope).
- `LogicalInstanceId` — a `ComponentDefinitionId` plus its own stable local binding/slot name; two instantiations of the same definition are distinguished by that local name, never a build-order counter. Carries no pose/transform field at all, so a pose change cannot possibly change this type.
- `OccurrencePath` — a non-empty, root-to-leaf `Vec<LogicalInstanceId>` built only through explicit `root`/`child` calls, never container iteration, so traversal order can never become semantic identity.
- `ConfigurationSlotId` — an `OccurrencePath` plus a declared slot name; never stores the currently-bound definition, so swapping a slot's bound definition (`AICAD-152`'s own worked example) never changes the slot's own identity.
- `ExternalAssetId` — a deterministic digest (hand-rolled FNV-1a 64-bit, following `cad_feature_graph::cache::StableHasher`'s own precedent rather than adding a hashing dependency) over caller-supplied `AssetProvenance { format, content }`. There is no `from_path` constructor anywhere in the module.
- `OccurrenceTopologyRef` — composes `OccurrencePath` with `cad_references::AnyRef` (the existing Stage-4 fail-closed reference recipe) without collapsing them: two occurrences of the same definition share an identical `AnyRef` recipe but differ in occurrence, so they never collapse to the same reference.
- `BomClassificationId` — a bare stable key string, with no `From`/`Into` to or from any other domain type.

Each domain derives `PartialEq, Eq, Hash, PartialOrd, Ord` (`OccurrenceTopologyRef` only `PartialEq`, since it wraps `cad_references::AnyRef`, which itself has no broader derive) and a hand-rolled `to_json()` using the existing dependency-free `cad_diagnostics::json::Json` canonical serializer (the established repo convention — no serde anywhere in this workspace).

## Verification

- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS
- `cargo test -p cad-assemblies` — 37/37 PASS: per-domain determinism/distinctness/serialization unit tests, plus an `identity_domains.rs` integration file proving the domains stay distinct *together* (four repeated `Wheel` instances under one `Gearbox` are four distinct occurrences sharing one definition identity; independently-rebuilt deep nesting reproduces an identical path; two occurrences of the same definition never collapse to the same topology reference; a configuration slot's identity is unchanged by which definition is currently bound; two notionally different import paths with identical bytes collapse to one external-asset identity; no domain is interchangeable with another even given the same source name).
- `cargo test --workspace` — PASS (0 failures workspace-wide).

## Limitations

- This is identity-primitives only — no definition/instance/nesting graph, no dependency/cycle detection, no real occurrence traversal over an actual assembly. That is `AICAD-134`/`AICAD-136`'s job.
- `ConfigurationSlotId` operationalizes D26's "configuration/variant identity" as the stable *slot* `AICAD-152`'s own replacement design needs; a configuration's own identity (which named configuration is active) is not yet defined — expected from `AICAD-149`/`150`.
- `ExternalAssetId`'s content digest is a placeholder shape; real import provenance capture (decoded-geometry checksums, source-tool metadata) is `AICAD-153`.

## Next

`AICAD-134` (component-definition/logical-instance semantic IR, batch S6-02) depends on both this task and `AICAD-132` and is the next executable task.
