# Stage-2 Batch checkpoint — Types/HIR (AICAD-052..053)

Prepared after `AICAD-053`, per the active scheduled-task brief's batch
checkpoint requirement (Batch S2-07). This is a **batch checkpoint**
gating Batch S2-08 (`AICAD-054` onward), not the Stage-2 owner gate
packet (that is `AICAD-064`'s job, per `project/gates/README.md`'s
format for `stage-<n>-gate.md`). Per `AGENTS.md` ("Stage gates"),
preparing evidence and a recommendation is within this agent's role;
this checkpoint does not itself constitute owner approval of anything —
Stage 2 as a whole still requires the owner-recorded decision `AICAD-064`
will seek, per `project/CURRENT_STAGE.md`.

## 1. Exact git revision at checkpoint time

Prepared on branch `claude/aicad-stage2-dev`, HEAD `d966f76` (the
`AICAD-053` commit), immediately before this document lands. Batch S2-07
spans two commits on top of `1a03e53` (the `AICAD-051` commit this
session started from): `0d6b05e` (`AICAD-052`) and `d966f76`
(`AICAD-053`). Working tree clean at the start of this checkpoint's own
verification run (confirmed via `git status --short` immediately before
§4 below).

## 2. Batch scope and task reports

| Task | Title | Report |
|---|---|---|
| AICAD-052 | Implement type checking for literals/bindings/functions/calls | `project/reports/AICAD-052.md` |
| AICAD-053 | Implement structs/enums field and variant typing | `project/reports/AICAD-053.md` |

Crate in scope: `crates/cad-hir` (specifically its new `src/typeck.rs`
module; `src/{hir,ids,lower,types}.rs`, all `AICAD-051`, are unmodified
by this batch — confirmed by `git show 1a03e53..HEAD --stat`, which
touches only `crates/cad-hir/src/lib.rs` (two `pub mod`/`pub use`
additions), `crates/cad-hir/src/typeck.rs` (new file), `crates/cad-hir/
README.md`, `project/TASKS.yaml`, and the two task reports plus this
document).

## 3. Checklist

### 3.1 Dimensional canonicalization

`HirType` (`= cad_units::OperandType`, `AICAD-049`) is reused verbatim as
the checker's own scalar/dimensional value representation
(`CheckedType::Value`) — this batch adds no second, parallel
representation of a dimension anywhere. Every dimension a type reference
resolves to (`Checker::resolve_type_ref`) or a literal infers
(`Checker::resolve_unit_literal`) goes through `cad_types::Dimension`'s
own canonical named identity (`Dimension::from_name`) or `cad_units`'s
own unit registry (`cad_units::lookup`/`lookup_any`), never a
locally-invented dimension representation. `crate::lower::lower_program`
already canonicalizes an unambiguous unit-suffixed literal the same way;
this batch's extension (resolving an *ambiguous* or *unresolved* literal
via a matching `expected` dimension — `Checker::resolve_unit_literal`)
still routes through the same `cad_units::lookup`/`Dimension` machinery,
never a shortcut. **PASS.**

### 3.2 Same-dimension conversions

`same_dimension_addition_type_checks` (`5mm + 2cm`) and
`comparison_across_compatible_dimension_succeeds`-shaped coverage
(inherited from `cad_units::arithmetic`'s own 052 reuse, exercised
end-to-end at the HIR layer by every arithmetic test in
`typeck::tests`) confirm same-dimension implicit conversion continues to
work exactly as DL-3 requires, entirely via delegation to
`cad_units::check_binary_arithmetic`/`check_comparison` — this batch
never implements a same-dimension conversion rule of its own (see §3.9's
audit for the one place a *different*, non-dimensional compatibility
rule was added, and why it does not overlap this one). **PASS.**

### 3.3 Illegal dimension arithmetic (rejected, not coerced)

