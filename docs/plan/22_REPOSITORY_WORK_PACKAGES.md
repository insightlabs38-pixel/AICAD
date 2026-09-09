# 22 — Repository Structure, Work Packages, and Build Boundaries

This file converts the architecture into concrete implementation ownership boundaries. It is intended to prevent the first implementation from collapsing into a monolith.

## 1. Proposed monorepo

```text
aicad/
  Cargo.toml / workspace config
  CMakeLists.txt                    # OCCT bridge only if needed
  package.json                      # IDE/web tooling

  specs/
    language/
      grammar.ebnf
      semantics.md
      types.md
      diagnostics.md
    schemas/
      build.schema.json
      diagnostic.schema.json
      package.schema.json
      skill.schema.json
      artifact.schema.json

  crates/                           # names illustrative
    cad-lexer/
    cad-parser/
    cad-ast/
    cad-hir/
    cad-types/
    cad-units/
    cad-runtime/
    cad-compiler/
    cad-diagnostics/
    cad-feature-graph/
    cad-constraints/
    cad-references/
    cad-query/
    cad-geometry-api/
    cad-geometry-runtime/
    cad-kernel-api/
    cad-occt-bridge/
    cad-validation/
    cad-assemblies/
    cad-configurations/
    cad-requirements/
    cad-artifact/
    cad-interchange/
    cad-packages/
    cad-provenance/
    cad-cli/
    cad-lsp/
    cad-agent-tools/

  native/
    occt_bridge/

  std/
    core/
    modeling/
    fasteners/
    bearings/
    threads/
    gears/

  editor/
    app/
    viewport/
    inspectors/
    sketcher/
    debugger/
    diff-viewer/

  tree-sitter-aicad/

  skills/
    cad-core.skill.md
    cad-geometry.skill.md
    cad-engineering.skill.md

  examples/
    plates/
    brackets/
    enclosures/
    freeform/
    assemblies/
    verification/

  tests/
    parser/
    typecheck/
    runtime/
    geometry/
    references/
    assemblies/
    interchange/
    ai-learnability/
    security/
    fixtures/

  benchmarks/
    topology-naming/
    low-level-completeness/
    performance/
    large-assembly/
    ai/

  docs/
    language/
    API/
    package-authoring/
    architecture/
```

## 2. Work Package WP-01 — Kernel bridge

### Owns

- kernel context lifecycle;
- exact geometry handles;
- curve/surface constructors;
- B-rep operations;
- topology traversal;
- validation/healing;
- meshing;
- STEP primitives.

### Must not own

- source syntax;
- semantic refs;
- user-facing package abstractions;
- AI logic.

### Initial API acceptance

A pure Rust/native test can construct, validate, query, mesh, and export the Stage-1 bracket without the source language existing.

## 3. WP-02 — Parser + AST

### Owns

- canonical grammar;
- parser;
- source spans;
- comments/doc comments;
- syntax error recovery;
- AST serialization/debugging;
- formatter hooks.

### Required tests

- precedence;
- generic/type syntax;
- unit literals;
- geometry blocks;
- attributes;
- malformed source recovery.

## 4. WP-03 — Types + units

### Owns

- symbol types;
- dimensional algebra;
- unit registry;
- inference;
- generics/interfaces;
- tolerances/ranges;
- numerical compatibility.

### Critical invariant

No API accepts an untyped numeric value where an engineering quantity is required unless it is explicitly dimensionless.

## 5. WP-04 — General runtime

### Owns

- function calls;
- scopes;
- control flow;
- collections;
- iterators/generators;
- recursion;
- pure-function cache;
- capability/resource accounting;
- deterministic standard operations.

## 6. WP-05 — Geometry language API

### Owns

- safe language-facing geometry types;
- high-level/low-level API signatures;
- Geometry IR;
- lowering to kernel bridge;
- normalized properties/errors.

### Rule

No public user type exposes OCCT-specific C++ classes.

## 7. WP-06 — Feature DAG / incremental engine

### Owns

- feature node identity;
- dependency edges;
- cache keys;
- dirty propagation;
- evaluation scheduling;
- source-to-feature mapping;
- feature-level provenance.

## 8. WP-07 — Semantic references / queries

### Owns

- stable reference objects;
- query AST/IR;
- lineage mappings;
- ambiguity handling;
- reference durability;
- raw-handle epochs;
- reference health.

### Gate

