# Core language semantics

Status: current canonical Stage-3 language semantics. This document consolidates already-approved decisions and implemented Stage-2/3 behavior; it does not authorize new future syntax.

## Source structure and evaluation

AICAD source uses brace-delimited blocks and explicit semicolon-terminated statements. Indentation is formatting only and automatic semicolon insertion is not part of the language (DL-1). Current declaration forms are `let`, `const`, `param`, `fn`, `struct`, `enum`, `part`, and `import`; future planning vocabulary is not a declaration form unless it appears in the canonical grammar and parser.

AICAD's semantic core is functional/value-oriented (DL-2). Geometry/modeling operations consume values and return new values. Method/builder notation, where accepted, is sugar over ordinary functional calls and must not imply hidden in-place mutation. `var` plus explicit assignment rebinds a binding to a new value; it does not grant semantic in-place mutation of an underlying geometry object.

Lexical scopes own bindings. Forward references and declaration visibility follow the compiler's established binding/lowering rules. `part` is a source-level modeling container whose top-level named values can become explicit model outputs. A Stage-3 named output such as `LBracket.body` is a source/model name, not topology identity.

## Functions, algebraic data types, and control flow

Functions are ordinary typed callables. User-defined structs/enums/functions may use bare generic type parameters under D17; generic bounds/interfaces are not part of the current Stage-3 generic model. D27/DL-29 approves a future general nominal interface/protocol capability with explicit conformance, static checking, and bounded generics for Stage 6+, but it does not add that syntax or implementation to the current language. Enums support unit, tuple-payload, and record-payload variants, with corresponding pattern destructuring. `Result<T,E>` and `Optional<T>` are ordinary generic prelude enum types, not compiler-special semantic categories. Result propagation is explicit through ordinary control flow such as `match`; no `?`-style propagation syntax is current.

Current control flow includes expression/statement `if` and `match`, `for`, `while`, `loop`, `break`, `continue`, and `return`. `List<T>` literals and integer ranges support the Stage-2 iteration minimum approved by D16. `for` evaluates its iterable once; iteration order is deterministic for current built-in iterable values. `Set<T>`, `Map<K,V>`, comprehensions, arbitrary source-defined iterator protocols, async/parallel iteration, and implicit dimensional-range stepping are future work.

Recursion is a language capability, but concrete evaluator limits are resource/safety budgets rather than a promise that one current numeric ceiling is a permanent language semantic. Native stack overflow or unbounded execution is never an acceptable result.

## Runtime-backed standard functions

D18 defines RuntimeBuiltins as ordinary source calls, not compiler geometry intrinsics. A runtime-backed standard function participates in ordinary name resolution, argument binding, type checking, HIR call semantics, and runtime resource accounting. Its implementation is owned by the trusted runtime through the closed `BuiltinFnId` catalogue. There is no `HirExpr::GeometryIntrinsic`, arbitrary host callback registration, package/plugin native registration, or public plugin ABI implied by this mechanism.

D21/DL-23 keeps that catalogue closed while allowing future catalogue growth to be driven from a declarative/single-source description. Stable builtin identity must be deterministic and deliberately managed rather than accidentally derived from enum/source ordering. The exact catalogue-generation mechanism remains a Stage-5 implementation choice and does not create an open registration ABI.

D23/DL-25 approves future kernel-backed queries needed during ordinary source evaluation as ordinary typed runtime-backed calls. When such a result is required to continue evaluation, the runtime may synchronously materialize the minimum required upstream geometry, execute through the kernel-neutral adapter, and return an ordinary AICAD value. The exact query API, effect metadata, scheduling, and cache representation remain deferred; this ruling does not add a Stage-3 source query surface.

The current Safe CAD source catalogue is documented at `docs/developer/geometry/safe-cad-api.md`. It is intentionally narrower than internal Geometry IR/kernel capability. An internal `GeometryOp` or query does not become `.aicad` syntax merely because the dispatcher can execute it.

## Geometry, identity, and the kernel boundary

Public language/HIR semantics are kernel-neutral (D6). A source `Geometry` value and AICAD-owned semantic IDs are not OCCT objects, pointers, or persistent kernel handles. Raw kernel topology is context/epoch-local implementation data below the semantic boundary.

