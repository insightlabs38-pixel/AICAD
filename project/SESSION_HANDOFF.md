# Session Handoff

## Latest: Stage 2 Batch S2-01 (AICAD-038, AICAD-039, AICAD-040) complete and canonical on `branch/determined-allen-4r42gi`.

### IMPORTANT — branch-naming note for the next invocation

The active scheduled-task brief for this campaign names
`origin/claude/aicad-stage2-dev` as "the" canonical Stage-2 branch and
instructs each invocation to fetch/rebuild on top of it. **That branch
does not exist in this repository** (confirmed via `git fetch origin
--prune` + `git branch -a` at the start of this session — only
`branch/loving-feynman-*`, `branch/festive-cori-*`,
`branch/compassionate-wright-*`, `branch/epic-archimedes-*`,
`branch/pensive-hopper-*`, `claude/aicad-stage-0-review-9leull`,
`claude/first-prompt-execution-kzavou`, and `main` existed). This matches
the repository's own established Stage-0/Stage-1 convention throughout
`project/reports/`: every prior session worked on its own harness-assigned
randomly-named branch and merged into `main` via a PR (see the git log —
`branch/loving-feynman-qde9m3` -> PR #2, `branch/pensive-hopper-5cbjby` ->
PR #3, `branch/compassionate-wright-lbvrc1` -> PR #4/#5,
`branch/festive-cori-pe2fun` -> PR #6/#7, `branch/epic-archimedes-f6d1fe`
-> PR #8) — there has never actually been one persistent
`claude/aicad-stage2-dev`-style branch spanning multiple sessions in this
repo's real history; `main` (via sequential merged PRs) has always been
that persistent lineage.

This session's own harness-assigned working branch (per the outer
"Git Development Branch Requirements"/"NEVER push to a different branch
without explicit permission" instructions, which are session-level
policy, not campaign-prompt text) is `branch/determined-allen-4r42gi`,
created from `origin/main` at `09fdef5` — exactly the owner-approved
Stage-1 commit the campaign brief itself calls for. Given the conflict
between "create/use `origin/claude/aicad-stage2-dev`" (campaign brief) and
"never push to a different branch than the one this session was assigned"
(harness policy), this session treated the harness-assigned branch as
authoritative and did **not** create `origin/claude/aicad-stage2-dev`,
consistent with how every real prior session in this repo actually
operated (own branch -> PR -> `main`).

**Recommended for the next invocation**: check whether
`origin/claude/aicad-stage2-dev` has been created by then (e.g. by a PR
merge or owner action). If not, and your own harness assignment gives you
a different branch name again, treat *your own* assigned branch the same
way this session did — base it on the latest state of Stage-2 work (either
`origin/main` if this branch has been merged, or this branch
`branch/determined-allen-4r42gi` directly if not yet merged — check both)
rather than trying to independently create the campaign brief's named
branch. Whoever has commit access to `main` should decide whether/when to
merge `branch/determined-allen-4r42gi`.

### What this session did

Started from `origin/main` at `09fdef5` (PR #8 merge: Stage-1 independent
adversarial review + context-lifetime use-after-free fix). This session's
own work is four commits on `branch/determined-allen-4r42gi`:

```
1c3c767 AICAD-040: Implement numeric literals with engineering-unit suffix tokenization
2b3c6fe AICAD-039: Create cad-ast and cad-lexer with token/span model
57a31d5 AICAD-038: Create cad-diagnostics crate and JSON-schema conformance tests
1eaac7c Record Stage-1 owner approval (DL-11) and D5 determinism policy (DL-12); advance to Stage 2
```

**Canonical-publishing status**: pushed to `origin/branch/determined-allen-4r42gi`
with `git push -u origin branch/determined-allen-4r42gi` (see below for the
exact command/result). Not merged into `main` — no PR was opened this
session (the task instructions did not ask for one; per the outer
CLAUDE.md/harness policy, do not open a PR unless explicitly asked).

