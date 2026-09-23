# AICAD-132 — general interfaces/protocols and bounded generics

## Result

Implemented D27/DL-29's restrained general nominal interface/protocol mechanism on top of D17's existing generics: `interface Name { field: Type, ... }` declarations, explicit `struct`/`part implements Interface, ...` conformance (statically verified), and `T: Interface1 + Interface2` bounds on `fn`/`struct`/`enum` type parameters, checked wherever a bounded parameter is instantiated. No method/`impl`-block syntax, dynamic dispatch, or trait objects were introduced.

## Design decision

D27/DL-29 approved the semantic baseline but explicitly deferred exact surface syntax to "the normal language RFC process." Given the language's existing shape (no methods, no `self`, structs are pure field data, D17 generics are Rust-like `<T>`/`<T, U>`), and the only concrete precedent anywhere in the repo (`docs/plan/07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md`'s `interface MotorMount { mounting_face: FaceRef; ... }` / `part NEMA17 implements MotorMount`), the smallest general mechanism is a **field-contract interface**: a closed list of required `name: Type` pairs (reusing `struct`'s own field shape), checked structurally against an implementer's own fields (`struct`) or top-level `param` declarations (`part`, the only part-body item with a mandatory explicit type). This is fixed as part of this task's own implementation, mirroring how D17's generics syntax was fixed by AICAD-057B/C without a separate numbered RFC; `specs/language/*.md` and `grammar.ebnf` are updated as the canonical record, matching that precedent. An interface is never resolvable as an ordinary value type (`resolve_type_ref` never returns one), so no dynamic-dispatch/trait-object capability exists even implicitly.

## Changes

- Lexer: new `implements` keyword (`interface` was already reserved but unused).
- AST/parser: `Item::Interface`; `Item::Struct`/`Item::Part` gain `implements: Vec<Spanned<String>>`; `TypeParam` replaces bare `Spanned<String>` type parameters, carrying an optional `bounds` list (`T: A + B`, `+`-separated).
- Binder: `SymbolKind::Interface` for ordinary duplicate-name/scope handling.
- HIR: `HirItem::Interface`; `HirItem::Struct`/`Part` gain `implements: Vec<HirInterfaceRef>`; `HirTypeParam` gains `bounds: Vec<HirInterfaceRef>` (unresolved, like every other type reference).
- Type checker (`cad-hir::typeck`): new passes register interface names/field contracts, resolve type-parameter bound names, and statically check every `implements` clause (missing field / field-type mismatch are hard errors; a clause is only recorded into the conformance table on success, so a failed clause never cascades into a second diagnostic). Bound satisfaction is checked both at generic-function call sites (extending `check_generic_call`'s existing inference/substitution) and at generic-struct/enum `Name<Args>` instantiation sites (extending `resolve_generic_type_application`) — conformance is nominal only (`implements`), never inferred from structural match alone.
- New diagnostics `TYPE-E462..465` (`UNKNOWN_INTERFACE_NAME`, `INTERFACE_CONFORMANCE_MISSING_FIELD`, `INTERFACE_CONFORMANCE_FIELD_TYPE_MISMATCH`, `TYPE_ARGUMENT_DOES_NOT_CONFORM`); `TYPE-E460/461` were already taken by `cad-hir::lower`'s query diagnostics.
- `specs/language/{types,semantics,grammar,README}` updated to make the syntax canonical.

## Verification

- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS
- `cargo test --workspace` — PASS (0 failures across every crate); 9 new parser tests, 1 new printer round-trip test, 11 new type-checker tests (positive conformance/bound-satisfaction, negative missing-field/type-mismatch/unknown-interface/non-conforming-argument, and a regression guard that ordinary unbounded D17 generics are unaffected).

## Limitations

- An interface can only ever require plain fields, never behavior — there is no way to require a function/method signature, since the language has no method-call concept at all. If a later Stage-6 task (e.g. `AICAD-138` mechanical interfaces) needs a behavioral contract, that is new scope for that task, not a gap in this one.
- No dedicated `.aicad` example was added: interfaces have no concrete standalone use case until AICAD-138 gives them a real domain (mechanical interfaces); examples policy favors a small number of strong, meaningful examples over an artificial one.
- `tree-sitter-aicad`'s own grammar (IDE syntax highlighting) was not updated to recognize `interface`/`implements`/bounds — no new `.aicad` corpus fixtures were added under `tests/parser/corpus/` (the directory shared with tree-sitter's own tests), so its existing test suite is unaffected, but it will not yet parse this new syntax. Follow-up for whichever task next touches IDE tooling.

## Next

`AICAD-133` (assembly semantic identity primitives) is the batch's other task; `AICAD-134` (component-definition/logical-instance IR) depends on both.