`cross_dimension_addition_is_rejected_not_coerced` (`5mm + 2kg` ->
`UNIT-E104`), `ambiguous_derived_dimension_without_context_is_reported`
(`1N * 1mm` with no target dimension -> `UNIT-E109`, never an arbitrary
pick between `Torque`/`Energy`), and
`unitless_literal_annotated_as_a_dimension_is_a_type_error` (`let x:
Length = 5;` — a bare numeral is never silently promoted to a
dimensional quantity, `TYPE-E411`) together confirm no path in this
batch's own code ever coerces across dimensions or defaults an ambiguous
result — every rejection produces a real `Diagnostic` (`UNIT-Exxx` via
`Checker::diag_from_unit_error`, or a `TYPE-Exxx` for this batch's own
non-arithmetic checks), never a panic, never a silent pass. **PASS.**

### 3.4 Affine absolute/delta temperature behavior

`affine_absolute_plus_absolute_is_rejected` (`20degC + 20degC` ->
`UNIT-E105`) and `affine_absolute_minus_absolute_is_a_delta` (`20degC -
5degC` -> `Temperature` with `AffineKind::Delta`, confirmed by directly
asserting the resulting `CheckedType`) both delegate to `cad_units::
check_binary_arithmetic`'s existing RFC-0004 §7 table, unmodified.
This batch's own addition — a bare type *annotation* for an affine
dimension (`: Temperature`) always resolving to `AffineKind::Absolute`
(no syntax anywhere spells a delta annotation) — was specifically
re-examined for this checkpoint: `types_compatible`'s strict
`affine == affine` requirement means an annotated `Temperature` binding
can only ever accept an `Absolute`-valued expression, so assigning a
`Delta` result (e.g. `let t: Temperature = 20degC - 5degC;`) is correctly
flagged as a type mismatch rather than silently accepted as if the
annotation meant "any affine kind." Re-verified directly for this
checkpoint (not merely re-cited from the task report):

