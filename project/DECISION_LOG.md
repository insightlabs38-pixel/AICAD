# AICAD Decision Log

Record owner-approved architectural/product decisions here with date, rationale,
alternatives considered, affected RFCs/tasks, and superseded decisions.

Entries move here from `project/OWNER_DECISIONS.md` once the owner rules on
them; do not add entries here unilaterally.

## Format

```text
## DL-<n>: <short title>

- Date:
- Resolves: OWNER_DECISIONS#<id>
- Decision:
- Rationale:
- Alternatives considered:
- Affected RFCs/tasks:
- Supersedes:
```

---

## DL-1: Canonical surface syntax family

- Date: 2026-09-09
- Resolves: OWNER_DECISIONS#D1
- Decision: Canonical AICAD syntax uses braces for blocks and explicit
  semicolons for statements. Broadly Rust/TypeScript-like without
  attempting source compatibility with either language. Indentation is
  formatting only, never syntax. Automatic semicolon insertion is not part
  of the initial language.
- Rationale: Owner ruling. Braces/semicolons make block and statement
  boundaries explicit and unambiguous for both human readers and
  general-purpose coding models parsing/generating source without a
  language-specific formatter in the loop; avoiding ASI removes a class of
  parser/generator ambiguity that would otherwise need its own
  disambiguation rule.
- Alternatives considered: Python-style significant indentation (rejected
  — `docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §4.1 listed it as an
  option, but it complicates nested nested nested geometry/query blocks and
  machine-generated diffs).
- Affected RFCs/tasks: RFC-0001 (AICAD-007); `specs/language/grammar.ebnf`;
  `crates/cad-lexer`, `crates/cad-parser`, `crates/cad-ast`;
  `tree-sitter-aicad/`.
- Supersedes: none (first ruling on D1).

---

## DL-2: Mutation semantics — functional core, method syntax as sugar

- Date: 2026-09-09
- Resolves: OWNER_DECISIONS#D2
- Decision: AICAD's semantic core is functional and value-oriented.
  Geometry/modeling operations consume semantic values and produce new
  semantic values. Method/builder syntax (`body.cut(hole)`) is ergonomic
  sugar that lowers to ordinary functional calls in HIR and never implies
  in-place mutation — `body.cut(hole);` does not rebind `body`.
  Mutation-like workflows require explicit rebinding (`body = body.cut(hole);`)
  or builder/chaining syntax whose HIR remains functional. HIR/Geometry IR
  exposes only the functional/SSA form; builder surface syntax never
  reaches those layers.
- Rationale: Owner ruling. A single functional core keeps the feature DAG
  (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §9) and incremental
  invalidation model exact and auditable regardless of which surface style
  an author or an AI model prefers, and avoids method syntax silently
  implying aliasing/mutation semantics that would need separate ownership
  rules.
- Alternatives considered: Two independently-meaningful surface forms with
  overlapping but distinct semantics (rejected — the plan itself warned
  this "can harm style consistency,"
  `docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §4.2); true in-place
  mutation of geometry values (rejected — conflicts with copy-on-write /
  value-semantics description in `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md`
  §13).
- Affected RFCs/tasks: RFC-0001 (AICAD-007), RFC-0002 (AICAD-008);
  `crates/cad-hir`, `crates/cad-geometry-api`; HIR lowering task AICAD-051;
  grammar tasks AICAD-039/041.
- Supersedes: none (first ruling on D2).

---

## DL-3: Type/units semantics specifics

- Date: 2026-09-09
- Resolves: OWNER_DECISIONS#D4
- Decision:
  - Dimensions have a unique canonical internal representation independent
    of user-selected display units.
  - Quantities of the same dimension may implicitly convert for arithmetic
    and comparison; different physical dimensions never implicitly
    convert.
  - `Tolerance<T>` is initially defined by conservative interval
    semantics; tolerance arithmetic propagates interval bounds.
    Statistical/RSS tolerance semantics require an explicit, separate,
    later API and are never assumed by default.
  - Affine units (e.g. Celsius) distinguish absolute quantities from delta
    quantities and do not use ordinary scale-only conversion rules.
  - Derived dimensions are canonicalized structurally (by dimension
    exponents), not by unit spelling.
