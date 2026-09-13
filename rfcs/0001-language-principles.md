# RFC-0001: Language Principles

- Status: **Accepted Stage-0 baseline** (`project/DECISION_LOG.md#DL-10`).
- Owner rulings incorporated/clarifying this RFC: DL-1 (surface syntax), DL-2 (value/mutation semantics), DL-4 (branding/file names), DL-7 (intrinsic RFC gate), DL-12/DL-17+AICAD-064A (determinism/equivalence profile), DL-13 (collections/iteration minimum), DL-14 (generic ADTs/functions), DL-15 (RuntimeBuiltin mechanism), DL-19 (sketch semantic object model), DL-21 (type-closed always-seeded RuntimeBuiltin environment).
- Canonical current source grammar: `specs/language/grammar.ebnf`.

## 1. Summary

AICAD is a typed, compiled, Turing-complete general-purpose programming language and execution environment for mechanical engineering. This RFC establishes the cross-cutting rules that later subsystems must preserve: explicit source semantics, value-oriented modeling, exact geometry below a kernel-neutral semantic layer, deterministic AICAD-owned state, typed engineering quantities, semantic identity above kernel topology, and a strict process for privileged compiler mechanisms.

The Stage-0 rationale remains valid, but later owner decisions supersede the RFC's original open-question labels. Current compatibility details live in `specs/language/` rather than in old planning examples.

## 2. Non-negotiable language/platform invariants

1. High-level and low-level CAD belong to one language; advanced work must not require a second hidden language.
2. Source and AICAD-owned semantic state are authoritative design intent. Generated B-rep, meshes, and exports are derived artifacts.
3. Exact B-rep is the primary compiled geometry; mesh/tessellation is not canonical design truth.
4. Units and dimensions are semantic types, not conventions over untyped floating-point values.
5. Public language/HIR/semantic types remain kernel-neutral; OCCT objects and pointer identity do not cross upward as public semantics.
6. Raw topology, where eventually exposed, is explicit, ephemeral, and epoch/context-bound rather than durable identity.
7. Semantic references are preferred over enumeration indices; ambiguity is an error, never an arbitrary selection.
8. AICAD-owned canonical structures/serialization are deterministic for identical inputs. Geometry equivalence follows D5/D19's versioned comparison profile; byte-identical OCCT B-rep is not required across platforms/kernel versions.
9. GUI/source/automation must operate over the same semantic model rather than maintaining hidden GUI-only design state.
10. AI/tooling may propose source changes and consume structured compiler/runtime output but may not bypass validation or semantic gates.
11. Extensions should prefer ordinary AICAD source/packages over compiler intrinsics; privileged mechanisms require explicit architectural justification.

## 3. Product identity and file conventions — D14/DL-4

The public product/language name is **AICAD**. Canonical source files use `.aicad`; the project manifest is `aicad.toml`; `.aicadpkg` is the approved packaged-artifact direction if/when a single-file package/bundle is defined. `CAD-IR` may remain an internal compiler/IR label but is not product branding.

The current executable is named `cad` and exposes `cad build`; that implementation detail does not rename the product or language.

## 4. Surface syntax family — D1/DL-1

AICAD uses brace-delimited blocks and explicit semicolon-terminated statements. Indentation is formatting only; automatic semicolon insertion is not part of the language. Current syntax is defined by `specs/language/grammar.ebnf`, not by future examples in `docs/plan/`.

The current grammar supports the declaration/control-flow/generic constructs implemented through Stage 3. Planned declarations such as interfaces, assemblies, requirements/tests, raw/unsafe geometry blocks, and direct sketch authoring are not current syntax merely because they appear in the foundation plan or are lexically reserved.

## 5. Value/functional semantics — D2/DL-2

AICAD's semantic core is functional and value-oriented. Modeling operations consume values and produce new values. Method/builder notation, where accepted, is sugar over ordinary functional calls; it never creates a second in-place semantic mutation model.

For example, the intended semantic distinction is:

```aicad
let cut_result: Geometry = cut(base, cutter);
var body: Geometry = base;
body = cut(body, cutter);
```

The assignment explicitly rebinds `body` to a new value. A bare expression such as `body.cut(cutter);` discards its result and does not mutate `body` in place. Internal OCCT algorithms may mutate kernel-owned implementation objects, but that is below the public/HIR semantic boundary.

## 6. Compiler intrinsics and RuntimeBuiltins — D9/D18

