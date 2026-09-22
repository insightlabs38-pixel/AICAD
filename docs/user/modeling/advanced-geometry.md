# Advanced Geometry and Topology

Stage 5 extends AICAD beyond the earlier Safe CAD feature catalogue while keeping public semantics kernel-neutral and source/provenance aware.

## Supported families

Current Stage-5 support includes:

- advanced analytic and freeform curve representations and operations;
- advanced surface representations and operations;
- trimmed geometry;
- intersection, projection, and distance queries;
- topology construction for the supported vertex/edge/wire/face/shell/solid/compound paths;
- bounded sewing and healing with explicit outcome evidence;
- deterministic topology traversal and inspection;
- controlled raw geometry access;
- functional raw-geometry editing;
- validation-driven raw-to-safe adoption;
- topology-change lineage integrated with provenance and persistent semantic references.

Representative maintained source examples live in:

- [`examples/curves/`](../../../examples/curves/)
- [`examples/surfaces/`](../../../examples/surfaces/)
- [`examples/topology/`](../../../examples/topology/)

The difficult/adversarial freeform corpus is test evidence rather than a teaching surface and remains under `project/benchmarks/stage5_freeform_corpus/`.

## Safety and identity boundary

A successful kernel operation or render is not sufficient evidence that a result is a valid safe B-rep. Construction, healing, raw editing, and adoption preserve explicit validation/provenance/lineage evidence. Raw topology/native identity is never promoted into durable semantic identity.

Persistent references continue to resolve through AICAD-owned semantic recipes and fail closed on ambiguity. Lineage can contribute evidence after topology-changing operations, but it does not authorize arbitrary fingerprint-based recovery.

## Important limitations

The Stage-5 gate records several boundaries that matter to users and later-stage code:

- Bezier/B-spline curves and surfaces, plus trimmed surfaces, are real AICAD values with construction/evaluation/query support, but they cannot yet be converted into real kernel topology through `make_edge` / `make_face_on_surface`; unsupported construction fails explicitly with `UNSUPPORTED_TOPOLOGY_CONSTRUCTION`;
- `List<Geometry>` builtin parameters are currently invisible to `geometry_inputs` in `FeatureGraph` / `TraceFeatureGraph`, so arbitrary dirty-set/incremental invalidation through that generic path is not treated as independently proven first-class behavior;
- numerical/kernel failure coverage is bounded by the operation classes and adversarial cases actually exercised;
- some raw/lineage behavior is better demonstrated by automated Rust/integration fixtures than by standalone `.aicad` teaching examples because those paths need a live kernel context.

See [Current limitations](../current-limitations.md) for the broader product boundary and the [developer geometry documentation](../../developer/geometry/) for API-level detail.
