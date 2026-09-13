# Session Handoff

## Canonical working branch

`origin/claude/aicad-stage4-transition`

This branch is a strict descendant of the owner-approved Stage-3 `main`
merge `15fc5a37e4382de717e426ccc5317a491be264cd`; it must not be restarted
from `main` or rebased in a way that discards the transition history.

## Current status

**Stage 3 is complete, owner-approved, and merged.** The approval is now
recorded in `project/DECISION_LOG.md#DL-22`, following the same governance
pattern as the earlier stage approvals. `project/CURRENT_STAGE.md` records
the active Stage-3 -> Stage-4 transition state.

**Stage 4 implementation has not started.** `AICAD-080` remains
`status: todo`. The current transition work does not authorize semantic-
reference implementation, Stage-5/6 implementation, CI expansion, or
AICAD-101+ tasks.

## Accepted Stage-3 implementation

The Stage-3 `main` merge is:

`15fc5a37e4382de717e426ccc5317a491be264cd`

Its lineage includes final remediation commit:

`99fb0d3b17000b0a1c6a1a3c175ea16f0d450180`

That remediation connected `ParamModel` and `FeatureGraph` through the real
`cad_cli::parametric_build::ParametricBuildSession` production path, fixed
the discovered parameter-evaluation ordering defect, and added the
in-process selective geometry reuse/recompute proof. The final Stage-3 gate
records PASS after remediation.

The last Stage-3 verification recorded by that remediation remains
historical evidence, not a fresh test run for this transition branch:

- `cargo fmt --all -- --check` — clean;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  zero warnings;
- `cargo test --workspace` — 1047 passed, 0 failed;
- Stage-3 parametric incremental rebuild integration — 4/4;
- Stage-3 ordinary-part integration — 7/7;
- frozen Stage-4 benchmark fixture integration — 20/20.

See the archived original at
`project/reports/archive/stage3/AICAD-079B-INCREMENTAL-REMEDIATION.md` for
exact commands/results.

## Transition work completed so far

### First reconciliation pass

- created/preserved `claude/aicad-stage4-transition` from the exact Stage-3
  `main` merge;
- imported the post-100 architecture/roadmap audit as a frozen, non-
  normative snapshot under `project/planning/roadmap/post100/`;
- preserved Stage-0..3 task reports and gate/checkpoint evidence under
  stage-specific archive directories while retaining compatibility pointers;
- established separate `docs/user/`, `docs/developer/`, and `project/`
  information surfaces;
- preserved the frozen `docs/plan/` bundle in place because moving it would
  create broad historical-reference churn;
- created the Stage-3 -> Stage-4 transition record.

### Second documentation pass

- rewrote the repository root README as a post-Stage-3 project landing page;
- populated the Stage-3 user guide with installation, first-part, language,
  parameters, current Safe CAD modeling, CLI, examples, and troubleshooting;
- documented the critical user boundary that the sketch/constraint engine
  exists but direct source sketch authoring is not yet integrated;
- populated current developer architecture docs for frontend/HIR/runtime,
  Geometry IR, the OCCT boundary, parametrics/incremental rebuild,
  constraints, testing, and contributing;
- documented the remediated incremental path accurately as an in-process
  session capability, not a persistent cross-process cache;
- recorded the already-issued Stage-3 approval as DL-22 without reopening
  or changing any owner architecture decision;
- clarified Stage-3 names/IDs/raw topology selectors versus Stage-4
  persistent-reference identity.

### Third normative specification pass

- restored the full canonical `specs/language/{grammar,semantics,types,diagnostics}` set;
- narrowed canonical grammar/spec wording to current approved source capability;
- reconciled accepted RFCs with current decisions and Stage-3 implementation boundaries;
- preserved exact pre-cleanup Stage-0 RFC bodies under `rfcs/history/stage0/`;
- reconciled stale D5/D10/D11 wording;
- separated D5 geometry-equivalence tolerance from solver, construction,
  approximation, verification, and private-validity thresholds;
- clarified RuntimeBuiltin, spatial-type, sketch/constraint, raw-topology,
  and persistent-reference boundaries;
- clarified that `docs/plan/` is frozen foundation planning rather than
  automatic current normative authority;
- corrected the later stale transition-brief treatment of D20: DL-21 remains
  authoritative, D20 is resolved, and AICAD-076A implements the ruling.

### Fourth owner-decision recording pass

- recorded D21-D30 as **RESOLVED** owner decisions;
- assigned D21-D30 to Decision Log entries DL-23 through DL-32 because
  DL-22 is already the Stage-3 approval record;
- resolved the semantic-baseline questions represented by frozen post-100
  audit recommendations OD-S5-02..OD-S5-06 and OD-S6-01..OD-S6-05 without
  rewriting the frozen audit itself;
- retained every implementation/representation detail the owner explicitly
  deferred for future Stage-4/5/6 evidence;