D22/DL-24 freezes the future semantic boundary among four distinct concepts: safe semantic geometry values, raw/unsafe AICAD-owned geometry handles, persistent semantic topology references, and kernel-native objects. A future raw handle is opaque at the source/HIR boundary, kernel-neutral in public semantics, scoped to its build/context/epoch, and invalid outside that lifetime. It is not an OCCT pointer or a Stage-4 persistent reference. Promotion from raw geometry into the safe semantic tier requires an explicit validation/adoption operation producing a new safe value with appropriate validation/provenance evidence; exact Stage-5 syntax, handle representation, and epoch encoding remain deferred.

Stage 3 provides feature identities, provenance, named outputs, operation-local lineage where available, and limited raw face/edge integer selectors. These mechanisms are distinct from persistent topology identity. Raw indices are topology/enumeration selectors only and may change meaning after topology-changing regeneration.

D25/DL-27 additionally requires future supported ordinary language abstraction to preserve engineering observability: wrapping geometry-producing/effectful work in functions, reusable parts/helpers, supported control flow, or libraries must not inherently erase feature identity, dependencies, parameter dependencies, dirty propagation, incremental rebuild behavior, source provenance, semantic-reference support, or source-to-geometry inspection. The exact graph/node/call-instance encoding is deliberately deferred and may be informed by Stage-4 evidence.

Persistent semantic topology references are Stage-4 work. The accepted D7 policy is fail-closed: resolution may produce `Resolved`, `Ambiguous`, or `Broken`; no arbitrary candidate may be silently selected. Automatic geometry-fingerprint fallback is disabled for the first Stage-4 implementation and may be used only for diagnostics/ranking/experiments unless a later owner decision changes that policy. Stage 4 has not started on the transition branch.

## Sketches, constraints, and source exposure

Stage 3 contains real internal sketch/entity/constraint/profile infrastructure: kernel-independent sketch/entity identity, solver-independent constraint semantics, numerical solving, solved-profile validation, and lowering of solved closed profiles to exact geometry. D11 makes the AICAD semantic constraint model authoritative above numerical solver adapters.

D28/DL-30 extends that solver-neutral principle to future Stage-6 assembly relations: mates/joints and observable pose remain AICAD-owned semantics, underconstraint/multiple-solution states must not silently inherit an arbitrary backend pose, and deterministic grounding/gauge handling belongs above the numerical solver. Exact Stage-6 canonicalization and solver implementation remain deferred.

That internal substrate is not a current source-language feature. The current `.aicad` grammar/runtime exposes no supported direct `sketch { ... }` authoring construct or source `Sketch`/`Profile` value. Current source modeling reaches geometry through the supported Safe CAD call surface. Future source integration must reuse the existing semantic substrate rather than redefine solver or kernel semantics.

## Determinism and equivalence

D5 defines layered determinism. AICAD-owned language/compiler structures and canonical AICAD serialization are deterministic for identical inputs, and byte-identical output is required where such canonical AICAD-owned serialization is defined. Exact B-rep serialization is not a cross-platform or cross-kernel-version identity guarantee.

Geometry is compared using the versioned D5 equivalence profile plus exact semantic requirements where explicitly required. Different kernel versions are compatibility comparisons, not deterministic identity. Kernel enumeration order is never canonical topology identity. D24/DL-26 makes the tolerance taxonomy normative: representation/validity, modeling/construction, approximation, solver, verification, and D5 equivalence/comparison tolerances are distinct domains. A value from one domain must not silently become another domain's default, and no tolerance may be silently widened merely to make a failed operation or test pass. Existing D5/D19 values remain unchanged; future domain-specific defaults remain deferred until separately specified.

## Future assembly/configuration/asset identity baselines

D26/DL-28, D29/DL-31, and D30/DL-32 establish semantic baselines for future Stage-6+ work without adding current Stage-3 syntax or implementation. Assembly component-definition identity, logical instance identity, nested occurrence/path identity, configuration identity, external-asset identity, in-instance semantic references, and BOM classification identity are distinct. Configurations are immutable overlays: suppression is not deletion, and replacement preserves logical-slot/configuration provenance where applicable. Imported/external assets use immutable content identity plus normalized provenance rather than filesystem paths, temporary URLs, importer indices, or kernel-object identity. Exact ID encodings, configuration storage/syntax, asset hash/schema/security/packaging details remain deliberately deferred.

## Product/file naming

The product and language name is **AICAD** (D14). Canonical source files use `.aicad`; the project manifest name is `aicad.toml`; `.aicadpkg` is the approved packaged-artifact direction if/when such a bundle format is defined. `CAD-IR` may remain an internal compiler/IR label but is not public product branding. The current executable happens to be named `cad`; that implementation identifier does not rename the product or language.
