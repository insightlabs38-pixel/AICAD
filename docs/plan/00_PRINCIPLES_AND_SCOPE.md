# 00 — Principles, Scope, and Non-Negotiable Invariants

## 1. Product definition

The project is a **general programming system for parametric mechanical design**. It should eventually cover the domain served by modern parametric CAD, while exposing enough programming and geometry power that unusual features do not require adding new compiler intrinsics.

It is simultaneously:

- a source language;
- a compiler;
- a geometry runtime;
- a parametric dependency system;
- an exact CAD modeler;
- an engineering verification framework;
- a package ecosystem;
- a human development environment;
- an AI-native tool protocol;
- an interoperable engineering artifact pipeline.

It is **not** primarily a chatbot, a mesh generator, a GUI macro system, or JSON serialization of B-rep internals.

## 2. Foundational thesis

Conventional AI-CAD interaction often asks the model to solve the wrong problem: camera orientation, 3D selection, GUI state, screen coordinates, hidden feature-tree state, and vendor-specific APIs. This platform changes the abstraction boundary.

Instead of:

```text
AI -> pixels/clicks/vendor API -> CAD program
```

use:

```text
AI/human -> typed CAD source -> compiler/runtime -> exact geometry -> standard CAD artifacts
```

The AI operates symbolically. Exact spatial computation is delegated to the geometry runtime.

## 3. Non-negotiable language rules

1. **The language is Turing-complete by design.** Loops, conditionals, functions, recursion, data structures, and user algorithms are first-class.
2. **High-level and low-level CAD live in one language.** There is no separate "advanced language" to learn.
3. **Low-level geometry is nearly kernel-complete.** Advanced users must not hit a high-level abstraction wall.
4. **Raw topology access exists.** It is explicitly ephemeral/unsafe and guarded by validation and lifetime rules.
5. **Units are in the type system.** Length, angle, force, pressure, mass, etc. are not plain untyped floats.
6. **Exact B-rep is the primary compiled geometry.** Meshes are views/export targets, not canonical design truth.
7. **Source is canonical design intent.** Cached B-rep may accelerate builds but never replaces source semantics.
8. **Semantic references are preferred to entity indices.** `housing.mounting_face` is valid design language; `face_17` is an escape hatch only.
9. **Determinism is the default.** Same source + lockfile + compiler/kernel versions produces equivalent geometry and validation results.
10. **Engineering assertions are executable.** Requirements, tests, and contracts belong beside design source.
11. **GUI and source are two projections over one model.** The GUI must not create irreducible hidden state.
12. **Extensions are first-class.** Packages can add abstractions, APIs, validators, exporters, solvers, and AI skills.
13. **The model does not require a CAD-specialized AI model.** A specialized model may improve performance, but the platform must remain usable by general coding models.

## 4. Design goals

### 4.1 For humans

- Concise enough for ordinary parts.
- Powerful enough for advanced geometry.
- Readable diffs.
- Strong static feedback.
- Reliable references after parametric edits.
- Familiar programming syntax.
- Visual editing without abandoning code.
- Reusable libraries rather than repeated manual modeling.
- Better review/version-control behavior than binary CAD.

### 4.2 For AI

- One compact core skill should teach ordinary usage.
- APIs should be discoverable at runtime.
- Diagnostics should be structured, typed, and actionable.
- Geometry should be introspectable without relying on screenshots.
- Model changes should be testable.
- Plugins should ship their own skill/schema metadata.
- Context loading should use progressive disclosure.
- Agents should be able to create, inspect, build, test, diagnose, modify, and export through deterministic tools.

### 4.3 For engineering organizations

- Deterministic/reproducible builds.
- Semantic change review.
- Traceability from requirements to geometry to analysis to drawing.
- AI provenance.
- Standard artifact exchange.
- Package governance and internal standards.
- Project/team/company instruction layers.
- Large-assembly scalability.

## 5. Explicit non-goals for early versions

Do not block the core project on:

- replacing every commercial CAD package immediately;
- perfect parametric reconstruction of arbitrary imported STEP files;
- implementing an in-house geometry kernel;
- implementing all FEA/CFD solvers internally;
- full CAM toolpath generation in the compiler core;
- photorealistic rendering;
- every industry-specific standard library;
- cloud PLM infrastructure;
- autonomous engineering sign-off.

These are extension-stage concerns.

## 6. Canonical data hierarchy

```text
Project
  -> Modules/packages
    -> Parts / assemblies / drawings / simulations / requirements
      -> Feature DAG
        -> semantic entities
          -> exact geometric operations
            -> B-rep topology + geometry
              -> display mesh / STEP / 3MF / STL / DXF / reports
```

## 7. Two abstraction levels, one type system

### High level

For ordinary engineering intent:

- plate;
- enclosure;
- sketch;
- hole;
- boss;
- rib;
- pocket;
- thread;
- pattern;
- mate;
- joint;
- configuration;
- bearing seat;
- sheet-metal feature;
- standard component.

### Low level

For precise geometric construction and manipulation:

- points/vectors/frames;
- analytic curves/surfaces;
- Bezier/B-spline/NURBS;
- wires/faces/shells/solids;
- sweep/loft/revolve;
- intersection/projection/trim;
- sewing/healing;
- topology traversal;
- direct raw handles;
- kernel-grade boolean and repair operations.

Both levels can be mixed inside one function and one part.

## 8. Safety model for raw topology

Raw B-rep handles should behave more like pointers than stable references.

Recommended rule:

- `FaceRef`, `EdgeRef`, etc. are semantic/stable references that can survive regeneration when resolvable.
- `Face*`, `Edge*`, `Solid*` are ephemeral raw handles valid only within a **geometry evaluation epoch**.
- Mutating topology invalidates affected raw handles.
- Crossing an epoch or storing raw handles in persistent project state is a compile/runtime error.
- `unsafe geometry { ... }` allows lower-level operations that can violate semantic assumptions.
- Leaving an unsafe block requires explicit `validate()` or `adopt_validated()` before the geometry is promoted to a normal `Solid`/`Part`.

This is a new but important refinement: it preserves pointer-like power while avoiding a large class of stale-topology bugs.

## 9. Long-term interoperability rule

The language should not compete with STEP at the same layer.

Use:

```text
source .aicad -> compile -> exact B-rep -> STEP AP242 / other outputs
```

The source keeps intent, parameters, tests, package references, provenance, and code. STEP is the neutral downstream engineering artifact where appropriate.

## 10. Quality bar

Every feature added to the language should be evaluated against five questions:

1. Can a human understand it without excessive ceremony?
2. Can a general coding model learn it from a small reference?
3. Is the behavior deterministic and testable?
4. Does it preserve semantic design intent?
5. Could it be a library instead of a compiler intrinsic?

If the answer to #5 is yes, prefer the library.
