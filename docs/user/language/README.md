# Language guide

AICAD source uses explicit braces and semicolons with a typed, value-oriented execution model. Geometry is not a separate macro language: modeling functions participate in the same name binding, type checking, function calls, and control flow as ordinary source functions.

The pages in this section focus on language features exercised by current compiler/runtime tests and Stage-2/3 examples:

- [Types and physical units](types-and-units.md)
- [Functions and control flow](functions-and-control-flow.md)
- [Parameters and derived expressions](parameters.md)

AICAD also implements general language machinery for user-defined structs, data-carrying enums, generic types/functions, lists/ranges, pattern matching, `Result<T,E>`, and `Optional<T>`. Those features share the same compiler/runtime pipeline; this guide concentrates on the subset most directly useful for current CAD authoring rather than reproducing the complete language specification.

For exact grammar/specification status, consult `specs/language/` and accepted RFCs. Do not treat examples in the frozen `docs/plan/` bundle as proof that a syntax form is implemented.
