# 01 — System Architecture

## 1. End-to-end architecture

```text
                          Human / AI
                              |
                     Source / visual edits
                              |
                         Frontend AST
                              |
                +-------------+--------------+
                |                            |
           Type/semantic                Documentation /
             analysis                    introspection
                |
             CAD-HIR
                |
        Parameter + constraint graph
                |
             Feature DAG
                |
        Geometry lowering/runtime
                |
        Kernel abstraction boundary
                |
      +---------+----------+-----------+
      |                    |           |
     OCCT              future       plugin
   backend             kernel       geometry
      |
 exact B-rep + topology + attribution
      |
 +----+---------+-------------+------------------+
 |              |             |                  |
validation   STEP/AP242   display mesh      analysis adapters
 |              |             |                  |
requirements  CAD tools      viewport       FEA/DFM/etc.
```

## 2. Major services

### 2.1 Compiler frontend

Responsibilities:

- lexical/syntax parsing;
- module resolution;
- name binding;
- type checking;
- dimensional analysis;
- generic instantiation;
- compile-time evaluation;
- AST/HIR diagnostics;
- source maps;
- static resource/capability checks.

### 2.2 Language runtime

Responsibilities:

- ordinary function execution;
- loops/conditionals/recursion;
- deterministic standard library;
- collections/iterators;
- exception/result handling;
- resource accounting;
- seeded randomness when explicitly allowed;
- sandbox capability enforcement;
- geometry call dispatch.

### 2.3 Parametric engine

Responsibilities:

- parameters and derived expressions;
- dependency graph;
- feature DAG;
- incremental invalidation;
- configuration/variant evaluation;
- semantic entity naming;
- persistent reference resolution;
- topology lineage records;
- cache keys.

### 2.4 Constraint subsystem

Responsibilities:

- 2D sketch constraints;
- 3D geometric constraints;
- assembly mate/joint constraints;
- algebraic parameter constraints;
- solver diagnostics;
- DOF analysis;
- over/under-constrained detection;
- solution stability tracking.

Do not expose a solver-specific model in public syntax. Maintain a normalized constraint IR and plug solver backends beneath it.

### 2.5 Geometry runtime

Responsibilities:

- typed geometry values;
- high-level feature lowering;
- low-level geometry operations;
- raw topology epochs;
- shape validation;
- healing/repair;
- geometry properties;
- exact intersections/projections/distances;
- triangulation for display;
- provenance from source operation -> resulting topology.

### 2.6 Kernel adapter

Public source should not mention OCCT types. Internally define a narrow backend interface, for example:

```text
KernelCurve
KernelSurface
KernelShape
KernelVertex
KernelEdge
KernelWire
KernelFace
KernelShell
KernelSolid
```

Operations should expose domain concepts, not vendor names.

The first implementation can map them to OCCT.

### 2.7 Verification engine

Responsibilities:

- topology validity;
- manifold/closed-solid checks;
- collision/interference;
- clearances;
- minimum wall thickness;
- mass/volume/bounding properties;
- requirement evaluation;
- CAD test runner;
- DFM/analysis plugin results;
- report generation.

### 2.8 Artifact/interchange engine

Responsibilities:

- `.aicad` container;
- source module packaging;
- deterministic manifest;
- STEP import/export;
- BREP cache;
- 3MF/STL/DXF/glTF;
- external component references;
- export metadata/PMI when supported;
- import normalization/healing.

### 2.9 Language service / IDE backend

Responsibilities:

- incremental parsing;
- hover/type information;
- completion;
- go-to definition;
- rename;
- source/geometry mapping;
- diagnostics;
- feature-DAG visualization;
- parameter edit transactions;
- structured refactoring;
- debugging/time travel.

### 2.10 AI tool server

Expose stable tool calls rather than raw UI automation:

- build;
- test;
- inspect;
- query;
- explain;
- docs;
- render;
- diff;
- export;
- package discovery;
- skill retrieval.

