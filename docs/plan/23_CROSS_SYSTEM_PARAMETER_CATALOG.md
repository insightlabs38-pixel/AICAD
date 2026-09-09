# 23 — Cross-System Parameter Catalog

`04_HIGH_LEVEL_MODELING_API.md` and `05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` contain the detailed geometry-function parameter tables. This file covers major **non-geometry** APIs so implementation teams have consistent contracts.

## 1. `mate()`

```text
mate(kind, a, b, offset?, angle?, flip?, priority?, tolerance?) -> Mate
```

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `kind` | `MateKind` | required | coincident/concentric/etc. |
| `a` | semantic entity/interface | required | first reference |
| `b` | semantic entity/interface | required | second reference |
| `offset` | `Length?` | 0 where applicable | separation |
| `angle` | `Angle?` | 0 where applicable | angular offset |
| `flip` | `Bool` | false | reverse orientation branch |
| `priority` | `ConstraintPriority` | normal | solver preference/diagnostics |
| `tolerance` | quantity | project default | satisfaction tolerance |

## 2. `joint()`

```text
joint(type, parent, child, axis/frame?, limits?, home?, friction?, metadata?) -> Joint
```

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `type` | `JointType` | required | revolute/prismatic/etc. |
| `parent` | `InstanceRef/FrameRef` | required | parent side |
| `child` | `InstanceRef/FrameRef` | required | child side |
| `axis`/`frame` | ref | type-dependent | joint geometry |
| `limits` | range | unlimited | allowed motion |
| `home` | quantity | 0 | reference state |
| `friction` | model? | none | later dynamic use |
| `metadata` | map | empty | domain extension |

## 3. `instance()`

```text
instance(component, name?, configuration?, initial_transform?, suppressed?) -> Instance
```

| Parameter | Type | Default |
|---|---|---|
| `component` | Part/Component/Assembly | required |
| `name` | String | generated stable local name |
| `configuration` | configuration selection | default |
| `initial_transform` | Transform | identity |
| `suppressed` | Bool | false |

## 4. Requirement declaration

```text
requirement(id, description, assert, severity?, source?, rationale?, applies_to?, verification?)
```

| Parameter | Type | Default |
|---|---|---|
| `id` | String | required |
| `description` | String | required |
| `assert` | Bool/verification expression | required |
| `severity` | info/warn/error/blocker | error |
| `source` | URI/String | none |
| `rationale` | String | none |
| `applies_to` | configuration/query | all |
| `verification` | geometry/test/simulation/manual/etc. | inferred |

## 5. Test declaration

```text
test(name, body, tags?, configurations?, timeout?, required_profile?)
```

| Parameter | Type | Default |
|---|---|---|
| `name` | String | required |
| `body` | test block | required |
| `tags` | Set<String> | empty |
| `configurations` | selector | active config |
| `timeout` | Time | project default |
| `required_profile` | BuildProfile | standard |

## 6. `query()`

```text
query(scope, entity_type, predicates, ordering?, cardinality?, durability_policy?) -> Query<T>
```

| Parameter | Type | Default |
|---|---|---|
| `scope` | Shape/Part/Assembly | required |
| `entity_type` | entity enum/type | inferred |
| `predicates` | list/expression | required |
| `ordering` | ranking | none |
| `cardinality` | any/one/exact/range | any |
| `durability_policy` | strong/allow_geometric/raw | strong |

## 7. `validate()`

```text
validate(target, level?, checks?, tolerance?, healing_allowed?) -> ValidationReport
```

| Parameter | Type | Default |
|---|---|---|
| `target` | Shape/Part/Assembly | required |
| `level` | basic/standard/strict/release | standard |
| `checks` | set | profile-derived |
| `tolerance` | Length | project tolerance |
| `healing_allowed` | Bool | false during validation |

Validation should not silently heal unless explicitly requested; healing is a separate transformation.

## 8. `heal()`

```text
heal(shape, profile?, target_tolerance?, max_tolerance?, operations?) -> HealingResult
```

| Parameter | Type | Default |
|---|---|---|
| `shape` | Shape | required |
| `profile` | HealingProfile | default |
| `target_tolerance` | Length | project |
| `max_tolerance` | Length | policy |
| `operations` | set | profile-derived |

