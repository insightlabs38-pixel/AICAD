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

## DL-10: Stage 0 passed — Stage 1 approved

- Date: 2026-09-09
- Resolves: Stage-0 exit gate (`project/CURRENT_STAGE.md`)
- Decision: Stage 0 is approved as passed. Two independent gate
  recommendations were available: the implementation team's own
  `project/gates/stage-0-gate.md` (AICAD-014) and a second, independent
  adversarial review, `project/reports/reviews/STAGE0-INDEPENDENT-REVIEW.md`
  (commit `aad72671b64e2471ca31c8fa332aeb098d486210`). The independent
  review found two MAJOR completeness gaps (F1: `if`/`match` never defined
  in expression position despite the paper example and `docs/plan/07`
  requiring it; F2: RFC-0004's quantity shape had no `affine_kind`
  discriminant or absolute-minus-absolute rule despite the affine-unit
  invariant requiring one) and one MINOR documentation gap (F3: two
  open-decision-dependent constructs in the paper example lacked inline
  caveats), patched all three without selecting among any open
  `project/OWNER_DECISIONS.md` alternative, and re-reviewed the patched
  state clean. Both recommendations are PASS. The owner accepts the PASS
  recommendation after the applied patches. Stage 1 is now active per
  `project/CURRENT_STAGE.md`.
- Rationale: Owner ruling. The independent review's patches operationalize
  already-approved rulings (DL-1/DL-2 for F1, DL-3 for F2) rather than
  making a new architecture choice, and its own re-review (§7) and
  unresolved-decisions cross-check (§8) confirm no open `OWNER_DECISIONS.md`
  item (D3, D5, D10, D11, D12, D15, plus the residual sub-items of D7, D8,
  D13) was silently touched or resolved. This satisfies
  `project/CURRENT_STAGE.md`'s exit gate (a paper/spec example spanning
  parameters, function, loop, conditional, sketch, extrusion, semantic
  query, low-level geometry, assembly, constraint, and test without
  semantic contradiction) and the evidence standard in `AGENTS.md`
  ("Stage gates ... Stage progression is an owner decision").
- Alternatives considered: Passing on the implementation team's own gate
  packet alone without an independent check (rejected — a second,
  adversarial review that does not merely trust the authoring batch's own
  conclusions is stronger evidence, and it found real gaps the first
  review missed); holding Stage 1 pending a new owner decision (rejected —
  neither patch required selecting among an open architecture alternative,
  so no new `OWNER_DECISIONS.md` entry was needed before ruling).
- Affected RFCs/tasks: Unblocks `AICAD-015` through `AICAD-037` (Stage 1,
  Batches 1A-1E), authorized through `AICAD-037` inclusive.
  `AICAD-038` and all Stage-2 work remain forbidden pending a future,
  separate owner approval.
- Supersedes: none (first Stage-0 pass ruling). Does not resolve, and does
  not supersede, any entry in `project/OWNER_DECISIONS.md` — D3, D5, D10,
  D11, D12, D15, and the residual sub-items of D7, D8, D13 remain open
  exactly as before.