A genuinely new compiler intrinsic requires an RFC demonstrating that it cannot reasonably be implemented as ordinary AICAD source, a standard package, or an existing kernel/service operation exposed through existing language mechanisms. The RFC must define semantics, types, lowering, determinism/resource behavior, and why an ordinary-library solution is inadequate (DL-7).

Runtime-backed standard functions are **not** compiler intrinsics under that rule (DL-15). A RuntimeBuiltin is an ordinary source call that goes through ordinary name resolution, argument binding, type checking, HIR call semantics, and runtime resource accounting. Only the callable implementation is runtime-owned through the closed `BuiltinFnId` catalogue. There is no `HirExpr::GeometryIntrinsic`, arbitrary host callback registration, or package/plugin native registration implied by this mechanism.

The current Safe CAD catalogue is documented at `docs/developer/geometry/safe-cad-api.md`. Internal Geometry IR/kernel operations are not automatically source functions.

D20 is resolved by DL-21 and implemented by AICAD-076A. The always-seeded builtin environment may reference approved AICAD standard nominal types such as `Point3`, `Axis3`, `Frame3`, and `Plane`. It must remain type-closed: every nominal type required by an automatically seeded builtin signature is automatically available to eager signature collection/type checking. That eager checking remains authoritative, and the catalogue plus its required standard type declarations must be independently type-validatable. `with_geometry_types` may remain as an idempotent compatibility/composition helper rather than a prerequisite for builtin type correctness.

This remains a closed first-party standard environment; it does not authorize arbitrary plugin/runtime type injection, host callbacks, OCCT types in public/HIR signatures, or a dynamic extension ABI. AICAD-076's scalar-decomposition signatures were temporary compatibility workarounds, not the intended long-term Safe CAD API architecture.

## 7. Current grammar vs. historical/future examples

The original Stage-0 grammar sketch intentionally seeded later parser work. Stage-2/3 subsequently added the owner-approved List/range minimum (D16), generic structs/enums/functions and payload variants (D17), and other implemented syntax. The **current** grammar is now `specs/language/grammar.ebnf`.

Accordingly, old RFC/plan examples containing `interface`, `assembly`, `requirement`, `test`, direct `sketch { ... }`, raw/unsafe geometry, closures/generators, or other future constructs are design-history/future examples unless separately present in the canonical current grammar. They are not silently promoted by being shown in an older document.

## 8. Determinism clarification — D5/D19

The original RFC left the cross-platform equivalence contract open. That question is resolved by DL-12 and D19/AICAD-064A:

- AICAD-owned compiler/semantic structures and canonical serialization are deterministic for identical inputs, byte-identical where a canonical AICAD serialization is defined.
- Same locked kernel environments and supported cross-platform comparisons use semantic/numerical validation, not byte-identical B-rep.
- Different kernel versions are compatibility comparisons rather than deterministic identity.
- The versioned D5 comparison profile is dimension-aware and has a calibrated v1 profile in `crates/cad-validation` / `specs/language/types.md`.

D5 equivalence tolerance must not be reused as solver convergence, construction, approximation, verification, or private validity tolerance without a separate approved policy.

## 9. Sketch object clarification — D3

The original RFC treated the sketch object/binding model as a later question. D3 is resolved by DL-19: Stage-3 sketch semantics use an explicit, kernel-independent `Sketch` object with local semantic entity IDs and no hidden ambient/global mutation. Surface block syntax may eventually lower to that same model, but the current `.aicad` grammar does **not** expose direct sketch authoring yet. This resolution does not create Stage-4 persistent topology identity.

## 10. Historical rationale and rejected alternatives

Stage-0 considered and rejected indentation-sensitive canonical syntax, two independently meaningful mutation models, an informal/no-RFC compiler-intrinsic boundary, and overloading one file extension for both source and packaged artifacts. The reasons remain: explicit delimiters improve deterministic parsing/tooling; one value-semantic core keeps HIR/feature dependencies inspectable; intrinsic creep needs an enforceable process; and source/package identity should be unambiguous.

Later D16/D17/D18 decisions followed the same principle: add the minimum general mechanism that solves the actual requirement instead of hard-coded `Result`/geometry special cases or an unrestricted host-extension channel.

## 11. Still-open boundaries

This RFC does not decide future interface/bounded-generic syntax, closures/generators, package/plugin architecture, source-visible kernel query evaluation, raw/unsafe Stage-5 syntax, or later verification declarations. Those require their own approved work/decisions before entering the canonical grammar.
