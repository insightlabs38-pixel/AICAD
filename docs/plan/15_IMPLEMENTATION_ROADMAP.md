# 15 — Ordered Implementation Roadmap

This roadmap is dependency-driven. It deliberately avoids dates; every stage has concrete exit gates.

## Stage 0 — Architecture constitution and RFC

### Build

- language design principles;
- grammar sketch;
- type/units model;
- safe vs raw topology model;
- source/canonical-state rules;
- kernel abstraction contract;
- feature/reference semantics;
- diagnostic schema;
- repository/module structure;
- initial core-skill draft used as a learnability constraint.

### Deliverables

```text
RFC-0001 language principles
RFC-0002 geometry/runtime model
RFC-0003 semantic references
RFC-0004 units/type system
RFC-0005 diagnostics
examples/*.cadl (paper/spec examples)
```

### Exit gate

A realistic example containing parameters, function, loop, conditional, sketch, extrusion, semantic query, low-level geometry, assembly, constraint, and test can be written without semantic contradiction.

---

## Stage 1 — Geometry kernel spike

### Build

- OCCT bridge;
- internal shape handles;
- primitive solids;
- basic curves;
- extrude/revolve/sweep/loft;
- booleans;
- fillet/chamfer/shell;
- transforms;
- topology enumeration;
- properties;
- validity checks;
- STEP export;
- display tessellation.

Drive it with temporary JSON/internal test calls.

### Do not build yet

- final parser;
- full GUI;
- AI agent.

### Exit gate

Complex bracket fixture -> valid B-rep -> STEP -> opens correctly in multiple external CAD viewers/tools used for verification.

---

## Stage 2 — Core language/compiler/runtime

### Build

- lexer/parser;
- AST;
- module system;
- names/symbols;
- primitive types;
- dimensional types/units;
- structs/enums;
- functions;
- loops;
- conditionals;
- match;
- recursion;
- collections/iterators;
- errors/results;
- deterministic evaluator;
- resource budgets;
- basic CLI;
- formatter;
- language conformance tests;
- Tree-sitter grammar in parallel.

### Exit gate

Ordinary programs can construct geometry and compute parameterized patterns without special interpreter shortcuts.

---

## Stage 3 — High-level parametric CAD MVP

### Build

- part/sketch/feature concepts;
- sketch geometry;
- initial sketch constraint solver;
- box/cylinder/plate;
- extrude/revolve/sweep/loft;
- hole/pocket/boss/rib;
- patterns/mirror;
- fillet/chamfer/shell/draft;
- parameters and derived expressions;
- feature DAG;
- named semantic outputs;
- material + mass basics;
- STEP/STL/glTF export;
- example library.

### AI benchmark begins

Give a fresh coding model only `cad-core.skill.md`. Track simple-part generation success.

### Exit gate

A human and a fresh model can create a representative set of ordinary mechanical parts concisely.

---

## Stage 4 — Semantic references and regeneration robustness

### Build

- `FaceRef`/`EdgeRef` etc.;
- query system;
- lineage recording;
- explicit named outputs;
- ambiguity diagnostics;
- reference health report;
- weak-reference linting;
- incremental parameter regeneration;
- regression corpus intentionally causing splits/merges.

### Exit gate

Benchmark models survive defined upstream parameter changes without incorrect silent downstream selections.

This is a **hard release gate**. Do not scale feature count before this is credible.

---

## Stage 5 — Advanced low-level geometry

### Build

- analytic curves/surfaces;
- Bezier/B-spline/NURBS;
- trimming/offsets;
- projection/intersection;
- low-level topology construction;
- sewing/healing;
- direct topology traversal;
- raw handle epochs;
- `unsafe geometry`;
- validated promotion;
- advanced introspection;
- `cad-geometry.skill.md`.

### Exit gate

A suite of intentionally unusual/freeform parts can be implemented without adding new compiler intrinsics.

This validates the "PyTorch + Triton / high level + near-kernel level" thesis.

---

## Stage 6 — Assemblies and configurations

### Build

- components/instances;
- local frames;
- mates;
- joint types;
- DOF analysis;
- subassemblies;
- mechanical interfaces;
- configurations/variants;
- suppression/replacement;
- imported STEP components;
- BOM basics;
- collision/interference basics.

