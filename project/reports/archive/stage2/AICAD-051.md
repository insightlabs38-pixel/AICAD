# AICAD-051: Create typed HIR and AST->HIR lowering skeleton

## Objective

Implement the typed HIR data model and AST -> HIR lowering skeleton per
`project/TASKS.yaml` AICAD-051 (Stage-2 Batch S2-06, sole task) —
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §17 phase 7 ("Lower to typed
HIR"), per the IR-layer strategy in `docs/plan/01_SYSTEM_ARCHITECTURE.md`
§5 (Source AST -> **Typed HIR** -> Engineering HIR -> Feature IR ->
Geometry IR -> Kernel call graph -> B-rep). `crates/cad-hir` was, until
this task, only a placeholder (`src/lib.rs` had a doc comment and nothing
else). Per the campaign brief, this batch is one architecture-dense task
by itself; `AICAD-052` (general type checking) is explicitly **not**
started in this session.

## Base commit

`c9e0a23` ("Update SESSION_HANDOFF.md: Stage-2 Batch S2-05 complete") —
this session's assigned starting point, confirmed as `HEAD` of
`claude/aicad-stage2-dev` (tracking `origin/claude/aicad-stage2-dev`) at
session start, working tree clean.

## Plan references read

`docs/plan/01_SYSTEM_ARCHITECTURE.md` §5 (the IR-layer diagram this task
implements the first new layer of) and §2.1 ("type checking; dimensional
analysis" as compiler-frontend responsibilities, and the recommended
17-phase pipeline in `docs/plan/02_LANGUAGE_AND_COMPILER.md` §17, which
lists phase 4 "type + dimensional checking" *before* phase 7 "lower to
typed HIR" — noted as a phase-ordering observation, not a contradiction
requiring escalation, since the project's own fixed task schedule
(`project/TASKS.yaml`) deliberately sequences `AICAD-051` (the HIR data
model + lowering skeleton) before `AICAD-052` (the type-checking
algorithm); `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §15 (the
frozen control-flow construct list HIR must represent with "unambiguous
value semantics", already fully covered by `cad-ast`'s AICAD-041/042/043
node shapes); `docs/plan/15_IMPLEMENTATION_ROADMAP.md` and
`docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md`/`docs/plan/
22_REPOSITORY_WORK_PACKAGES.md` (confirmed neither names AICAD-051-
specific requirements beyond the crate boundary `22` already lists);
`rfcs/0004-units-type-system.md` §8-9 (frozen geometry/semantic/
structural types and ownership/control-flow/safety rules — confirmed
nothing there is reachable by the AST shapes lowering actually has to
handle yet: no generics/collections/interfaces/geometry-type syntax
exists in `cad-ast` as of AICAD-045, so none of it is invented here
either, per `AGENTS.md`'s "no speculative future work").

Source read in full: `crates/cad-hir/README.md`/`src/lib.rs` (the
placeholder this task fills in); `crates/cad-ast/src/{lib,expr,item}.rs`
(the AST this lowers from); `crates/cad-compiler/src/binder.rs`
(AICAD-050's name binding — the scope/symbol-table structure this task's
own lowering-time scope walk mirrors); `crates/cad-units/src/
arithmetic.rs` (AICAD-049's `OperandType`, reused directly as `HirType`)
and `registry.rs` (`lookup_any`, used to resolve an unambiguous literal
unit suffix); `crates/cad-types/src/{primitive,dimension}.rs`
(`PrimitiveType`/`Dimension`/`AffineKind`); `project/reports/
AICAD-050.md`/`049.md`/`046.md` (continuity); `project/OWNER_DECISIONS.md`
and `project/DECISION_LOG.md` (no open item there governs HIR shape).

## Implementation

New crate content under `crates/cad-hir/src/`:

- **`ids.rs`** — `BindingId` (a `u32` newtype), `BindingKind` (mirrors
  `cad_compiler::binder::SymbolKind` one-for-one: `Let`/`Var`/`Const`/
  `Param`/`Fn`/`Struct`/`Enum`/`EnumVariant{enum_name}`/`Part`/`Import`/
  `ForLoopVar`/`MatchBinding`), and `Binding { id, name, kind, span }`.
  This is the concrete answer to `AGENTS.md`'s HIR invariant "lexical
  binding identity is explicit": every declaration lowering encounters
  mints a fresh `BindingId`, and every reference to that name in the
  resulting HIR carries that same id, not the name alone.
- **`types.rs`** — `HirType` (`pub use cad_units::OperandType as HirType`
  — reused directly rather than duplicated, since `OperandType` already
  is exactly "an ordinary numeric scalar, or a quantity carrying one of
  `cad_types::Dimension`'s 21 named dimensions plus an optional affine
  discriminant", i.e. `AGENTS.md`'s "typed engineering quantities, not
  untyped floats" non-negotiable) and `HirTypeRef` (`Named`/`Generic`,
  the unresolved syntactic counterpart of `cad_ast::item::Type`).
- **`hir.rs`** — the typed HIR node types: `HirProgram`, `HirItem`
  (`Let`/`Const`/`Param`/`Fn`/`Struct`/`Enum`/`Part`/`Import`, each
  carrying its own `BindingId`), `HirStmt` (`Let`/`Var`/`Assign`/`Expr`/
  `If`/`For`/`While`/`Loop`/`Match`/`Return`/`Break`/`Continue`),
  `HirExpr` (`Literal`/`Ident`/`Unary`/`Binary`/`Call`/`Field`/`Block`/
  `If`/`Match`), `HirBlock`, `HirLiteral`, `HirPattern`, `HirMatchArm`,
  `HirCallee`, `HirArg`, `HirParam`/`HirField`/`HirEnumVariant`/
  `HirImportedName`/`HirImportPath`, plus `pub use cad_ast::{BinaryOp,
  UnaryOp}` (reused as-is — bare operator tags with no syntax-specific
  payload).
- **`lower.rs`** — `lower_program(&Program, file, source) -> LowerResult`
  (`LowerResult { program: HirProgram, bindings: Vec<Binding>,
  diagnostics: Vec<Diagnostic> }`) and the `Lowerer` struct that performs
  it, plus 31 unit tests.

### Decisions

1. **Lowering performs its own scope walk; it does not call
   `cad_compiler::binder`.** Not a preference — a structural
   necessity: `crates/cad-compiler`'s own module doc comment already
   lists `cad-hir` as one of the crates *it* composes, so a `cad-hir ->
   cad-compiler` dependency would be a workspace cycle (confirmed by
   inspecting every crate's `Cargo.toml`: `cad-compiler` depends on
   `cad-ast`/`cad-diagnostics`/`cad-parser` only, not `cad-hir`, exactly
   because the dependency is meant to run the other way once
   `cad-compiler` is wired up as the pipeline driver). `Lowerer` mirrors
   `Binder`'s scope-stack/two-pass-declare-then-check structure closely
   (see `lower.rs` module doc comment "Relationship to `cad_compiler::
   binder`") but mints its own `BindingId`s rather than reusing anything
   `crate::binder` produces (that module currently returns only
   diagnostics, no resolution table, and adding one was judged out of
   this task's minimal-change scope — see "Known limitations").
2. **Skeleton scope boundary vs. `AICAD-052`.** Explicitly deferred,
   documented in `lower.rs`'s own module doc comment "Scope boundary":
   duplicate-declaration detection (two same-scope `let a`s each mint a
   fresh id; the second silently shadows the first in this pass's own
   scope map — `crate::binder`'s `DUPLICATE_BINDING` already owns this
   check and runs earlier in the real pipeline); DL-2 mutability
   re-enforcement (`HirStmt::Assign::target` resolves to *some* binding
   regardless of `let` vs. `var` — `crate::binder`'s
   `ASSIGN_TO_IMMUTABLE` already owns this); numeric-literal-type
   defaulting (`Int` vs. `Float` — `cad_units::arithmetic`'s own module
   doc comment already assigns this to `AICAD-052`, so `HirLiteral::
   Number` keeps the raw source text unparsed, matching the lexer's own
   established "raw text, parsed later" convention); method/field member
   resolution against a receiver's type; type-name resolution
   (`HirTypeRef` stays syntactic, unresolved); struct-field/import-
   target/package-path resolution (same boundary `crate::binder`/
   `crate::loader` already established for the AST layer).
3. **Unresolved names are handled, not panicked on, and get one new
   diagnostic code.** Even though a binder-clean program is the expected
   real-pipeline input, `cad-hir`'s own API does not (and structurally
   cannot) force a caller to run `crate::binder` first, so an unresolved
   name is a real, reachable input shape. `HirExpr::Ident::binding`/
   `HirStmt::Assign::target`/`HirCallee::Fn::binding` are `Option<
   BindingId>`, `None` exactly when lowering's own scope walk cannot
   resolve the name, and a `TYPE-E410` (`UNRESOLVED_BINDING`) diagnostic
   is recorded — a distinct code from `crate::binder`'s `TYPE-E401`
   (`UNDEFINED_NAME`) for what is, in a correctly-ordered pipeline, the
   same underlying condition caught by two different phases. This is
   also this task's answer to the brief's requested "negative/adversarial
   case: a construct that cannot legally lower" — see tests.
4. **Method-call desugaring (`AGENTS.md` HIR invariant: "source AST
   builder/method-call syntax must already be desugared to functional
   calls in HIR").** `Expr::MethodCall { receiver, method, args }` lowers
   to `HirExpr::Call { callee: HirCallee::Method { name, span }, args:
   [receiver] ++ args, span }` — the receiver becomes the desugared
   call's first argument. `HirCallee` has two variants, `Fn` (a lexically
   resolved name, carrying `Option<BindingId>`) and `Method` (resolved
   against the first argument's type later, `AICAD-052`+, so it carries
   no `binding` field at all — not merely an always-`None` one, matching
   `crate::binder`'s own established rule that method/field names are
   never scope-checked). After lowering there is exactly one call-shaped
   `HirExpr` variant for both surface forms.
5. **Value-semantics unification (`AGENTS.md`: "control flow must have
   unambiguous value semantics").** `cad_ast` keeps `item::Block`
   (statement position, no trailing value) and `expr::BlockExpr`
   (expression position, optional trailing value) as separate types for
   purely syntactic reasons; HIR collapses both into one `HirBlock {
   stmts, trailing: Option<Box<HirExpr>>, span }` (`trailing: None` is
   exactly the statement-position case). Similarly `cad_ast::ElseBranch`
   (an `if`-expression's mandatory `else`, `Block`/`If` alternatives) is
   not kept as its own HIR type at all — it lowers directly to a plain
   `HirExpr` (always `HirExpr::Block` or a nested `HirExpr::If`), and
   `MatchArmBody::Expr`/`::Block` likewise both lower to a plain
   `HirExpr`. A HIR consumer checks one shape, not two, to learn whether
   a construct produces a value. Statement-position `if`'s `else`
   (`ElseClause`) stays a distinct `HirElseStmt` type, deliberately *not*
   unified with the expression case — a statement has no value to unify
   around, so keeping the two visibly different types is itself the
   "unambiguous value semantics" property, not a violation of it.
6. **`HirPattern` splits `cad_ast::Pattern::Ident`'s one ambiguous shape
   into two unambiguous HIR variants.** `cad_ast::expr::Pattern`'s own
   doc comment states the AST's `Ident` pattern "binds a name, or matches
   an enum-variant-shaped name" and explicitly assigns disambiguating
   that to the binding phase. `crate::binder` already resolves this
   question for its own diagnostics; HIR lowering resolves it
   independently (same reason as decision 1) and, since HIR is a
   semantic IR that should never re-encode a question already answered,
   represents the two readings as `HirPattern::Variant { name, variant:
   BindingId, span }` (a *reference* to the existing variant's binding)
   and `HirPattern::Binding { name, binding: BindingId, span }` (a fresh
   *declaration*) rather than one ambiguous `Ident` case.
7. **`Expr::Paren` grouping is discarded.** `cad_ast::Expr::Paren`'s own
   doc comment already states it "carries no semantic meaning beyond its
   `inner`" and exists only so the AST pretty-printer can round-trip
   explicit source parentheses — a requirement that applies to the AST
   layer, not HIR (which has no formatter/round-trip contract). Lowering
   `(expr)` therefore produces exactly `lower_expr(inner)`, one fewer
   node than the AST had.
8. **Literal `HirType` resolution is minimal and non-guessing.** A
   `Bool`/`Str`/`RawStr` literal always resolves to the obvious
   `PrimitiveType`. A `Number` literal with no unit suffix is left `ty:
   None` (defaulting deferred to `AICAD-052`, decision 2). A `Number`
   literal with a unit suffix resolves only when `cad_units::lookup_any`
   finds **exactly one** matching dimension (e.g. `5mm` -> `Length`,
   `20degC` -> `Temperature` defaulting to `AffineKind::Absolute` per
   RFC-0004 §7 — a bare literal is never itself a subtraction result, so
   `Absolute` is the only sound default); an unknown symbol (`5xyz`, no
   match at all) or an ambiguous one (`5Pa`, matching both `Pressure` and
   `Stress` — `cad_units::registry`'s own documented multiplicity) is
   left `None` rather than guessed at, mirroring `cad_units::arithmetic`'s
   own identical ambiguity rule for derived dimensions and `AGENTS.md`'s
   "ambiguity is an error, never an arbitrary selection".
9. **New crate dependencies**: `cad-ast`, `cad-diagnostics`, `cad-types`,
   `cad-units` (all path dependencies, no third-party additions); dev-
   dependency `cad-parser` (test fixtures only, mirroring `crate::
   binder`'s own test setup). No dependency cycle: `cad-units` depends
   only on `cad-types`; neither depends on `cad-hir` or `cad-compiler`.

## Tests / commands run

```
$ cargo build -p cad-hir
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.20s

$ cargo test -p cad-hir
running 31 tests
test lower::tests::affine_temperature_literal_defaults_to_absolute ... ok
test lower::tests::ambiguous_unit_literal_is_left_unresolved ... ok
test lower::tests::bindings_registry_is_indexed_by_binding_id ... ok
test lower::tests::bool_and_string_literals_resolve_to_scalar_types ... ok
test lower::tests::const_and_item_level_param_lower_with_bindings ... ok
test lower::tests::diagnostics_carry_a_source_span ... ok
test lower::tests::expression_block_carries_its_trailing_value ... ok
test lower::tests::field_access_is_not_desugared_to_a_call ... ok
test lower::tests::fn_param_gets_its_own_binding_kind ... ok
test lower::tests::for_loop_variable_gets_its_own_binding_scoped_to_the_body ... ok
test lower::tests::forward_reference_between_sibling_fns_resolves_to_a_real_binding ... ok
test lower::tests::if_expr_else_if_chain_lowers_to_nested_if_exprs ... ok
test lower::tests::if_stmt_else_clause_lowers_to_hir_else_stmt ... ok
test lower::tests::length_literal_resolves_to_dimensional_length ... ok
test lower::tests::let_and_later_reference_share_the_same_binding_id ... ok
test lower::tests::match_arm_identifier_matching_a_variant_references_the_variant_binding ... ok
test lower::tests::match_arm_identifier_not_matching_a_variant_is_a_fresh_binding ... ok
test lower::tests::method_and_field_names_are_never_checked_as_scope_names ... ok
test lower::tests::method_call_desugars_to_a_functional_call_with_receiver_as_first_arg ... ok
test lower::tests::paren_grouping_is_discarded_keeping_only_the_inner_expr ... ok
test lower::tests::selective_import_binds_a_name_whole_module_import_binds_nothing ... ok
test lower::tests::part_body_sees_module_scope_and_gets_its_own_bindings ... ok
test lower::tests::spans_survive_lowering ... ok
test lower::tests::statement_block_never_has_a_trailing_value ... ok
test lower::tests::struct_and_enum_declarations_lower_with_bindings ... ok
test lower::tests::undefined_assign_target_lowers_with_no_binding_and_a_diagnostic ... ok
test lower::tests::undefined_call_callee_lowers_with_no_binding_and_a_diagnostic ... ok
test lower::tests::undefined_identifier_lowers_with_no_binding_and_a_diagnostic ... ok
test lower::tests::unitless_number_literal_type_defaulting_is_deferred ... ok
test lower::tests::unknown_unit_symbol_is_left_unresolved ... ok
test lower::tests::while_and_loop_bodies_lower ... ok
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt --all -- --check
(clean, no diff, exit code 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.08s
(zero warnings)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s

$ cargo test --workspace
33 test binaries executed; every one `test result: ok`; 455 total tests
passed, 0 failed, 0 ignored; exit code 0.
```

(`cargo fmt --all -- --check` was run once, found real formatting diffs
against this task's newly written files — all mechanical brace/wrapping
style, e.g. multi-field struct-literal patterns rustfmt prefers on
separate lines — fixed with `cargo fmt --all`, then re-checked clean. A
first `cargo clippy` pass found 5 `redundant_closure` warnings
(`ty.as_ref().map(|t| lower_type(t))` should be `ty.as_ref().map
(lower_type)`); fixed, re-run clean, re-ran `cargo fmt --all -- --check`
again after the fix — still clean.)

## Tests / regressions

31 new tests in `crates/cad-hir/src/lower.rs`, covering every category the
task brief required:

- **Literals with units** (typed engineering quantities): `5mm` ->
  `Dimensional(Length)`; `20degC` -> `Dimensional(Temperature, Absolute)`;
  ambiguous `5Pa` and unknown `5xyz` both left unresolved; unitless `5`
  left unresolved (deferred to `AICAD-052`); `Bool`/`Str`/`RawStr`
  literals resolve to their `PrimitiveType`.
- **Declarations**: `let`/`const`/item-level `param`/`fn`
  (parameters + body scope)/`struct` (fields, no binding)/`enum`
  (variants, each with an `EnumVariant{enum_name}` binding)/`part`
  (nested scope, sees module scope)/selective vs. whole-module `import`.
- **Control flow / value semantics**: statement-block `trailing` is
  always `None`; expression-block carries its trailing value; `if`-
  expression `else`-`if` chains lower to nested `HirExpr::If` (not a
  wrapper type); statement-`if`'s `else` lowers to `HirElseStmt`;
  `for`-loop variable scoped to just the body; `while`/`loop` bodies.
- **Explicit binding identity**: a `let` and a later reference share the
  same `BindingId`; forward reference between sibling `fn`s resolves to
  a real binding (two-pass declare-then-lower); a fn parameter's
  `BindingKind` and name are recorded correctly in the bindings registry.
- **Match variant-vs-binding disambiguation**: a variant-shaped pattern
  identifier resolves to `HirPattern::Variant` referencing the existing
  variant's own binding id; a non-variant identifier becomes
  `HirPattern::Binding` with a fresh id, used correctly in the arm body.
- **Method-call desugaring**: `x.foo(y)` lowers to `HirExpr::Call` with
  `HirCallee::Method { name: "foo", .. }` and `args == [receiver, y]`;
  plain field access (`x.foo`) is confirmed *not* desugared to a call.
- **Grouping discarded, spans survive**: `(1 + 2)` lowers to a bare
  `HirExpr::Binary`; a `let` item's span covers its exact source text
  including the trailing `;`, and its value expression's span covers
  exactly `5mm`.
- **Negative/adversarial — unresolved names**: an undefined identifier,
  call callee, and assignment target each lower with `binding: None`/
  `target: None` and exactly one `TYPE-E410` diagnostic; diagnostics
  carry a source span; method/field names are confirmed to raise **no**
  diagnostic even when undeclared anywhere (resolved by type later, not
  lexical scope — matching `crate::binder`'s own established rule).
- **Bindings registry**: `bindings[id.index()]` always returns that
  binding's own record.

Full workspace regression: `cargo test --workspace` — 33 test binaries,
455 tests total, all passing, 0 failures. No pre-existing test was
touched; no regression found or introduced.

## Known limitations

- `crate::binder`'s `BindResult` still exposes only diagnostics, no
  resolution/symbol table a later caller could reuse — this task's own
  `Lowerer` therefore performs an independent (structurally forced, see
  Decision 1) scope walk rather than consuming one. If a future task
  wires `cad-compiler` into an actual multi-phase pipeline driver, it may
  be worth revisiting whether `crate::binder` should expose a
  `Span -> BindingId`-shaped resolution table `cad-hir` (or, more likely,
  `cad-compiler` itself, which *can* depend on both) could reconcile
  against this crate's own — out of scope for this task to design without
  a concrete consumer.
- No duplicate-binding or DL-2 mutability diagnostics are (re-)raised by
  this crate — intentional (Decision 2); `crate::binder` already owns
  both and runs earlier in the real pipeline.
- `HirLiteral::Number` keeps its magnitude as raw source text, not a
  parsed numeric value — intentional (Decision 2/8); Engineering HIR
  (the next IR layer, `docs/plan/01_SYSTEM_ARCHITECTURE.md` §5) is where
  a concrete numeric representation and unit-to-canonical conversion
  belong, not Typed HIR.
- No handling exists yet for any AST shape newer tasks haven't built
  (generics, collections, interfaces, geometry-type syntax, data-carrying
  enum variants, `interface`/`assembly`/`requirement`/`test` decls) —
  none of it exists in `cad-ast` yet either, so there is nothing to lower
  and nothing was speculatively invented, per `AGENTS.md`.
- `crate::loader`'s multi-file module graph is still not wired to either
  `crate::binder` or this crate — `lower_program` binds/lowers one
  already-parsed program at a time, exactly like `bind_program` and
  `cad_parser::parse_program` already do (unchanged pre-existing
  limitation, not this task's to fix).

## Unresolved questions

None requiring `project/OWNER_DECISIONS.md` escalation. The one place
lowering had to choose between two structurally different designs —
whether to give `cad-hir` a dependency on `cad-compiler::binder` for
resolved-name reuse, or perform its own independent scope walk — was not
a judgment call between materially different *architectures* in the
`AGENTS.md` escalation sense; it was forced by the existing, already-
approved crate dependency graph (`cad-compiler` composes `cad-hir`, not
the reverse), so only one of the two was actually implementable. Per the
campaign brief, `AICAD-052` (general type checking) is Batch S2-07's
first task and is deliberately **not** started in this session.