```
$ cat > /tmp/gate_affine_check.aicad <<'EOF'
let t: Temperature = 20degC - 5degC;
EOF
```
(checked via the same `cad_parser`/`cad_hir::lower_program`/
`cad_hir::check_program` pipeline the crate's own tests use) produces
exactly one `TYPE-E411 TYPE_ANNOTATION_MISMATCH` diagnostic — confirming
the annotation-defaults-to-`Absolute` policy does not silently widen to
accept a `Delta` value. **PASS.**

### 3.5 Binding/scoping

This batch's own design ("no scope stack needed" — `typeck.rs`'s module
doc comment) relies entirely on `crate::lower`'s already-completed,
unmodified scope resolution: every `HirExpr::Ident`/`HirStmt::Assign`
target already carries a concrete `BindingId` (or `None`, already
diagnosed) by the time this batch's code runs. Re-confirmed for this
checkpoint: `crates/cad-hir/src/lower.rs` and `src/ids.rs` are
byte-for-byte unchanged since `AICAD-051` (`git diff 1a03e53..HEAD --
crates/cad-hir/src/lower.rs crates/cad-hir/src/ids.rs` is empty). The
one new binding-identity behavior this batch adds —
`register_type_names` assigning every enum variant's own `BindingId` a
`CheckedType::Enum(...)` up front — extends the *type* table
(`Checker::binding_types`, this batch's own new structure), never the
*binding-identity* table (`crate::lower::LowerResult::bindings`, still
owned entirely by `AICAD-051`'s lowering pass). `unresolved_identifier_
reference_does_not_crash_the_checker` confirms an unresolved binding
(already diagnosed by lowering, `TYPE-E410`) produces no additional,
duplicate diagnostic from this batch's own code. **PASS.**

### 3.6 Functional method-call desugaring (from AICAD-051)

Re-confirmed unmodified: `crate::hir::HirCallee::Method` still has no
`binding` field at all (not merely an always-`None` one), and `crate::
lower::lower_expr`'s `Expr::MethodCall` desugaring (receiver prepended as
the call's first argument) is untouched by this batch (see §2's
`lower.rs` diff-emptiness confirmation above). This batch's own
`Checker::check_call` explicitly declines to invent a resolution
mechanism for `HirCallee::Method` — `method_call_is_walked_without_a_
resolvable_signature` pins this: each argument (receiver included, as
`args[0]`) is still type-checked independently, but the call's own
signature/result stays unresolved, with no panic and no fabricated
diagnostic. No new method/interface-implementation declaration syntax
was added anywhere (`crates/cad-ast` is untouched by this batch — see
§3.9). **PASS.**

### 3.7 Typed HIR invariants

`AGENTS.md`'s HIR invariants ("lexical binding identity is explicit",
"builder/method-call syntax already desugared", "control flow has
unambiguous value semantics") are properties of the HIR *shape*
(`AICAD-051`), not of this batch — reconfirmed that this batch never
mutates a `crate::hir` node in place (`Checker::check_expr` and friends
are read-only over `&HirExpr`/`&HirStmt`/`&HirItem`, computing/returning
a `CheckedType` alongside the original tree rather than writing back into
it — see `typeck.rs`'s own module doc comment "no scope stack needed" and
"`Option<CheckedType>`, not `Result`" for why: this checker's contract is
`(program, bindings) -> diagnostics + a side table`, never `program ->
mutated program`). `crates/cad-hir/src/hir.rs` is byte-for-byte unchanged
since `AICAD-051` (`git diff 1a03e53..HEAD -- crates/cad-hir/src/hir.rs`
is empty). Value-semantics unification (`HirExpr::If`/`HirExpr::Match`
producing one unambiguous value) is now also *type*-checked, not merely
*shaped* correctly: `unify_value_type` requires branch/arm-type agreement
(`TYPE-E424 BRANCH_TYPE_MISMATCH` on disagreement) — extending the
invariant from "structurally one value slot" (AICAD-051) to "that one
value slot's type is actually consistent" (this batch). **PASS.**

### 3.8 Structs/enums typing

Struct-literal construction (via ordinary call syntax — no separate
constructor syntax exists, consistent with DL-2), field access (including
through a two-level nested chain), enum-variant construction (a bare
variant resolves to its owning enum's type), enum-variant equality
comparison, and match-arm variant-pattern consistency against the
scrutinee's own enum identity are all implemented and independently
tested (20 new tests in `AICAD-053`, itemized in `project/reports/
AICAD-053.md` "Tests / regressions"). Struct/enum type identity is
nominal (declaring item's own `BindingId`), never structural — two
structs with identical field shapes remain distinct types, matching
ordinary Rust/TypeScript-class nominal typing (DL-1, "broadly
Rust/TypeScript-like") and the only interpretation any frozen RFC/plan
text evidences. **PASS**, with two explicitly tracked, non-blocking gaps
carried into §7 below: no struct-declaration duplicate-field-name
checking, and no struct-pattern destructuring (no such pattern shape
exists in `cad_ast::Pattern` yet — `docs/plan/03` §10's "support
destructuring" remains a target for a future grammar-extension task, not
silently dropped).

### 3.9 Diagnostic stability

Every diagnostic this batch raises is a real `cad_diagnostics::
Diagnostic` (never a bare string) — verified structurally exactly as
`STAGE2-A_FRONTEND.md`'s own precedent did: `Checker::diag`/`Checker::
diag_from_unit_error` are the only diagnostic-construction paths in
`typeck.rs`, both going through `Diagnostic::new`. Two families are in
use: `TYPE` (this batch's own new codes, `E411`-`E437`, none reused from
a different meaning — cross-checked via `grep -rn "TYPE-E4" crates/
project/` before each new code's assignment, confirmed in both task
reports) and `UNIT` (every `cad_units::DimensionalArithmeticError` code,
`E101`-`E110`, reused **verbatim** via `DiagnosticCode::parse(err.code())`
— never renumbered under a `TYPE` code, directly fulfilling
`cad_units::arithmetic`'s own module doc comment: its `code()` string
exists "for a later `cad-diagnostics`-aware caller to build a real
`Diagnostic` from," and this batch is that caller). No diagnostic code
already in use by an earlier batch (`PARSE`, `IMPORT`, the pre-`AICAD-052`
`TYPE-E401`-`E410`) was renumbered, repurposed, or removed — reconfirmed
by `git diff 1a03e53..HEAD -- crates/cad-compiler/src/binder.rs
crates/cad-hir/src/lower.rs` being empty. Every code remains provisional
pending D10 (`project/OWNER_DECISIONS.md`, still open), exactly like
every prior batch's own codes — this batch does not attempt to resolve
D10, only to follow the existing provisional-numbering convention.
**PASS.**

## 4. Fresh verification run at checkpoint time

```
$ git status --short
(empty)

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0)

$ cargo build --workspace --all-targets
(exit 0)

$ cargo test -p cad-ast -p cad-lexer -p cad-diagnostics -p cad-parser -p cad-compiler -p cad-types -p cad-units -p cad-hir
test result: ok. 7 passed (cad-ast lib)
test result: ok. 14 passed (cad-ast printer_round_trip)
test result: ok. 43 passed (cad-compiler)
test result: ok. 20 passed (cad-diagnostics lib)
test result: ok. 10 passed (cad-diagnostics schema_conformance)
test result: ok. 94 passed (cad-hir — this batch's own crate, up from
  AICAD-051's 31)
test result: ok. 27 passed (cad-lexer)
test result: ok. 89 passed (cad-parser)
test result: ok. 14 passed (cad-types)
test result: ok. 75 passed (cad-units)
Total: 393 passed, 0 failed.

$ cargo test --workspace
61 test binaries executed; every one `test result: ok`; 518 tests total
passed, 0 failed, 0 ignored (includes the full native/OCCT Stage-1 Rust
suite — cad-occt-bridge 84 unit + 9 adversarial_sweep + 6
concurrency_probe + 3 stage1_bracket, cad-kernel-api 23 — all unaffected
by this batch, included only because `--workspace` runs everything).
```

Environment: same as every prior Stage-1/Stage-2 session (Rust 1.98.1,
edition 2024, per `rust-toolchain.toml`) — reconfirmed, not assumed.

## 5. Cross-check against D5/DL-12 (determinism)

This batch introduces three new `HashMap`s (`Checker::fn_signatures`,
`Checker::type_names`, `Checker::struct_fields`), all keyed by either
`BindingId` or `String`. Re-examined specifically for this checkpoint,
mirroring `STAGE2-A_FRONTEND.md`'s own precedent exactly: every one is
used **only** for point lookups (`.get()`/`.insert()`), never iterated to
produce an observable output — `TypeCheckResult::diagnostics` is a `Vec`
built in the checker's own fixed, deterministic traversal order (source
order for pass 2, declaration order within each pass for passes 0/0.5/1),
and `TypeCheckResult::binding_types` is a `Vec` indexed positionally by
`BindingId::index()`, never keyed or ordered by any hash. No new
concurrency, filesystem enumeration, or randomized-hashing dependence was
introduced anywhere in this batch. **No D5 Level-1 violation found.**

## 6. Adversarial audit: accidental public semantic changes relative to RFC-0001/RFC-0004

Per the campaign brief, this is an explicit adversarial pass — not a
formality. Read RFC-0001 and RFC-0004 in full for this checkpoint
(not merely re-cited from either task report) and checked this batch's
own implementation against each frozen ruling specifically for silent
drift:

- **DL-1 (syntax, RFC-0001 §4).** No lexer/parser/grammar file was
  touched by this batch at all (`git diff 1a03e53..HEAD -- crates/
  cad-lexer crates/cad-parser crates/cad-ast specs/language/grammar.ebnf`
  is empty) — no risk of syntax drift by construction. **Checked, none
  found.**
- **DL-2 (functional core / method-call sugar, RFC-0001 §5).** The
  specific risk: could this batch's own type-checking of `HirExpr::Call`
  accidentally treat a method-call-desugared call as anything other than
  an ordinary functional call, e.g. by inferring some special "mutates
  the receiver" typing rule for `HirCallee::Method`? Checked
  `Checker::check_call`'s `Method` arm directly — it treats every
  argument (including the desugared receiver, already `args[0]` per
  `AICAD-051`) as an ordinary, independent expression to type-check, with
  no receiver-specific special-casing anywhere. No rebinding, no
  mutation-flavored typing rule exists anywhere in this batch's code —
  `body.cut(hole);`'s value is exactly as discardable/non-mutating after
  this batch as it was before it. **Checked, none found.**
- **DL-3 (units/dimension/affine semantics, RFC-0004 §5/§7).** The
  specific risk this checkpoint looked for: any path where this batch's
  own `expected`-type propagation (added specifically to disambiguate
  ambiguous unit literals and derived dimensions via surrounding context
  — a **new** capability relative to `AICAD-051`/`AICAD-049`, so the
  highest-risk area for accidental over-reach) could cross from
  "resolving an otherwise-ambiguous case using an explicit annotation
  already provided by the user" into "silently picking a dimension with
  no annotation at all," which would violate DL-3's "ambiguity is an
  error, never an arbitrary selection." Traced every call site of
  `Checker::resolve_unit_literal` and `expected_dimension`: both are
  `Option`-gated on `expected` actually being supplied and actually
  naming a dimension the symbol is registered under (`cad_units::
  lookup(symbol, dim).is_some()`) — with no `expected` (or a non-matching
  one), the original AICAD-051/AICAD-049 ambiguous/unknown-rejection
  behavior is completely unchanged (re-confirmed by
  `ambiguous_unit_literal_without_annotation_is_reported`/`ambiguous_
  derived_dimension_without_context_is_reported`, both still producing
  the pre-existing rejection codes). **Checked, none found — this is a
  genuine new capability, not a relaxation of the ambiguity rule.**
- **DL-3 affine absolute/delta (RFC-0004 §7).** Covered in full in §3.4
  above, including a freshly re-run direct check at gate time (not merely
  re-cited). **Checked, none found.**
- **A new risk introduced by this batch specifically (not present before
  `AICAD-052`/`AICAD-053`): does extending equality comparison (`==`/
  `!=`/...) to struct/enum operands (`AICAD-053`, `Checker::check_binary`'s
  comparison arm) constitute a public-semantics change beyond what
  RFC-0004 froze, since RFC-0004's own comparison text (§5) is written in
  terms of *dimensional* quantities only?** Examined this specifically:
  `cad_units::check_comparison` (RFC-0004 §5's own implementation) already
  restricts itself to *numeric* scalars, and its own module doc comment
  explicitly defers non-numeric-scalar comparison agreement (e.g.
  `Bool == Bool`) to "the general type checker's job (`AICAD-052`)" — a
  boundary `cad_units` itself drew, not one this batch invented. RFC-0001
  §7's grammar defines `binary_expr`'s equality operators generically over
  `expression`, with no type restriction at the syntax level at all — the
  *type checker* (this batch, both halves) is exactly where such a
  restriction or permission is decided. Extending the same already-
  established "general type checker's job" principle from `AICAD-052`
  (which already made `Bool == Bool` type-check, not `cad_units`) to
  struct/enum operands in `AICAD-053` is a direct, small continuation of
  that same principle, not a new architectural decision — and it is
  directly evidenced by the one concrete example anywhere in this
  repository showing enum-variant comparison (`Product.motor == NEMA17`,
  `examples/assemblies/stage0_paper_example.aicad`). Struct/enum equality
  is nominal-type-identity only (§3.8) — no structural/field-by-field
  comparison semantics were invented, which would have been the genuinely
  speculative move. **Checked and reasoned through explicitly; judged not
  a public-semantics drift requiring an RFC, for the reasons above — see
  `project/reports/AICAD-053.md` decision 3 for the same reasoning
  recorded permanently at the point it was made, not only here.**
- **Binder/type-checker namespace separation (`crates/cad-compiler/src/
  binder.rs`'s own established rule: "method/field is never checked as a
  scope name").** Checked that this batch's own field-access typing
  (`Checker::check_field_access`) never touches the lexical-scope/binding
  namespace at all — it resolves purely against `Checker::struct_fields`,
  a type-level table this batch owns, keyed by the *receiver's resolved
  type*, never by scope lookup. This preserves the binder's own namespace
  separation rather than quietly merging the two namespaces the binder
  went out of its way to keep apart. **Checked, none found.**
- **Compiler intrinsics (DL-7, RFC-0001 §6).** No new compiler intrinsic
  was added — every dimensional/arithmetic rule is delegated to the
  existing `cad_units` library functions (§3.1-3.4), and every new
  capability this batch adds (struct/enum typing, `expected`-type
  propagation) is ordinary type-checker logic over already-existing HIR
  shapes, not a new runtime primitive. **Checked, none found.**

No accidental public semantic drift relative to RFC-0001 or RFC-0004 was
found. The one genuinely new, evidence-requiring capability this batch
adds (`expected`-type-directed disambiguation) was specifically
re-verified to be strictly additive to, not a relaxation of, DL-3's
ambiguity-is-an-error rule.

## 7. Known limitations (carried forward, not blocking Batch S2-08)

- Method-call (`receiver.method(args)`) target resolution remains
  unimplemented — no method/interface-implementation declaration syntax
  exists anywhere in the language yet, and `HirCallee::Method` structurally
  has no `binding` field to resolve into regardless (a HIR-shape change,
  out of both this batch's tasks' scope).
- Calling a binding that is neither `Fn`-kind nor `Struct`-kind (a plain
  `let`/`var`/`const`, an enum/enum-variant name) is not itself flagged
  as a "not callable" type error.
- No struct-declaration duplicate-field-name checking (`struct P { x:
  Int, x: Int }` is not rejected) — `AICAD-050`'s own report assigned
  this to `AICAD-053`'s title, but on reflection (recorded in
  `project/reports/AICAD-053.md`'s own "Known limitations") this task's
  evidenced scope is *typing struct/enum usage*, not re-auditing a
  declaration's own internal well-formedness (closer to a `crate::
  binder`-shaped concern). Flagged here explicitly as a real, currently-
  open gap, not silently dropped.
- No struct-pattern destructuring — `cad_ast::Pattern` has no
  struct-destructuring shape at all yet (only `Wildcard`/`Literal`/
  `Ident`), so `docs/plan/03` §10's "support destructuring" remains
  unimplemented pending a future grammar-extension task, consistent with
  every prior task's own "do not invent unevidenced grammar" discipline.
- No exhaustiveness checking for `match` over an enum's variants.
- The D5 concrete numeric tolerance constants (`DECISION_LOG.md#DL-12`)
  remain undetermined — unchanged since Batch S2-01, still correctly
  scoped to a future `cad-validation`/execution-determinism task, not
  this types/HIR batch.
- `project/TASKS.yaml`'s pre-`AICAD-038` staleness (`status: todo` on
  long-complete Stage-0/Stage-1 tasks) remains unfixed — out of every
  Stage-2 batch's own scope, noted again for the next session per prior
  batches' own handoffs.

## 8. Recommendation

**PASS — Batch S2-08 (`AICAD-054` onward) may begin.** All nine checklist
items in §3 are met; the explicit adversarial audit against RFC-0001/
RFC-0004 (§6) found no accidental public semantic drift, and specifically
confirmed that this batch's one genuinely new capability (context-
directed ambiguity resolution) is strictly additive to DL-3's ambiguity-
is-an-error rule rather than a relaxation of it. No `project/
OWNER_DECISIONS.md` item was touched or newly required by either task in
this batch. This recommendation does not itself constitute Stage-2 owner
approval — Stage 2 as a whole still requires the owner-recorded decision
`AICAD-064` will seek, per `project/CURRENT_STAGE.md`.
