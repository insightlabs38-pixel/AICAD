# AICAD-136 — nested-assembly dependency graph and cycle diagnostics

## Result

`cad-assemblies::graph::expand` deterministically resolves a `ComponentDefinitionRegistry` rooted at one `LogicalInstanceId` into every nested `Occurrence` (path + composed `WorldPose`), detecting definition-level cycles and undefined-definition references and reporting both as structured `ASM`-family diagnostics rather than recursing without termination or panicking.

## Changes

- `graph.rs` (new): `Occurrence { path: OccurrencePath, world_pose: WorldPose }`; `expand(registry, root_instance, root_local_pose) -> Result<Vec<Occurrence>, AssemblyGraphError>` — a pre-order walk of each `ComponentDefinition::children` `Vec` (declaration order, never `HashMap`/`HashSet`), composing `ChildInstance::local_pose` into a running `WorldPose` at each level.
- Cycle detection tracks an `on_stack: Vec<ComponentDefinitionId>` during the walk (mirroring `cad_compiler::loader::Loader::load_file`'s own module-import-cycle detection exactly) — it is definition-level, not occurrence-level, since an occurrence path can never revisit itself by construction but two definitions can reference each other.
- `AssemblyGraphError::{UndefinedDefinition, CyclicDefinition { chain }}`, each with `to_diagnostic()` producing `ASM-E001`/`ASM-E002` (`specs/language/diagnostics.md`'s already-reserved `assembly` family) with the full cyclic chain in the message, not just "a cycle exists."

## Verification

- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS
- `cargo test -p cad-assemblies` — 65/65 PASS (53 unit incl. 7 new `graph::tests`, `component_ir.rs` 4/4, `identity_domains.rs` 6/6, `instance_pose.rs` 2/2, new `nested_assembly.rs` 2/2)
- `cargo test --workspace` — PASS (100 test-result groups, all `ok`)

## Limitations

- `expand`'s recursion has no explicit depth/size budget beyond the cycle check itself — adversarial deep-nesting/resource-limit behavior is `AICAD-158`'s scope, not this task's.
- No cross-instance semantic-reference addressing over the resolved tree yet (`AICAD-137`).

## Next

Batch `S6-02` (`AICAD-134`/`135`/`136`) is complete. `AICAD-137` (assembly-level semantic-reference addressing, batch `S6-03`) depends on `AICAD-133` and this task and is the next executable task.