AI support must sit **on top of** the compiler/runtime, not inside the geometry engine.

## 3. Recommended process boundaries

For reliability and security, use separate processes for:

1. IDE/frontend.
2. Compiler/language service.
3. Geometry worker.
4. Untrusted package/plugin worker.
5. External solver worker.
6. AI tool gateway.

Benefits:

- a geometry-kernel crash does not take down the editor;
- package limits can be enforced;
- expensive geometry evaluation can be cancelled;
- remote/distributed geometry workers become possible later;
- deterministic compiler state is easier to preserve.

## 4. Suggested first implementation technology

### Compiler/runtime

**Rust** is recommended because the project needs:

- strong types;
- memory safety at the compiler/runtime layer;
- good CLI/tooling ecosystem;
- straightforward C ABI integration;
- WASM support;
- good concurrency when incremental builds arrive.

### Geometry bridge

Start with a deliberately narrow **C++ bridge around OCCT** rather than binding the entire OCCT API into the language runtime. This prevents public semantics from accidentally tracking OCCT class design.

Bridge operations should be coarse and domain-shaped:

```text
create_box
create_cylinder
make_edge
make_wire
make_face
extrude
revolve
sweep
loft
boolean_union
boolean_cut
boolean_intersect
fillet
chamfer
offset
shell
heal
validate
explore_topology
surface_info
curve_info
export_step
```

Add capabilities only as required by the low-level API.

## 5. Internal IR layers

Use multiple compiler IRs even though users see one language.

```text
Source AST
   |
Typed HIR
   |
Engineering HIR
   |  (part/assembly/feature/requirement concepts)
Feature IR / dependency DAG
   |
Geometry IR
   |  (curves/surfaces/topology operations)
Kernel call graph
   |
B-rep
```

Why multiple IRs matter:

- high-level features can lower predictably;
- optimization/caching works before touching the kernel;
- diagnostics can point back to semantic concepts;
- alternative kernels remain possible;
- analysis tools can inspect models without materializing all geometry.

## 6. Canonical state model

The source tree + lockfile + external immutable assets are canonical.

Generated state is cacheable:

```text
.aicad-cache/
  AST cache
  typed HIR cache
  feature DAG cache
  BREP cache
  display mesh cache
  analysis cache
```

Cache entries are content-addressed by:

- source digest;
- dependency digest;
- compiler version;
- kernel version;
- target configuration;
- relevant plugin/solver versions;
- deterministic build options.

## 7. Incremental build strategy

Each feature node records:

- source inputs;
- parameter inputs;
- semantic references consumed;
- geometry outputs;
- topology lineage;
- validation outputs.

On change:

1. invalidate directly dependent nodes;
2. recompute changed constraints;
3. rebuild affected feature DAG subgraph;
4. remap semantic references;
5. rerun only tests/analyses depending on changed entities;
6. reuse unaffected geometry and meshes.

## 8. Kernel-independence contract

The public language must not guarantee behavior that can only be implemented by OCCT-specific topology IDs or class semantics.

Acceptable public concepts:

- NURBS curve;
- planar face;
- manifold solid;
- tangent continuity;
- boolean union;
- semantic face reference.

Unacceptable public concepts:

- `TopoDS_Face`;
- `BRepAlgoAPI_Fuse`;
- OCCT-specific numerical status enums.

Kernel-specific diagnostics may appear in a nested `backend_details` field for debugging but not as the only error explanation.

## 9. New architectural addition: capability tiers

Define execution capability tiers to make the system safe and portable:

### Tier A — Pure language

Math, collections, types, no filesystem/network/native calls.

### Tier B — Safe CAD

High-level/low-level geometry through validated operations.

### Tier C — Unsafe geometry

Raw topology handles, manual topology construction, healing, direct kernel-grade operations.

### Tier D — External capabilities

Filesystem, network, native solver, custom exporter, organization data. Must be declared and approved.

This creates a consistent security model for humans, packages, and AI-generated code.
