# Architecture

AICAD is layered so that language semantics, parametric identity, and geometry intent remain independent of the concrete CAD kernel.

```text
.aicad source
  ↓
lexer / parser / AST
  ↓
HIR + name/type/unit checking
  ↓
runtime + ParamModel + FeatureGraph
  ↓
GeometryGraph / Geometry IR
  ↓
geometry runtime dispatcher
  ↓
cad-kernel-api
  ↓
cad-occt-bridge
  ↓
native/occt_bridge
  ↓
OCCT exact B-rep
```

The critical direction is downward: higher layers can request geometry through stable AICAD abstractions, but OCCT classes, handles, and enumeration details must not become language/HIR identity.

Read [system-overview.md](system-overview.md) for the layer contracts and [repository-layout.md](repository-layout.md) for ownership by directory/crate.

The frozen foundation plan under `docs/plan/` remains useful for original intent and traceability, but current code plus accepted decisions/specifications define what this documentation describes today.