1. **Recorded Stage-1 owner approval and the D5 owner ruling** (the task's
   own explicit authorization to advance to Stage 2), per
   `project/DECISION_LOG.md#DL-11`/`#DL-12`, and advanced
   `project/CURRENT_STAGE.md` to Stage 2 with its own goal/exit-gate/
   allowed-work sections. `project/OWNER_DECISIONS.md` D5 marked resolved
   (policy shape only — concrete v1 tolerance constants still need
   deriving from Stage-1 evidence during Stage 2, per DL-12's own text;
   not yet done).
2. **AICAD-038** — `crates/cad-diagnostics`: `Diagnostic`/`DiagnosticCode`/
   `Severity`/`SourceSpan`/`Suggestion` types matching RFC-0005 §3-4
   exactly; a dependency-free canonical `json` module (parser +
   byte-identical-where-defined serializer — no third-party crate added,
   first such decision point in this workspace, see that task's report);
   a narrow JSON-Schema-subset validator (`type`/`required`/`properties`/
   `items`/`enum`, no `$ref`/`pattern`); populated
   `specs/schemas/diagnostic.schema.json`; conformance tests including
   RFC-0005's own worked examples and adversarial negative cases. Found
   and fixed one validator bug (`required` wrongly applied to a `null`
   instance under a nullable object type) during testing.
3. **AICAD-039** — `crates/cad-ast`: `Span`/`Spanned<T>`/`LineIndex`
   (byte offset -> 1-based line/column). `crates/cad-lexer`: a
   hand-written scanner producing spanned tokens for the 24 keywords
   already reserved by `specs/language/grammar.ebnf` (deliberately not
   `expose`/`query`/`unsafe`/etc. — not part of the frozen grammar
   artifact yet), bool literals, raw numeric text, strings/raw strings,
   `///` doc comments, and the evidenced operator set; lexical errors as
   `cad_diagnostics::Diagnostic`s (`PARSE-E001..E003`). AST node types
   (`Expr`/`Stmt`/`Item`) deliberately deferred to the parser tasks
   (`AICAD-041`+) that will actually produce them. Found and fixed one
   real lexer bug: the raw-string (`r"..."`) prefix check ran after the
   generic identifier-start check, so raw strings were never recognized.
4. **AICAD-040** — extended `TokenKind::Number` with an optional
   immediately-adjacent unit suffix (`5mm`, `12.4MPa`, `30deg`, per
   RFC-0004 §4), fused only with zero intervening whitespace and **not**
   validated against the unit list (deferred to the future unit registry,
   `AICAD-048`, per RFC-0004 §4's own "may expand without a grammar
   change"). Verified every one of RFC-0004 §4's 26 initial units
   individually. Found and resolved (by evidence, not by arbitrary
   choice) a real spelling collision between the `in` (inches) unit and
   the `in` for-loop keyword — they never compete for the same token
   because of how each can actually appear in valid source; documented
   in-code and tested both directions.

**Batch S2-01 (`AICAD-038` -> `AICAD-039` -> `AICAD-040`) is now
complete.** Per the fixed batch order in the campaign brief, do not begin
Batch S2-02 (`AICAD-041`) in a session that hasn't yet confirmed this
batch is on the canonical branch it will build on.

## Current state / next action

- **Active stage**: Stage 2 (`project/CURRENT_STAGE.md` updated this
  session). Stage 1 is closed (DL-11).
- **Current/next batch**: S2-01 done. **Next is Batch S2-02**
  (`AICAD-041` expression parser and precedence -> `AICAD-042`
  declarations -> `AICAD-043` control-flow syntax), strictly in that
  order, per the campaign brief's fixed batching. Do not start
  `AICAD-044` within that batch.
- **No partial task.** All three of this batch's tasks (038/039/040) are
  fully implemented, tested, reported, and committed.
- **Exact recent test status** (this session's own fresh run, see each
  task's report for the exact commands):
  - `cargo test -p cad-ast -p cad-lexer -p cad-diagnostics`: 7 + 27 + 20 +
    10 = 64 tests, all passing.
  - `cargo build --workspace --all-targets`: clean.
  - `cargo clippy --workspace --all-targets --all-features -- -D
    warnings`: zero warnings.
  - `cargo fmt --all -- --check`: clean.
  - Native `ctest`/full Stage-1 Rust suite was **not** re-run this session
    (no native/kernel code was touched) — last confirmed green at
    `cad4422`/`e05d791` per the Stage-1 independent review.
- **No open regressions.** Two real bugs were found and fixed within this
  same session (the JSON-schema-validator `required`/`null` bug and the
  lexer's raw-string-prefix-ordering bug) — both have permanent regression
  tests, both are described above and in their tasks' own reports.
- **Unresolved owner decisions** (unchanged by this session except D5):
  D3 (sketch entity/object model), D5 **partially** — policy resolved
  (DL-12) but the concrete v1 tolerance constants are NOT yet derived
  (Stage-2 follow-up, see below), D10 (diagnostic code/schema stability —
  `cad-diagnostics` treats every code as provisional per RFC-0005 §7), D11
  (constraint IR/solver independence), D12 (trusted native plugin
  boundary), D15 (plugin runtime). None of these blocked Batch S2-01.
- **D5 status/evidence**: `DECISION_LOG.md#DL-12` records the layered
  Level 1-4 policy and comparison-profile *shape*. The concrete numeric
  constants (linear/area/volume/center-of-mass absolute+relative
  tolerances) still need to be derived from Stage-1's own evidence
  (`project/reports/AICAD-034.md`'s closed-form-vs-OCCT agreement, the
  fillet/chamfer bounding-box tolerance already calibrated there) — **not
  done this session**, since Batch S2-01 was diagnostics/lexer work, not
  `crates/cad-validation`. A future batch (S2-09's execution-determinism
  checkpoint, or whenever `cad-validation` is first implemented) should
  do this derivation and either add the constants directly or escalate
  them to `project/OWNER_DECISIONS.md` per DL-12's own instruction if
  Stage-2 evidence alone doesn't make a specific constant obvious.
- **Pre-existing TASKS.yaml staleness noticed (not fixed, out of this
  batch's scope)**: every Stage-0/Stage-1 task (`AICAD-001` through
  `AICAD-037`) still shows `status: todo` in `project/TASKS.yaml` despite
  being long complete per `project/reports/` and git history — no prior
  session ever updated that field. This session set `status: done`
  correctly for its own three tasks (038/039/040) but did **not** attempt
  to retroactively fix the pre-existing 037 stale entries (out of scope
  for a Stage-2 batch; would be a large, unrelated diff). A future
  session should not trust `status: todo` in `project/TASKS.yaml` alone
  as evidence a pre-038 task is incomplete — check `project/reports/` and
  git history instead.
- **Recommended next action**: start Batch S2-02 (`AICAD-041`) from
  `branch/determined-allen-4r42gi`'s current head (`1c3c767`), after
  resolving the branch-naming situation described above (check if
  `origin/claude/aicad-stage2-dev` or a merge of this branch into `main`
  has happened in the meantime; if not, continue on this branch or your
  own newly-assigned one, based on whichever already carries this
  session's Batch S2-01 commits).

## Environment
Unchanged from Stage 1 (reconfirm rather than assume): Rust 1.98.1
(`rust-toolchain.toml`), edition 2024. This session's own three crates
(`cad-diagnostics`, `cad-ast`, `cad-lexer`) added **zero** third-party
crate dependencies — `Cargo.lock` still has no non-`cad-*` entries as of
this session's own commits (see AICAD-038's report for the reasoning).
No native/OCCT work was touched this session.

## Git identity
This session found no git identity configured in the environment (fresh
container) and explicitly ran `git config user.name
"insightlabs38-pixel"` / `git config user.email
"insightlabs38@gmail.com"` before any commit, per `CLAUDE.md`. All four of
this session's commits use that identity. No hook was bypassed or
modified; `core.hooksPath` was left untouched; `--no-verify` was never
used.