- Rationale: Owner ruling. Structural canonicalization plus same-dimension
  implicit conversion gives ordinary engineering ergonomics
  (`5mm + 2cm` just works) while cross-dimension conversion staying
  forbidden preserves the type-safety invariant in
  `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §3 rule 5. Conservative interval
  tolerance arithmetic avoids silently understating worst-case stacks —
  the more permissive statistical/RSS treatment is opt-in only, consistent
  with never silently weakening a validation guarantee.
- Alternatives considered: Explicit conversion required even within one
  dimension (rejected as excessive ceremony for ordinary unit mixing);
  defaulting tolerance arithmetic to RSS/statistical composition (rejected
  — would silently understate worst-case bounds unless a user opts in).
- Affected RFCs/tasks: RFC-0004 (AICAD-010); `crates/cad-units`
  (AICAD-047-049); `crates/cad-types`.
- Supersedes: none (first ruling on D4).

---

## DL-4: File extension / project branding

- Date: 2026-09-09
- Resolves: OWNER_DECISIONS#D14
- Decision: Working product/language name is **AICAD**. Canonical source
  extension is **`.aicad`**. Canonical project manifest is **`aicad.toml`**.
  If a single-file packaged/archive bundle format is needed, it uses a
  distinct extension (**`.aicadpkg`**) rather than overloading `.aicad`.
  `CAD-IR` may remain an internal compiler/representation name but is not
  the product brand.
- Rationale: Owner ruling. Using one extension (`.aicad`) for source
  removes the module-vs-bundle ambiguity flagged in
  `project/reports/ORIENTATION_PASS.md` §6 (contradiction 2) between
  `docs/plan/02_LANGUAGE_AND_COMPILER.md` §2 and
  `docs/plan/09_FILE_FORMAT_INTERCHANGE_RECONSTRUCTION.md` §1; giving the
  packaged/archive bundle its own extension (`.aicadpkg`) avoids the same
  ambiguity recurring at the tooling layer (a file picker or `cad build`
  invocation must never have to guess whether a `.aicad` path is a source
  module or an archive).
- Alternatives considered: `.cadl` for source / `.aicad` for bundles (the
  plan's own alternate proposal — rejected in favor of the product name
  itself being the source extension); overloading `.aicad` for both source
  and bundle "if ambiguity is undesirable" (rejected — that phrasing from
  `docs/plan/02_LANGUAGE_AND_COMPILER.md` §2 is exactly the ambiguity being
  resolved here).
- Affected RFCs/tasks: RFC-0001 (AICAD-007); `crates/cad-artifact`,
  `crates/cad-cli`; repository `README.md`; `examples/*` file naming going
  forward.
- Supersedes: none (first ruling on D14).

---

## DL-5: Kernel abstraction boundary and lineage exposure

- Date: 2026-09-09
- Resolves: OWNER_DECISIONS#D6
- Decision: The public kernel abstraction (`cad-kernel-api`) is
  kernel-neutral and exposes only domain-level geometry operations, opaque
  topology handles, validation, properties, traversal, interchange, and
  operation-local lineage. No OCCT class/type may cross the
  `cad-occt-bridge` boundary. The operation set is intentionally minimal
  and capability-driven rather than a comprehensive OCCT wrapper. For
  topology-changing operations the adapter exposes available
  created/modified/deleted/split/merge lineage evidence. Kernel topology
  handles are build/epoch-local and are not persistent AICAD semantic
  references; the semantic-reference layer owns durable identity above the
  kernel.
- Rationale: Owner ruling. Directly operationalizes the kernel-independence
  contract already stated in `docs/plan/01_SYSTEM_ARCHITECTURE.md` §8 and
  the raw-handle/epoch safety model in
  `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §8 — this decision fixes *how*
  those contracts are enforced at the crate boundary (capability-driven
  minimal surface, lineage-evidence-not-lineage-storage at the kernel
  layer) rather than changing what they require.
- Alternatives considered: A comprehensive/near-total OCCT wrapper exposed
  through `cad-kernel-api` (rejected — directly violates the
  kernel-independence contract and would make later kernel-backend
  swaps infeasible); persisting lineage inside the kernel adapter itself
  (rejected — lineage persistence belongs to the semantic-reference layer,
  per D8 below, not the kernel boundary).
- Affected RFCs/tasks: RFC-0002 (AICAD-008); `crates/cad-kernel-api`,
  `crates/cad-occt-bridge`, `native/occt_bridge` (Stage 1: AICAD-016/017/018);
  lineage capture tasks AICAD-086/087 (Stage 4).
- Supersedes: none (first ruling on D6).

---

## DL-6: OCCT and standards-derived content — development-phase licensing policy

- Date: 2026-09-09
- Resolves: OWNER_DECISIONS#D13 (development-phase policy only)
- Decision: OCCT remains an explicitly identified third-party dependency
  behind the native kernel adapter. AICAD will preserve applicable OCCT
  licensing/notices and will not assume OCCT source may be incorporated
  into AICAD-owned source. The initial architecture must permit compliant
  dynamic/shared-library distribution and must not depend on copying OCCT
  implementation code into AICAD. Standards-derived functionality
  (GD&T/AP242/etc.) must be independently implemented; ISO/ASME
  copyrighted prose, tables, figures, or other protected standards content
  must not be copied into source, documentation, or fixtures without an
  appropriate license. **A formal distribution/license review is required
  before a public binary/commercial distribution policy is frozen** — this
  decision covers development only, and that review remains a separate,
  future, open gate (tracked in `project/OWNER_DECISIONS.md`).
- Rationale: Owner ruling. Unblocks Stage-1 native-bridge work
  (AICAD-015/016) without pretending the broader commercial-distribution
  licensing question is closed; treats OCCT strictly as an external
  dependency (dynamic linkage, preserved notices) rather than absorbed
  code, which is the safe default regardless of which license review
  outcome eventually applies.
- Alternatives considered: Deferring all native-bridge work until a full
  legal review completes (rejected — would block Stage 1 entirely for a
  question that a conservative dynamic-linkage-plus-notice-preservation
  policy already answers safely for development purposes); vendoring/
  statically incorporating OCCT source into the AICAD tree (rejected
  outright by this ruling).
- Affected RFCs/tasks: `native/occt_bridge` (AICAD-015/016);
  `crates/cad-occt-bridge`; future `NOTICE`/third-party-license file
  (tracked as follow-up, not yet created).
- Supersedes: none (first ruling on D13; the public-distribution licensing
  question remains open and is not superseded).

---

## DL-7: Compiler intrinsics require an RFC

- Date: 2026-09-09
- Resolves: OWNER_DECISIONS#D9
- Decision: Adding a new compiler intrinsic requires an RFC. The RFC must
  demonstrate the capability cannot reasonably be implemented as (1)
  ordinary AICAD source, (2) a standard package, or (3) a kernel API
  operation exposed through existing language mechanisms. Each intrinsic
  must document semantics, type rules, deterministic behavior, IR
  lowering, and why a library solution is inadequate. No intrinsic may be
  introduced solely as an implementation convenience.
- Rationale: Owner ruling. Converts the qualitative "could this be a
  library?" test in `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §10 into an
  enforceable process, directly satisfying the AGENTS.md non-negotiable
  "Prefer library/std-package features over new compiler intrinsics unless
  an approved RFC says otherwise" and its escalation trigger "add a
  compiler intrinsic where a library solution may work."
- Alternatives considered: Leaving the boundary as an informal design
  guideline with no enforcement mechanism (rejected — provides no actual
  gate against intrinsic creep as the standard library grows in Stage 3+).
- Affected RFCs/tasks: RFC-0001 (AICAD-007) should record this process;
  applies to every future task proposing a new compiler intrinsic,
  starting with Stage 2 (`crates/cad-compiler`, `crates/cad-runtime`).
- Supersedes: none (first ruling on D9).

---

## DL-8: Semantic-reference resolution — fail-closed policy (partial)

- Date: 2026-09-09
- Resolves: OWNER_DECISIONS#D7 (partial — see remaining open item below)
- Decision: AICAD reference resolution is fail-closed. Resolution may
  produce only `Resolved(exactly one entity)`, `Ambiguous(candidate set +
  evidence)`, or `Broken(reason)` — it must never select an arbitrary best
  candidate. Geometry-fingerprint matching is **disabled as an automatic
  fallback** in the first Stage-4 implementation. Fingerprinting may
  initially be used only for diagnostics, ranking candidates shown to a
  user, and experiments/benchmarking. Automatic fingerprint-based recovery
  can only be enabled later by a separate owner-approved policy supported
  by benchmark evidence showing an acceptably negligible
  silent-wrong-resolution rate.
- Rationale: Owner ruling. Directly implements the plan's own
  non-negotiable invariant that ambiguity must be an error, never an
  arbitrary selection (`docs/plan/00_PRINCIPLES_AND_SCOPE.md` §3 rule 8;
  `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §7), and gives
  AICAD-088 through AICAD-092 (Stage 4) a concrete three-outcome contract
  to implement instead of an open-ended resolution policy.
- Alternatives considered: Allowing geometry-fingerprint fallback to
  auto-resolve low-ambiguity cases from the start (rejected — this is
  exactly the "silent wrong resolution" failure class the Stage-4 hard
  gate exists to eliminate, per `docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md`
  §5).
- Affected RFCs/tasks: RFC-0003 (AICAD-009); `crates/cad-references`,
  `crates/cad-query`; directly gates AICAD-088 (resolver), AICAD-089/090
  (ambiguity/broken diagnostics), AICAD-092 (fingerprint fallback, now
  scoped to diagnostics/ranking/experiments only, never automatic
  resolution, for the first implementation).
- Supersedes: none (first ruling on D7).
- **Remaining open item (still tracked in `project/OWNER_DECISIONS.md`
  as D7):** whether/when to enable automatic fingerprint-based recovery is
  explicitly deferred to a future, separate owner decision gated on
  benchmark evidence — not resolved by this entry and not currently
  blocking any Stage 0-4 task.

---

## DL-9: Kernel-independent semantic graph is authoritative (directional)

- Date: 2026-09-09
- Resolves: OWNER_DECISIONS#D8 (directional — see remaining open item below)
- Decision: AICAD owns a kernel-independent semantic graph that is
  authoritative for language-level identity, features, dependencies, and
  durable references. OCAF may be prototyped and used internally as an
  OCCT-side persistence, labeling, or lineage aid. AICAD public semantics
  and serialized semantic identity must not depend on OCAF. Whether OCAF
  is used internally, and for which subset of responsibilities, remains
  prototype-driven.
- Rationale: Owner ruling. Matches the plan's own lean
  ("likely needs a kernel-independent semantic graph above OCAF",
  `docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §4.5) while keeping the
  kernel-independence contract (DL-5/D6) intact: serialized/public
  semantic identity can never become implicitly coupled to an
  OCCT-specific persistence mechanism, even if OCAF proves useful as an
  internal implementation detail on the OCCT side.
- Alternatives considered: Building semantic identity directly on OCAF
  (rejected — would violate the kernel-independence contract and make a
  future non-OCCT kernel backend require reinventing reference identity);
  banning OCAF outright even as an internal prototyping tool (rejected —
  forecloses a legitimate implementation-detail experiment without
  evidence either way).
- Affected RFCs/tasks: RFC-0003 (AICAD-009), RFC-0002 (AICAD-008);
  `crates/cad-references`; `project/experiments/` (OCAF-as-internal-aid
  prototyping, per its own README).
- Supersedes: none (first ruling on D8).
- **Remaining open item (still tracked in `project/OWNER_DECISIONS.md`
  as D8):** the exact extent (if any) of internal OCAF usage is
  prototype-driven and not yet decided — track experiments under
  `project/experiments/` before committing `crates/cad-occt-bridge` or
  `crates/cad-references` to a specific internal persistence mechanism.

---

## DL-10: Stage 0 passed — owner approval after independent review

- Date: 2026-09-09
- Resolves: Stage-0 exit gate (`project/CURRENT_STAGE.md`); not an
  `OWNER_DECISIONS.md` item.
- Decision: **Stage 0 has passed.** The owner has reviewed the independent
  Stage-0 adversarial review, `project/reports/reviews/STAGE0-INDEPENDENT-REVIEW.md`
  (commit `aad7267`, branch `claude/aicad-stage-0-review-9leull`, merged
  into `branch/loving-feynman-qcrzen`), and accepts its final PASS
  recommendation after the applied patches. This decision supersedes the
  "recommendation only" status both `project/gates/stage-0-gate.md`
  (AICAD-014) and the independent review document carried, and is the
  owner-recorded pass decision `AGENTS.md`/`project/CURRENT_STAGE.md`
  require before Stage 1 may begin.
- Rationale: The independent review identified and corrected three
  concrete gaps in the Stage-0 RFCs/paper example, none of which required
  selecting among an open `OWNER_DECISIONS.md` alternative:
  - F1 (MAJOR): RFC-0001's frozen grammar sketch never defined `if`/`match`
    in expression position, even though the paper example's required
    "conditional" concept and `docs/plan/07`'s own precedent both need a
    value-producing `if`/`match`. Patched by adding `if_expr`/`match_expr`/
    `block_expr` productions to `rfcs/0001-language-principles.md` §7 and
    `specs/language/grammar.ebnf`, operationalizing DL-1/DL-2 rather than
    creating new syntax.
  - F2 (MAJOR): RFC-0004's frozen quantity shape (§5) had no field for the
    absolute-vs-delta affine distinction its own affine-unit rule (§7)
    required, and no rule for what `absolute - absolute` produces. Patched
    by adding an `affine_kind: absolute | delta` discriminant and a
    subtraction rule, operationalizing DL-3's already-approved invariant.
  - F3 (MINOR): two D3/coercion-dependent constructs in the paper example
    carried their caveat only in the companion `.md`, not inline in the
    `.aicad` source most likely to be reused as a worked reference. Patched
    with inline caveats; resolves nothing, clarifies only.
  All three patches operationalized already-approved rulings or were
  documentation-only; none touched an open `OWNER_DECISIONS.md` item's
  status, and no BLOCKER-level finding was made. The independent review's
  post-patch re-review (§7 of the review document) re-ran all eleven
  adversarial probes and confirmed the patched RFCs/example are internally
  consistent.
- Alternatives considered: Deferring Stage-0 approval pending further
  review rounds (rejected — two independent reviews, `project/reports/AICAD-013.md`
  and the independent review, both concluded PASS with no BLOCKER, and the
  three gaps found were fixed in place rather than left as reasons to
  withhold approval); treating F1/F2 as requiring new
  `OWNER_DECISIONS.md` entries (rejected — both patches operationalize
  rulings already recorded as DL-1/DL-2/DL-3, they do not select among an
  unresolved alternative).
- Affected RFCs/tasks: Closes out Stage 0 (AICAD-001 through AICAD-014 plus
  the independent review). Unblocks Stage 1 (`project/CURRENT_STAGE.md`
  advanced to Stage 1; `AICAD-015` onward, per `project/TASKS.yaml`, subject
  to the Stage-1 authorized roadmap window and per-batch checkpoints).
- Supersedes: none (first Stage-0 pass ruling). Does not reopen or alter
  any `OWNER_DECISIONS.md` item — D3, D5, D10, D11, D12, D15 and the
  residual sub-items of D7/D8/D13 remain open exactly as before, and none
  was found by either Stage-0 review to block Stage 1.

---

## DL-11: Stage 1 passed — owner approval after independent adversarial review and UAF fix

- Date: 2026-09-10
- Resolves: Stage-1 exit gate (`project/CURRENT_STAGE.md`); not an
  `OWNER_DECISIONS.md` item.
- Decision: **Stage 1 has passed.** The owner has reviewed the independent
  adversarial Stage-1 gate review (`project/reports/reviews/STAGE1-INDEPENDENT-REVIEW.md`,
  commit `cad4422`) — a fresh clean-rebuild reconstruction of all Stage-1
  evidence directly from `origin/main`, including a fresh valgrind run, a
  new ASan/UBSan instrumented rebuild (closing the long-open G2 sanitizer
  gap), and an original 1920-call concurrent-stress probe against kernel
  operations the batch's own concurrency investigation had not exercised
  (sweep/loft/shell/offset/tessellate/topology exploration) — together with
  the follow-up patch commit `e05d791` that closed the review's own
  documentation/coverage gaps (missing `AICAD-037.md` report stub, a
  narrowed STEP-independence claim in `project/gates/stage-1-gate.md`, a
  new permanent `concurrency_probe.rs`, two new degenerate-input adversarial
  cases) and fixed a genuine native use-after-free in
  `aicad_occt_context_destroy`/`CheckContext` (a destroyed context's freed
  control block could be dereferenced by a second destroy call or any
  subsequent operation on a stale handle). This decision is the
  owner-recorded pass decision `AGENTS.md`/`project/CURRENT_STAGE.md`
  require before Stage 2 may begin, following the `DL-10` pattern.
- Rationale: The independent review found no BLOCKER; one MAJOR
  (STEP-verification independence framing — the gate packet's composite
  claim could read as independently verifying re-imported geometry, when
  only file-structure/entity-counts were independently verified) and four
  MINOR coverage/documentation gaps, all closed by the follow-up patch
  commit rather than left as unactioned recommendations. The
  context-lifetime use-after-free was a genuine defect (memory safety, not
  a style/coverage nit) discovered by the review's own stress probing; it
  was root-caused, fixed by making a context's control block live in a
  process-wide append-only registry for the life of the process with a
  single atomic `live` flag as the sole safety-critical field (never
  individually freed, so a second destroy or a stale-handle use can be
  rejected rather than dereferencing freed memory), and given a permanent
  regression test — matching `AGENTS.md`'s native crash/hang policy. The
  fix is scoped to context lifetime management only; it does not change
  the kernel-neutral public API, ABI shape, or any frozen semantics, so it
  did not itself require a new `OWNER_DECISIONS.md` entry.
- Alternatives considered: Deferring Stage-1 approval pending a further
  review round (rejected — the UAF fix was itself re-verified by the
  fix commit's own regression test and the pre-existing full suite
  re-run, and no further BLOCKER/MAJOR finding remained open); treating
  the context-lifetime fix as an architecture change requiring a new
  `OWNER_DECISIONS.md` entry (rejected — it is an internal correctness fix
  to lifecycle management inside `cad-occt-bridge`/`native/occt_bridge`,
  not a change to `cad-kernel-api`'s public surface, ABI, or any DL-5/DL-6
  ruling).
- Affected RFCs/tasks: Closes out Stage 1 (`AICAD-015` through `AICAD-037`
  plus the independent review and its follow-up patch). Unblocks Stage 2
  (`project/CURRENT_STAGE.md` advanced to Stage 2; `AICAD-038` onward, per
  `project/TASKS.yaml`, subject to the Stage-2 authorized roadmap window
  through `AICAD-064` inclusive and per-batch checkpoints — `AICAD-065` and
  all Stage-3 work remain forbidden until a later explicit owner approval).
- Supersedes: none (first Stage-1 pass ruling). Does not reopen or alter
  any other `OWNER_DECISIONS.md` item.

---

## DL-12: D5 canonical-state/deterministic-equivalence contract — layered v1 policy

- Date: 2026-09-10
- Resolves: `OWNER_DECISIONS.md#D5`.
- Decision: AICAD does **not** define deterministic geometry as
  byte-identical OCCT B-rep serialization. Canonical determinism is
  layered:
  - **Level 1 — language/compiler.** For identical source, dependencies,
    parameters/configuration, compiler version, feature flags, and
    execution profile: parsing/semantic structure, binding, normalized
    quantities, typed HIR, Geometry IR, diagnostics, and canonical
    AICAD-owned serialization must be deterministic. Canonical AICAD-owned
    serialization must be byte-identical where such a serialization is
    defined. Unordered maps, randomized hashing, concurrency scheduling,
    and filesystem enumeration must not affect canonical compiler output.
  - **Level 2 — same locked kernel environment.** With the same compiler
    inputs plus an identical OCCT build/environment, geometry must satisfy
    the same semantic/numerical verification profile. Byte-identical B-rep
    output is NOT required.
  - **Level 3 — supported cross-platform.** With identical compiler/kernel
    versions on supported platforms, results are equivalent when they
    validate successfully under the same contract, preserve semantically
    required counts/classes, agree within the versioned engineering
    equivalence profile (below), and preserve required semantic outcomes.
    Kernel topology enumeration and serialized B-rep bytes are not
    canonical.
  - **Level 4 — different kernel version.** Different kernel versions are
    compatibility comparisons, not deterministic identity, and may be
    considered compatible only when they satisfy the versioned equivalence
    profile. Kernel upgrades must not silently alter AICAD-owned canonical
    semantic state.
  - **Comparison profile.** A versioned, dimension-aware equivalence
    profile, parameterized by characteristic linear scale `S`:
    `linear: max(linear_abs, linear_rel * S)`;
    `area: max(area_abs, area_rel * S^2)`;
    `volume: max(volume_abs, volume_rel * S^3)`;
    `center-of-mass: linear tolerance`; other measurements use
    dimensionally appropriate tolerances. Validity and explicitly semantic
    counts/requirements remain exact where specified — exact face/edge
    counts are required only when a test explicitly declares them
    semantically meaningful; kernel face/edge enumeration order is never
    identity. The default comparison profile may not be silently widened;
    changing its default tolerances requires explicit documented review
    (i.e. a new `DECISION_LOG.md` entry, not a code-only change).
  - The concrete numeric v1 tolerance constants for the comparison profile
    are **not** fixed by this decision. Stage 2 must derive and document
    them from actual Stage-1 evidence (the bracket's own closed-form-vs-
    OCCT bounding-box/volume/center-of-mass agreement in
    `project/reports/AICAD-034.md`, and the fillet/chamfer numerical noise
    already calibrated there) and kernel precision behavior, and escalate
    the derived constants to `project/OWNER_DECISIONS.md` for ruling if
    Stage-2 evidence alone does not make a specific constant obvious
    (`AGENTS.md`'s "produce the measurements/recommendation and escalate
    the constants rather than guessing").
- Rationale: Owner ruling. `docs/plan/14_COLLABORATION_PROVENANCE_SECURITY.md`
  §18 already warned against claiming bit-identical B-rep across kernel/
  platform versions, and Stage-1's own evidence (AICAD-034 gate;
  fillet/chamfer bounding-box tolerance calibrated to observed OCCT
  numerical noise) already treats exact/analytic-vs-kernel agreement as a
  tolerance-bounded comparison rather than bit-identity — this decision
  makes that practice an explicit, versioned, escalation-gated policy
  instead of an implicit per-task judgment call, closing the D5 gap RFC-0001
  §9 and RFC-0004 explicitly left open pending an owner ruling.
- Alternatives considered: Requiring byte-identical B-rep serialization as
  the determinism bar (rejected — explicitly warned against by `14` §18 and
  infeasible across kernel/platform versions per widely-documented OCCT
  behavior); leaving determinism entirely undefined into Stage 2 (rejected
  — blocks `AICAD-052`/`AICAD-058`/`AICAD-063`'s own determinism
  requirements and the Stage-2 hardening-mode D5 re-audit `AGENTS.md`
  requires); fixing concrete numeric tolerance constants directly in this
  ruling without Stage-2 evidence (rejected — `AGENTS.md` requires
  escalating constants that require material judgment not yet supported by
  evidence, rather than guessing them here).
- Affected RFCs/tasks: RFC-0001 §2 invariant 9 and §9 (D5 was the RFC's own
  explicitly flagged open question); RFC-0004 (unit/quantity equivalence
  comparisons); Stage-2 tasks that touch determinism directly —
  `AICAD-052` (type checking), `AICAD-058`/Stage-2C checkpoint (execution
  determinism), `AICAD-063` (end-to-end proof's own final-part
  verification), `AICAD-064` (Stage-2 gate, must re-audit D5 evidence per
  the Stage-2C checkpoint instructions); `crates/cad-validation` (owns the
  concrete comparison-profile implementation once that crate's own task is
  reached).
- Supersedes: none (first ruling on D5).

## DL-13: D16 — collection/iteration semantics (Stage-2 minimum foundation)

- Date: 2026-09-10
- Resolves: `OWNER_DECISIONS.md#D16`.
- Decision: `for var in iterable { ... }` is core language control flow,
  operating over values satisfying AICAD's iteration protocol. This does
  **not** authorize a general-purpose compiler-intrinsic mechanism. Stage 2
  supports, at minimum: `List<T>` (immutable, via a new `[e1, e2, ...]`
  list-literal expression — elements unify to one compatible element type
  using the existing type/unit-conversion rules, e.g. `[5mm, 2cm, 1in] ->
  List<Length>`; incompatible dimensional elements are a type error; an
  empty `[]` requires contextual type information or a stable
  type-inference diagnostic is emitted); `Range<Int>`/`Range<UInt>` (via
  new `start..end` half-open and `start..=end` inclusive range-expression
  syntax; automatic `for` iteration is ascending by one, empty rather than
  reversing direction when `start` is beyond the terminal bound;
  `Range<T>` may exist for other element types, dimensional ones included,
  but is not automatically iterable without a future explicit-stepping
  API this decision does not build); and `Iterator<T>` as an internal/
  runtime iteration abstraction only, never exposed as compiler magic
  (`for` may lower to `iter`/`next`-shaped operations internally, but
  those are implementation machinery, not new source-level intrinsics).
  `iterable` is evaluated exactly once per `for` loop; each iteration
  introduces a fresh immutable loop binding scoped to the loop body and
  participates in the approved execution resource-budget accounting;
  iteration order is deterministic (list order for `List<T>`, numeric
  ascending order for integer ranges). Explicitly **not** authorized by
  this decision: full `Set<T>`/`Map<K,V>` semantics, collection
  comprehensions, arbitrary user-defined iterator protocols, async/
  parallel iteration, implicit dimensional-range stepping, or any new
  general compiler-intrinsic facility — all remain future work requiring
  their own decision.
- Rationale: Owner ruling, in direct response to `AICAD-056`'s own
  escalation (`project/reports/AICAD-056.md`'s first session): no
  `.aicad` source program could construct a collection/iterator value
  under the grammar as it stood after `AICAD-045` (no array/list-literal
  or range-operator syntax, no compiler-intrinsic-function mechanism), so
  `for`-loop execution — named by `AICAD-056`'s own title — could not
  proceed without either new public syntax or a new intrinsic boundary,
  both `AGENTS.md` owner-escalation triggers. This ruling supplies the
  minimum coherent collection/iteration model `AICAD-056` needs while
  preserving `AGENTS.md`'s "prefer library/std-package features over new
  compiler intrinsics" principle: `List`/`Range` are ordinary expression
  syntax reusing the existing type/unit-conversion machinery, not a new
  intrinsic-function boundary, and `Iterator<T>`'s `iter`/`next` shape
  stays private lowering machinery rather than surfaced language syntax.
- Alternatives considered: A compiler-intrinsic `range(...)`/`list(...)`
  constructor-function boundary requiring no new expression grammar
  (rejected by the owner — "This does NOT authorize a general-purpose
  compiler-intrinsic mechanism," and per D9/DL-7 a new intrinsic needs an
  RFC showing a library solution cannot work, which is circular before any
  collection primitive exists at all); implicit direction-reversing
  iteration when `start > end` (rejected — "iteration is empty rather than
  implicitly reversing direction," consistent with `AGENTS.md`'s "ambiguity
  is an error, never an arbitrary selection"); automatic dimensional-range
  stepping (e.g. inferring a 1mm step for `Range<Length>`) (rejected — no
  step size is defined, and inventing one would be exactly the kind of
  silent semantic invention `AGENTS.md` forbids; left to "a future explicit
  stepping API"); building the complete `docs/plan/02_LANGUAGE_AND_COMPILER
  .md` §9 collection list (`Optional<T>`, `Result<T,E>`, `Generator<T>`,
  `Graph<N,E>`) now (rejected — explicitly out of scope: "AICAD-056 does
  not need to implement the entire future collection library").
- Affected RFCs/tasks: `AICAD-056` (resumes with this ruling — grammar/
  AST/lexer/parser/HIR/type-checker/runtime changes are explicitly
  authorized "even where those components were introduced by earlier
  Stage-2 tasks", i.e. `AICAD-039`-`045`'s frozen grammar and
  `specs/language/grammar.ebnf`); `AICAD-057` (`Result<T,E>`/error
  propagation — D16 does not authorize `Result<T,E>` itself, only
  `Iterator<T>`/`List<T>`/`Range<T>`; `AICAD-057` remains its own,
  separate scope); any later task assuming a collection/iterator value or
  type exists should re-check this entry's explicit scope limit before
  extending it.
- Supersedes: none (first ruling on D16).

---

## DL-14: D17 — generics, data-carrying enums, and Result/Optional

- Date: 2026-09-10
- Resolves: `OWNER_DECISIONS.md#D17`.
- Decision: `AICAD-057` must not special-case `Result<T,E>`. Stage 2 is
  authorized to add the minimum *general* language machinery for ordinary
  generic algebraic data types and generic functions:
  - **Data-carrying enums.** AICAD enums support three general variant
    forms — `Unit`, `Tuple(T1, T2)`, `Record { x: T1, y: T2 }` — as
    ordinary language constructs, not `Result`-specific machinery.
    Variants are constructors usable as expressions. Pattern matching
    supports corresponding destructuring (`Unit => ...`, `Tuple(a, b) =>
    ...`, `Record { x, y } => ...`); pattern bindings have normal lexical
    scope and are typed from the matched variant. For nominal enum types,
    the compiler must diagnose non-exhaustive matches unless a wildcard or
    otherwise-exhaustive pattern is present.
  - **Generic types.** User-defined structs and enums may declare type
    parameters (`struct Pair<T, U> { ... }`, `enum Optional<T> { Some(T),
    None }`, `enum Result<T, E> { Ok(T), Err(E) }`); type application uses
    `Name<T, U>`. Generic parameters are compile-time type parameters, not
    runtime dynamic types.
  - **Generic functions.** Functions may declare ordinary type parameters
    (`fn identity<T>(value: T) -> T { return value; }`). Stage 2 must
    support ordinary call-site instantiation and type inference when type
    parameters are determinable unambiguously from arguments/expected
    types; explicit type arguments may also be supported if the approved
    grammar requires them.
  - **Scope limit.** Stage 2 does *not* need higher-kinded types, variance
    annotations, specialization, generic metaprogramming, variadic
    generics, dependent types, generic associated types, or Rust-style
    lifetime parameters. Interface/trait bounds (`T: MotorMount`) may
    remain deferred until the interface system exists, unless the Stage-2
    coverage audit demonstrates they are required by the Stage-2 gate —
    `project/reports/AICAD-057A.md` found no such requirement. The generic
    foundation must be designed so bounds can be added later without
    replacing the generic type model.
  - **Result/Optional.** `Result<T,E>` and `Optional<T>` are ordinary
    generic prelude/library types built using the same enum/generic
    machinery available to user code (conceptually `enum Result<T, E> {
    Ok(T), Err(E) }`, `enum Optional<T> { Some(T), None }`). The compiler/
    runtime must not contain `Result`-specific semantic machinery beyond
    ordinary prelude registration/loading. Optimized internal
    representations are permitted provided observable language semantics
    remain identical to ordinary generic enum values.
  - **Result propagation.** Stage 2 does not add a `?` operator or other
    new propagation syntax merely to complete `AICAD-057`. `Result` values
    are propagated explicitly via ordinary `match` (`match operation() {
    Ok(value) => ..., Err(error) => return Err(error), }`); ergonomic
    propagation syntax requires a later, separate language decision.
  - **`List`/`Range` cleanup.** `D16`'s `List<T>`/`Range<T>` may retain
    optimized runtime representations, but their type-application handling
    should use the new general generic-type machinery wherever practical
    rather than permanently accumulating name-specific generic parsing/
    type-resolution special cases. No general compiler-intrinsic facility
    is authorized.
  - **Recursion policy (clarification, not new scope).** AICAD does not
    define 64 calls as a language-level recursion limit. The existing
    `DEFAULT_MAX_CALL_DEPTH = 64` (`AICAD-057`,
    `crates/cad-runtime/src/interp.rs`) is a conservative *implementation
    safety ceiling* for the Stage-2 tree-walking evaluator, distinct from
    any future language-level resource budget (`AICAD-058`). A user may
    request a stricter recursion budget but may not raise execution beyond
    the engine-safe ceiling; a native host stack overflow is never an
    acceptable AICAD program outcome. A future runtime may support
    substantially deeper recursion (e.g. by moving call state onto an
    explicit interpreter/VM stack) without changing AICAD language
    semantics; that migration is not authorized as part of this
    remediation unless evidence shows it is necessary for the Stage-2 gate.
  - **Fixed remediation sequence**, required before the original
    `AICAD-057` resumes: `AICAD-057A` (Stage-2 coverage audit — this
    entry's own trigger, `project/reports/AICAD-057A.md`) ->
    `AICAD-057B` (generic parameter/type-application syntax plus AST/HIR
    representation) -> `AICAD-057C` (general data-carrying enum variants,
    constructors, destructuring patterns, nominal-enum-match exhaustiveness)
    -> `AICAD-057D` (generic instantiation/inference/type checking for the
    approved subset) -> `AICAD-057E` (`Result<T,E>`/`Optional<T>` via the
    ordinary generic-enum machinery) -> `AICAD-057F` (adversarial
    integration pass proving the machinery is general, not hard-coded for
    `Result`) -> original `AICAD-057` -> `AICAD-058` -> the
    `STAGE2-C_EXECUTION` checkpoint. `AICAD-059` may not begin before that
    checkpoint passes.
  - **Required remediation tests (minimum):** generic struct with one type
    parameter; generic struct with two type parameters; generic enum;
    generic function; successful inferred generic call; ambiguous generic
    call -> diagnostic; wrong number of type arguments -> diagnostic; unit/
    tuple/record enum variants; payload construction; tuple destructuring;
    record destructuring; payload binding has correct type; non-exhaustive
    enum match -> diagnostic; `Result<Int,String>`; `Result<Length,
    SomeErrorType>`; `Optional<Length>`; explicit successful `Result`
    match; explicit `Err` propagation through nested function calls; nested
    generic types (`Optional<Result<Int,E>>`); a user-defined generic enum
    whose name is not `Result`/`Optional`/`List`/`Range`, proving the
    machinery generalizes. No `?` syntax; no general compiler-intrinsic
    mechanism.
- Rationale: Owner ruling, in direct response to `AICAD-057`'s own
  escalation (`project/reports/AICAD-057.md`, `OWNER_DECISIONS.md#D17`'s
  original text) that `Result<T,E>` construction needs either general
  data-carrying enums + generics, a `D16`-style narrow special case, or
  deferral. The owner judged the narrow special case (option 2) would not
  generalize to `Optional<T>` next and would accumulate unprincipled
  special cases in `cad_hir::typeck::resolve_type_ref`, and that deferral
  (option 3) would leave `AICAD-057` permanently incomplete, so authorized
  the general mechanism (option 1) instead, scoped tightly to what
  `Result`/`Optional`/ordinary user ADTs need and explicitly excluding
  interface bounds, higher-kinded types, and every other item `AGENTS.md`
  would otherwise treat as scope creep. The owner additionally required a
  Stage-2 coverage audit (`AICAD-057A`) first, since `D16`'s and `D17`'s
  back-to-back escalations both stemmed from `project/TASKS.yaml`'s
  Stage-2 task list never having enumerated the full general-language
  surface `docs/plan/` assumes, and directed that the audit classify gaps
  rather than silently widen Stage 2 to match the long-term language plan.
- Alternatives considered: narrow `Result`-only special-casing mirroring
  `D16`'s `List`/`Range` treatment inside `resolve_type_ref` (rejected —
  does not generalize to `Optional<T>`, accumulates special cases,
  contradicts "prefer library/std-package features over new compiler
  intrinsics" once a second data-carrying type is needed); deferring
  `Result<T,E>` for all of Stage 2 (rejected — leaves `AICAD-057`
  permanently incomplete and blocks `AICAD-063`'s end-to-end proof from
  ever using typed error handling); adding a `?` propagation operator now
  (rejected — explicit non-goal, "does not add a `?` operator... merely to
  complete AICAD-057"); rewriting the evaluator into an explicit-stack VM
  now to raise the recursion ceiling (rejected — explicit non-goal unless
  the Stage-2 gate needs it; tracked as future architectural work instead).
- Affected RFCs/tasks: `AICAD-057A` (this decision's own trigger, coverage
  audit); `AICAD-057B`/`057C`/`057D`/`057E`/`057F` (new remediation tasks,
  added to `project/TASKS.yaml` by `AICAD-057A`); `AICAD-057` (resumes only
  after `057B`-`057F` pass); `AICAD-058` (must keep the language resource
  budget distinct from the engine-safety recursion ceiling this decision
  clarifies); `AICAD-059`/`STAGE2-C_EXECUTION` checkpoint (blocked until
  the full remediation sequence and original `AICAD-057` complete); `D16`
  (`List<T>`/`Range<T>` type-application handling should migrate onto this
  decision's general generic machinery where practical, per "List/Range
  cleanup" above, without being forced to before `AICAD-057F`).
- Supersedes: none (first ruling on D17).

---

## DL-15: D18 — runtime-backed standard functions and safe geometry invocation

- Date: 2026-09-12
- Resolves: `OWNER_DECISIONS.md#D18`.
- Decision: AICAD ordinary safe geometry operations (`box(...)`,
  `cylinder(...)`, `cut(a, b)`, `transform(body, ...)`, ...) are invoked
  using **ordinary function-call syntax**. No new geometry-specific call
  syntax is introduced. AICAD supports a general, non-geometry-specific
  mechanism: **compiler/runtime-owned standard functions** whose
  implementation is provided by the runtime rather than by an AICAD-source
  `HirBlock`.
  - **Function representation.** A callable function retains ordinary
    function binding and call semantics. The HIR function representation
    distinguishes implementation source conceptually as
    `FunctionImplementation::Aicad(HirBlock)` vs.
    `FunctionImplementation::RuntimeBuiltin(BuiltinFnId)` (an equivalent
    internal representation is acceptable). No geometry-specific
    expression form (`HirExpr::GeometryCall`, `HirExpr::GeometryIntrinsic`)
    is authorized. Runtime-backed functions participate in the same
    ordinary name resolution, argument checking, type checking, overload
    rules (if/when supported), source-span diagnostics, and call-expression
    semantics as AICAD-defined functions.
  - **Not a compiler intrinsic.** A runtime-backed standard function is not
    a compiler intrinsic merely because its implementation is native/
    runtime code: it uses ordinary syntax/binding/typing/`HirExpr::Call`,
    only its implementation differs. A construct requiring compiler-
    specific syntax, typing, lowering, or semantic rules unavailable to
    ordinary functions remains a compiler intrinsic, still governed by
    `D9`/`DL-7`'s RFC requirement, unweakened by this ruling.
  - **No arbitrary native callback facility.** Stage 2 does not expose
    arbitrary Rust/C++ callbacks, FFI functions, native plugins, or
    user-defined host functions. `RuntimeBuiltin` ids are a closed,
    compiler/runtime-owned, finite mechanism — never a serialized raw
    function pointer, and never alters the later plugin/native-extension
    security boundary (`D12`).
  - **Tier B (Safe CAD).** Ordinary safe CAD operations use runtime-backed
    standard functions whose implementations construct/extend the
    backend-independent `GeometryGraph` (`box(...)` -> `GeometryOp::Box`,
    `cut(a, b)` -> `GeometryOp::Cut`, ...), returning the appropriate AICAD
    geometry value referencing the resulting `GeometryGraph` node. They
    never expose an OCCT object, a raw kernel topology pointer, persistent
    identity from an OCCT handle, or a call path around Geometry IR; kernel
    execution continues through the Stage-1 kernel-neutral boundary.
  - **Tier C (unsafe geometry) stays reserved.** RFC-0002's
    `unsafe geometry { ... }` mechanism remains reserved for raw/kernel-
    grade topology capabilities. Ordinary `box`/`cylinder`/`cut`/... must
    never require an `unsafe geometry` block; Tier B and Tier C stay
    semantically distinct.
  - **Public API is not the Geometry IR.** The public AICAD Safe CAD
    function catalogue must **not** automatically expose every
    `GeometryOp`/`GeometryQuery` variant one-for-one — Geometry IR is an
    internal, backend-independent execution representation that must
    remain free to be refactored/split/combined/extended without
    automatically changing the language API. A deliberate source-level Safe
    CAD API sits above Geometry IR, documented in a small,
    version-controlled specification (`docs/API/safe-cad-api.md`).
  - **Stage-2 surface.** Stage 2 exposes only the Safe CAD operations
    needed to prove the Stage-2 language-to-geometry slice
    (`AICAD-063`'s bracket proof) plus operations already unambiguously
    supported by the approved Safe CAD design: at minimum `box`,
    `cylinder`, `transform`, `union`, `cut`, `intersect`, `fillet`,
    `chamfer`. `export_step`/`import_step`/`tessellate`/low-level edge-wire
    construction/raw topology traversal/`validate`/`adopt_validated`/
    diagnostic-kernel-inspection operations do **not** automatically become
    Stage-2 source functions merely because a corresponding IR/runtime
    operation exists — STEP export for the `AICAD-063` gate may remain part
    of the build/output pipeline rather than an arbitrary source-level I/O
    operation, avoiding accidental file-I/O/effect semantics in the
    language. Geometry queries (`volume`/`area`/`bounding_box`/
    `center_of_mass`/`is_valid`) may use the same runtime-backed-function
    architecture when/if their source-visible semantics are implemented;
    this ruling does not require all of them to become source-visible in
    Stage 2, and if letting source control flow depend on kernel-evaluated
    queries needs a materially different execution/evaluation model, that
    is its own separate architecture decision, not something to fold
    silently into `AICAD-060`.
  - **Standard function catalogue.** Runtime-backed standard functions are
    described by typed declarations carrying at least: name (for
    resolution), parameter types, return type, runtime builtin identity,
    and diagnostics. A single authoritative declaration/catalogue feeds
    both binding/type checking and runtime dispatch where practical — the
    type checker must not independently duplicate geometry signatures.
  - **Prelude/module binding.** Runtime-backed functions may be made
    available through AICAD's existing module/prelude machinery. No new
    import syntax is authorized. For Stage 2, preserve the
    already-approved/illustrated ordinary call style without inventing new
    grammar.
  - **Determinism/resource accounting.** Runtime-backed functions remain
    subject to `D5` deterministic-execution requirements, execution-
    resource accounting, structured diagnostics, the approved kernel
    abstraction, and normal error propagation — they cannot bypass AICAD
    execution budgets merely because their implementation is runtime-
    provided.
- Rationale: Owner ruling. AICAD needs a controlled bridge between ordinary
  language functions and capabilities the host runtime implements; geometry
  is the first major use of that bridge, but the mechanism must not let
  geometry-specific semantics infect the general compiler. This design
  preserves ordinary AICAD call semantics, a backend-independent Geometry
  IR, and a narrow kernel boundary simultaneously, without a general
  compiler-intrinsic facility and without collapsing Safe CAD into unsafe
  kernel access.
- Alternatives considered (`OWNER_DECISIONS.md#D18`'s own three options):
  option 1 (a new `BindingKind`/geometry-specific dispatch, e.g.
  `BindingKind::GeometryIntrinsic`) — rejected as stated, in favor of the
  more general `FunctionImplementation::RuntimeBuiltin` shape that is not
  geometry-specific and reuses ordinary `HirExpr::Call`/`BindingKind::Fn`
  machinery rather than adding a new binding kind or expression form;
  option 2 (reusing/extending RFC-0002 §4's `unsafe geometry` blocks for
  ordinary Safe CAD operations) — rejected, exactly as `D18`'s own option-2
  writeup warned, to keep Tier B/Tier C semantically distinct; option 3
  (deferring language-surface invocation further) — superseded by this
  ruling authorizing the general mechanism now, resuming `AICAD-060` from
  its existing partial implementation rather than deferring again.
- Affected RFCs/tasks: `AICAD-060` (resumes, implementing the general
  `RuntimeBuiltin` mechanism plus the Stage-2 Safe CAD catalogue and source-
  to-`GeometryGraph` path; the already-tested `GeometryGraph -> kernel`
  dispatcher and `NumberValue -> Quantity` bridge from this task's own
  prior session are kept, not discarded, absent a concrete defect);
  `AICAD-061` (proceeds only after `AICAD-060` fully completes, per the
  fixed Batch S2-11 order); `RFC-0001`/`RFC-0002` (updated only as
  necessary to document the general runtime-backed standard-function
  mechanism and its Tier-B/Tier-C relationship — `RuntimeBuiltin` functions
  must never be described as compiler intrinsics); a new
  `docs/API/safe-cad-api.md` (the small, version-controlled Safe CAD
  source API specification this ruling requires); `D9`/`DL-7` (unweakened —
  a genuine future compiler intrinsic still needs its own RFC).
- Supersedes: none (first ruling on D18).

---

## DL-16: Stage 2 passed — owner approval of the `AICAD-064` PASS WITH CONDITIONS gate

- Date: 2026-09-12
- Resolves: Stage-2 exit gate (`project/CURRENT_STAGE.md`); not an
  `OWNER_DECISIONS.md` item.
- Decision: **Stage 2 has passed.** The owner accepts `project/gates/
  stage-2-gate.md`'s (`AICAD-064`) PASS WITH CONDITIONS recommendation: the
  complete Stage-2 source -> parser/type/HIR/runtime -> Geometry IR ->
  kernel-neutral API -> exact B-rep -> STEP slice is accepted as evidenced
  (full clean re-run: `cargo fmt --all -- --check`, `cargo clippy
  --workspace --all-targets --all-features -- -D warnings` zero warnings
  across 27 crates, `cargo test --workspace` zero failures, `cargo test -p
  cad-cli --test stage2_end_to_end -- --test-threads=1` 3/3; independently
  re-verified kernel/Geometry-IR boundary and no-demo-shortcut claims). The
  gate's one condition — `project/OWNER_DECISIONS.md#D19` (D5 v1
  comparison-profile numeric tolerance constants) — is explicitly
  **non-blocking** for Stage-2 completion itself, per `DECISION_LOG.md
  #DL-12`'s own framing ("before the Stage 8/13 determinism benchmarks
  mature") and `AICAD-064`'s own escalation. This decision is the
  owner-recorded pass decision `AGENTS.md`/`project/CURRENT_STAGE.md`
  require before Stage 3 may begin, following the `DL-10`/`DL-11` pattern.
- Rationale: `AICAD-064`'s audit independently re-verified every Stage-2
  exit-gate criterion (not merely cited prior task reports) against actual
  current code and a from-clean-state re-run of the full verification
  suite, found no BLOCKER, and the one open condition (D19's remaining
  four unevidenced constants: `linear_rel`/`area_abs`/`area_rel`/
  `volume_abs`) does not gate any Stage-2 acceptance criterion — no
  `AICAD-038`..`AICAD-063` task ever required those constants to exist.
  Carrying D19 forward as the first Stage-3 task (`AICAD-064A`, Batch
  S3-00) rather than blocking Stage-2 exit on it matches `AGENTS.md`'s
  "quality gates are requirements for continued development, not
  substitutes for continued development" and avoids inventing an
  unscheduled stabilization period the campaign brief explicitly forbids.
- Alternatives considered: withholding Stage-2 approval until D19 fully
  resolves (rejected — D19 blocks no Stage-2 acceptance criterion, and
  `crates/cad-validation` was never a Stage-2 task in the first place, so
  there is nothing Stage-2-scoped left to wait on); re-litigating any of
  the already-closed Stage-2 batch checkpoints (rejected — no new evidence
  contradicts `STAGE2-A_FRONTEND.md`/`STAGE2-B_TYPES_HIR.md`/
  `STAGE2-C_EXECUTION.md`, all independently re-audited by `AICAD-064`
  itself).
- Affected RFCs/tasks: Closes out Stage 2 (`AICAD-038` through `AICAD-064`).
  Unblocks Stage 3 (`project/CURRENT_STAGE.md` advances to Stage 3;
  `AICAD-064A` onward, per `project/TASKS.yaml` and the fixed Stage-3 batch
  order S3-00 through S3-10 — `AICAD-080` and all Stage-4 implementation
  remain forbidden until a later, separate, explicit owner approval after
  the Stage-3 final gate, `AICAD-079B`).
- Supersedes: none (first Stage-2 pass ruling). Does not reopen or alter
  any other `OWNER_DECISIONS.md`/`DECISION_LOG.md` entry.

---

## DL-17: D19 partial ruling — three of seven D5 v1 comparison-profile constants accepted; the rest require `AICAD-064A` calibration

- Date: 2026-09-12
- Resolves: `OWNER_DECISIONS.md#D19` (partial).
- Decision: The owner accepts the three directly-evidenced constants
  `AICAD-064`'s audit (`project/OWNER_DECISIONS.md#D19`, sourced from
  `project/reports/AICAD-034.md`) already produced, as v1 defaults:

  ```text
  linear_abs         = 0.0001 mm   (1e-4 mm)
  center_of_mass_abs = linear_abs
  volume_rel         = 0.001       (1e-3)
  ```

  `linear_abs` is a physical quantity expressed canonically (millimetres),
  never a dimensionless number reinterpreted in a caller's own display
  units — consistent with `DL-3`'s structural-canonicalization ruling. The
  owner does **not** authorize guessed defaults for the remaining four:
  `linear_rel`, `area_abs`, `area_rel`, `volume_abs` — no existing Stage-1/
  Stage-2 evidence exercised a multi-scale fixture or measured an area
  comparison at all, and a dimensional-analogy guess (e.g. `area_rel ≈
  volume_rel`) would be exactly the unsupported guess `AGENTS.md`
  prohibits. `AICAD-064A` (Batch S3-00) must run a focused, bounded,
  multi-scale calibration corpus (characteristic scales spanning
  approximately 1 mm, 10 mm, 100 mm, 1000 mm; primitives, transforms,
  booleans, fillets/chamfers, analytically checkable area, analytically
  checkable volume, center of mass, repeated rebuilds; absolute error,
  relative error, and repeat-run numerical drift measured separately, on
  one machine — cross-platform behavior must not be inferred from
  same-machine repeated runs) and derive the remaining constants from that
  measured evidence, escalating back here (as a superseding entry) rather
  than guessing if the evidence is ambiguous, or if a required tolerance
  looks obviously unreasonable, or if the data conflicts materially with
  `DL-12`'s existing D5 contract.
- Rationale: Owner ruling, per `AGENTS.md`'s "produce the
  measurements/recommendation and escalate the constants rather than
  guessing" and `DECISION_LOG.md#DL-12`'s own instruction that Stage 2/3
  "derive and document" the v1 constants "from actual ... evidence." The
  three accepted constants already have direct, repeated, evidenced
  support (`AICAD-034`'s five-repeat-run bounding-box/volume measurements);
  the other four have never been measured against any real fixture at any
  scale, so accepting them now would be exactly the "guessed default" `D19`
  itself flagged as prohibited.
- Alternatives considered: accepting all seven constants now via
  dimensional-analogy extrapolation from the three evidenced ones
  (rejected outright by `D19`'s own text: "The owner does NOT authorize
  guessed defaults"); deferring all seven, including the three already
  evidenced, until `AICAD-064A` completes (rejected — the three evidenced
  constants have real, repeated, multi-run supporting data today; there is
  no reason to withhold them pending a calibration pass that is not
  measuring them again from scratch, only extending coverage to the other
  four and to multiple scales).
- Affected RFCs/tasks: `AICAD-064A` (Batch S3-00, must implement/calibrate
  the complete v1 profile in `crates/cad-validation` per this ruling and
  `DL-12`'s comparison-profile shape); `DECISION_LOG.md#DL-12` (unweakened —
  this only fixes concrete numeric constants, not the profile's shape or
  layering).
- Supersedes: none (first ruling on the concrete D19 constants; narrows,
  does not reopen, `DL-12`).

---

## DL-18: D10 — diagnostic code/schema stability policy

- Date: 2026-09-12
- Resolves: `OWNER_DECISIONS.md#D10`.
- Decision: Committed diagnostic identifiers (`FAMILY-Exxx`/`Wxxx`/`Ixxx`
  codes already used by a merged commit) are durable. A committed code must
  never be silently repurposed for a different meaning. Pre-1.0
  diagnostics may be deprecated or replaced, but never silently renumbered
  or reused for an unrelated condition — a deprecated code is retired
  (documented as deprecated, optionally kept emitting with a
  superseded-by note) rather than reassigned. Machine-readable diagnostic
  schemas (`docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md`'s JSON schema) are
  versioned; a compatibility-breaking schema change (removing a field,
  changing a field's meaning/type, removing a family) requires explicit
  review — recorded here, not silently shipped in an ordinary task commit.
  Adding a *new* code within an already-reserved family (e.g. the next
  `RUNTIME-E1xx`), or adding a wholly new family for a genuinely new
  diagnostic domain, is ordinary task work and does not itself require an
  owner ruling — only *repurposing* an existing committed code, or a
  breaking schema change, does.
- Rationale: Owner ruling, per the Stage-3 campaign brief's explicit
  requirement that this decision be in force before `AICAD-078` normalizes
  new modeling diagnostics, and per every existing diagnostic module's own
  already-stated assumption ("provisional per D10" — `crates/cad-runtime/
  src/error.rs`, `crates/cad-hir/src/typeck.rs`) that such a policy would
  eventually be recorded. Codifying "durable once committed, not
  repurposed, versioned schema" gives every future diagnostic-adding task
  (Stage 3's `AICAD-072`/`073`/`074`/`075`/`076`/`077`/`078`/`079` included)
  a fixed rule rather than an implicit convention.
- Alternatives considered: allowing pre-1.0 codes to be freely renumbered
  (rejected — every diagnostic module in this codebase already documents
  its own codes as "provisional per D10" in anticipation of exactly this
  ruling landing before renumbering became a real risk with real external
  consumers, e.g. `cad-lsp`/tooling reading fixed codes); requiring a full
  1.0-style stability guarantee immediately (rejected — pre-1.0
  deprecate-and-replace remains allowed, matching how every other
  Stage 0-2 provisional decision in this log treats pre-1.0 flexibility).
- Affected RFCs/tasks: `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §10-12;
  every diagnostic-emitting crate (`cad-diagnostics`, `cad-hir::typeck`,
  `cad-runtime::error`, `cad-units::arithmetic`, `cad-geometry-api::ir`,
  `cad-occt-bridge`); `AICAD-078` (Stage 3, "normalized diagnostics" —
  this ruling must be in force first, per the campaign brief).
- Supersedes: none (first ruling on D10).

---

## DL-19: D3 — sketch entity/object model

- Date: 2026-09-12
- Resolves: `OWNER_DECISIONS.md#D3`.
- Decision: The authoritative Stage-3 sketch model is an explicit,
  kernel-independent semantic `Sketch` object. A `Sketch` owns/contains an
  explicit plane/frame, explicit entity nodes (`line`, `circle`, `arc`,
  ...), explicit constraint nodes, deterministic local semantic entity
  identities suitable for the sketch IR (mirroring `param`'s own
  `BindingId`-based identity precedent from `AICAD-065`/this log's `DL-16`
  era, not a parallel identity scheme), and source/provenance links where
  available. Sketch entities are **not** registered through hidden global
  mutable state — an entity created inside a `sketch { ... }` block may use
  lowering-context information to attach itself to that block's own
  `Sketch`, but the resulting IR must explicitly identify both the owning
  sketch and the entity itself; no entity's existence may depend on an
  implicit, unobservable side effect. Surface block syntax (`sketch(plane)
  { line(...); circle(...); }`) may exist as ergonomic sugar, but it must
  lower to this same explicit functional/value semantic model — matching
  `DL-2`'s own "method/builder syntax is sugar lowering to functional
  semantics" precedent. Stage-3 sketch entity identity must **not** be
  represented by OCCT topology IDs, and this decision does **not** claim to
  solve Stage-4 persistent topological naming — a sketch entity's Stage-3
  identity is stable only within one build/edit of that sketch's own IR,
  not across arbitrary topology-changing rebuilds.
- Rationale: Owner ruling, per the Stage-3 campaign brief's explicit
  requirement that this decision be recorded before the sketch-entity-IR
  batch (`AICAD-072`, Batch S3-04) begins. `docs/plan/
  04_HIGH_LEVEL_MODELING_API.md` §3 already favors explicit sketch objects
  internally with block syntax as sugar but never specified the actual
  binding mechanism (`project/reports/ORIENTATION_PASS.md` §6, contradiction
  4) — this ruling picks the explicit-object option the plan itself leaned
  toward, and forecloses the alternative (implicit global registration)
  `AGENTS.md`'s "no secret mutation semantics" non-negotiable would
  otherwise leave ambiguous going into Stage 3.
- Alternatives considered: implicit global sketch-registration (a bare
  `line(...)` call mutating some ambient "current sketch" global state)
  (rejected — "hidden mutable global registration" is exactly what
  `AGENTS.md`'s functional-core/no-secret-mutation non-negotiables and this
  ruling's own text forbid); deferring the sketch object model until
  `AICAD-072` itself decides it ad hoc (rejected — sketch/constraint IR is
  exactly the kind of "select between major unresolved architecture
  alternatives" `AGENTS.md` requires escalating rather than deciding
  silently inside a single task).
- Affected RFCs/tasks: `AICAD-072` (Batch S3-04, "Create minimal sketch
  entity IR" — must implement exactly this model); `AICAD-073`/`074`/`075`
  (Batch S3-05, constraint IR/lowering, build on this same `Sketch` object);
  `docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3.
- Supersedes: none (first ruling on D3).

---

## DL-20: D11 — constraint IR semantics and solver-independence rules

- Date: 2026-09-12
- Resolves: `OWNER_DECISIONS.md#D11`.
- Decision: The AICAD constraint IR (not a solver backend) is authoritative
  for every observable constraint semantic. It owns: typed/dimensioned
  constraint variables; constraint kinds and their parameters; semantic IDs
  (mirroring `DL-19`'s own sketch-entity-identity precedent — AICAD-owned,
  not kernel/solver-owned); source/provenance mapping; the solve-status
  vocabulary (at minimum: solved, underconstrained, overconstrained); and a
  structured diagnostic/evidence vocabulary. A solver backend owns
  numerical algorithms only, and must **not** redefine dimensional
  semantics, the meaning of any constraint kind, success/failure
  classifications, or observable ambiguity/underconstraint semantics. For a
  multiple-solution or underconstrained case, the backend must expose
  structured status/degrees-of-freedom/evidence — it must never let a
  backend-specific arbitrary branch silently become AICAD language
  semantics; a deterministic branch-selection policy, if ever required,
  belongs above the solver adapter and needs its own explicit
  specification (not authorized by this ruling). For an overconstrained
  case, structured conflict evidence is desirable, but a provably minimal
  conflict set is **not** required in the Stage-3 baseline. Exactly one
  initial solver implementation is permitted behind this interface; the
  interface itself must permit a later solver replacement without changing
  public constraint semantics. Numerical tolerances that affect observable
  behavior must be explicit and versioned (mirroring `DL-12`/`DL-17`'s D5
  versioned-profile precedent) and must never be silently widened merely to
  make a test pass.
- Rationale: Owner ruling, per the Stage-3 campaign brief's explicit
  requirement that this decision be recorded before the constraint-IR batch
  (`AICAD-073`, Batch S3-05) begins, and per `AGENTS.md`'s own non-
  negotiable "stable semantic references are preferred; ambiguity is an
  error, never an arbitrary selection" extended here to constraint solving
  specifically. `docs/plan/08_CONSTRAINTS_REQUIREMENTS_TESTS.md` §4/§6
  describes a constraint IR conceptually but never froze the precise
  solver-independence boundary (`project/OWNER_DECISIONS.md#D11`'s own
  original "Status: Open" text) — this ruling freezes that boundary before
  any solver-backed implementation exists to entrench an ad hoc one.
- Alternatives considered: letting the first solver implementation
  implicitly define constraint semantics by precedent (rejected — exactly
  the "select between major unresolved architecture alternatives"
  escalation trigger `AGENTS.md` requires an owner ruling for, and would
  make later solver replacement a breaking change rather than an internal
  swap); requiring a provably minimal overconstraint conflict set in the
  Stage-3 baseline (rejected as premature — no Stage-3 task needs it, and
  demanding it now would be exactly the kind of scope-creep the campaign
  brief's "do not pull forward... full verification framework" line
  forbids).
- Affected RFCs/tasks: `AICAD-073` (Batch S3-05, "Create solver-independent
  sketch constraint IR/adapter" — must implement exactly this boundary);
  `AICAD-074`/`075` (build on the same IR); `docs/plan/
  08_CONSTRAINTS_REQUIREMENTS_TESTS.md` §4, §6.
- Supersedes: none (first ruling on D11).
