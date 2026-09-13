# AICAD-013 — Run contradiction review across RFCs and paper example

## Objective
Run a contradiction review across RFC-0001 through RFC-0005 and the
Stage-0 paper/spec example, and confirm (or refute) the Stage-0 exit gate:
"a realistic example ... can be written without semantic contradiction"
(`docs/plan/15_IMPLEMENTATION_ROADMAP.md`), per `project/TASKS.yaml`
(AICAD-013). This is a review task: no product feature is implemented; it
either finds problems (fixed if trivial/non-architectural, escalated if
not) or confirms none exist.

## Dependencies checked
AICAD-012 (Stage-0 paper example) — complete, see
`project/reports/AICAD-012.md`. All five RFCs (AICAD-007..011) and the
paper example (AICAD-012) exist and are the review's actual subject.

## Review method

1. **Cross-RFC terminology/naming check**: grepped every RFC and the
   example for file-extension mentions (`.cadl`, `.aicad`, `.aicadpkg`,
   `aicad.toml`), `unsafe geometry` usage, and durability-tier naming, to
   confirm DL-4's naming ruling is applied identically everywhere it is
   invoked rather than re-derived inconsistently per RFC.
2. **Cross-RFC dependency check**: confirmed each RFC's stated "Depends
   on" line matches what it actually cites from earlier RFCs (RFC-0002 on
   RFC-0001's mutation semantics; RFC-0003 on RFC-0002's epoch-local
   kernel-handle ruling; RFC-0004 on RFC-0001's literal syntax; RFC-0005 on
   RFC-0002's kernel-independence contract).
3. **RFC-vs-plan-section check**: for every "frozen, from `NN` §x" citation
   across all five RFCs, re-opened the cited `docs/plan/` section and
   confirmed the RFC text is a faithful restatement, not a drift.
4. **Example-vs-RFC compliance check**: re-read `stage0_paper_example.aicad`
   line by line against RFC-0001's syntax/mutation rules and the concept
   table in `stage0_paper_example.md`, checking every unit expression for
   dimensional consistency under RFC-0004 §5 and every semantic-reference
   usage against RFC-0003.
5. **Owner-ruling completeness check**: re-confirmed that no RFC clause
   states something as settled that is actually still `open` in
   `project/OWNER_DECISIONS.md` (D3, D5, D10, D11, D12, D15), and that
   every clause that *is* stated as settled cites a real `DECISION_LOG.md`
   entry or an uncontested plan section.

## Findings

### Fixed during this review (non-architectural, in scope to fix directly)

1. **RFC-0002 §5 lockfile naming was internally confused.** It read
   "the lockfile (`cad.lock`/`aicad.toml`'s referenced lock data, per
   RFC-0001 §3 naming)" — presenting two names ambiguously and incorrectly
   implying DL-4 had frozen a lockfile name (it only froze the manifest
   name, `aicad.toml`). **Fixed**: rewrote the bullet to state plainly that
   DL-4 froze only the manifest name, the plan's own placeholder lockfile
   name is `cad.lock`, and harmonizing it to `aicad.lock` is a natural but
   **not yet frozen** choice, left open until `crates/cad-packages`/
   `crates/cad-cli` need it. This is a documentation-clarity fix, not a new
   architecture decision — no escalation triggered.
2. **The paper example used `Product.motor == MotorSize.NEMA17`**, a
   dot-qualified enum-variant equality form not shown anywhere in the
   plan (the plan's own examples, `docs/plan/07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md`
   §10 and `docs/plan/18_REFERENCE_EXAMPLES.md` Example 2, always compare
   against a bare variant name inside `match`, e.g. `NEMA17 =>`).
   **Fixed**: changed to `if Product.motor == NEMA17 { ... } else { ... }`,
   matching the plan's own precedent exactly instead of inventing an
   unverified qualification syntax.

### Found and explicitly flagged, not resolved (genuine open gaps, correctly left open)

