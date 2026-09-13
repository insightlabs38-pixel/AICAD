# Language surface audit

Audit basis: `d82bf82e53a98f3117d3dc17707f1756822e593d`.

The future planning corpus contains both normative capability goals and illustrative syntax. This audit classifies each assumed construct against the actually implemented/approved language. “Approved” means an owner decision/RFC has accepted the *capability or general mechanism*; it does not imply that every example spelling is canonical.

## Current implemented baseline

**CURRENT IMPLEMENTATION FACTS**

- `cad_ast::Item` implements `let`, `const`, `param`, `fn`, `struct`, `enum`, `part`, and `import` items. It does **not** implement `interface`, `assembly`, `requirement`, `test`, or configuration items even where the grammar/lexer anticipates names.
- The lexer reserves `interface`, `assembly`, `requirement`, and `test`; reservation is not semantic implementation.
- Expressions include ordinary calls, method-style syntax/lowering mechanisms, field/index access, list/range/record forms, control flow and approximate equality support. There is no implemented closure literal, map/set literal, query-DSL syntax, or pattern-predicate language.
- Generic type references and ordinary generic machinery exist in stages, but generic **bounds/interfaces** are not implemented.
- `Result`/`Optional` demonstrate that general language/library mechanisms should be used instead of compiler-special-casing generic data types.
- D18/DL-15 provides runtime-backed standard functions through ordinary calls. This is an implementation mechanism, not new call syntax.
- The canonical language-spec directory is incomplete: `specs/language/README.md` names semantics/types/diagnostics specifications that do not exist at the frozen revision.

## Construct-by-construct classification

