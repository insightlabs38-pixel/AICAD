# STAGE0-INDEPENDENT-REVIEW

Independent, adversarial Stage-0 architecture/gate review. This review was
conducted with no assumed prior conversation context; everything below was
reconstructed from the repository as it stood at the start of the review.
It supersedes nothing — it is a second, independent opinion alongside
`project/reports/AICAD-013.md` (the implementation team's own contradiction
review) and `project/gates/stage-0-gate.md` (the implementation team's own
gate packet), both of which this review re-checked rather than trusted.

## 1. Exact git revision reviewed

```text
02b89c89074ab4dee47b3a0171eafe44f531d210
```
(`main`, merged PR #1, tip of the AICAD-001..AICAD-014 batch; working tree
was clean at the start of this review.)

This report itself was written, and three patches described in §4 were
applied, on top of that revision, on branch
`claude/aicad-stage-0-review-9leull`. §7 records the post-patch re-review.

## 2. Documents reviewed

- `CLAUDE.md`, `AGENTS.md`, `AICAD_AGENT_OPERATING_MODEL.md`
- `project/CURRENT_STAGE.md`, `project/TASKS.yaml`, `project/DECISION_LOG.md`,
  `project/OWNER_DECISIONS.md` (no `project/SESSION_HANDOFF.md` existed yet)
- `rfcs/0001-language-principles.md` through `rfcs/0005-diagnostics.md`,
  `rfcs/README.md`
- `examples/assemblies/stage0_paper_example.aicad` and its companion
  `stage0_paper_example.md`
- `project/reports/AICAD-001.md` through `AICAD-014.md`,
  `project/reports/ORIENTATION_PASS.md`
- `project/gates/stage-0-gate.md`, `project/gates/stage-0-4-traceability-matrix.md`
- `specs/language/grammar.ebnf`
- `docs/plan/00_PRINCIPLES_AND_SCOPE.md`, `01_SYSTEM_ARCHITECTURE.md`,
  `03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md`, `04_HIGH_LEVEL_MODELING_API.md`,
  `06_REFERENCES_QUERIES_FEATURE_DAG.md`, `07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md`,
  `08_CONSTRAINTS_REQUIREMENTS_TESTS.md`, `20_REVIEW_PASS_GAPS_AND_DECISIONS.md`
- `git log` for AICAD-001 through AICAD-014 (commits `58f540a` through
  `22d65c3`, plus the orientation-pass and owner-ruling-recording commits)

## 3. Method

1. Read the governance chain top-down (`CLAUDE.md` -> `AGENTS.md` ->
   `CURRENT_STAGE.md` -> `TASKS.yaml` -> `DECISION_LOG.md` ->
   `OWNER_DECISIONS.md`) before opening any RFC, to establish what is
   actually frozen vs. still open, independent of what the RFCs claim.
2. Read all five RFCs in full, cross-checking every "frozen, from `NN` §x"
   citation against the cited `docs/plan/` section directly, rather than
   trusting the RFC's restatement.
