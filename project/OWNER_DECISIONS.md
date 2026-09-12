# AICAD Owner Decisions

Unresolved architecture/product decisions requiring owner approval belong here.
Do not silently resolve them in implementation work.

Populated by the orientation pass over `docs/plan/` performed per
`project/FIRST_PROMPT.md`, before executing AICAD-002 onward, and finalized
by task AICAD-005 ("Create OWNER_DECISIONS.md and DECISION_LOG.md from
unresolved plan decisions"; see `project/reports/AICAD-005.md`). See
`project/reports/ORIENTATION_PASS.md` for the full analysis (document map,
invariants, traceability matrix, contradictions) this list was drawn from.
This file is not closed — re-run the review whenever a later task's
`plan_references` surface a new unresolved decision, and add it here rather
than resolving it implicitly.

Status legend: `open` = no owner ruling yet; `RESOLVED` = fully settled, see
the cited `project/DECISION_LOG.md` entry; `PARTIALLY RESOLVED` = the owner
ruled on the part needed now, with an explicit remaining sub-question still
open below and in the log entry itself. A resolved/partial entry's original
question/plan-references are kept here for context; the ruling and its
rationale live in `project/DECISION_LOG.md`.

**Quick index (as of 2026-09-09):**

| ID | Status |
|---|---|
| D1 Canonical surface syntax | RESOLVED — DL-1 |
| D2 Mutation semantics | RESOLVED — DL-2 |
| D3 Sketch entity/object model | RESOLVED — DL-19 |
| D4 Type/units semantics | RESOLVED — DL-3 |
| D5 Determinism-equivalence contract | RESOLVED (v1 policy) — DL-12 |
| D6 Kernel abstraction boundary | RESOLVED — DL-5 |
| D7 Reference resolution/fallback policy | PARTIALLY RESOLVED — DL-8 |
| D8 OCAF vs. kernel-independent graph | PARTIALLY RESOLVED (directional) — DL-9 |
| D9 Intrinsic-vs-package boundary | RESOLVED — DL-7 |
| D10 Diagnostic stability policy | RESOLVED — DL-18 |
| D11 Constraint IR/solver independence | RESOLVED — DL-20 |
| D12 Trusted native plugin boundary | open |
| D13 OCCT/standards licensing | PARTIALLY RESOLVED (development policy) — DL-6 |
| D14 File extension/branding | RESOLVED — DL-4 |
| D15 Plugin runtime (WASM vs. external) | open |
| D16 Collection/iterator construction syntax | RESOLVED (Stage-2 minimum) — DL-13 |
| D17 Result<T,E>/data-carrying enum variants | RESOLVED (general generics + enums) — DL-14 |
| D18 Geometry-operation invocation mechanism from `.aicad` source | RESOLVED (runtime-backed standard functions) — DL-15 |
| D19 D5 v1 comparison-profile numeric tolerance constants | RESOLVED — DL-17 + AICAD-064A |
| D20 Struct-typed parameters in the always-seeded `RuntimeBuiltin` catalogue | open (safe workaround shipped in `AICAD-076`) |

---

## D1. Canonical surface syntax family

**Status: RESOLVED — see `project/DECISION_LOG.md#DL-1`.** Braces for
blocks, explicit semicolons for statements, broadly Rust/TypeScript-like,
no ASI. Original question/context kept below for record.

**Question:** Rust/TypeScript-style braces-and-semicolons vs. Python-style
indentation for the canonical `.aicad`/`.cadl` grammar (RFC-0001 scope)?

**Plan references:** `docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §4.1;
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §1.

**Prior framing (superseded — see resolution above):** The plan recommends
Rust/TypeScript-style but explicitly labels this a decision to "prototype
before freezing," not a settled choice.

**Blocking impact:** RFC-0001 (AICAD-007) and every subsequent parser task
(AICAD-039+) depend on this being frozen first.

---

## D2. Mutation semantics: functional core vs. builder/method sugar

**Status: RESOLVED — see `project/DECISION_LOG.md#DL-2`.** Functional,
value-oriented semantic core; method/builder syntax is sugar lowering to
functional HIR calls and never implies in-place mutation. Original
question/context kept below for record.

**Question:** Is `body = cut(base, holes);` (functional) or
`base.cut(hole(...));` (method/builder) the canonical surface form, and what
exactly does builder-style syntax lower to in HIR?

**Plan references:** `docs/plan/02_LANGUAGE_AND_COMPILER.md` §5;
`docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §4.2. The plan's own
examples are inconsistent on this without resolving it — see
`project/reports/ORIENTATION_PASS.md` §6 (contradiction 3).

**Prior framing (superseded — see resolution above):** Plan recommends
supporting both (functional core + ergonomic sugar) but the exact
desugaring/SSA lowering rule is not specified.

**Blocking impact:** RFC-0001, grammar (AICAD-039/041), HIR lowering
(AICAD-051).

---

## D3. Sketch entity/object model

**Status: RESOLVED — see `project/DECISION_LOG.md#DL-19`.** An explicit,
kernel-independent semantic `Sketch` object owns its plane/frame, explicit
entity nodes, explicit constraint nodes, deterministic local semantic entity
identities, and source/provenance links; no hidden global mutable
registration; block syntax may exist as sugar but must lower to this same
explicit model; Stage-3 sketch entity identity is not OCCT topology IDs and
does not claim to solve Stage-4 persistent topological naming. Original
question/context kept below for record.

**Question:** Are sketch entities (`line`, `circle`, `arc`, ...) explicit
named objects registered onto a `Sketch` value, or sugar over a declarative
block? How does an entity created by a free function actually attach to a
specific `Sketch`'s plane and constraint solver?

**Plan references:** `docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3 (`sketch`,
`line`, `circle`, ... signatures); `docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md`
§4.3. No example in the bundle (checked 02, 04, 18) shows the binding
mechanism — see `project/reports/ORIENTATION_PASS.md` §6 (contradiction 4).

**Prior framing (superseded — see resolution above):** Plan favored explicit
objects internally with block syntax as sugar, but the mechanism was
undefined.

**Blocking impact:** Stage 3 sketch IR/constraint tasks (AICAD-072, AICAD-073,
AICAD-074, AICAD-075).

---

## D4. Type/units semantics specifics

**Status: RESOLVED — see `project/DECISION_LOG.md#DL-3`.** Structural
canonicalization, same-dimension implicit conversion only, conservative
interval `Tolerance<T>` by default (RSS/statistical is a separate later
API), affine units distinguish absolute vs. delta. Original question/
context kept below for record.

**Question:** Exact dimension-canonicalization algorithm, whether/when
implicit unit conversions are allowed, and tolerance/range arithmetic
semantics (e.g. how `Tolerance<T> + Tolerance<T>` behaves).

**Plan references:** `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §3-6,
§21; `AICAD_AGENT_OPERATING_MODEL.md` §7 item 3.

**Prior framing (superseded — see resolution above):** Baseline dimension
list and quantity representation are given, but canonicalization/
conversion/tolerance-arithmetic rules are not fully specified.

**Blocking impact:** RFC-0004 (AICAD-010), `cad-units` (AICAD-047-049). This
is an AGENTS.md escalation trigger ("change typed-units semantics") — must
not be resolved implicitly.

---

## D5. Canonical-state / deterministic-equivalence contract

**Status: RESOLVED (v1 policy) — see `project/DECISION_LOG.md#DL-12`.**
Determinism is layered (Level 1 language/compiler exact-and-byte-identical-
where-defined; Level 2 same locked kernel environment, semantic/numerical
verification, no byte-identical B-rep requirement; Level 3 supported
cross-platform, versioned dimension-aware equivalence profile; Level 4
different kernel version, compatibility not identity), with a versioned
dimension-aware comparison profile (linear/area/volume/center-of-mass
tolerances parameterized by characteristic scale `S`) whose *shape* is
frozen but whose concrete v1 numeric constants remain to be derived from
Stage-1 evidence and documented during Stage 2 (not yet done as of this
ruling — track under Stage-2 `crates/cad-validation` work, escalating the
derived constants here if Stage-2 evidence does not make them obvious).
Original question/context kept below for record.

**Question:** What does "equivalent geometry and validation results" mean
in testable terms across kernel/platform versions? What tolerance/comparison
method defines pass/fail for the determinism benchmark?

**Plan references:** `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §3 invariant 9;
`docs/plan/14_COLLABORATION_PROVENANCE_SECURITY.md` §18 (explicitly warns
against claiming bit-identical B-rep across kernel/platform versions);
`docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §9. See
`project/reports/ORIENTATION_PASS.md` §6 (contradiction 5).

**Prior framing (superseded — see resolution above):** The two documents do
not contradict each other outright, but neither defines the actual
comparison tolerance, and this is an AGENTS.md escalation trigger ("change
the canonical-state/determinism contract").

**Blocking impact:** The concrete v1 tolerance constants (not the policy
shape, which is now resolved) should be settled during Stage 2, before the
Stage 8/13 determinism benchmarks mature.

---

## D6. Kernel abstraction boundary and lineage exposure

**Status: RESOLVED — see `project/DECISION_LOG.md#DL-5`.**
Capability-driven minimal kernel-neutral surface; no OCCT type crosses
`cad-occt-bridge`; adapter exposes operation-local lineage evidence only,
durable identity is owned above the kernel. Original question/context kept
below for record.

**Question:** Exact narrow bridge operation set and what lineage information
(entity split/merge/creation history) the kernel adapter must expose to the
semantic-reference layer above it.

**Plan references:** `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.6, §4, §8;
`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §8; `AICAD_AGENT_OPERATING_MODEL.md`
§7 item 5.

**Prior framing (superseded — see resolution above):** A representative
bridge operation list exists (§4 of doc 01), but it is stated as "add
capabilities only as required" — the boundary is not frozen, and
lineage-exposure requirements are described conceptually without a
concrete adapter-level contract.

**Blocking impact:** AICAD-016/017/018 (kernel bridge crates), and
AICAD-086/087 (lineage capture) in Stage 4.

---

## D7. Semantic-reference resolution model, durability, and fallback policy

**Status: PARTIALLY RESOLVED — see `project/DECISION_LOG.md#DL-8`.**
Fail-closed resolution (`Resolved`/`Ambiguous`/`Broken` only, never an
arbitrary best candidate) is settled; geometry-fingerprint matching is
disabled as an automatic fallback for the first Stage-4 implementation and
usable only for diagnostics/ranking/experiments. **Still open:** whether/
when to enable automatic fingerprint-based recovery later — deferred to a
future owner decision gated on benchmark evidence of a negligible
silent-wrong-resolution rate, not currently blocking any Stage 0-4 task.
Original question/context kept below for record.

**Question:** Exact resolution precedence across the reference-construction
strategies in `06_REFERENCES_QUERIES_FEATURE_DAG.md` §3, the precise
durability-level assignment rules (§11), and — critically — under what
"approved policy" (if any) a geometry-fingerprint fallback is permitted
before it must be treated as ambiguous/broken instead of silently resolved.

**Plan references:** `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §3, §7,
§11; `AICAD_AGENT_OPERATING_MODEL.md` §7 item 6. Task AICAD-092 in
`project/TASKS.yaml` ("Implement geometry-fingerprint fallback only under
approved policy") presupposes this decision exists by the time Stage 4
reaches it.

**Prior framing (largely superseded — see resolution above; the remaining
open sub-item is restated there):** This was the single highest-priority
decision in the plan: `AGENTS.md` and `docs/plan/00_PRINCIPLES_AND_SCOPE.md`
§3 invariant 8 both state ambiguity must be an error, never an arbitrary
selection, and Stage 4 is a hard release gate specifically about
silent-wrong-resolution risk (`docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md`
§5, `docs/plan/15_IMPLEMENTATION_ROADMAP.md` Stage 4).

**Blocking impact:** AICAD-088 through AICAD-092 directly; do not implement
a fallback-selection policy implicitly when that stage is reached.

---

## D8. OCAF usage vs. kernel-independent semantic graph

**Status: PARTIALLY RESOLVED (directional) — see
`project/DECISION_LOG.md#DL-9`.** AICAD owns a kernel-independent semantic
graph, authoritative for identity/features/dependencies/references; public
semantics/serialized identity must never depend on OCAF. **Still open:**
the exact extent (if any) of OCAF's use as an internal OCCT-side
persistence/labeling/lineage aid remains prototype-driven — track under
`project/experiments/` before `crates/cad-occt-bridge` or
`crates/cad-references` commit to a specific internal mechanism. Original
question/context kept below for record.

**Question:** Should persistent semantic references and lineage be built on
OCCT's OCAF framework, on a kernel-independent semantic graph layered above
it, or a hybrid — and what happens if OCAF's native topological-naming
capability is later found insufficient?

**Plan references:** `docs/plan/19_RESEARCH_NOTES_AND_SOURCES.md` §3;
`docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §4.5;
`AICAD_AGENT_OPERATING_MODEL.md` §7 item 7. This is also an explicit
AGENTS.md escalation trigger ("select between major unresolved architecture
alternatives").

**Prior framing (largely superseded — see resolution above; the remaining
open sub-item is restated there):** Plan leans toward "likely needs a
kernel-independent semantic graph above OCAF" but explicitly calls for
prototyping both before committing.

**Blocking impact:** WP-07 (semantic references), Stage 4 tasks broadly.

---

## D9. Compiler-intrinsic vs. standard-package boundary enforcement

**Status: RESOLVED — see `project/DECISION_LOG.md#DL-7`.** A new compiler
intrinsic requires an RFC demonstrating it cannot reasonably be ordinary
source, a standard package, or an existing kernel API operation; no
intrinsic may be added solely for implementation convenience. Original
question/context kept below for record.

**Question:** The plan gives a qualitative test ("could this be a library
instead of a compiler intrinsic?" — `docs/plan/00_PRINCIPLES_AND_SCOPE.md`
§10) but no enforced process (e.g. an RFC requirement or CI check) for new
intrinsics. Should one be adopted formally?

**Plan references:** `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §10;
`docs/plan/12_PACKAGES_PLUGINS_EXTENSIONS.md` §16; `AICAD_AGENT_OPERATING_MODEL.md`
§7 item 9. AGENTS.md escalation trigger: "add a compiler intrinsic where a
library solution may work."

**Prior framing (superseded — see resolution above):** Open, low urgency
until the standard library work in Stage 3+ begins in earnest.

---

## D10. Diagnostic code/schema stability policy

**Status: RESOLVED — see `project/DECISION_LOG.md#DL-18`.** Committed
diagnostic codes are durable and never silently repurposed; pre-1.0 codes
may be deprecated/replaced but not silently renumbered/reused; the
machine-readable schema is versioned; a compatibility-breaking schema
change requires explicit review. Adding a new code in an existing family,
or a wholly new family, is ordinary task work. Original question/context
kept below for record.

**Question:** What is the exact stability/deprecation policy for diagnostic
codes (the `PARSE-E###`/`GEOM-E###`/etc. families) once published — can
codes be renumbered pre-1.0, and what is the process for adding new
families?

**Plan references:** `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §10-12;
`AICAD_AGENT_OPERATING_MODEL.md` §7 item 10.

**Prior framing (superseded — see resolution above):** A taxonomy and JSON
schema existed; a stability policy did not.

**Blocking impact:** AICAD-038 (`cad-diagnostics` crate + JSON-schema
conformance tests) is the first task that materialized this schema;
AICAD-078 (Stage 3, normalized diagnostics) requires this ruling in force
first.

---

## D11. Constraint IR semantics and solver-independence rules

**Status: RESOLVED — see `project/DECISION_LOG.md#DL-20`.** The AICAD
constraint IR (typed variables, constraint kinds/parameters, semantic IDs,
provenance, solve-status vocabulary, structured diagnostics) is
authoritative; a solver backend owns numerical algorithms only and may
never redefine dimensional semantics, constraint-kind meaning, success/
failure classification, or silently let an arbitrary branch become
language semantics; a provably minimal overconstraint conflict set is not
required in the Stage-3 baseline. Original question/context kept below for
record.

**Question:** Exact constraint IR contract (`docs/plan/08_CONSTRAINTS_REQUIREMENTS_TESTS.md`
§4) and the precise boundary of what a pluggable solver backend may vs. may
not change about observable solving behavior (e.g. solution-branch
selection, conflict-set minimality guarantees).

**Plan references:** `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.4;
`docs/plan/08_CONSTRAINTS_REQUIREMENTS_TESTS.md` §4, §6;
`AICAD_AGENT_OPERATING_MODEL.md` §7 item 11.

**Prior framing (superseded — see resolution above):** Open; needed before
sketch/assembly solving expands (Stage 3 AICAD-073/074, Stage 6).

**Blocking impact:** AICAD-073 (Batch S3-05) implements this boundary
directly.

---

## D12. Trusted native extension boundary and plugin security model

**Question:** Exact approval process and technical sandbbox boundary for
"Level 4 — Trusted native plugin" extensions (kernels, kernel-level
importers/exporters, native solvers).

**Plan references:** `docs/plan/12_PACKAGES_PLUGINS_EXTENSIONS.md` §6, §8;
`docs/plan/14_COLLABORATION_PROVENANCE_SECURITY.md` §11, §15;
`AICAD_AGENT_OPERATING_MODEL.md` §7 item 12. AGENTS.md escalation trigger:
"add a new trusted native/plugin boundary."

**Status:** Open, low urgency — relevant starting at Stage 11.

---

## D13. Licensing/redistribution for standards-derived content and OCCT

**Status: PARTIALLY RESOLVED (development-phase policy) — see
`project/DECISION_LOG.md#DL-6`.** OCCT treated strictly as an external
dependency (dynamic linkage, preserved notices, no incorporation of OCCT
source), and standards-derived functionality must be independently
implemented with no copied ISO/ASME protected content. **Still open:** a
formal distribution/license review is required before any public
binary/commercial distribution policy is frozen — this is a separate,
future gate, not yet scheduled. Original question/context kept below for
record.

**Question:** (a) OCCT's license (LGPL-2.1-with-exception family) and its
implications for how the native bridge is built/distributed/linked; (b)
licensing/redistribution constraints for content derived from ISO/ASME
standards (STEP AP242, GD&T symbology/semantics in
`docs/plan/13_ENGINEERING_MODULES.md` §11) if the standard's text or tables
are reproduced in code, docs, or test fixtures.

**Plan references:** `docs/plan/19_RESEARCH_NOTES_AND_SOURCES.md` §10 ("licensing/
redistribution constraints for standards-derived libraries/data");
`docs/plan/13_ENGINEERING_MODULES.md` §11 ("implement against
licensed/authoritative standards work during the engineering stage").

**Prior framing (largely superseded — see resolution above; the remaining
open sub-item is restated there):** This was an explicit AGENTS.md/
operating-model escalation trigger ("require a license/security policy
decision") and had not been addressed anywhere in the plan bundle.

**Blocking impact:** Relevant as soon as Stage 1 links against OCCT
(AICAD-015/016), and required before Stage 12B (GD&T) implementation.

---

## D14. File extension / project branding

**Status: RESOLVED — see `project/DECISION_LOG.md#DL-4`.** Product name
AICAD; source extension `.aicad`; project manifest `aicad.toml`; a
distinct `.aicadpkg` extension if a packaged archive format is needed;
`CAD-IR` stays an internal name only. Original question/context kept below
for record.

**Question:** Final module and bundle file extensions (candidates: `.cadl`
source / `.aicad` bundle, vs. `.aicad` for both) and product branding
(`AICAD` / `CAD-IR` / `CADLang`).

**Plan references:** `docs/plan/README.md` ("Working names ... placeholder");
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §2;
`docs/plan/09_FILE_FORMAT_INTERCHANGE_RECONSTRUCTION.md` §1;
`docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §4.6. The two normative
documents (02, 09) do not fully agree with each other even though both
flag the choice as unresolved — see
`project/reports/ORIENTATION_PASS.md` §6 (contradiction 2).

**Prior framing (superseded — see resolution above):** Open, explicitly
deferred by the plan itself.

**Blocking impact:** Low urgency for Stage 0, but should be resolved before
RFC-0001 finalizes source-file conventions and before AICAD-002's repository
skeleton commits to concrete file extensions.

---

## D15. Package plugin runtime (WASM vs. external process) — prototype first

**Question:** Which sandboxed plugin runtime is the default "Level 2" ABI
before committing to one implementation.

**Plan references:** `docs/plan/12_PACKAGES_PLUGINS_EXTENSIONS.md` §6, §8;
`docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §4.4.

**Status:** Open; plan explicitly calls for prototyping both. Low urgency —
relevant starting at Stage 11.

---

## D16. Collection/iterator construction syntax (for `for`-loop execution)

**Status: RESOLVED (Stage-2 minimum foundation) — see `project/
DECISION_LOG.md#DL-13`.** `for` operates over AICAD's iteration protocol;
Stage 2 supports at minimum `List<T>` (new `[e1, e2, ...]` list-literal
syntax), `Range<Int>`/`Range<UInt>` (new `start..end`/`start..=end` range
syntax, auto-iterable ascending by one), and `Iterator<T>` as an internal/
runtime abstraction never exposed as compiler magic — explicitly not a
general-purpose compiler-intrinsic mechanism, and explicitly not
authorizing `Set<T>`/`Map<K,V>`/comprehensions/user-defined iterator
protocols/async-or-parallel iteration/implicit dimensional-range stepping.
Original question/context kept below for record.

**Question:** `AICAD-056` ("Implement loops and basic collections/
iterators") needs to give `for var in iterable { ... }` a real runtime
meaning, which requires at least one constructible collection/iterator
`Value` — but no `.aicad` source program can construct one today.
`specs/language/grammar.ebnf`'s frozen `expression` production (`call_expr
| method_call_expr | binary_expr | literal | identifier | "(" expression
")" | block_expr | if_expr | match_expr`) has no array/list-literal syntax
and no range operator; `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md`'s
own `for i in 0..count { ... }` generator example and `docs/plan/
02_LANGUAGE_AND_COMPILER.md` §9's required `List<T>`/`Range<T>`/
`Iterator<T>`/`Generator<T>` collections are plan-level sketches that were
never promoted into the grammar any completed Stage-2 batch actually
implements (`AICAD-039`-`045`'s frozen lexer/parser/grammar-checkpoint
scope has no such production, confirmed by `project/gates/
STAGE2-A_FRONTEND.md`). Nor does a compiler-intrinsic/builtin-function
mechanism exist as an alternative path: `cad_hir`'s name binding only ever
resolves user-declared `fn`/`struct`/`enum`/`let`/`const`/`param` items, so
even a `range(a, b)`-shaped built-in constructor callable through the
existing `call_expr` syntax would need a new kind of binding the language
does not have yet.

Three live options, none decided here:
1. Add a range-operator expression (`a..b`, possibly `a..=b`) as new
   grammar/AST/HIR surface syntax, with `for`/`Value` runtime support for a
   `Range` value specifically (narrowest scope, matches the plan's own
   `for i in 0..count` example, but is new public expression syntax).
2. Add array/list-literal expression syntax (`[e1, e2, ...]`) plus a `List`
   runtime value (broader — also gives `struct`/function code a way to
   build ad hoc collections — but is new public expression syntax and a
   real generic-type-system question for `List<T>`'s own typing).
3. Introduce a narrow compiler-intrinsic constructor-function boundary
   (e.g. `range`/`list` resolved specially by binding resolution rather
   than through ordinary `fn` declarations) so no new *expression* grammar
   is needed — but this is itself a `D9`/`DL-7`-governed decision ("a new
   compiler intrinsic requires an RFC demonstrating it cannot reasonably be
   ordinary source, a standard package, or an existing kernel API
   operation") and needs its own justification for why a library solution
   (option 1/2, once collections exist as real values) would not do.

**Plan references:** `docs/plan/02_LANGUAGE_AND_COMPILER.md` §9;
`docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` (the `0..count` generator
example); `specs/language/grammar.ebnf` (frozen `expression` production,
no collection-literal/range syntax); `AGENTS.md` escalation triggers
"change public language syntax or semantics beyond an approved RFC" and
"add a compiler intrinsic where a library solution may work"; `project/
TASKS.yaml`'s `AICAD-056` entry lists both as its own `escalate_if`
conditions verbatim.

**Prior framing (superseded — see resolution above):** Raised by
`AICAD-056` (`project/reports/AICAD-056.md`'s first session), which had
implemented `while`/`loop`/`break`/`continue` without needing a collection
value at all and left `for` reporting `RuntimeError::Unsupported` pending
this ruling.

**Blocking impact:** `AICAD-056` (resumed and completed with this ruling —
see `project/reports/AICAD-056.md`'s second session); any later task that
assumes a `List<T>`/`Range<T>`/`Iterator<T>` value or type exists should
check this entry's explicit scope limit (`Set<T>`/`Map<K,V>`/
comprehensions/user-defined iterators/dimensional stepping remain future
work) before extending it — `AICAD-059`'s Geometry IR and `AICAD-063`'s
end-to-end proof may need to iterate over geometry query results per
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §"All major collections should be
iterable", which this decision's `List<T>`/`Range<T>` foundation can now
support.

---

## D17. `Result<T,E>` construction syntax (data-carrying enum variants + user-defined generics)

**Question:** `AICAD-057` ("Implement recursion and Result/error
propagation") needs to give the language a real `Result<T,E>` value a
program can construct (`Ok(value)`) and match (`Ok(x) => ...`/`Err(e) =>
...`) — but no `.aicad` source program can do either today, for two
independent, more fundamental reasons than `D16`'s collection-syntax gap:

1. **AICAD enum variants cannot carry data at all.** `cad-ast`'s own enum-
   declaration parser (`Parser::parse_enum_variants`) returns
   `Vec<Spanned<String>>` — bare variant names only, confirmed by direct
   inspection, and `cad_ast::item`'s own module doc comment independently
   records the identical finding ("no evidence anywhere supports enum
   variants carrying data at all"). `Result<T,E>` (`Ok(T)` / `Err(E)`) is
   structurally a two-variant tagged union where *each variant carries a
   payload* — a different, larger kind of enum than anything approved so
   far (every existing AICAD enum, including the paper example's
   `MotorType`, is unit-variants-only).
2. **AICAD has no user-defined generic types or functions at all.**
   `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §12's own `fn
   mount<T: MotorMount>(...)` example is listed as a still-to-support
   future feature, not implemented syntax — confirmed against
   `cad-parser`'s `parse_type`/function-declaration parsing, neither of
   which accepts a type-parameter list anywhere. `D16` special-cased
   exactly two built-in generic names (`List`, `Range`) inside
   `cad_hir::typeck::resolve_type_ref` specifically *because* no general
   generic-type system exists — `Result<T,E>` as a *user-visible, fully
   general* two-parameter generic type is a different, larger question
   `D16` explicitly did not answer (see that entry's own "Blocking
   impact": "`AICAD-057` remains its own, separate scope").

Both are `AGENTS.md` owner-escalation triggers in their own right
("change public language syntax or semantics beyond an approved RFC" for
new enum-variant-payload/generic-type-parameter grammar; "select between
major unresolved architecture alternatives" for how generics/data-carrying
enums should work at all), and both are listed verbatim as `AICAD-057`'s
own `project/TASKS.yaml` `escalate_if` conditions.

**What `AICAD-057` implemented without needing this ruling:** recursion
(self- and mutual-recursive function calls, already expressible via
ordinary function-call syntax with no new grammar) and error propagation
through the call stack (a `RuntimeError` raised at any depth, through any
control-flow construct, correctly unwinds to the top as one diagnostic,
never silently swallowed) — both real WP-04 execution-layer requirements
with zero syntax gap, plus a new recursion-depth budget
(`RuntimeError::RecursionLimitExceeded`) satisfying `AGENTS.md`'s
"Recursion... must fail with structured diagnostics rather than crashing
the host process" (a *native Rust stack overflow* was actually reproduced
during this task's own testing at a surprisingly shallow depth in this
crate's debug-build environment — see `project/reports/AICAD-057.md` and
`crates/cad-runtime/src/interp.rs`'s own `DEFAULT_MAX_CALL_DEPTH` doc
comment for the exact empirical finding). See that report for the full
scope split.

Live options for `Result<T,E>` itself, none decided here:
1. Add general data-carrying enum-variant syntax (`enum Name { Variant(T),
   ... }`) plus general user-defined generic type parameters (`enum
   Result<T,E> { Ok(T), Err(E) }`, `fn f<T>(...)`), then define `Result<T,
   E>` as an ordinary standard-library enum built from those two features
   — broadest, most general-purpose, but the largest new-syntax surface
   (touches declaration grammar, typed HIR, pattern matching, and the type
   checker's whole nominal-type story at once).
2. Special-case `Result<T,E>`/`Ok`/`Err` narrowly (mirroring `D16`'s own
   `List`/`Range` special-casing inside `resolve_type_ref`) without
   general user-defined generics or general data-carrying enums — smaller
   surface, but does not generalize to any other future data-carrying type
   (`Optional<T>` next, from the same §9 collection list) without
   repeating the same special-case exercise.
3. Defer `Result<T,E>` entirely for Stage 2 and scope `AICAD-057` to
   recursion + call-stack error propagation only (already implemented,
   see above) — `Result<T,E>` becomes explicit future work, tracked here
   rather than silently dropped.

**Plan references:** `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §9
(`Result<T,E>` in the required-collections list), §12 (`fn mount<T:
MotorMount>` generics example, unimplemented); `docs/plan/
04_HIGH_LEVEL_MODELING_API.md` ("`try_*` forms can expose `Result<T,E>`
explicitly" — a future high-level API detail, not concrete syntax);
`cad_ast::item`'s own module doc comment (unit-only enum variants);
`AGENTS.md` escalation triggers "change public language syntax or
semantics beyond an approved RFC" and "select between major unresolved
architecture alternatives"; `project/TASKS.yaml`'s `AICAD-057` entry
lists both as its own `escalate_if` conditions verbatim.

**Status: RESOLVED (option 1 — general generics + general data-carrying
enums) — see `project/DECISION_LOG.md#DL-14`.** The owner selected option 1
above, not the narrow `D16`-style special case (option 2) or deferral
(option 3): Stage 2 adds the minimum *general* language machinery for
ordinary generic algebraic data types and generic functions (type
parameters on `struct`/`enum`/`fn` declarations, `Name<T,U>` type
application at declaration sites, tuple/record enum-variant payloads,
corresponding destructuring patterns, nominal-enum match-exhaustiveness
checking, and call-site generic instantiation/inference for the approved
subset), and `Result<T,E>`/`Optional<T>` are then defined as ordinary
prelude enums built from that machinery — no `Result`-specific compiler
semantics beyond ordinary prelude registration. Higher-kinded types,
variance, specialization, generic metaprogramming, variadic generics,
dependent types, generic associated types, and lifetime parameters remain
out of scope; interface/trait bounds (`T: MotorMount`) remain deferred
until the interface system exists, unless a later Stage-2 coverage-audit
finding shows otherwise (`project/reports/AICAD-057A.md` found no such
requirement). No `?`-operator or other new propagation syntax is
authorized — `Result` values are propagated with ordinary `match` for now.
See `project/DECISION_LOG.md#DL-14` for the full ruling text and the fixed
remediation task sequence (`AICAD-057A`..`AICAD-057F`) it prescribes before
the original `AICAD-057` resumes.

**Blocking impact:** Resolved; implementation proceeds via
`AICAD-057B`(generics syntax/AST/HIR) -> `AICAD-057C` (data-carrying enums/
patterns/exhaustiveness) -> `AICAD-057D` (generic instantiation/inference)
-> `AICAD-057E` (`Result<T,E>`/`Optional<T>` as ordinary prelude generics)
-> `AICAD-057F` (adversarial generality proof) -> original `AICAD-057`
resumes -> `AICAD-058` -> the `STAGE2-C_EXECUTION` checkpoint, in that fixed
order (`project/TASKS.yaml`, `project/SESSION_HANDOFF.md`).

---

## D18. Geometry-operation invocation mechanism from `.aicad` source

**Status: RESOLVED — see `project/DECISION_LOG.md#DL-15`.** Ordinary safe
geometry operations use ordinary function-call syntax, backed by a general
(not geometry-specific) compiler/runtime-owned "standard function"
mechanism — `FunctionImplementation::Aicad(HirBlock)` vs.
`FunctionImplementation::RuntimeBuiltin(BuiltinFnId)` — participating in
the same ordinary binding/typing/call-expression semantics as
AICAD-defined functions, explicitly not a compiler intrinsic and not an
arbitrary native-callback facility. A deliberate, documented Safe CAD
source API sits above Geometry IR (not a 1:1 exposure of every
`GeometryOp`/`GeometryQuery` variant). Original question/context kept below
for record.

**Question:** `AICAD-060` ("Implement HIR/runtime geometry dispatch into
Geometry IR/kernel API") needs to give a running `.aicad` program a way to
actually *invoke* a geometry operation (`box(...)`, `cylinder(...)`,
`cut(...)`, `fillet(...)`, ...) so that evaluating an ordinary expression
builds `cad_geometry_api::GeometryGraph` nodes — but today, for the same
structural reason `D16`/`D17` each found before it, no `.aicad` source
program can do this at all, and this task's own coverage audit (below)
found no existing extension point.

**What this task's own audit found (see
`project/reports/AICAD-060.md` for the full research trail):**

1. `cad_runtime::interp::Interpreter::call` dispatches exactly two callee
   kinds today: `BindingKind::Fn` (looks up the callee's own
   `HirItem::Fn { body: HirBlock, .. }` and executes that body via
   `run_fn_body`/`exec_block`) and `BindingKind::EnumVariant` (constructs a
   `Value::EnumVariant` directly, no HIR body at all). `HirItem::Fn::body`
   is a mandatory `HirBlock` field — there is no variant, no `BindingKind`,
   and no field anywhere that lets a function's implementation be "a Rust
   callback" instead of real AICAD-source-derived HIR. Confirmed by direct
   inspection of `cad-hir`'s `hir.rs`/`ids.rs` and `cad-runtime`'s
   `interp.rs`; grepping the whole workspace for
   `Builtin|NativeFn|intrinsic|host_fn` (excluding the English word
   "intrinsically" in one doc comment) returns nothing structural.
2. The existing precedent for "a compiler-builtin construct with no
   user-visible source declaration" (`D16`'s `List<T>`/`Range<T>`) lives
   entirely at the *type-checker/HIR-node* level
   (`cad_hir::typeck::CheckedType::List`/`CheckedType::Range`,
   `HirExpr::ListLiteral`/`HirExpr::Range`) — it special-cases new
   *expression syntax*, not a *callable name*. There is no analogous
   precedent for a function whose implementation is not an AICAD-source
   `HirBlock`; `BindingKind::EnumVariant`'s own special-casing in
   `Interpreter::call` is the closest thing (a callee dispatched without
   running a `HirBlock`), but it is enum-construction-specific, not a
   general mechanism.
3. RFC-0002 §4 already froze a *raw/unsafe* topology-handle mechanism
   (`unsafe geometry { ... }` blocks, `validate()`/`adopt_validated()`) for
   Tier-C direct kernel-grade access, but that is explicitly the *unsafe*
   tier (RFC-0002 §6's Capability Tiers table) — collapsing ordinary
   Tier-B "Safe CAD" operations like `box(...)`/`cylinder(...)` onto that
   mechanism would erase the Tier B/C distinction the same RFC freezes, and
   `docs/plan/02_LANGUAGE_AND_COMPILER.md`'s own illustrative examples
   (`let body = box(...);`, `cut(working, hole)`) show ordinary call syntax
   with no `unsafe geometry` wrapper at all.
4. No `docs/plan/` document defines an authoritative builtin-function name/
   signature catalogue (`box`, `cylinder`, `extrude`, ...) — only
   illustrative example syntax. `AICAD-060`'s author must derive the exact
   surface from `cad_geometry_api::ir::GeometryOp`/`GeometryQuery`'s own
   variant set, which is itself only Stage-2-authorized IR shape, not a
   frozen language-surface spec.

This is squarely both `AGENTS.md` escalation triggers `AICAD-060`'s own
`project/TASKS.yaml` entry lists verbatim ("public syntax/semantics must
change beyond an approved RFC" and "an unresolved architecture alternative
must be selected") for the same reason `D16`/`D17` each were: giving a
program a way to invoke a geometry operation requires *some* new
binding/dispatch mechanism `cad-hir`/`cad-runtime` do not have today, and
more than one shape for that mechanism is defensible.

**Live options, none decided here:**

1. **A new `BindingKind` dispatched specially at the call site**, mirroring
   `BindingKind::EnumVariant`'s own existing precedent exactly: add e.g.
   `BindingKind::GeometryIntrinsic(GeometryIntrinsicOp)` (or similar), have
   a prelude-like mechanism (extending `cad_hir::prelude`'s existing
   `with_prelude` pattern, currently type-only for `Result`/`Optional`)
   pre-populate global scope with names mapping 1:1 onto every
   `GeometryOp`/`GeometryQuery` variant (`box`, `cylinder`, `extrude`,
   `revolve`, `sweep`, `loft`, `union`, `cut`, `intersect`, `fillet`,
   `chamfer`, `shell`, `offset`, `transform`, `import_step`, `line_edge`,
   `circle_wire`, `wire_from_edges`, `make_face`, `is_valid`, `volume`,
   `area`, `bounding_box`, `center_of_mass`, `validate`, `tessellate`,
   `export_step`), and dispatch each specially in `Interpreter::call`
   without ever running a `HirBlock` for it. Broadest and most direct match
   to `cad-geometry-api`'s already-frozen IR surface; requires an RFC-0001
   §6/RFC-0002 amendment documenting why this is "a kernel API operation
   exposed through existing language mechanisms" (DL-7's own carve-out,
   since every one of these names maps 1:1 onto an already-RFC-0002-frozen
   Stage-1 kernel capability) rather than a from-scratch intrinsic needing
   its own per-operation RFC.
2. **Reuse/extend RFC-0002 §4's already-frozen `unsafe geometry { ... }`
   mechanism** for ordinary geometry construction too, rather than adding a
   second mechanism. Smaller surface (no new binding kind), but blurs the
   Tier B ("Safe CAD", validated operations) / Tier C ("Unsafe geometry",
   raw topology, requires explicit `validate()`/`adopt_validated()`)
   distinction RFC-0002 §6 already froze for a different purpose — every
   ordinary `box(...)`/`cut(...)` call would need to run inside an
   `unsafe` block and be explicitly validated back out, which
   `docs/plan/02_LANGUAGE_AND_COMPILER.md`'s own example syntax does not
   show and which would make routine modeling code visually and
   semantically "unsafe" for no safety reason (a freshly-constructed,
   already-kernel-validated `Box`/`Cylinder` is never raw/stale topology in
   the sense §4 protects against).
3. **Defer language-surface invocation for this task**, scoping
   `AICAD-060` down to exactly what this session implemented: the
   `GeometryGraph -> kernel` dispatcher (`crates/cad-geometry-runtime::
   dispatch`) plus the `NumberValue -> Quantity` conversion helper
   (`crates/cad-geometry-runtime::bridge`), both fully tested against a
   real `OcctContext`, with "wire this into actual `.aicad` call syntax"
   left as explicit follow-up work once this ruling lands — mirrors `D16`/
   `D17`'s own precedent of leaving the *introducing* task (`AICAD-056`/
   `AICAD-057`) blocked rather than silently inventing a mechanism.
   `AICAD-063`'s own end-to-end gate ("no demo-specific interpreter
   shortcut... `.aicad` source -> ... -> Geometry IR -> Stage-1 kernel
   API") cannot pass without *some* ruling on this question eventually, so
   this option only postpones the decision, it does not remove the need
   for one.

**What this task implemented without needing this ruling:** the complete
`GeometryGraph -> kernel` dispatch executor (`crates/cad-geometry-runtime::
dispatch::dispatch_graph`, covering every `GeometryOp`/`GeometryQuery`
variant, with `EdgeIndex`/`FaceIndex` resolved to real edge/face `Shape`s at
dispatch time per `AGENTS.md`'s raw-topology-is-ephemeral rule) and the
`NumberValue -> Quantity` bridge (`crates/cad-geometry-runtime::bridge`) — a
direct field-copy conversion, since both types already store a canonical-
unit magnitude plus `cad_units::OperandType` by independent design on each
side. Both are proven against a real `cad_occt_bridge::OcctContext` with
exact B-rep validity/closed-form volume/bounding-box/center-of-mass/STEP-
export-and-reimport evidence (never a render-only check), not merely unit
tests of data conversion. See `project/reports/AICAD-060.md`.

**Plan references:** `docs/plan/02_LANGUAGE_AND_COMPILER.md` (illustrative
`box(...)`/`cut(...)` example syntax, no frozen builtin catalogue);
`rfcs/0001-language-principles.md` §6 (DL-7, compiler intrinsics require an
RFC); `rfcs/0002-geometry-runtime-kernel-abstraction.md` §4 (raw topology/
`unsafe geometry` blocks), §6 (capability tiers); `cad_hir::prelude`'s own
module doc comment (the existing type-only prelude mechanism); `AGENTS.md`
escalation triggers "change public language syntax or semantics beyond an
approved RFC" and "select between major unresolved architecture
alternatives"; `project/TASKS.yaml`'s `AICAD-060` entry lists both verbatim
as its own `escalate_if` conditions.

**Blocking impact:** Resolved; `AICAD-060` resumes from its existing
partial implementation (the already-tested `GeometryGraph -> kernel`
dispatcher and `NumberValue -> Quantity` bridge are kept, not discarded)
and implements the general `RuntimeBuiltin` mechanism plus the Stage-2 Safe
CAD catalogue (`docs/API/safe-cad-api.md`) and source-to-`GeometryGraph`
path per `DL-15`. `AICAD-061` proceeds only after `AICAD-060` fully
completes, per the fixed Batch S2-11 order.

---

## D19. D5 v1 comparison-profile numeric tolerance constants

**Status: RESOLVED — see `project/DECISION_LOG.md#DL-17`.** `DL-17`
accepted three directly-evidenced constants (`linear_abs = 0.0001 mm`,
`center_of_mass_abs = linear_abs`, `volume_rel = 0.001`) as v1 defaults,
and authorized `AICAD-064A`'s bounded multi-scale calibration corpus to
derive the remaining four (`linear_rel`, `area_abs`, `area_rel`,
`volume_abs`) without a further owner ruling unless that evidence proved
ambiguous. `project/reports/AICAD-064A.md` derived all four from clear,
unambiguous evidence (`linear_rel = 0.0`, `area_abs = 1e-6`, `area_rel =
1e-3`, `volume_abs = 1e-6`) — no escalation was needed; the complete v1
profile is implemented in `crates/cad-validation::profile::
ComparisonProfile::v1`. Original question/context kept below for record.

`DECISION_LOG.md#DL-12` froze the *shape* of the D5
comparison profile (`linear`/`area`/`volume`/`center-of-mass`, each scaled
by characteristic linear scale `S`) but explicitly left the concrete v1
numeric constants unfixed, assigning Stage 2 to "derive and document them
from actual Stage-1 evidence ... and escalate the derived constants ...
for ruling if Stage-2 evidence alone does not make a specific constant
obvious," and named `AICAD-064` as the task that "must re-audit D5
evidence." No Stage-2 task (`AICAD-038`..`AICAD-063`) implemented
`crates/cad-validation`'s comparison-profile module or otherwise derived
these constants; the crate remains the unmodified `AICAD-002` placeholder
stub. This audit (`AICAD-064`) produces the measurements below rather than
leaving the gap unaddressed, per `AGENTS.md`'s "produce the measurements/
recommendation and escalate the constants rather than guessing."

**Evidence (`project/reports/AICAD-034.md`, Stage-1 bracket, characteristic
scale `S ≈ 80` mm):**
- Pre-fillet/chamfer bounding box matched the closed-form
  `(0,0,0)`–`(80,60,60)` corners to `~1.5e-7`. After fillet/chamfer, OCCT's
  own curve-approximation widened this to `~1e-6`, empirically observed
  identically (bit-for-bit) across 5 repeated runs — a genuine kernel
  numerical-noise floor, not flakiness.
- The bounding-box assertion used a `1e-4` absolute tolerance (two orders
  of magnitude above the observed `~1e-6` noise floor) specifically
  because it is a fillet/chamfer-affected quantity; the center-of-mass
  symmetry check and edge-selection logic (quantities never run through
  fillet/chamfer curve-fitting) instead used `1e-6`.
- Closed-form volume (`88000 − 2010.62 + 274.69 − 160 ≈ 86104.07`) matched
  the kernel-computed volume to within a `0.1%` (`1e-3`) relative
  tolerance on every run.
- Separately, `project/reports/AICAD-063.md`'s Stage-2 mounting-plate
  fixture (deliberately non-self-overlapping geometry) matched its
  closed-form volume to `~13` significant figures — far tighter than
  `1e-3`, but not independent evidence for a *general* constant since that
  fixture has no boolean-overlap or fillet/chamfer curve-fitting error
  source to bound.

**Recommended v1 constants, evidence-supported only:**
- `linear_abs = 1e-4` (length units matching `S`, e.g. mm) — directly
  evidenced (bounding-box tolerance, calibrated to `~100x` the observed
  noise floor).
- `volume_rel = 1e-3` — directly evidenced (closed-form-vs-kernel volume
  agreement, used consistently and successfully across every Stage-1
  boolean/fillet/chamfer combination tested).
- `center-of-mass` linear tolerance = `linear_abs` (`1e-4`) for the
  *general* profile. Note this is looser than the `1e-6` Stage-1 used for
  its own mirror-symmetry invariant — that `1e-6` figure is a
  fillet/chamfer-uninvolved quantity's much stronger *test-specific*
  invariant, not evidence for the general cross-comparison constant.

**Explicitly NOT evidence-supported — escalated rather than guessed:**
- `linear_rel`: no Stage-1 evidence exercised a part at a different
  characteristic scale, so no relative-term behavior was ever observed;
  Stage 1/2 fixtures are all tens-of-mm scale. Recommend the owner either
  set `linear_rel = 0` (pure absolute floor) until a multi-scale fixture
  produces real evidence, or set a conservative placeholder (e.g. `1e-6`)
  explicitly labeled provisional.
- `area_abs`/`area_rel`: no report measured an area comparison directly
  (only volume/bounding-box/center-of-mass were checked). A dimensional-
  analogy guess (e.g. `area_rel ≈ volume_rel`) would be exactly the kind
  of unsupported guess `AGENTS.md` prohibits; recommend either deferring
  area constants until a fixture measures them, or an explicit owner
  placeholder ruling.
- `volume_abs`: no fixture ever compared a near-zero volume, so no floor
  value has evidence either way.

**Plan references:** `DECISION_LOG.md#DL-12` (D5 v1 policy, assigns this
derivation to Stage 2 and names `AICAD-064` as the re-audit point);
`project/reports/AICAD-034.md` (source of all cited measurements);
`crates/cad-validation` (owns the eventual comparison-profile
implementation once a task is assigned to it — none has been yet).

**Blocking impact:** Non-blocking for the Stage-2 exit gate itself (no
`AICAD-038`..`AICAD-063` acceptance criterion required these constants to
exist), but `DECISION_LOG.md#DL-12` ties it to "before the Stage 8/13
determinism benchmarks mature." Recommend the owner rule on this before
`crates/cad-validation` is first implemented (a Stage-3-or-later task),
so that task starts from owner-approved constants rather than picking its
own.

---

## D20. Struct-typed parameters in the always-seeded `RuntimeBuiltin` catalogue

**Status: open (a safe, fully-typed workaround was shipped in `AICAD-076`
for that task's own four builtins; the underlying architecture question
is not resolved).**

**Question:** Can a `cad_hir::builtins::BuiltinFnId` catalogue entry's
signature reference a `cad_hir::geometry_types`-declared struct type
(`Point3`, `Axis3`, `Frame3`, `Plane`) as a `Named` parameter/return type,
the way `AICAD-075A` intended (`revolve(axis: Axis3, ...)`,
`hole(axis: Axis3, ...)`, `pocket(frame: Frame3, ...)`) — and if not
directly, should the compiler's builtin-seeding mechanism change so that
it can?

**What `AICAD-076` found:** `crate::lower::Lowerer::seed_builtins` seeds
*every* `BuiltinFnId` into *every* compiled program's global scope
unconditionally (there is no way to seed a subset), and `cad_hir::
typeck::Checker::collect_signatures` eagerly resolves every seeded
function's own parameter/return types up front — for every function,
including one the program never actually calls. A `HirTypeRef::Named`
reference to a `cad_hir::geometry_types` struct that is not itself in
scope (i.e. any program that has not separately composed `cad_hir::
geometry_types::with_geometry_types`, which is the overwhelming majority
of existing programs/tests, since that composition has always been
caller-optional — see that module's own doc comment) fails eagerly with
an `UNKNOWN_TYPE_NAME` diagnostic for *that program*, even when the
program never references the offending builtin at all. This was found by
direct experiment, not inferred: adding `axis: Axis3`/`frame: Frame3`
parameters to `revolve`/`hole`/`pocket` broke **149** previously-passing
`cad-hir` tests that have nothing to do with Stage-3 modeling, confirmed
by the exact diagnostic count matching the exact number of `Named`
geometry-type references added. By contrast, a `HirTypeRef::Generic`
reference to a user-defined generic struct (`Vector3<Float>`) does *not*
have this problem — an unresolvable generic base returns `None` silently,
with no diagnostic (`Checker::resolve_generic_type_application`'s own
`?`-early-return) — which is why `BuiltinFnId::Extrude`'s `direction:
Vector3<Float>` parameter is safe while a hypothetical `axis: Axis3`
parameter is not.

**What `AICAD-076` shipped instead, without resolving this:** `revolve`/
`hole`/`pocket` decompose what would ideally be one `Axis3`/`Frame3`
value into flat `Length` scalars (an axis/frame origin) plus a
`Vector3<Float>` (a direction) — the same scalar-decomposition narrowing
`BuiltinFnId::Transform` already established at Stage 2 for an analogous
reason (no safe way to reference the richer type yet). This preserves
correct typed-unit semantics (`Length` for position, never a bare float)
and violates no semantic distinction (`Vector3<Length>` was considered
and rejected as a `Point3` substitute specifically because `AICAD-075A`'s
own required semantic distinctions forbid collapsing position into
displacement merely because both would type-check). It is a safe,
complete, fully-tested workaround for `AICAD-076`'s own four builtins —
see `project/reports/AICAD-076.md` — not a resolution of the general
question, and it does not scale past a small, fixed number of flattened
scalar parameters: `AICAD-077`'s own planned `mirror(target, plane:
Plane)` hits the identical wall (a mirror plane has no natural
"decompose to N scalars" narrowing without collapsing `Plane` into
`Point3`+`Vector3` fields spelled out separately, at minimum).

**Live options, none decided here:**
1. **Always bundle `cad_hir::geometry_types`'s struct declarations with
   builtin seeding** (e.g. `seed_builtins`'s caller unconditionally
   composes `with_geometry_types`, or an equivalent mechanism), making
   every `cad_hir::geometry_types` type name resolvable in every compiled
   program regardless of whether the caller separately opts in. Broadest
   fix, changes `lower_program`'s own observable behavior for literally
   every caller (verified this does not disturb any *existing* item-index
   assertion, since builtin items are appended after user items and
   geometry-type items could be appended after that — but it is still a
   public-semantics change to what is implicitly in scope for every
   `.aicad` program, and duplicates `with_geometry_types`'s existing
   caller-composition role).
2. **Keep the caller-composition convention, and permanently restrict any
   builtin's own signature to primitive/dimensional/`List<T>`/generic-only
   types** (never a `cad_hir::geometry_types` struct), accepting the
   scalar-decomposition narrowing `AICAD-076` already applied as the
   long-term pattern, not just a stopgap. Zero architecture change, but
   permanently blocks a true `Axis3`/`Frame3`/`Plane`-typed Safe CAD
   builtin parameter, and produces increasingly awkward flattened
   signatures as richer geometry types (assembly frames, GD&T datums, ...)
   arrive in later stages.
3. **Make builtin (and/or all function) signature resolution lazy —
   resolved per call site rather than eagerly for every seeded
   declaration.** Would fix this specific problem without bundling extra
   scope into every program, but is a materially larger change to
   `cad_hir::typeck`'s own established eager-resolution architecture,
   affecting error-surfacing behavior for *every* function (user-defined
   or builtin), not just newly-added ones — a bigger and riskier change
   than either option above, and its own new "when exactly does a type
   error surface" semantics would need its own careful specification.

**Plan references:** `project/reports/AICAD-075A.md` (established
`cad_hir::geometry_types`/`cad_runtime::spatial` expecting a future
builtin to consume them directly as struct-typed parameters);
`project/reports/AICAD-076.md` (this finding, in full, plus the shipped
workaround); `crates/cad-hir/src/builtins.rs`'s own "Why no `Axis3`/
`Frame3`-typed parameter yet" note (mirrors this entry, colocated with
the affected code); `AGENTS.md` escalation triggers "public syntax/
semantics must change beyond an approved RFC" and "an unresolved
architecture alternative must be selected".

**Blocking impact:** Not blocking `AICAD-076` itself (a safe workaround
shipped). Relevant to `AICAD-077` (`mirror`'s own `Plane` parameter hits
the identical wall) and any later task wanting a struct-typed Safe CAD
builtin parameter — read this entry before guessing at a per-task
workaround; if `AICAD-077`'s own workaround would meaningfully differ
from `AICAD-076`'s pattern above, that is itself a sign this general
question should be resolved rather than accumulating ad hoc per-builtin
narrowings.

---

## Non-decision items carried forward for awareness (not owner rulings needed yet)

These are plan-acknowledged gaps/research items that do not currently block
Stage 0 work but should stay visible:

- Task list asymmetry: Stage 0/1/2/4 each end with an explicit "prepare
  owner gate packet" task; Stage 3 (AICAD-065..079) does not. See
  `project/reports/ORIENTATION_PASS.md` §6 (contradiction 6). Recommend
  owner confirm whether a Stage-3 gate task should be added before Stage 3
  begins, rather than silently adding or omitting one.
- `docs/plan/README.md`'s document index does not list
  `22_REPOSITORY_WORK_PACKAGES.md` or `23_CROSS_SYSTEM_PARAMETER_CATALOG.md`
  even though both are present in the bundle and are actively cited by
  every Stage-0 task. Treat the index as stale, not as evidence those files
  are non-canonical. See `project/reports/ORIENTATION_PASS.md` §6
  (contradiction 1).
- No explicit stage-to-work-package mapping exists in the plan; any such
  mapping (including the one in `project/reports/ORIENTATION_PASS.md` §2)
  is inference for planning convenience only.
- Research items in `docs/plan/19_RESEARCH_NOTES_AND_SOURCES.md` §10
  (constraint-solver robustness, topological-naming algorithms beyond OCAF,
  AP242 PMI/GD&T mapping coverage, B-rep fingerprinting across kernel
  versions, WASM plugin overhead, large-assembly streaming, solver-neutral
  FEA/CFD schema, affine-temperature typing, incremental-compiler
  architecture) are flagged by the plan itself as pre-implementation
  research, not yet owner decisions.
- Deliberately deferred product scope (not to be pulled forward without an
  owner decision to widen scope): ECAD/schematic authoring, architecture/BIM,
  full CAM toolpaths, photorealistic rendering, in-house topology-optimization
  solver, cloud PLM, real-time multi-user editing, certified standards
  database, native mobile CAD UI, domain-specific aerospace/medical
  validation (`docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §5).
