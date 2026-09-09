# AI-Native CAD Language & Engineering Platform — Full Implementation Plan

**Working names:** `AICAD`, `CAD-IR`, `CADLang` (placeholder; choose branding later)

This package consolidates the complete design developed in the conversation into a buildable implementation plan. The core idea is not merely a new CAD file format. It is a **typed, compiled, Turing-complete programming language and execution environment for mechanical engineering**, designed to be equally usable by humans and general-purpose AI coding models.

The central thesis is:

> Do not force AI to operate CAD primarily through pixels, camera navigation, GUI state, or vendor-specific APIs. Give it a compact symbolic engineering language with exact geometry, deterministic execution, semantic references, tests, compiler diagnostics, packages, and structured introspection.

The system combines:

- High-level parametric engineering constructs for common work.
- A nearly kernel-complete low-level geometry DSL in the **same language**.
- Direct/raw topology access when necessary, analogous to pointer-level access in systems languages.
- Exact B-rep geometry as compiled output.
- Persistent semantic identity so geometry survives regeneration.
- Unified constraints across sketches, parts, assemblies, and engineering requirements.
- First-class assemblies, configurations, interfaces, kinematics, drawings, PMI/GD&T, DFM, simulation, optimization, and uncertainty.
- CAD unit tests and executable requirements.
- A package/plugin ecosystem where extensions can ship their own concise AI skill files.
- A human IDE in which source and geometry are bidirectionally linked.
- A model-facing tool protocol based on structured compiler/build/inspect/test responses.
- Deterministic builds, semantic diffs/merges, provenance, and production collaboration features.

## Documents

| File | Purpose |
|---|---|
| `00_PRINCIPLES_AND_SCOPE.md` | Product thesis, invariants, goals, non-goals, architectural rules |
| `01_SYSTEM_ARCHITECTURE.md` | End-to-end architecture and runtime boundaries |
| `02_LANGUAGE_AND_COMPILER.md` | Language design, grammar strategy, compiler pipeline, execution semantics |
| `03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` | Types, units, tolerances, functions, loops, conditionals, recursion, metaprogramming |
| `04_HIGH_LEVEL_MODELING_API.md` | High-level modeling feature catalog and parameter contracts |
| `05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` | Low-level curves/surfaces/B-rep/topology/raw access catalog |
| `06_REFERENCES_QUERIES_FEATURE_DAG.md` | Stable references, topological naming, query system, lineage, feature DAG |
| `07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md` | Components, mates, joints, interfaces, configurations, kinematics |
| `08_CONSTRAINTS_REQUIREMENTS_TESTS.md` | Constraint solver, executable requirements, contracts, CAD tests |
| `09_FILE_FORMAT_INTERCHANGE_RECONSTRUCTION.md` | `.aicad` container, STEP, mesh formats, native cache, reverse engineering |
| `10_IDE_HUMAN_UX.md` | Human editor, 3D viewport, debugger, REPL, visual editing, source/GUI sync |
| `11_AI_NATIVE_SKILLS_AND_AGENT_PROTOCOL.md` | Core skill, advanced skills, agent tools, introspection, learnability benchmark |
| `12_PACKAGES_PLUGINS_EXTENSIONS.md` | Package manager, plugin ABI, custom skills, schemas, sandboxing |
| `13_ENGINEERING_MODULES.md` | Drawings, PMI/GD&T, DFM, simulation, optimization, uncertainty, BOMs |
| `14_COLLABORATION_PROVENANCE_SECURITY.md` | Semantic Git, provenance, reproducibility, permissions, security budgets |
| `15_IMPLEMENTATION_ROADMAP.md` | Ordered implementation stages and exit criteria |
| `16_TESTING_BENCHMARKS_ACCEPTANCE.md` | Unit/integration/geometry/AI benchmarks and release gates |
| `17_CLI_DIAGNOSTICS_SCHEMA.md` | CLI command surface and structured diagnostic conventions |
| `18_REFERENCE_EXAMPLES.md` | Representative source examples spanning simple to advanced usage |
| `19_RESEARCH_NOTES_AND_SOURCES.md` | External references and lessons incorporated into the plan |
| `20_REVIEW_PASS_GAPS_AND_DECISIONS.md` | Deliberate second-pass review, risks, missing areas, decisions to lock later |
| `21_FEATURE_INVENTORY.md` | Master checklist ensuring all discussed features are represented |

## Recommended implementation stack

The plan intentionally keeps the public language independent of implementation details, but a practical first implementation is:

- **Compiler/runtime:** Rust.
- **Geometry kernel:** Open CASCADE Technology (OCCT) behind a narrow C++ bridge or Rust FFI layer.
- **Parser:** handwritten parser or parser generator for the canonical compiler; Tree-sitter grammar maintained in parallel for editor tooling.
- **Constraint solver:** purpose-built geometric constraint layer with pluggable numerical/nonlinear solver backends; do not tie public semantics to one solver implementation.
- **IDE:** TypeScript + React/WebGL or desktop shell, backed by an LSP-like language service and a geometry daemon.
- **Viewport interchange:** glTF or compact triangulated scene protocol generated from exact B-rep; viewport mesh is never canonical design geometry.
- **Plugins:** safe WASM/plugin process model for ordinary extensions; native ABI reserved for trusted kernels/solvers/importers.
- **Artifact interchange:** STEP AP242 as the primary neutral engineering export target where supported; STL/3MF/DXF/glTF for narrower purposes.

The implementation language is not part of the user-facing specification. A future kernel or compiler implementation can change without changing source semantics.

## Core success criteria

The project is successful when all of the following are true:

1. A human can author a useful parametric mechanical part in concise text.
2. The same file can mix high-level features with low-level exact geometry without crossing into another language.
3. A general-purpose coding model can learn ordinary usage from one short core skill file.
4. The compiler produces exact valid B-rep and exports interoperable STEP.
5. Upstream edits do not casually break downstream face/edge references.
6. Assemblies are expressed through semantic mates/joints rather than manually managed world transforms.
7. Designs can carry executable requirements and tests.
8. Compiler errors are structured and repair-oriented enough for both humans and agents.
9. GUI edits and source edits operate on the same semantic model rather than creating hidden state.
10. Third-party packages can add engineering abstractions and teach AI to use them without changing the core model or language.