3. Read the Stage-0 paper example (`stage0_paper_example.aicad`) as a
   *program*, not a narrative — walked it top to bottom checking whether
   every construct used has a defined grammar production, a defined type
   rule, and a defined semantic-reference/mutation rule, rather than
   checking only "does this look like what the RFC prose describes"
   (which is what AICAD-013's review did; see §8).
4. Constructed the adversarial mini-programs in §6 to probe the specific
   combinations the task brief calls out (chained ops, rebind-vs-mutation,
   mixed units, invalid dimensional arithmetic, absolute/delta
   temperature, loops/conditionals producing geometry, semantic query
   followed by upstream edit, ambiguous reference, raw/unsafe topology,
   mixed-level geometry).
5. Re-derived `project/OWNER_DECISIONS.md`'s open/resolved status
   independently from `DECISION_LOG.md`'s entries, rather than trusting
   the quick-index table, then checked every RFC clause claiming
   "resolved" against an actual `DL-n` entry.
6. Where a genuine gap was found, classified it (BLOCKER/MAJOR/MINOR/NOTE),
   checked whether patching it would require choosing between unresolved
   architecture alternatives (if so: do not patch, escalate to
   `OWNER_DECISIONS.md`) or would only make an already-approved ruling
   internally consistent/complete (if so: patch directly, per the task's
   "PASS WITH PATCHES" instructions).

## 4. Findings

### F1 — MAJOR (patched). Frozen grammar sketch never defined `if`/`match` in expression position, contradicting existing precedent the Stage-0 gate itself relies on.

`rfcs/0001-language-principles.md` §7 (and the identical
`specs/language/grammar.ebnf`) defined `if_stmt`/`match_stmt` only as
members of `statement`, and `expression`'s alternatives were
`call_expr | method_call_expr | binary_expr | literal | identifier |
"(" expression ")" | block_expr` — with `block_expr` referenced but never
defined anywhere, and no `if`/`match` alternative under `expression` at
all.

This directly conflicts with:
- `docs/plan/07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md` §10's own
  canonical, already-frozen-by-inclusion example: `let wall = match
  Product.material { Plastic => 3mm, Aluminum => 2mm, };`
- The Stage-0 paper example itself: `let wall = if Product.motor == NEMA17
  { 3mm } else { 4mm };` (`stage0_paper_example.aicad`, the "conditional"
  construct the Stage-0 exit gate explicitly requires).

Both require `if`/`match` to produce a value in a `let` binding. Under the
grammar as originally drafted, neither program actually parses: `if`/
`match` were statement-only, and no expression alternative could produce
their value. AICAD-013's review (`project/reports/AICAD-013.md`) checked
the example against RFC-0001's *prose* (mutation semantics, unit
consistency, syntax family) but did not check it against RFC-0001's own
*grammar productions* nonterminal-by-nonterminal, which is how this gap
survived that review. This is exactly the class of issue the task brief
calls out under item 1 ("RFC-0001 syntax can express every construct
required by the Stage-0 paper example without hidden special cases") and
item 11 ("paper example... rather than relying on syntax the RFCs never
define") — the "conditional" gate requirement specifically was not
actually satisfiable by the frozen grammar.

**Patch applied** (§4 of this list is itself the record; see §5 below for
the concrete diff): added `if_expr`, `match_expr`, and a definition of
`block_expr` to `expression`'s alternatives in both
`rfcs/0001-language-principles.md` §7 and `specs/language/grammar.ebnf`,
with the rule that `if` requires an `else` arm to be used in expression
position (an `else`-less `if` remains statement-only). This follows
directly from DL-1's "broadly Rust/TypeScript-like" ruling (Rust's `if`/
`match` are expressions) and the precedent already present, uncontested,
in `docs/plan/07` and the paper example — it does not select among any
open `OWNER_DECISIONS.md` item and required no new owner ruling.

**Residual note (NOTE, not blocking):** `match_arm`'s existing production
(`pattern => (expression "," | block)`) allows a `block`-form arm, and it
is not yet specified whether a `block`-form arm may be used inside a
`match_expr` (where every arm must yield a value) or only inside
`match_stmt`. The Stage-0 paper example does not use `match` at all (only
`if`), so this residual ambiguity does not block the Stage-0 exit gate;
it is left as a Stage-2 grammar-formalization item (AICAD-039+), not
patched here to avoid inventing grammar beyond what this review's evidence
requires.

### F2 — MAJOR (patched). RFC-0004's own frozen quantity shape had no field for the absolute/delta affine distinction its own affine-unit rule requires.

RFC-0004 §5 adopts the quantity shape from `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md`
§5 "as-is": `canonical_value, unit_dimension, preferred_display_unit,
precision/uncertainty metadata` — no absolute/delta field anywhere. Yet
RFC-0004 §7 (DL-3) requires: "The type system must make it a type error to
add two *absolute* affine quantities together... while adding an absolute
quantity and a delta quantity... is well-defined." Nothing in the frozen
material specifies *how* the type checker would ever know a given
`Temperature` value is "absolute" vs. "delta" — the only representation
that exists (§5's shape) carries no such tag. `docs/plan/03` §4's one-line
mention ("affine-temperature semantics handled carefully") does not
resolve this either. This is a genuine internal inconsistency *within
RFC-0004 itself* (§5 vs. §7), not merely an unimplemented detail — as
drafted, §7's invariant was unimplementable against §5's own frozen shape.

A second, related gap: the RFC stated `absolute + absolute` is a type
error but never stated what `absolute - absolute` produces. Without that
rule there was no way to *construct* a delta value at all under the frozen
rules — an author could never legally obtain the very "delta quantity"
type §7 talks about adding to an absolute.

This is exactly what the task brief's check item 9 ("RFC-0004 does not
accidentally treat affine units as ordinary scaled quantities") and the
"absolute and delta temperatures" adversarial probe are designed to catch:
the *policy* (§7's prose) correctly avoids treating affine units as
ordinary scaled quantities, but the *mechanism* to enforce that policy was
absent from the RFC's own frozen data shape.

**Patch applied:** added an `affine_kind: absolute | delta` discriminant
to the quantity shape (RFC-0004 §5), cross-referenced from §7, plus an
explicit subtraction rule (`absolute - absolute -> delta`,
`absolute - delta -> absolute`, `delta - delta -> delta`). This
operationalizes DL-3's already-approved invariant; it does not choose a
concrete type-name/surface-syntax encoding (RFC-0004 §7 already correctly
deferred that specific choice to Stage 2), so it does not require a new
owner decision.

### F3 — MINOR (patched). Two open-decision-dependent constructs in the paper example carried their caveat only in the companion `.md`, not inline in the `.aicad` source itself.

AICAD-012/AICAD-013 correctly identified and did not silently resolve two
gaps:
- `r.left_edge`/`r.right_edge` field access on a `rectangle(...)`'s
  `Profile` result — a plausible reading of open decision D3 (sketch
  entity/object model), not a documented API.
- `mate concentric(motor.output_axis, bracket.mount_axis)` — passing a
  `Datum` (the return type of `datum_axis(...)`) where the plan documents
  the mate as taking "axes/cylinders," with no stated coercion rule.

Both caveats existed only in `stage0_paper_example.md`'s prose. The
`.aicad` file itself — the artifact most likely to be copy-pasted as a
worked reference by whoever starts Stage 1-3 implementation — carried no
signal at the point of use. This creates a real risk of these two
D3-dependent/undocumented-coercion constructs becoming a de-facto frozen
API through repeated reuse, exactly the "accidental implementation
commitment" class of risk this review was asked to look for, even though
the *documents* never claim to have resolved them.

**Patch applied:** added inline comments at both lines in
`stage0_paper_example.aicad` itself, stating plainly that each construct
is a plausible-but-unverified reading pending D3 / an undocumented
coercion rule, and pointing to the `.md` file for the full note. This is a
documentation-only change; it resolves nothing and does not touch
`OWNER_DECISIONS.md`.

### F4 — NOTE (not patched, no action required at Stage 0).

`body.cut(hole_a);` as a bare statement (result discarded, no rebind) is
unambiguous under RFC-0001 §5 — the RFC is explicit that this is inert,
not an error. However, no diagnostic-schema item in RFC-0005 (§2's code
families) yet earmarks a warning class for "expression-statement result of
a geometry-bearing type discarded" — exactly the kind of silent-no-op
footgun the language's own AI-learnability goals (`docs/plan/00` §4.2, §10
quality-bar question 1: "Can a human understand it without excessive
ceremony?") would benefit from catching early. This is not a Stage-0 gate
requirement (RFC-0005 is explicit that only the taxonomy *shape* is frozen,
not the concrete code registry — that is `crates/cad-diagnostics`,
Stage 2, AICAD-038) and is recorded here only as a recommendation for
whoever designs the initial `RUNTIME-W###`/lint set at that time, not as a
Stage-0 blocker.

### No BLOCKER-level findings.

No finding in this review required stopping Stage 1 outright: F1 and F2
were internal completeness/consistency gaps in already-approved rulings
(DL-1/DL-2 surface syntax; DL-3 affine units), not disputes over what was
decided, and were resolvable without touching any open
`OWNER_DECISIONS.md` item. F3 is documentation-only. F4 is a forward-looking
recommendation, not a defect.

## 5. Files patched (this review)

- `rfcs/0001-language-principles.md` — added `if_expr`/`match_expr`/
  `block_expr` grammar productions and an explanatory note (F1).
- `specs/language/grammar.ebnf` — mirrored the same grammar addition (F1).
- `rfcs/0004-units-type-system.md` — added the `affine_kind` discriminant
  to the quantity shape and the absolute/delta subtraction rule (F2).
- `examples/assemblies/stage0_paper_example.aicad` — added two inline
  caveats at the D3-dependent and coercion-dependent lines (F3).

No file under `project/OWNER_DECISIONS.md`, `project/DECISION_LOG.md`, or
`project/CURRENT_STAGE.md` was touched — no stage gate, benchmark, or
owner-decision status was altered by this review.

## 6. Adversarial paper-example probes performed

Each probe below is a short mental program checked against the frozen
RFCs/plan (post-patch, unless noted), with the result. Only probes that
surfaced a finding are cited above by Fn; the rest are recorded here as
negative results (checked, no issue found) for audit completeness.

1. **Chained modeling operations, no rebind:**
   `body.cut(hole_a).cut(hole_b);` as a bare statement.
   -> RFC-0001 §5: desugars to nested functional calls; the outer result is
   an unused expression-statement result, discarded; `body` is untouched.
   Unambiguous. (See F4 for a non-blocking recommendation.)
2. **Rebinding vs. apparent mutation:**
   `var body = base; body.cut(hole);` (no rebind) followed later by
   `body = body.cut(hole);` (rebind). -> Unambiguous per RFC-0001 §5; the
   paper example itself demonstrates the correct pattern. No issue.
3. **Mixed engineering units:** `let total: Length = 5mm + 2cm;` -> RFC-0004
   §5: same-dimension implicit conversion, type-checks, canonical value is
   dimension-correct regardless of literal unit. No issue.
4. **Invalid dimensional arithmetic:** `let x: Length = 5kg;` and
   `let y = 5mm + 3s;` -> RFC-0004 §5: both are type errors, no numeric
   escape hatch exists. No issue.
5. **Absolute and delta temperatures:**
   `let boiling: Temperature = 100degC; let freezing: Temperature = 0degC;
   let diff = boiling - freezing; let x = diff + freezing;` -> Before the
   patch, `diff`'s type/kind and the legality of `diff + freezing` were
   undecidable from the frozen text (F2). After the patch: `diff` is
   `affine_kind: delta`; `diff + freezing` (`delta + absolute`) is
   well-defined per §7's already-existing rule; `boiling + freezing`
   (`absolute + absolute`) remains a type error. Resolved by F2's patch.
6. **Loops creating geometry:** the paper example's
   `for p in corner_points(...) { body = body.cut(...); }` -> unambiguous:
   ordinary value iteration, explicit rebind each iteration, consistent
   with RFC-0001 §5 and RFC-0002's functional value model. No issue.
7. **Conditionals producing geometry:**
   `let wall = if Product.motor == NEMA17 { 3mm } else { 4mm };` -> this
   is F1; before the patch the grammar did not actually admit this
   program. Resolved by F1's patch.
8. **Semantic query followed by upstream edit:**
   ```
   let f = query body.faces { planar; largest(area); };
   body = body.cut(hole);
   // does `f` still refer to the pre-cut body's face?
   ```
   -> `body` is an ordinary immutable value (RFC-0001 §5); `query body.faces
   {...}` is evaluated against the specific value `body` was bound to at
   that point, not a live handle to the variable `body`. Rebinding `body`
   afterward does not retroactively change what `f` resolved against. This
   is ordinary value-semantics, not new semantics — no issue found, though
   it is worth Stage-3/4 documenting this explicitly in `cad-references`'
   own docs (not a Stage-0 gap; RFC-0001 §5's value semantics already
   settle it).
9. **Ambiguous semantic reference:**
   `expose top = query body.faces { planar; };` where two faces are
   equally planar and no `largest(...)`/`unique()` disambiguator is given.
   -> RFC-0003 §6: resolves to `Ambiguous(candidates, evidence)`, never an
   arbitrary pick. Unambiguous, fail-closed as required. No issue.
10. **Raw/unsafe topology access:** the paper example's
    `unsafe geometry { let raw: KernelShape* = raw_shape(hook); ... hook =
    adopt_validated(healed, ...); }` -> matches RFC-0002 §4's promotion
    rule (`adopt_validated`/`validate()` required before leaving the
    block). One minor observation: RFC-0002 §4 defines the *promotion*
    direction (raw -> validated) precisely but does not name/define the
    *entry* function (`raw_shape`, safe value -> raw handle) used by the
    example; this is plausibly Stage-1 low-level-API detail
    (`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`) rather than a
    Stage-0 RFC-0002 gap, and does not affect the safety invariant itself
    (the raw handle is still epoch-local and still requires validation
    before promotion either way). Recorded as a NOTE for Stage 1
    (`cad-kernel-api`/`cad-occt-bridge`, AICAD-016..019) to make explicit,
    not a Stage-0 blocker.
11. **Ordinary high-level and low-level geometry in one module:** the
    paper example's `Bracket` (sketch/extrude/hole, high-level) and
    `retaining_hook` (bspline/sweep/unsafe geometry, low-level), combined
    via `union(...)` inside `Bracket`. -> Directly demonstrates
    `docs/plan/00` §7's "both levels can be mixed inside one function and
    one part" invariant; consistent throughout. No issue.

## 7. Re-review of the patched state

After applying F1-F3's patches:

- Re-read `rfcs/0001-language-principles.md` §7 and
  `specs/language/grammar.ebnf` end to end: `if_expr`/`match_expr`/
  `block_expr` are self-consistent with the pre-existing productions
  (`fn_decl` still uses the narrower `block`, not `block_expr`; `if_stmt`
  is untouched and still uses `block`; `match_arm` is shared unchanged
  between `match_stmt` and the new `match_expr`). The Stage-0 paper
  example's `let wall = if ... else ...;` now parses under the stated
  grammar.
- Re-read `rfcs/0004-units-type-system.md` §5-7 end to end: the
  `affine_kind` discriminant and the subtraction rule make §7's invariant
  representable by §5's own shape; no other RFC section references the
  quantity shape in a way this patch could have broken (checked via
  `grep -n "canonical_value\|quantity" rfcs/*.md`).
- Re-read `stage0_paper_example.aicad` end to end after the two inline
  comments: no line length/comment placement interferes with any other
  construct; the file remains a paper example only (still not claimed as
  compilable, per its own header).
- Re-ran the two required workspace checks fresh against the patched tree:
  `cargo fmt --all -- --check` (exit 0) and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  (exit 0, zero warnings) — no Rust source was touched by these patches,
  so this confirms only that the patches did not disturb the toolchain
  baseline, not that they were type-checked (no parser exists yet, as
  `project/CURRENT_STAGE.md` disallows at Stage 0).
- Re-ran all eleven adversarial probes in §6 against the patched state:
  all eleven now resolve unambiguously (probes 5 and 7, previously
  blocked on F1/F2, now resolve as shown).
- Re-checked `project/OWNER_DECISIONS.md`'s quick-index table against
  `project/DECISION_LOG.md` once more: unchanged by this review's patches
  (as expected — none of F1-F3 touched an owner-decision item). D3, D5,
  D10, D11, D12, D15 remain open and are not frozen, implicitly or
  explicitly, by any patch applied here.

No new issue was introduced by the patches; no patch required choosing
among an open architecture alternative.

## 8. Unresolved owner decisions encountered

No new owner decision is required by this review. The following remain
open exactly as `project/OWNER_DECISIONS.md` already states, and none
blocks the Stage-0 exit gate (confirmed independently, not merely taken on
the implementation team's word — see the per-decision blocking-impact
notes already in `OWNER_DECISIONS.md`, which this review's own findings do
not contradict):

- **D3** (sketch entity/object model) — directly touched by this review's
  F3 (the `r.left_edge` inline caveat makes the paper example's reliance on
  D3 more visible, it does not resolve D3).
- **D5** (determinism-equivalence contract), **D10** (diagnostic stability
  policy), **D11** (constraint IR/solver-independence), **D12** (trusted
  native plugin boundary), **D15** (plugin runtime) — reviewed, none
  touched by any Stage-0 artifact in a way that silently freezes them
  (confirmed by direct re-reading of each; e.g. the paper example's
  `constraint {...}`/`mate`/`symmetric(...)` blocks declare constraints
  syntactically without relying on any particular solver's observable
  behavior, so D11 is not implicitly frozen).
- **D7, D8, D13** — partially resolved per DL-8/DL-9/DL-6; their residual
  open sub-items (automatic fingerprint-recovery policy; extent of
  internal OCAF usage; formal distribution/license review) are unaffected
  by this review.

## 9. Assumptions this review had to make

- Treated `specs/language/grammar.ebnf` and RFC-0001 §7's embedded grammar
  block as intended to be kept identical (both are explicitly described as
  the same sketch, one "seeding" the other); patched both in lockstep
  rather than only one, since no document states one is authoritative over
  the other at Stage 0.
- Treated the absence of a `project/SESSION_HANDOFF.md` file as meaning no
  prior session left an in-progress handoff note, not as an omission to
  flag — `AGENTS.md`/`CURRENT_STAGE.md` do not require one to exist at
  Stage 0, and the task brief says "if present."
- Treated "the paper example is a paper example, not compilable source"
  (stated in the `.aicad` file's own header and reconfirmed by
  `project/CURRENT_STAGE.md`'s "Not allowed yet: final parser
  implementation") as license to evaluate it against the *grammar
  productions and type rules stated in the RFCs*, rather than against any
  actual parser/compiler — there is none to run. F1 was found this way:
  by manual grammar derivation, not by attempting to parse the file with
  tooling.
- Did not treat `docs/plan/`'s own illustrative code snippets (e.g. `07`
  §10's `match`, `08` §3's `constraint { }`) as themselves "frozen" —
  they were used only as *evidence of pre-existing, uncontested precedent*
  when checking whether an RFC's omission (F1) was a deliberate policy
  choice or an oversight; the actual freeze authority remains the RFCs
  plus `DECISION_LOG.md`, per `AGENTS.md`.

## 10. Final recommendation

**PASS** (after patches and this second review).

Rationale: three concrete, evidence-backed gaps were found (F1, F2 —
MAJOR; F3 — MINOR) by treating the RFCs and paper example as programs to
be mechanically derived rather than prose to be read charitably, as the
task brief required. All three were patchable without selecting among any
open `project/OWNER_DECISIONS.md` alternative — each patch operationalizes
an already-approved owner ruling (DL-1/DL-2 for F1, DL-3 for F2) or is
purely documentation-clarity (F3) — so none required a new
`OWNER_DECISIONS.md` entry or a "DO NOT PASS pending owner input" outcome.
After applying the patches, §7's re-review found the patched RFCs/example
internally consistent, found no new issue, and confirmed all eleven
adversarial probes in §6 now resolve unambiguously, including the two
(absolute/delta temperature; conditional-as-expression) that were
previously blocked. No BLOCKER was found at any point in this review. The
remaining open `OWNER_DECISIONS.md` items (§8) are correctly tracked, not
silently resolved, and do not block Stage 1 per their own recorded
blocking-impact notes, independently re-confirmed here rather than taken
on trust from `project/gates/stage-0-gate.md`.

This review's own patches (§5) still require the same owner sign-off as
the rest of Stage 0 — this document recommends; it does not approve. Per
`AGENTS.md` ("Stage gates: ... The agent may not approve a roadmap stage")
and `CURRENT_STAGE.md` ("Owner approval required to advance: Yes"), Stage 1
(`AICAD-015` onward) must not begin until the owner records a Stage-0 pass
decision in `project/DECISION_LOG.md`. AICAD-015 was not started by this
review, per the task brief's explicit instruction.