| Construct | Evidence / current state | Classification | Roadmap treatment |
|---|---|---|---|
| Closures / lambdas | RFC-0004 and `docs/plan/02...`/`03...` adopt callable/closure capability; no closure literal in current AST/expression implementation | APPROVED_BUT_NOT_IMPLEMENTED | Specify syntax, capture/value semantics, typing and resource rules before Stage5 APIs require callback arguments. Prefer ordinary callable values over geometry-specific callback magic. |
| `Map<K,V>` | D16/RFC-0004 accepts general collections; future plans use maps for semantic outputs/configuration; current standard value/runtime surface does not establish full Map semantics | APPROVED_BUT_NOT_IMPLEMENTED | Implement as general collection with deterministic iteration/order contract where externally observed. No special `Map` compiler behavior for geometry. |
| `Set<T>` | Same collection decision as Map; not current implemented source/runtime collection | APPROVED_BUT_NOT_IMPLEMENTED | General collection; define equality/hash/order requirements and deterministic serialization. |
| Richer iterators | DL-13/D16 approves `for var in iterable` and an iteration protocol; current implementation is narrower than future lazy/query examples | APPROVED_BUT_NOT_IMPLEMENTED | Implement general iteration protocol incrementally; geometry collections participate through it. |
| Interfaces / protocols | Plans 02/03/07 depend on interfaces; lexer/grammar anticipate `interface`, but AST/runtime/type checker do not implement it | APPROVED_BUT_NOT_IMPLEMENTED | General language feature needed before Stage6 reusable mechanical interfaces and generic bounds. Do not make an assembly-only protocol subsystem. |
| `implements` | Used in future assembly examples; no implemented item/member semantics | SPECIFICATION_NEEDED | Freeze conformance declaration/coherence/visibility rules as part of general interfaces. Could be explicit declaration syntax or derivable structural mechanism, but must not be invented only inside assembly parser code. |
| Generic bounds | Plan uses examples such as `T: MotorMount`; type parameters currently do not carry such bounds | SPECIFICATION_NEEDED | Define bound syntax and type-checking after interface semantics. Stage6 tasks that rely on bounded reusable components must depend on it. |
| Annotations / attributes | Future docs use annotations/attributes for metadata/suppression/etc.; no canonical implemented general attribute model | SPECIFICATION_NEEDED | Add only if cross-cutting metadata needs compiler-visible attachment. Prefer ordinary declarations/data for configuration values. Define namespace and unknown-attribute behavior before use. |
| Configuration declarations | Plan07 uses `configuration` concepts; no current grammar/AST declaration | SPECIFICATION_NEEDED | Stage6 owner/spec decision. Could be privileged declaration if project-wide discoverability/stable identity requires it; otherwise ordinary typed data plus a resolved configuration service may suffice. |
| Configuration rules | Future examples imply declarative rules/validity constraints | REPLACE_WITH_EXISTING_GENERAL_MECHANISM | Prefer ordinary boolean/requirement expressions over a bespoke mini-language unless rule dependency/discovery needs privileged semantics. Configuration declaration may own named rules, but predicates should remain general expressions/functions. |
| Suppression syntax | Plan examples imply suppression attribute/state; no current syntax | SPECIFICATION_NEEDED | First define suppression semantics (identity/refs/BOM/provenance). Then choose the smallest spelling; likely configuration overlay data, not mutable syntax on parts. |
| Replacement semantics | Plan07 needs component replacement/variants; no language construct | REPLACE_WITH_EXISTING_GENERAL_MECHANISM | Prefer typed configuration/variant data plus interface compatibility rather than a dedicated compiler replacement operator. Semantic rules still need a spec. |
| Test declarations | `test` reserved/mentioned but not in implemented `Item` | SPECIFICATION_NEEDED | Likely justified privileged declaration because discovery, stable source spans/IDs, runner integration and profiles are universal. Keep assertion predicates ordinary where possible. |
| Requirement declarations | `requirement` reserved/mentioned but not implemented | SPECIFICATION_NEEDED | Likely core declaration for stable IDs, traceability, hard/soft metadata and discovery; evaluation predicates can be general expressions/library calls. |
| `assert` / `expect` | Future verification examples use assertion behavior; no approved implemented statement identified | SPECIFICATION_NEEDED | Decide whether a minimal assertion primitive is needed to capture source span/evidence. Most domain-specific assertions should be library predicates evaluated by verification runtime. |
| `~=` approximate comparison | Grammar/expression machinery includes approximate-equality representation; RFC-0004 recognizes approximate comparison, but full tolerance semantics are not frozen | APPROVED_BUT_NOT_IMPLEMENTED | Keep general operator only after explicit tolerance operand/profile semantics are canonical. Do not bind it to D5 equivalence tolerance. |
| `requires` / `ensures` | Plan08 illustrative contract syntax; no current keyword/AST/runtime | DEFER | Preserve contract capability in Stage7 design, but do not block first verification slice. Promote only after call-boundary semantics, inheritance/composition and evidence value are specified. |
| `invariant` | Plan08 illustrative; no implementation | DEFER | Same as contracts. Functional/value semantics reduce need for object-mutation invariants; assembly/config/part invariants may still justify a declaration later. |
| Hard / soft requirements | Concept required by plan08; no dedicated source syntax | SPECIFICATION_NEEDED | Semantics belong in requirement/constraint IR. Source spelling can be metadata/fields; do not make solver penalty weights source semantics. |
| Unsafe geometry | RFC-0002 requires an unsafe/raw tier concept; no implemented `unsafe geometry` block/token at audit revision | SPECIFICATION_NEEDED | Decide whether type separation alone is sufficient or language-level unsafe effect/scoping is justified. Capability is preserved either way. |
| Low-level kernel namespaces | Plan needs low-level geometry, but D6 forbids kernel-specific public APIs | REMOVE_FROM_PLAN_EXAMPLE | Replace any `occt.*`, kernel-class, or backend-named source example with AICAD-owned `geometry.raw`/`topology`-style kernel-neutral APIs. Internal adapter namespaces remain implementation detail. |
| Handles / raw topology types | RFC-0002 and Stage4 AICAD-093 require epoch-bound raw handles; concrete public type spellings not fixed | SPECIFICATION_NEEDED | Use opaque AICAD-owned `RawVertexHandle`/`RawEdgeHandle`/... or equivalent; nonserializable, scope/epoch checked. Never pointer-looking `KernelShape*`. |
| Query DSL constructs | Stage4 plan defines query AST/IR and predicates; source examples may use fluent/query-like expressions; no dedicated query syntax | REPLACE_WITH_EXISTING_GENERAL_MECHANISM | Prefer ordinary typed query builder values/functions + closures/predicates. Add syntax only if a demonstrated ergonomic/semantic need survives Stage4. |
| Pattern predicates | Reference/query plans need generated-by/adjacent/planar/etc.; no pattern language implementation | REPLACE_WITH_EXISTING_GENERAL_MECHANISM | Typed predicate functions/combinators over query objects should suffice initially. Keep query IR richer internally. |
| Assembly declarations | Plan07 expects assembly definitions; lexer reserves `assembly`, AST does not implement it | SPECIFICATION_NEEDED | A dedicated declaration may be justified for stable identity/discovery/nesting. Specify after instance/identity model is agreed. |
| Component declarations | Examples use component concepts; no dedicated language item | REPLACE_WITH_EXISTING_GENERAL_MECHANISM | Prefer typed parts/functions/records or assembly-domain library values unless stable definition identity requires a declaration form. Decide alongside assembly identity model. |
| Instance declarations | Needed for named stable assembly instance identity; no current syntax | SPECIFICATION_NEEDED | Likely assembly-body semantic declaration, because a plain list index/runtime value is insufficient for stable identity/ref paths. |
| Mate / joint declarations | Plan uses domain-specific declarations | SPECIFICATION_NEEDED | They may be ordinary constructors assigned to named assembly relations if stable IDs/source spans are preserved. Avoid making each mate kind grammar syntax. |
| Verification profiles | Plan08 concept, no syntax | REPLACE_WITH_EXISTING_GENERAL_MECHANISM | Prefer manifest/build configuration or typed profile data interpreted by runner; no language keyword required unless source-owned profiles prove necessary. |
| Parameter sweeps | Verification plan requires them; no syntax | REPLACE_WITH_EXISTING_GENERAL_MECHANISM | Runner orchestration over parameter maps/ranges. Keep stable case identity/evidence, not special interpreter loops. |
| Configuration matrices | Stage7 requires cross-config testing | REPLACE_WITH_EXISTING_GENERAL_MECHANISM | Runner combines existing configurations; no new language feature. |
| Semantic-output maps for raw→safe adoption | Plan05 examples use maps | APPROVED_BUT_NOT_IMPLEMENTED | General `Map<String, SemanticRef>` or stronger typed record; do not invent adoption-only associative syntax. |
| Function-valued geometry parameters (`radius_fn`, sweep laws, etc.) | Plan04 uses callable parameters; closure implementation absent | APPROVED_BUT_NOT_IMPLEMENTED | General callable/closure support should precede APIs that require arbitrary laws. For Stage5 first slice, named `fn` values may be acceptable if callable values already work; do not permanently narrow APIs to constants. |
| Generators | Plan02/03 mention generators; not implemented | DEFER | Useful for large query/geometry sequences but not required to freeze Stage5 core. Preserve iterator protocol so generator addition is compatible. |
| `comptime` / compile-time evaluation | Future language plan mentions it; not needed by Stage5-7 core | DEFER | Do not introduce for roadmap convenience. Reassess with packages/generative libraries. |
| Method syntax | D2 says method sugar lowers functionally; method-like APIs may appear in examples | IMPLEMENTATION_MECHANISM_EXISTS | Keep as sugar over ordinary functions/immutable returns. Rewrite any example that implies in-place mutation. |
| Record/list/range literals | Existing general mechanisms | ALREADY_APPROVED_AND_IMPLEMENTED | Reuse for configuration/test input where adequate rather than creating domain literal syntax. |
| Result/Optional error handling | General enum/prelude mechanisms implemented/approved | ALREADY_APPROVED_AND_IMPLEMENTED | Advanced geometry operations that can legitimately fail should use structured `Result`-style values where recoverable; diagnostics remain for program errors. |
| Source-level geometry queries returning measurements | D18 mechanism exists, but Safe CAD doc flags two-phase execution architecture question | SPECIFICATION_NEEDED | Decide evaluation model for kernel-backed query values used by source control flow. Avoid silently forcing kernel execution into current pure graph-building interpreter. |

