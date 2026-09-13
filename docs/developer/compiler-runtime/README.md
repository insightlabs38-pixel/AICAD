# Compiler and runtime

The current pipeline is a conventional typed-language frontend feeding a bounded interpreter and a separate geometry realization phase:

```text
source
  → lexer/parser/AST
  → HIR lowering + bindings
  → type/unit checking
  → HIR interpretation
  → GeometryGraph construction
  → kernel dispatch when geometry is requested
```

This separation is deliberate. The compiler does not lower every CAD operation into bespoke compiler intrinsics, and the interpreter does not carry live OCCT objects as ordinary source values.

Read:

- [Frontend](frontend.md)
- [HIR and type checking](hir-and-typechecking.md)
- [Runtime](runtime.md)

For the geometry representation produced by runtime-backed calls, continue to [Geometry IR](../geometry/geometry-ir.md).
