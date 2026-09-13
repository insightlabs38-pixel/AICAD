# Compiler and runtime

The core Rust workspace separates parsing, semantic lowering/type checking, runtime evaluation, feature/incremental analysis, geometry graph construction, and kernel dispatch.

Key crates include `cad-lexer`, `cad-parser`, `cad-ast`, `cad-hir`, `cad-types`, `cad-units`, `cad-compiler`, `cad-runtime`, and `cad-diagnostics`.

Runtime-backed standard functions are ordinary source calls: they resolve/type-check through the same language call machinery, while their implementations are supplied by trusted runtime dispatch rather than a user-defined function body. The closed catalogue is defined in `cad-hir` and consumed by the runtime; adding a builtin is not equivalent to adding a compiler intrinsic.

Geometry evaluation is staged through backend-neutral geometry values/IR before native-kernel realization. Keep compiler semantics independent of OCCT object identity or pointers.
