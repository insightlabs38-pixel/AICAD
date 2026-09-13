# Core language semantics

Status: current canonical Stage-3 language semantics. This document consolidates already-approved decisions and implemented Stage-2/3 behavior; it does not authorize new future syntax.

## Source structure and evaluation

AICAD source uses brace-delimited blocks and explicit semicolon-terminated statements. Indentation is formatting only and automatic semicolon insertion is not part of the language (DL-1). Current declaration forms are `let`, `const`, `param`, `fn`, `struct`, `enum`, `part`, and `import`; future planning vocabulary is not a declaration form unless it appears in the canonical grammar and parser.

AICAD's semantic core is functional/value-oriented (DL-2). Geometry/modeling operations consume values and return new values. Method/builder notation, where accepted, is sugar over ordinary functional calls and must not imply hidden in-place mutation. `var` plus explicit assignment rebinds a binding to a new value; it does not grant semantic in-place mutation of an underlying geometry object.

Lexical scopes own bindings. Forward references and declaration visibility follow the compiler's established binding/lowering rules. `part` is a source-level modeling container whose top-level named values can become explicit model outputs. A Stage-3 named output such as `LBracket.body` is a source/model name, not topology identity.

## Functions, algebraic data types, and control flow

Functions are ordinary typed callables. User-defined structs/enums/functions may use bare generic type parameters under D17; generic bounds/interfaces are not part of the current Stage-3 generic model. Enums support unit, tuple-payload, and record-payload variants, with corresponding pattern destructuring. `Result<T,E>` and `Optional<T>` are ordinary generic prelude enum types, not compiler-special semantic categories. Result propagation is explicit through ordinary control flow such as `match`; no `?`-style propagation syntax is current.

Current control flow includes expression/statement `if` and `match`, `for`, `while`, `loop`, `break`, `continue`, and `return`. `List<T>` literals and integer ranges support the Stage-2 iteration minimum approved by D16. `for` evaluates its iterable once; iteration order is deterministic for current built-in iterable values. `Set<T>`, `Map<K,V>`, comprehensions, arbitrary source-defined iterator protocols, async/parallel iteration, and implicit dimensional-range stepping are future work.

Recursion is a language capability, but concrete evaluator limits are resource/safety budgets rather than a promise that one current numeric ceiling is a permanent language semantic. Native stack overflow or unbounded execution is never an acceptable result.

## Runtime-backed standard functions

D18 defines RuntimeBuiltins as ordinary source calls, not compiler geometry intrinsics. A runtime-backed standard function participates in ordinary name resolution, argument binding, type checking, HIR call semantics, and runtime resource accounting. Its implementation is owned by the trusted runtime through the closed `BuiltinFnId` catalogue. There is no `HirExpr::GeometryIntrinsic`, arbitrary host callback registration, package/plugin native registration, or public plugin ABI implied by this mechanism.

The current Safe CAD source catalogue is documented at `docs/developer/geometry/safe-cad-api.md`. It is intentionally narrower than internal Geometry IR/kernel capability. An internal `GeometryOp` or query does not become `.aicad` syntax merely because the dispatcher can execute it.

## Geometry, identity, and the kernel boundary

Public language/HIR semantics are kernel-neutral (D6). A source `Geometry` value and AICAD-owned semantic IDs are not OCCT objects, pointers, or persistent kernel handles. Raw kernel topology is context/epoch-local implementation data below the semantic boundary.

Stage 3 provides feature identities, provenance, named outputs, operation-local lineage where available, and limited raw face/edge integer selectors. These mechanisms are distinct from persistent topology identity. Raw indices are topology/enumeration selectors only and may change meaning after topology-changing regeneration.

Persistent semantic topology references are Stage-4 work. The accepted D7 policy is fail-closed: resolution may produce `Resolved`, `Ambiguous`, or `Broken`; no arbitrary candidate may be silently selected. Automatic geometry-fingerprint fallback is disabled for the first Stage-4 implementation and may be used only for diagnostics/ranking/experiments unless a later owner decision changes that policy. Stage 4 has not started on the transition branch.

## Sketches, constraints, and source exposure

Stage 3 contains real internal sketch/entity/constraint/profile infrastructure: kernel-independent sketch/entity identity, solver-independent constraint semantics, numerical solving, solved-profile validation, and lowering of solved closed profiles to exact geometry. D11 makes the AICAD semantic constraint model authoritative above numerical solver adapters.

That internal substrate is not a current source-language feature. The current `.aicad` grammar/runtime exposes no supported direct `sketch { ... }` authoring construct or source `Sketch`/`Profile` value. Current source modeling reaches geometry through the supported Safe CAD call surface. Future source integration must reuse the existing semantic substrate rather than redefine solver or kernel semantics.

## Determinism and equivalence

D5 defines layered determinism. AICAD-owned language/compiler structures and canonical AICAD serialization are deterministic for identical inputs, and byte-identical output is required where such canonical AICAD-owned serialization is defined. Exact B-rep serialization is not a cross-platform or cross-kernel-version identity guarantee.

Geometry is compared using the versioned D5 equivalence profile plus exact semantic requirements where explicitly required. Different kernel versions are compatibility comparisons, not deterministic identity. Kernel enumeration order is never canonical topology identity. The D5 comparison profile is distinct from solver convergence, modeling/construction, approximation, verification/assertion, and private representation-validity tolerances; values from one category must not be copied into another without an approved policy.

## Product/file naming

The product and language name is **AICAD** (D14). Canonical source files use `.aicad`; the project manifest name is `aicad.toml`; `.aicadpkg` is the approved packaged-artifact direction if/when such a bundle format is defined. `CAD-IR` may remain an internal compiler/IR label but is not public product branding. The current executable happens to be named `cad`; that implementation identifier does not rename the product or language.
