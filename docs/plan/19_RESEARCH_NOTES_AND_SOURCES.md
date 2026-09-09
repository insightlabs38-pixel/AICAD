# 19 — Research Notes and Sources

This plan was reviewed against current CAD/kernel/language-tooling references to avoid reinventing known-good concepts blindly and to distinguish the proposed innovations from existing precedent.

## 1. Open CASCADE Technology (OCCT)

**Source:** https://dev.opencascade.org/doc/overview/html/index.html

Relevant facts incorporated:

- OCCT is a modular CAD/CAM/CAE development platform with exact 2D/3D geometry, modeling algorithms, mesh, visualization, data exchange, shape healing, and an application framework.
- Its modeling data explicitly separates mathematical geometry (points, curves, surfaces, including Bezier/NURBS) from B-rep topology.
- Its topology hierarchy includes vertex, edge, wire, face, shell, solid, compsolid, and compound.
- It supports STEP/IGES data exchange.
- Shape Healing provides analysis/fixing/upgrade operations for imported/invalid geometry.
- OCAF provides document/app data, undo/redo, parametric dependencies, modification tracking, and reference-key-oriented data structures.

**Design consequence:** OCCT is a strong initial kernel backend, but its C++ class names should remain behind a backend abstraction. The proposed low-level language maps naturally to its geometry/topology concepts without needing to expose `TopoDS_*` or `BRep*` names publicly.

## 2. OCCT Modeling Data

**Source:** https://dev.opencascade.org/doc/occt-7.8.0/overview/html/occt_user_guides__modeling_data.html

Relevant facts incorporated:

- OCCT distinguishes geometry from topology.
- Topological structures can be traversed by entity type.
- Shapes can be shared with orientation and location metadata.
- Curves/surfaces can be approximated/interpolated and converted to B-splines.

**Design consequence:** This validates the plan's two concepts: mathematical geometry objects and topological entities, plus explicit coordinate frames/transforms.

## 3. OCAF / persistent references

**Sources:**

- https://dev.opencascade.org/doc/occt-7.2.0/overview/html/occt_user_guides__ocaf.html
- https://dev.opencascade.org/doc/occt-6.7.1/overview/html/occt_user_guides__ocaf.html

Relevant facts incorporated:

- OCAF is reference-key-driven rather than treating mutable shape objects as the stable application identity.
- It can track shape evolution and topological naming through model modifications.
- It provides parametric dependency and document transaction concepts.

**Design consequence:** The plan makes stable `FaceRef`/`EdgeRef` semantic references separate from ephemeral raw B-rep handles. The proposed query + lineage + explicit semantic export system extends this idea into a language-level contract.

## 4. ISO 10303-242:2025 (STEP AP242 Edition 4)

**Source:** https://www.iso.org/standard/84300.html

Relevant facts incorporated:

- AP242:2025 is the current Edition 4 application protocol for managed model-based 3D engineering.
- Its scope includes mechanical products, parts, assemblies, tools/raw materials, and product/engineering data for long-term archiving and product data management.

**Design consequence:** STEP AP242 is the correct long-term neutral engineering export direction rather than trying to replace STEP with the source language. The source language should retain richer executable intent and generate AP242 where mappings exist.

## 5. Onshape FeatureScript

**Sources:**

- https://cad.onshape.com/FsDoc/index.html
- https://cad.onshape.com/FsDoc/library.html
- https://cad.onshape.com/FsDoc/modeling.html

Relevant facts incorporated:

- FeatureScript is a programming language for parametric 3D modeling.
- Standard Onshape modeling features are implemented as FeatureScript functions.
- Queries describe criteria for finding topology rather than merely storing lists of entity IDs.
- Queries can be composed and evaluated against a context.
- Attributes can be attached to topological entities.

**Design consequence:** This strongly validates CAD-as-code, library-defined feature systems, query-based selection, and entity metadata. The proposed platform goes further by aiming for vendor-neutral exact CAD, raw near-kernel access, Turing-complete general programming, executable requirements/tests, package skills, and AI-oriented diagnostics.

## 6. CadQuery

**Sources:**

- https://cadquery.readthedocs.io/en/latest/intro.html
- https://cadquery.readthedocs.io/en/latest/importexport.html
- https://cadquery.readthedocs.io/en/stable/selectors.html
- https://cadquery.readthedocs.io/en/stable/workplane.html

Relevant facts incorporated:

- CadQuery uses Python to create parametric CAD and is based on OpenCascade bindings.
- It supports STEP export and assemblies.
- It uses workplanes to avoid manually managing world coordinates for ordinary modeling.
- Its selectors filter edges/faces/vertices/solids and can be composed.

**Design consequence:** Human-readable programmable CAD is already practical. The proposed language should preserve the ergonomics of workplanes/frames/selectors while adding stronger engineering types, persistent semantic identity, a compiler/test model, and integrated low-level geometry.

## 7. OpenSCAD

**Sources:**

- https://openscad.org/documentation
- https://files.openscad.org/documentation/manual/The_OpenSCAD_Language.html

Relevant facts incorporated:

- OpenSCAD is a programming-language-based 2D/3D solid modeler.
- Its language includes control flow, loops, mathematical operations, functions/modules, transformations, booleans, and extrusion.

**Design consequence:** Loops/conditionals in text-based CAD are not unusual and should not be artificially forbidden. However, the proposed platform targets exact engineering B-rep/semantics/assemblies/verification rather than remaining primarily constructive mesh/CSG-oriented.

## 8. Tree-sitter

**Source:** https://tree-sitter.github.io/tree-sitter/index.html

Relevant facts incorporated:

- Tree-sitter is an incremental parsing library designed to update syntax trees efficiently as source is edited and remain useful in the presence of syntax errors.

**Design consequence:** Maintain a Tree-sitter grammar for IDE parsing/highlighting even if the authoritative compiler parser is separately implemented. Conformance tests must keep them synchronized.

## 9. What appears genuinely differentiating in the proposed architecture

The individual ingredients have precedent, but the combination is unusual:

1. One unified language spanning high-level parametric CAD and nearly kernel-level geometry.
2. Explicit safe semantic refs vs raw pointer-like topology handles with lifetime/epoch semantics.
3. Executable engineering requirements and CAD unit tests as first-class source.
4. Structured diagnostics designed for repair by humans and agents.
5. A formal requirement that a general coding model learn core usage from one compact skill file.
6. Extension packages that ship both exact schemas and their own AI skills.
7. Bidirectional visual CAD/source editing with source remaining canonical.
8. Semantic Git-style diffs/merges tied to feature DAG and requirement impact.
9. Export fidelity reports rather than pretending neutral formats preserve all semantics.
10. Requirement -> geometry -> analysis -> drawing/manufacturing -> evidence traceability in one programming model.

## 10. Research items intentionally left for implementation stages

Before production-grade implementation, perform targeted research/prototyping on:

- geometric constraint solvers and numerical robustness;
- persistent topological naming algorithms beyond raw OCAF capabilities;
- AP242 PMI/GD&T mapping coverage in chosen data-exchange library;
- licensing/redistribution constraints for standards-derived libraries/data;
- robust B-rep equivalence/fingerprinting across kernel versions;
- WASM numerical/plugin overhead for geometry extensions;
- large-assembly scene streaming/indexing;
- solver-neutral FEA/CFD result schema;
- formal dimensional type implementation and affine temperature handling;
- parser/compiler incremental architecture tradeoffs.
