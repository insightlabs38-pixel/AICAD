# HIR and type checking

`cad-hir` is the semantic boundary between parsed source shape and executable language meaning.

## HIR and bindings

Lowering resolves declarations into HIR structures and AICAD-owned `BindingId`s. Those identities feed later systems such as parameter dependency analysis, feature provenance, and runtime global lookup.

The HIR remains kernel-neutral. A source `Geometry` value is an opaque nominal type; HIR does not embed an OCCT `TopoDS_Shape`, kernel pointer, or persistent topology handle.

## Type checking and units

The checker validates ordinary language calls, generic/data-type usage, and engineering dimensions. Dimensioned quantities are resolved through the same type/unit rules regardless of whether the eventual consumer is scalar source code or a geometry function.

For example, the Safe CAD signature:

```text
cylinder(radius: Length, height: Length) -> Geometry
```

is checked as a normal function signature. It does not get a geometry-specific bypass that silently accepts arbitrary floats.

## Standard types

Stage 3 makes the standard spatial structs needed by the builtin catalogue available in every program's standard type environment. This is necessary because RuntimeBuiltin signatures are seeded globally and type-checked even when a particular source file does not call every builtin.

The standard geometry/spatial set includes `Vector2<T>`, `Vector3<T>`, `Point2`, `Point3`, `Axis3`, `Frame3`, and `Plane`. Runtime conversion performs the numerical spatial-invariant checks that cannot be established by nominal source typing alone.

## RuntimeBuiltin functions

`cad_hir::builtins::BuiltinFnId` is a closed compiler/runtime-owned identity set. Each spec supplies the same sort of parameter/return signature used for an ordinary source function. Lowering seeds those names as `FunctionImplementation::RuntimeBuiltin`; call syntax, binding, and type checking remain ordinary.

This is distinct from a compiler intrinsic. A RuntimeBuiltin does not introduce grammar or a special type/lowering path inaccessible to normal calls, and user packages cannot register arbitrary native callbacks into this set.

The catalogue is documented in [Safe CAD source API](../geometry/safe-cad-api.md).
