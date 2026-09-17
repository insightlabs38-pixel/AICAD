# Architecture

AICAD is layered so language semantics, parametric identity, persistent-reference identity, and geometry intent remain independent of the concrete CAD kernel.

```text
.aicad source
  -> lexer / parser / AST
  -> HIR + binding/type/unit checking
  -> runtime + ParamModel + FeatureGraph
  -> GeometryGraph / Geometry IR
  -> geometry runtime -> kernel-neutral API -> OCCT
                       \
                        -> lineage/evidence
source query recipes ----> scoped resolver
                          -> Resolved / Ambiguous / Broken
                          -> health / diagnostics
```

Higher layers request exact geometry through AICAD abstractions; OCCT classes, handles, and enumeration details do not become language/HIR/reference identity.

Read:

- [system-overview.md](system-overview.md) for layer contracts and production flow;
- [semantic-references.md](semantic-references.md) for Stage-4 reference architecture;
- [repository-layout.md](repository-layout.md) for ownership by directory/crate.

The frozen foundation plan under `docs/plan/` is traceability/history. Accepted decisions/specifications plus current implementation define what these docs describe.