This work package must reach benchmark quality before broad high-level feature expansion.

## 9. WP-08 — Constraint system

### Owns

- constraint IR;
- sketch solver adapter;
- assembly solver adapter;
- equation constraints;
- DOF/conflict reporting;
- solver tolerance policy.

### Architecture rule

Solvers are replaceable; the source-level constraint semantics are not solver-specific.

## 10. WP-09 — High-level modeling standard library

### Owns

- ordinary part features;
- semantic outputs;
- standard patterns;
- high-level engineering convenience functions.

Prefer source-level implementation over compiler intrinsics whenever performance/semantic behavior permits.

## 11. WP-10 — Assemblies/configurations

### Owns

- components/instances;
- mates/joints;
- interfaces;
- configurations;
- kinematic state;
- BOM identity;
- collision broad phase.

## 12. WP-11 — Verification

### Owns

- tests;
- requirements;
- contracts/invariants;
- build profiles;
- geometry assertions;
- verification evidence/coverage;
- mutation testing.

## 13. WP-12 — Interchange/artifacts

### Owns

- bundle format;
- manifest/lockfile;
- STEP/3MF/STL/DXF/glTF;
- healing/import reports;
- export fidelity reports;
- reconstruction pipeline later.

## 14. WP-13 — CLI + schemas

### Owns

- stable command surface;
- JSON tool schemas;
- exit codes;
- human output formatting;
- cancellation/progress.

All IDE/agent backends should reuse service-layer APIs rather than shelling out when embedded, but CLI behavior remains a conformance reference.

## 15. WP-14 — Language server

### Owns

- incremental parse integration;
- diagnostics;
- completion;
- signatures;
- navigation;
- refactors;
- package schema/docs lookup;
- semantic tokens.

## 16. WP-15 — Viewport/IDE

### Owns

- rendering scene;
- picking;
- overlays;
- inspector;
- source/geometry cross-highlighting;
- sketch interaction;
- DAG/debugger UI;
- parameter edit transactions.

Must consume normalized semantic scene data, not kernel pointers.

## 17. WP-16 — AI tools + skills

### Owns

- core/geometry skill files;
- structured tool server;
- context packets;
- package skill loading;
- learnability benchmark harness;
- AI-specific lint rules.

Must not bypass compiler validation.

## 18. WP-17 — Package/plugin system

### Owns

- resolver;
- lockfile;
- registry client;
- schemas;
- WASM/external/native plugin hosts;
- capability policy;
- package skills.

## 19. WP-18 — Engineering modules

Split later by discipline:

```text
WP-18A drawings/PMI
WP-18B DFM
WP-18C CAE adapters
WP-18D optimization/uncertainty
WP-18E cost/sustainability plugins
```

## 20. WP-19 — Collaboration/reproducibility

### Owns

- semantic diff/merge;
- geometry fingerprints;
- attestations;
- provenance;
- Git drivers;
- release/audit policy.

## 21. Build dependency graph

```text
WP01 Kernel
  |
WP05 Geometry API ----+
  |                   |
WP07 Refs/queries     |
  |                   |
WP06 Feature DAG      |
  |                   |
WP09 Modeling --------+

WP02 Parser -> WP03 Types -> WP04 Runtime -> WP05

WP08 Constraints -> WP09/WP10
WP10 Assemblies -> WP11 Verification
WP11 Verification -> WP12 Release artifacts

WP13 CLI sits across services
WP14/15 IDE depend on stable service APIs
WP16 AI depends on WP13-like structured services
WP17 packages requires stable core language/API
WP18 engineering modules sit on packages + semantics
WP19 collaboration requires mature feature/ref graph
```

## 22. First vertical integration target

The first fully integrated model should exercise:

```text
source parser
units/types
loops/functions
feature DAG
high-level enclosure
low-level spline/sweep
semantic refs
imported STEP component
assembly mates
configuration conditional
requirements/tests
STEP export
viewport/source mapping
AI skill + build/test tools
```

Use the motor/gearbox enclosure suggested in the review file.

## 23. Definition of done for any feature

A new user-facing feature is not done until it has:

- specification;
- type/signature schema;
- positive/negative compiler tests;
- geometry tests;
- diagnostics;
- semantic-reference behavior;
- incremental invalidation behavior;
- docs/example;
- core/package skill update if AI-relevant;
- CLI/IDE introspection support;
- serialization/cache behavior;
- security/resource-budget behavior where relevant.