## Stage-specific language prerequisites

### Before Stage 5

1. Complete/freeze AICAD-075A spatial semantics.
2. Restore the canonical language specification set.
3. Decide unsafe/raw geometry surface semantics.
4. Decide the scaling mechanism for trusted runtime-backed standard functions.
5. Define callable/closure minimum if Stage5 API requires arbitrary laws; alternatively sequence such APIs after the general callable feature.
6. Implement/confirm Map/Set only where Stage5 public API truly requires them; a typed record can be used for fixed validation reports without losing general future map support.
7. Resolve kernel-backed source query evaluation if Stage5 exposes projection/distance/intersection results to control flow.

### Before Stage 6

Stage6 is the first stage whose advertised design materially depends on unimplemented general language mechanisms. Before assembly API tasks rely on them, freeze/implement:

- interfaces/protocols;
- `implements` semantics;
- generic bounds where reusable interfaces need them;
- assembly/instance declaration identity model;
- configuration declaration/overlay semantics.

Do **not** solve these with assembly-only parser/compiler branches if the capability is general.

### Before Stage 7

Freeze the minimum core verification surface:

- `test` declaration;
- `requirement` declaration with stable identity;
- assertion/evidence primitive, if ordinary function return values cannot capture source/evidence needs;
- approximate comparison/tolerance semantics;
- hard/soft requirement metadata;
- discovery/profile boundaries.

Contracts and invariants should remain separately reviewable; they are not prerequisites for a useful verification-first gate.

## Examples that should not be treated as normative syntax

The following patterns in future-plan examples are useful design sketches but unsafe to copy directly into implementation without a spec delta:

- pointer-like `KernelShape*` or backend-specific topology names — conflicts with D6 and epoch-bound opaque handle semantics;
- mutation-looking topology editing (`shape.foo_in_place(...)`) — conflicts with D2 unless clearly method sugar returning a new value;
- assembly `implements`/generic-bound examples — capability is valid but syntax/type rules are not implemented;
- configuration/suppression attributes — semantics are not frozen;
- test/requirement/contracts — declaration grammar alone does not define runtime/discovery/evidence behavior;
- closure-heavy query/freeform examples — general closure capability is approved conceptually but not implemented;
- maps/sets with unspecified iteration order — external deterministic behavior requires an ordering/serialization rule.

## Core-versus-library conclusion

The compiler/runtime should own semantics that cannot be cleanly expressed by a library: type/units, declarations with universal discovery/stable identity, unsafe capability enforcement if language-level, semantic references, assembly identity/dependency semantics, verification evidence primitives, and trusted runtime-call boundaries. Geometry predicates, mate constructors, most query combinators, domain assertions, gears/fasteners, manufacturing checks and engineering modules should remain ordinary first-party libraries/services unless they demonstrably need privileged access.