- added only targeted current spec/RFC cross-references for the new rulings;
- recorded the anti-drift rule: Stage-4 evidence may refine explicitly
  deferred details through review but may not silently weaken D21-D30.

See `project/planning/transitions/stage3-to-stage4/NORMATIVE_SPEC_CLEANUP.md`
and `DEFERRED_TRANSITION_ITEMS.md`.

## Documentation/specification sources and precedence

When current architecture or semantics need verification, prefer:

1. explicit owner decisions and `project/DECISION_LOG.md`;
2. accepted RFC/spec semantics;
3. current code where those specs intentionally define current behavior;
4. final gate evidence;
5. relevant archived task reports;
6. frozen planning prose/examples.

Completed reports/gates are evidence and should be retrieved when relevant;
they are not routine startup context or the current developer manual.

If two owner-level inputs genuinely conflict, record the conflict rather
than choosing silently through implementation or documentation cleanup.

## D20 status — resolved by DL-21

D20 is **resolved**, not open. `project/OWNER_DECISIONS.md` points to
`project/DECISION_LOG.md#DL-21`, and AICAD-076A implements that ruling. The
normative-cleanup brief's later statement that D20 was open was stale
transition wording and has been corrected; it did not reopen or supersede
DL-21.

The authoritative D20 contract is captured in `specs/language/types.md` and
the accepted RFCs: approved standard nominal types may appear in the
always-seeded RuntimeBuiltin catalogue; that environment is type-closed and
eagerly signature-checked; its catalogue/type declarations are independently
type-validatable; `with_geometry_types` remains an idempotent compatibility
helper; and this does not open arbitrary plugin/runtime type injection, host
callbacks, or OCCT types in public/HIR signatures. AICAD-076's scalar
signature decomposition was temporary compatibility work, not the intended
long-term Safe CAD API architecture.

## D21-D30 status — resolved by DL-23 through DL-32

The new owner rulings establish future semantic baselines for closed
RuntimeBuiltin scaling (D21), safe/raw geometry tiers (D22), kernel-backed
source queries (D23), distinct tolerance domains (D24), feature/provenance
preservation through abstraction (D25), assembly identity domains (D26), a
general nominal interface/protocol mechanism (D27), solver-neutral assembly
relations and deterministic observable pose (D28), immutable configuration
overlays (D29), and external-asset content identity/provenance (D30).

These are decisions, not implementation authorization. Their explicitly
deferred representation/syntax/default/scheduling/solver/schema details
remain in `DEFERRED_TRANSITION_ITEMS.md`. Stage-4 evidence may refine those
details only through explicit review; it must not silently weaken the
approved semantic invariants.

## Remaining genuinely open owner decisions relevant to later stages

- D7 automatic fingerprint-recovery policy;
- D8 exact internal OCAF usage;
- D12 trusted native extension/plugin security boundary;
- D13 final public/commercial distribution licensing policy;
- D15 default sandboxed plugin runtime choice;
- later Stage-7+ decisions not covered by D21-D30.

## Known current boundaries

- `.aicad` source has no supported direct sketch-authoring construct even
  though the sketch/constraint subsystem is implemented below the source
  layer;
- Stage-3 raw face/edge integer selectors are not persistent references;
- `FeatureGraph` does not yet provide general interprocedural flattening of
  arbitrary geometry-producing source functions/branches; D25 defines the
  future required observability invariant without implementing it here;
- `ParametricBuildSession` reuses geometry only within the live in-process
  session/kernel context; no persistent disk/remote cache is implied;
- the implemented CLI is `cad build <path.aicad> [--json] [--output <path>]
  [--name <binding>[.<field>]]`; broader planned commands are not current;
- future Stage-5/6 syntax/APIs remain unimplemented even where D21-D30 now
  provide an approved semantic baseline.

## Remaining transition work

The major transition work intentionally remaining is:

1. expanded Stage-4 CI/CD preparation under separate instruction;
2. final Stage-4 initialization and explicit implementation authorization.

Normative specification cleanup and D21-D30 owner-decision recording are
complete. Do not begin AICAD-080 or any Stage-5/6 implementation merely
because those decisions are now recorded.

## Next action

**Owner review of the completed governance/normative transition state.** Do
not start AICAD-080 or expand CI/CD until separately instructed.

## Validation note

The execution environment cannot resolve `github.com` from local Git, so
checkout-dependent startup, `git diff --check`, Cargo formatting/tests, and
repository-local documentation validation cannot be reported as fresh local
runs. Remote GitHub compare/scope plus patch/whitespace validation is used
instead. This owner-decision pass changes no production Rust/build path.

## Git policy

Required commit author/committer identity is
`insightlabs38-pixel <insightlabs38@gmail.com>`. Keep repository hooks
active; never use `--no-verify`, never force-push transition history, and do
not add AI/session attribution metadata.
