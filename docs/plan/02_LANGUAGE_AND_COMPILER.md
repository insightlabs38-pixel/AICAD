# 02 — Language and Compiler Specification

## 1. Syntax philosophy

Use familiar syntax. The novelty belongs in engineering semantics, not punctuation.

Recommended influences:

- Rust: typed bindings, enums, pattern matching, explicit unsafe sections, traits/interfaces.
- TypeScript: approachable type syntax and editor ergonomics.
- Python: iterators/generators/readability.
- C/C++: systems-level mental model for raw access.

Example:

```aicad
part MotorMount {
    param width: Length = 80mm;
    param height: Length = 60mm;
    param thickness: Length = 5mm;

    let base = plate([width, height], thickness);

    for p in corner_points(
        size=[width, height],
        inset=8mm
    ) {
        base.cut(hole(center=p, diameter=5.5mm));
    }

    expose mounting_face = query base.faces {
        planar;
        normal ~= +Z;
        largest(area);
    };
}
```

## 2. Source units

Recommended source forms:

- `.cad` or `.aicad` for modules;
- `.aicad` may also be the bundled project container. If ambiguity is undesirable, use `.cadl` for source and `.aicad` for project bundles.

This is a branding decision, not a semantic dependency.

## 3. Lexical elements

Support:

- UTF-8 source;
- ASCII keywords;
- Unicode identifiers optionally allowed but normalized;
- decimal/scientific numeric literals;
- unit suffixes (`5mm`, `12.4MPa`, `30deg`);
- strings and raw strings;
- line and block comments;
- doc comments (`///`);
- attributes/annotations (`@material(...)`);
- module/import paths.

## 4. Declarations

Core declaration forms:

```aicad
let x = ...;
const G = ...;
param width: Length = 80mm;

fn name(...) -> Type { ... }
struct Name { ... }
enum Name { ... }
interface Name { ... }
part Name { ... }
assembly Name { ... }
drawing Name { ... }
simulation Name { ... }
requirement Name { ... }
test "name" { ... }
configuration Name { ... }
```

## 5. Mutability model

Prefer immutable values by default.

```aicad
let body = box(...);          // immutable binding
var working = body;           // mutable binding
working = cut(working, hole); // explicit reassignment
```

For ergonomic high-level modeling, method-style builder syntax can desugar to functional operations:

```aicad
body.cut(hole(...));
```

must have deterministic, documented semantics. Internally it can lower to an SSA-like form so the feature DAG remains explicit.

## 6. Functions

Support:

- typed positional parameters;
- named parameters;
- optional/default parameters;
- generic parameters;
- pure function annotation;
- methods/extensions;
- closures;
- recursion;
- generators/yield;
- result/error types.

Example:

```aicad
pure fn polar_point(
    radius: Length,
    angle: Angle
) -> Point2 {
    return [radius * cos(angle), radius * sin(angle)];
}
```

## 7. Control flow

Required:

```aicad
if / else
for
while
loop
break
continue
match
return
yield
```

Loops and conditionals must work at both engineering and raw geometry layers.

## 8. Iterators and geometry queries

All major collections should be iterable:

```aicad
for face in body.faces() { ... }
for edge in body.edges().where(|e| e.convex) { ... }
for component in assembly.components() { ... }
```

Queries should support lazy semantics where practical so filtering a large assembly does not materialize unnecessary arrays.

## 9. Pattern matching

Useful for geometry types:

```aicad
match surface {
    Plane(p) => ..., 
    Cylinder { radius, axis } => ...,
    BSpline(s) => ...,
    _ => ...
}
```

and semantic features:

```aicad
match feature {
    Hole { diameter, .. } => ...,
    Fillet { radius, .. } => ...,
    _ => ...
}
```

## 10. Modules and imports

```aicad
import std.fasteners::{ISO4762};
import robotics.cycloidal;
import ./housing;
```

Requirements:

- deterministic resolution;
- package namespaces;
- explicit versioning in lockfile, not source unless desired;
- private/public symbols;
- re-exports;
- cyclic import detection.

## 11. Compile-time evaluation

Support a clean `comptime` model:

```aicad
comptime {
    assert bolt_count >= 3;
}
```

Use cases:

- generate lookup tables;
- derive feature definitions;
- validate configuration combinations;
- resolve standard component metadata;
- perform cheap symbolic checks.

Avoid unstructured textual macros.

## 12. Macros / metaprogramming

If introduced, prefer typed AST macros or derives:

```aicad
@derive(Inspectable, Serializable)
struct BearingMount { ... }
```

or declarative feature-generation macros with hygiene.

Do not introduce C-preprocessor-style token substitution into the core language.

## 13. Purity and determinism

Functions may be declared:

```aicad
pure fn ...
```

Pure functions:

- cannot access nondeterministic state;
- can be memoized;
- can be parallelized;
- can be compile-time evaluated where arguments are constant.

Nondeterministic APIs require explicit capabilities:

```aicad
Random(seed=1234)
```

instead of implicit global randomness.

## 14. Error model

Use typed errors/results for recoverable operations:

```aicad
let result: Result<Solid, GeometryError> = try_fillet(...);
```

Compiler/runtime failures carry:

- stable error code;
- category;
- source span;
- semantic entity path;
- observed values;
- expected values;
- candidate fixes where reliable;
- backend details optionally.

## 15. Execution budgets

Turing completeness means nontermination/resource explosion is possible. Every build runs under a budget:

```aicad
execution {
    max_cpu_time: 30s;
    max_memory: 2GiB;
    max_geometry_ops: 100_000;
    max_faces: 5_000_000;
    max_solids: 50_000;
}
```

Project defaults can be overridden by trusted users, but packages cannot silently raise them.

## 16. Raw-handle lifetime model

Recommended syntax:

```aicad
unsafe geometry {
    let face: Face* = raw_face(body, 17);
    let edges: List<Edge*> = raw_edges(face);
    ...
    body = adopt_validated(raw_shape);
}
```

Rules:

- raw handles carry an internal geometry epoch;
- a topology-mutating operation starts a new epoch for affected shapes;
- using a stale raw handle is an error;
- raw handles cannot be serialized or exported from public APIs without explicit unsafe wrapper types;
- stable references must be recreated/exported through semantic naming/query logic.

This gives the power of pointer-like access without pretending raw topology is persistent.

## 17. Compiler phases

Recommended pipeline:

1. Parse source -> AST.
2. Resolve modules/imports.
3. Name binding.
4. Type + dimensional checking.
5. Generic instantiation.
6. Compile-time evaluation.
7. Lower to typed HIR.
8. Build parameter dependency graph.
9. Normalize constraints.
10. Build feature DAG.
11. Resolve package/plugin capability requirements.
12. Lower high-level features to Geometry IR.
13. Execute geometry graph incrementally.
14. Capture topology lineage and semantic mappings.
15. Validate geometry.
16. Execute requirements/tests requested by build profile.
17. Emit artifacts/caches/reports.

## 18. Editor parser strategy

Maintain a Tree-sitter grammar for editor responsiveness even if the production compiler uses another parser implementation. Tree-sitter supports incremental syntax-tree updates and tolerates incomplete/errorful source, which fits interactive editing.

The production parser and Tree-sitter grammar must share a conformance test corpus so syntax never diverges.

## 19. Compiler conformance artifacts

Maintain:

```text
spec/grammar.ebnf
spec/semantics.md
spec/type-system.md
spec/diagnostics.md
tests/parser/
tests/typecheck/
tests/runtime/
tests/geometry/
```

Every syntax/semantic change requires:

- spec update;
- positive test;
- negative test;
- core-skill update if user-visible;
- compatibility note.