## 9. `import_step()`

```text
import_step(path, units?, heal?, preserve_metadata?, coordinate_policy?, naming_policy?) -> ImportedModel
```

| Parameter | Type | Default |
|---|---|---|
| `path` | asset path | required |
| `units` | explicit/infer | infer |
| `heal` | HealingProfile? | inspect-only/no mutation |
| `preserve_metadata` | Bool | true |
| `coordinate_policy` | preserve/normalize | preserve |
| `naming_policy` | preserve/generate | preserve |

## 10. `export_step()`

```text
export_step(target, path, configuration?, units?, include_metadata?, include_pmi?, heal?, validate?, fidelity_report?)
```

| Parameter | Type | Default |
|---|---|---|
| `target` | Part/Assembly | required |
| `path` | output path | required |
| `configuration` | selection | active |
| `units` | unit | project/preferred |
| `include_metadata` | Bool | true |
| `include_pmi` | Bool | true when supported |
| `heal` | Bool/profile | false unless needed |
| `validate` | Bool | true |
| `fidelity_report` | Bool | true in release profile |

## 11. `render()`

```text
render(target, camera?, projection?, style?, selection?, section?, resolution?, background?) -> RenderArtifact
```

Parameters:

- `camera`: named/explicit frame;
- `projection`: perspective/orthographic;
- `style`: shaded/edges/xray/wireframe/analysis overlay;
- `selection`: semantic refs to highlight;
- `section`: optional clipping/section plane;
- `resolution`: pixel size;
- `background`: presentation only.

Rendering is evidence/UX, not the authoritative geometry computation.

## 12. `inspect()`

```text
inspect(target, fields?, depth?, configuration?, include_raw?) -> InspectionRecord
```

Common `fields`:

```text
parameters
bounds
mass
volume
area
topology
features
constraints
references
lineage
provenance
dependencies
requirements
manufacturing
analysis
```

`include_raw=false` by default to prevent AI/human tooling from depending on backend-specific IDs.

## 13. `optimize()`

Core fields:

| Field | Type | Meaning |
|---|---|---|
| `variables` | variable/range definitions | design space |
| `objectives` | one or more expressions | min/max targets |
| `constraints` | assertions | feasibility |
| `algorithm` | plugin/auto | optimizer |
| `evaluation_budget` | integer/time | cap |
| `parallelism` | integer/auto | evaluations |
| `seed` | integer | reproducibility |
| `cache` | Bool | reuse equivalent evaluations |
| `produce` | best/pareto/history | result set |

## 14. `simulation()`

Common fields:

| Field | Type | Meaning |
|---|---|---|
| `type` | structural/thermal/etc. | discipline |
| `model` | semantic target | geometry |
| `configuration` | selection | variant |
| `solver` | plugin | backend |
| `materials` | mapping | physical data |
| `mesh` | policy | meshing |
| `contacts` | list | interactions |
| `boundary_conditions` | list | supports/thermal/etc. |
| `loads` | list | forces/heat/etc. |
| `settings` | solver schema | convergence/etc. |
| `outputs` | requested fields | result quantities |

## 15. `drawing()`

Common fields:

```text
sheet size
orientation
projection standard
units
default scale
template/title block
views
dimensions
annotations
PMI links
revision metadata
```

View parameters:

```text
source
orientation
scale
position/alignment
hidden-line policy
section/detail settings
```

## 16. Package skill manifest

```text
name
version
core skill dependencies
trigger phrases/concepts
entrypoint
API schema path
examples path
recommended token budget
required capabilities
validation command
```

## 17. Agent build tool

Inputs:

```text
project
configuration
profile
optional source patch
resource budget
requested artifacts
```

Outputs:

```text
status
build ID
changed features
structured diagnostics
reference health
verification summary
artifacts
resource usage
```

## 18. Agent context packet

Inputs:

```text
target semantic entity/module
purpose hint
max size/token budget
include dependencies depth
include failing diagnostics
```

Output should prioritize semantic summaries and omit irrelevant raw geometry arrays.
