# 09 — Native Artifact, Interchange, Import, and Parametric Reconstruction

## 1. Source vs project artifact

Prefer plain-text modules for Git and a portable project bundle for exchange.

Possible naming:

```text
*.cadl       plain source module
*.aicad      bundled project/artifact
```

The exact extensions can change later.

## 2. `.aicad` bundle structure

Use a deterministic ZIP-compatible container or similarly well-supported archive format.

```text
project.aicad/
  manifest.json
  cad.lock
  src/
    main.cadl
    housing.cadl
  requirements/
    requirements.cadl
  drawings/
    assembly.cadl
  assets/
    bearing.step
    pcb.step
  cache/
    geometry.brep
    viewport.glb
  metadata/
    provenance.json
    build.json
  evidence/
    verification.json
```

Cache/evidence sections can be optional so source-only packages remain small.

## 3. Manifest fields

Recommended fields:

```json
{
  "format_version": "1.0",
  "language_version": "1.x",
  "entrypoint": "src/main.cadl",
  "project_name": "...",
  "default_units": "mm",
  "default_configuration": "...",
  "lockfile": "cad.lock",
  "assets": [],
  "required_capabilities": [],
  "build_profiles": [],
  "provenance_policy": "..."
}
```

## 4. Deterministic packaging

For reproducibility:

- canonical path ordering;
- normalized timestamps or omitted archive timestamps;
- normalized manifest JSON ordering;
- content digests for assets;
- explicit compression/version policy;
- signature support later.

## 5. Primary interchange targets

### STEP AP242

Primary neutral engineering export target for rich mechanical exchange where the backend supports it.

Target progressively:

1. exact part geometry;
2. assemblies;
3. names/colors/layers;
4. product metadata;
5. PMI/GD&T and semantic manufacturing data where mapping is reliable;
6. configuration/kinematic metadata where feasible and supported by libraries/tooling.

### BREP

Kernel-native exact-geometry cache/debug artifact. Not the public semantic source format.

### 3MF

Useful additive-manufacturing interchange including units/metadata capabilities beyond STL.

### STL

Legacy mesh export only. Always require explicit tessellation tolerance/profile.

### DXF

2D sketch/drawing profile interchange.

### glTF/GLB

Viewport/web visualization, not manufacturing truth.

## 6. Export API

```text
export_step(model, path, options)
export_3mf(model, path, options)
export_stl(model, path, tessellation)
export_dxf(sketch_or_drawing, path, options)
export_gltf(scene, path, options)
```

Common export options:

| Option | Meaning |
|---|---|
| `configuration` | Product configuration to build |
| `units` | Output unit declaration where applicable |
| `include_metadata` | Names/materials/layers/etc. |
| `include_pmi` | PMI export attempt |
| `heal_before_export` | Run configured healing |
| `validate_after_export` | Round-trip/import validation |
| `tessellation` | Mesh linear/angular tolerance |

## 7. Import model

Imported exact geometry initially becomes an opaque-but-queryable semantic node:

```aicad
let bearing = import_step("bearing.step");
```

It should expose:

- bodies;
- faces/edges;
- colors/names/layers if present;
- assembly structure if present;
- coordinate systems/datums if recognized;
- PMI/product metadata when available;
- raw imported source digest.

Do not pretend to possess feature history that does not exist.

## 8. Healing and normalization

On import:

1. parse artifact;
2. preserve original artifact and digest;
3. inspect units/coordinate frame;
4. validate topology;
5. optionally heal using a recorded profile;
6. retain a transformation/healing report;
7. expose normalized shape to source.

Never silently mutate imported geometry without recording what changed.

## 9. Parametric reconstruction

Long-term command:

```text
cad reconstruct bracket.step
```

Pipeline:

```text
Imported B-rep
   -> analytic surface/feature detection
   -> symmetry/pattern detection
   -> sketch/profile inference
   -> feature-order candidate generation
   -> semantic feature classification
   -> parameter extraction
   -> reconstruction candidate program(s)
   -> rebuild
   -> geometry comparison
   -> confidence/evidence report
```

## 10. Reconstruction output

```text
Candidate 1 confidence: 92%

base_extrude       99.8%
4-hole pattern     99.1%
pocket             97.3%
fillet sequence    88.4%
design rationale   unknown

Geometric deviation:
max 0.004mm
RMS 0.0007mm
```

Reconstruction confidence must separate:

- geometric match confidence;
- feature-class confidence;
- parameter confidence;
- ordering/history confidence;
- design-intent confidence.

## 11. AI-assisted reconstruction

AI can rank/interpret candidate histories, but deterministic geometry comparison verifies the rebuilt result. The system should never label inferred rationale as known fact.

## 12. Round-trip testing

Maintain fixtures for:

```text
CAD source -> STEP -> import -> geometric comparison
STEP -> imported node -> STEP -> comparison
source assembly -> STEP assembly -> import structure comparison
PMI source -> AP242 -> import -> semantic subset comparison
```

## 13. External asset policy

Assets support:

- embedded;
- local relative path;
- content-addressed package asset;
- remote URI only under explicit capability and lock/digest policy.

Production builds should pin asset digests.

## 14. New feature: artifact fidelity report

Every export can optionally produce:

```text
Geometry: exact B-rep transferred
Assembly names: transferred
Colors: transferred
PMI: 18/20 entities mapped
Unsupported PMI: 2 datum-target constructs
Configurations: flattened to selected configuration
Kinematics: not exported
```

This avoids assuming a neutral format preserved every semantic layer.
