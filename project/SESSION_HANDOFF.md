# Session Handoff

## Latest: `AICAD-060` PARTIALLY COMPLETE — escalated as `project/
OWNER_DECISIONS.md#D18`. Batch S2-11 (`AICAD-060 -> AICAD-061`) is NOT
complete. **The next invocation to touch Batch S2-11 must resume
`AICAD-060`, not start `AICAD-061`, and must not invent an answer to `D18`
— wait for/apply an actual owner ruling, following the exact `D16`/`D17`
precedent.**

This session started from `06eb780` ("AICAD-059: Create backend-independent
Geometry IR"), the tip of `origin/claude/aicad-stage2-dev` at session
start. The session's container initially checked out `main`; per the
active campaign brief's explicit instruction, the working branch was reset
to the newest `origin/claude/aicad-stage2-dev` before any roadmap work
began (`git checkout -B claude/aicad-stage2-dev origin/claude/aicad-stage2-dev`).
Stage-1 owner approval (`DECISION_LOG.md#DL-11`) and the D5
determinism-equivalence policy (`DECISION_LOG.md#DL-12`) were already
durably recorded from prior sessions and were **not** re-recorded, per the
campaign brief's "Do not duplicate the approval on later invocations."

### What this session did

Started `AICAD-060` ("Implement HIR/runtime geometry dispatch into
Geometry IR/kernel API", Batch S2-11). A dedicated research pass first
mapped `cad-runtime`'s `Interpreter`/`Value`/`NumberValue`, `cad-hir`'s
`HirItem`/`BindingKind`/prelude shapes, `cad-occt-bridge`'s complete
`OcctContext`/`Shape` API, and `cad-kernel-api`'s kernel-neutral
vocabulary, then re-read `project/reports/AICAD-059.md` and the relevant
`docs/plan/` sections.

**Key finding:** calling a function today unconditionally requires a real
AICAD-source `HirBlock` body (`HirItem::Fn::body` is mandatory;
`Interpreter::run_fn_body` always executes it). No `Builtin`/`NativeFn`/
intrinsic-function mechanism exists anywhere in the workspace — the only
"callee dispatched without running a `HirBlock`" precedent is
`BindingKind::EnumVariant`, which is enum-construction-specific. Giving an
`.aicad` program a way to actually *invoke* `box(...)`/`cylinder(...)`/etc.
therefore needs a new binding/dispatch mechanism this task must not
silently invent (`AGENTS.md`/`project/TASKS.yaml`'s own `AICAD-060`
`escalate_if` list: "public syntax/semantics must change beyond an
approved RFC", "an unresolved architecture alternative must be selected").

**Escalated as `project/OWNER_DECISIONS.md#D18`** ("Geometry-operation
invocation mechanism from `.aicad` source"), following the exact `D16`/
`D17` precedent: full audit findings recorded, three live options listed
(a new `BindingKind::GeometryIntrinsic` mirroring `BindingKind::
EnumVariant`'s own precedent; reusing/extending RFC-0002 §4's already-
frozen `unsafe geometry` blocks; deferring language-surface invocation
further), none decided. Did **not** invent an owner answer.

**What was fully implemented and tested regardless** (the half of
`AICAD-060` that needs no owner ruling — see `project/reports/AICAD-060.md`
for full detail):

- **`crates/cad-geometry-runtime`** (previously an empty `AICAD-002`/
  `AICAD-003` placeholder; per RFC-0002 §2's own architecture table, this
  is the "geometry runtime" half's designated crate, distinct from
  `cad-geometry-api`'s pure IR): a `dispatch` module
  (`dispatch_graph(&GeometryGraph, &OcctContext) -> Result<GraphResults,
  DispatchError>`) that walks an already-validated `GeometryGraph` and
  calls the matching `cad-occt-bridge` operation for all 18 `GeometryOp`
  and all 8 `GeometryQuery` variants, resolving `EdgeIndex`/`FaceIndex`
  selectors to real edge/face `Shape`s at dispatch time (never cached, per
  `AGENTS.md`'s raw-topology rule); and a `bridge` module
  (`number_value_to_quantity`) converting a runtime `NumberValue` into a
  Geometry IR `Quantity` — a direct field copy, since both types already
  store a canonical-unit magnitude plus `cad_units::OperandType`
  independently. Proven against a real `OcctContext` with exact B-rep
  validity/closed-form volume/bounding-box/center-of-mass/STEP-export-and-
  reimport evidence (never a render-only check).
- **A real bug found and fixed in already-committed `AICAD-059` code**:
  `cad_geometry_api::ir::GeometryQuery::Tessellate` carried only one
  `deflection` field, but `Shape::tessellate` needs both a linear and an
  angular deflection. Split into `linear_deflection`/`angular_deflection`
  (`Dimension::Length`/`Dimension::Angle`), with a new regression test; no
  existing test referenced the old shape, so this is a clean additive fix.

**Escalations filed this session:** exactly one — `D18` (above).

## Current state / next action

- **Active stage**: Stage 2. Stage 1 closed (DL-11); D5 policy-shape closed
  (DL-12); D16 closed (DL-13); D17 closed (DL-14). **D18 open, escalated
  this session.**
- **Batch S2-10 remains COMPLETE** (`AICAD-059`, unchanged this session).
- **Batch S2-11 is IN PROGRESS, blocked on `D18`**: `AICAD-060` partially
  implemented (dispatcher + bridge complete and tested; language-surface
  invocation escalated). `AICAD-061` has not been started and must not
  begin until `AICAD-060` fully completes.
- **THE NEXT INVOCATION MUST**: check whether `project/OWNER_DECISIONS.md
  #D18` has been ruled on (moved to `project/DECISION_LOG.md` as a new
  `DL-` entry, `D18`'s own status line updated to `RESOLVED`/`PARTIALLY
  RESOLVED`). If ruled on: resume `AICAD-060` implementing whichever
  mechanism the ruling selects (likely adding the actual `box`/`cylinder`/
  etc. callable surface to `cad-hir`/`cad-runtime`, wired to
  `cad-geometry-runtime::dispatch`/`bridge`, which this session already
  built and does not need to be redone), then finish `AICAD-060`'s own
  report/commit, then proceed to `AICAD-061` per the fixed batch order. If
  **not** yet ruled on: do not invent an answer — re-audit only if new
  evidence changed since this session (unlikely), otherwise leave `D18`
  open and stop cleanly rather than guessing, exactly as prior sessions
  did for `D16`/`D17`.
- **Exact recent state**: this session's own fresh runs, all from a clean
  working tree at this session's own final commit: `cargo build -p
  cad-geometry-api -p cad-geometry-runtime` clean; `cargo test -p
  cad-geometry-api -p cad-geometry-runtime` — 17 + 9 passed, 0 failed;
  `cargo fmt --all -- --check` clean (after one `cargo fmt --all` pass);
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  clean; `cargo test --workspace` — every crate "test result: ok", 0
  failures anywhere (unchanged from the `AICAD-059` session's own baseline
  except the two crates this session touched: `cad-geometry-api` 16 -> 17,
  `cad-geometry-runtime` 0 -> 9).
- **No open regressions.**
- **Escalations filed this session**: `D18` (see above), recorded in
  `project/OWNER_DECISIONS.md`, not yet in `project/DECISION_LOG.md` (no
  owner ruling has occurred yet).
- **Unresolved owner decisions**: D3, D5 (concrete tolerance constants
  only), D10, D11, D12, D15 (all unchanged, pre-existing), plus the new
  **D18** this session added.
- **D5 status/evidence**: unchanged from prior sessions' own findings.
  This session's `cad-geometry-runtime::dispatch` module makes real kernel
  calls (the first Stage-2 code to do so), but every test asserts either
  exact B-rep validity or a closed-form/analytic numeric comparison with an
  explicitly documented and justified tolerance (never a bare "looks
  right" check) — consistent with `DECISION_LOG.md#DL-12` Level 2's "same
  semantic/numerical verification profile, byte-identical B-rep not
  required." Concrete D5 tolerance constants (the still-open half of D5)
  remain undetermined, unchanged.
- **Pre-existing `TASKS.yaml` staleness** (unchanged, not this session's
  scope): `AICAD-001` through `AICAD-037` still show `status: todo` despite
  being long complete. `AICAD-060`'s own `status:` field is correctly left
  `todo` (it is genuinely incomplete).
- **Recommended next action**: check `project/OWNER_DECISIONS.md#D18`'s
  status; if resolved, resume `AICAD-060` per the notes above; if not,
  stop cleanly rather than guessing.

## Environment

Unchanged from prior sessions (reconfirmed at session start): Rust 1.98.1
(auto-installed via `rustup`, matching `rust-toolchain.toml` — the
toolchain does not persist across sessions in this container), edition
2024. This session added one new populated crate
(`crates/cad-geometry-runtime`, previously an empty placeholder) depending
on `cad-ast`, `cad-diagnostics`, `cad-geometry-api`, `cad-kernel-api`,
`cad-occt-bridge`, `cad-runtime`, `cad-types`, `cad-units`. Zero new
third-party (crates.io) dependencies. No `specs/language/grammar.ebnf`
change (no new syntax was added this session — the surface-syntax question
is exactly what was escalated as `D18`, not implemented). This session
made real native-bridge (`crates/cad-occt-bridge`, `native/occt_bridge`)
calls for the first time from a Stage-2 crate, but did not modify either
of those crates.

## Git identity

This container's global git config is `Claude <noreply@anthropic.com>`
(with `user.useConfigOnly=true` and `core.hooksPath` pointed at the repo's
identity-enforcing hooks) — matching every prior session's own finding, and
**not modified** by this session (per `CLAUDE.md`/`AGENTS.md`: "NEVER
update the git config"). This session's commit sets `GIT_AUTHOR_NAME`/
`GIT_AUTHOR_EMAIL`/`GIT_COMMITTER_NAME`/`GIT_COMMITTER_EMAIL` to
`insightlabs38-pixel`/`insightlabs38@gmail.com` as process-local
environment variables for the `git commit` invocation only — the repo's
`prepare-commit-msg` hook passed without needing any config change. No
hook was bypassed or modified; `core.hooksPath` was left untouched;
`--no-verify` was never used.