### Exit gate

Build a multi-part mechanism with purchased parts, repeated instances, subassembly, at least one moving joint, and no manually hard-coded final world coordinates.

---

## Stage 7 — Verification-first CAD

### Build

- `assert`, `requirement`, `test`;
- geometry assertion library;
- assembly tests;
- configuration test matrix;
- contracts/invariants;
- build profiles;
- structured test reports;
- verification evidence;
- AI fix-failing-test benchmark.

### Exit gate — First serious MVP

The system supports:

```text
real language
+ exact parametric CAD
+ stable references
+ advanced escape hatch
+ assemblies
+ executable verification
```

This is the first milestone suitable for serious external evaluation.

---

## Stage 8 — Native `.aicad` artifact + interoperability

### Build

- bundle manifest;
- lockfile;
- deterministic packing;
- STEP import/export hardening;
- BREP cache;
- 3MF/STL/DXF/glTF;
- healing reports;
- export fidelity reports;
- external asset provenance;
- round-trip test suite.

### Exit gate

Representative parts/assemblies round-trip through supported formats within documented fidelity limits.

---

## Stage 9 — Human CAD IDE

### Build

- language server;
- code editor;
- 3D viewport;
- source <-> geometry highlighting;
- project/feature DAG view;
- parameter inspector;
- visual sketch editing;
- visual feature insertion;
- REPL;
- geometry debugger;
- time-travel build view;
- source-aware visual edits;
- semantic diff UI.

### Exit gate

A mechanical CAD user can complete ordinary modeling without being forced to hand-write source, while every operation remains source-representable and reviewable.

---

## Stage 10 — AI-native tool layer

### Build

- core skill finalized;
- geometry skill;
- docs/schema introspection;
- structured build/test/inspect/query/explain/render/diff/export tools;
- compact agent context packets;
- package/skill discovery;
- agent benchmark harness;
- source-style linter for AI reliability.

### Exit gate — AI proof

A fresh general-purpose coding model, given only the core skill and tool interface, can create, inspect, debug, modify, and verify unseen ordinary CAD tasks at target reliability.

---

## Stage 11 — Package/plugin ecosystem

### Build

- package manifest;
- dependency resolver;
- registry client;
- lockfile hardening;
- package API schemas;
- package skills;
- WASM sandbox/plugin ABI;
- external process plugin protocol;
- trusted native plugin protocol;
- package validation/publishing tests;
- standard packages for fasteners/bearings/threads.

### Exit gate

A third party can build an unfamiliar specialized package, publish its API schema + skill, and a fresh AI model can correctly use it without language retraining.

---

## Stage 12 — Engineering semantics

Implement in sub-stages:

### 12A

- materials database interfaces;
- BOM;
- drawings;
- dimensions;
- basic PMI;

### 12B

- GD&T/datums;
- richer AP242 mapping;
- surface finish;
- fits/tolerances.

### 12C

- DFM packages: CNC, additive, injection molding, sheet metal.

### 12D

- structural/thermal solver adapters;
- simulation result objects;
- analysis-backed requirements.

### 12E

- optimization;
- Pareto/design-space exploration;
- uncertainty/tolerance stacks.

### Exit gate

A release workflow can connect requirements -> design -> manufacturing/analysis checks -> drawing/PMI -> evidence.

---

## Stage 13 — Production collaboration and scale

### Build

- deep incremental compilation;
- geometry caches;
- large assembly streaming/LOD;
- parallel DAG evaluation;
- semantic diff/merge;
- Git integration;
- provenance policies;
- deterministic attestations;
- organization package/skill governance;
- access-control hooks;
- build farm/distributed worker support if needed.

### Exit gate

Large real-world projects can be reviewed, merged, reproduced, and rebuilt without treating CAD as opaque binary state.

---

# Cross-stage rule — continuously maintain three proof tracks

## Language proof

```text
source -> compiler -> exact part -> external CAD artifact
```

## Architecture proof

```text
high-level feature + low-level custom geometry + semantic refs + assembly + tests
```

## AI proof

```text
fresh model + small skill + tools -> verified unfamiliar design
```

No stage should improve one proof track by breaking another.