3. **`Datum` vs. axis-reference coercion.** `datum_axis(...)` returns
   `Datum` (`docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3) while `mate
   concentric(a, b)` is documented as taking "axes/cylinders"
   (`docs/plan/07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md` §5), with no
   stated coercion rule. Already noted in `stage0_paper_example.md` from
   AICAD-012; reconfirmed during this review as a genuine plan gap, not a
   contradiction the example introduces (the plan's own `18_REFERENCE_EXAMPLES.md`
   Example 5 shows the same informality). Left open — resolving it would
   be a type-system design decision, and it does not currently block any
   Stage 0 task.
4. **Rectangle sub-entity field access (`r.left_edge`, `r.right_edge`).**
   Found during this review: nothing in
   `docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3-4 documents `Profile`
   exposing named edge fields. This falls squarely inside the already-open
   `project/OWNER_DECISIONS.md` D3 (sketch entity/object model) — it is an
   illustration of that exact open question, not a new one. Added a
   dedicated note to `stage0_paper_example.md`'s "Contradiction-review
   notes" section explaining this, and explicitly stating it must not be
   read as resolving D3. Considered rewriting the sketch to use
   explicitly-named `line(...)` calls instead, but that would only
   relocate the same open question (how independently-drawn lines close
   into one profile) rather than resolve it, at the cost of a
   significantly longer example — not worth it for a paper example whose
   job is to demonstrate concept coverage, not to prejudge D3.

### No other issues found

- No RFC states a still-`open` `project/OWNER_DECISIONS.md` item as
  settled.
- No RFC-to-plan-section citation was found to drift from what the cited
  section actually says.
- No RFC's "Depends on" line was found to be inaccurate.
- No dimensional inconsistency was found in the paper example's unit
  expressions.
- No leftover `.cadl` references exist outside RFC-0005 §3's one,
  explicitly-labeled before/after comparison (intentional, not a leftover).

## Conclusion

**The Stage-0 exit gate is met**: the paper/spec example, built entirely
from RFC-0001 through RFC-0005, contains parameters, a function, a loop, a
conditional, a sketch, an extrusion, a semantic query, low-level geometry,
an assembly, a constraint (in all three domains the plan names), and a
test — and after this review's two fixes, **no semantic contradiction was
found** between the RFCs, or between the RFCs and the example. Two
pre-existing plan-level gaps (findings 3-4) remain, correctly recorded as
open rather than silently resolved, consistent with `AGENTS.md`'s rule
against letting ambiguous decisions select silently.

This finding — pass, with two fixes applied and two gaps recorded — is
carried into AICAD-014's Stage-0 gate packet as the evidence for this exit
gate specifically.

No escalation condition was triggered: both fixes made here are
non-architectural documentation/example corrections (a naming
clarification and reverting an invented syntax to the plan's own
precedent), not changes to public syntax/semantics, a gate/benchmark, a
kernel type boundary, or a reference-resolution rule; both flagged gaps
were left open rather than resolved.

## Files changed
- Edited: `rfcs/0002-geometry-runtime-kernel-abstraction.md` (§5 lockfile
  naming clarification).
- Edited: `examples/assemblies/stage0_paper_example.aicad` (reverted
  `MotorSize.NEMA17` to bare `NEMA17`, matching plan precedent).
- Edited: `examples/assemblies/stage0_paper_example.md` (added the
  rectangle-sub-entity-field-access note).

## Verification (exact commands/results)
```
$ grep -rn "\.cadl" rfcs/*.md examples/assemblies/stage0_paper_example.aicad examples/assemblies/stage0_paper_example.md
rfcs/0005-diagnostics.md:60:(The "file" example is updated from the plan's `.cadl` placeholder to the
(only the one intentional, explained mention remains)

$ grep -rn "TODO\|FIXME\|XXX" rfcs/*.md examples/assemblies/stage0_paper_example.aicad examples/assemblies/stage0_paper_example.md
(no output)

$ grep -n "MotorSize.NEMA17" examples/assemblies/stage0_paper_example.aicad
(no output — confirms the fix was applied)

$ cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
(both exit 0 — no Rust source touched by this task)
```

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific check: the five-point review method above was carried out
  in full; findings and fixes are listed above with commands confirming
  each fix landed.

## Limitations / follow-up
- Findings 3 and 4 remain open plan-level gaps. Finding 3 (Datum/axis
  coercion) is not yet in `project/OWNER_DECISIONS.md` as its own entry
  since it does not currently block a task; finding 4 (rectangle
  sub-entity access) is already covered by existing entry D3 and needs no
  new entry.
- This review covers RFC-0001 through RFC-0005 and the one paper example
  that exists at Stage 0. It does not re-review `docs/plan/` itself (that
  was `project/reports/ORIENTATION_PASS.md`'s job) or anticipate
  contradictions in RFCs/examples that do not yet exist (Stage 1+).
