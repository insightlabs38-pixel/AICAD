# Stage 3 → Stage 4 normative specification cleanup

Status: **COMPLETE FOR OWNER REVIEW — D20 stale-brief correction applied; no Stage-4 implementation**.

This record covers the third transition pass on `claude/aicad-stage4-transition`: reconcile current normative language/RFC authority after the Stage-3 documentation pass, without starting AICAD-080, expanding CI/CD, modifying production behavior, or promoting the frozen post-100 roadmap into active tasks.

A subsequent narrow correction fixes one error in the transition brief used during that pass: it incorrectly described D20 as open. Repository authority is unambiguous: `project/OWNER_DECISIONS.md` records D20 as **RESOLVED — DL-21**, `project/DECISION_LOG.md#DL-21` contains the owner ruling, and AICAD-076A implements it. The stale transition-brief wording does not reopen or supersede DL-21.

## Scope and authority

The pass used the following precedence when reconciling stale text:

1. explicit owner decisions and `project/DECISION_LOG.md`;
2. accepted RFC/spec semantics;
3. current Stage-3 implementation where the specification intentionally defines current behavior;
4. accepted gate/task evidence;
5. frozen planning examples and the frozen post-100 audit.

The D20 correction applies that precedence directly: DL-21 controls over the later stale transition-brief statement.

## Canonical language specification set restored

`specs/language/README.md` had long claimed four canonical language artifacts while three were absent. This pass restores the complete set:

- `grammar.ebnf` — current source grammar;
- `semantics.md` — current language/evaluation/runtime/geometry-boundary semantics;
- `types.md` — current type/unit/generic/spatial/tolerance boundaries;
- `diagnostics.md` — current structured-diagnostic compatibility contract.

The grammar now describes only parser-supported current declarations (`let`, `const`, `param`, `fn`, `struct`, `enum`, `part`, `import`) and current Stage-2/3 expression/control-flow additions. Reserved/future vocabulary such as interfaces, assemblies, requirements/tests, raw/unsafe geometry, and direct sketch authoring is not represented as current syntax.

## Current-vs-future language boundaries made explicit

The canonical specs/RFCs now state explicitly that:

- internal Stage-3 sketch/entity/constraint/profile infrastructure is real, but `.aicad` does not currently expose direct `sketch { ... }` authoring or source `Sketch`/`Profile` values;
- current Safe CAD RuntimeBuiltins are ordinary source functions over a closed trusted runtime catalogue, not compiler geometry intrinsics;
- internal Geometry IR/kernel capabilities do not automatically become source-visible functions;
- `List<T>` plus integer `Range`/iteration is the approved D16 minimum; `Set`, `Map`, comprehensions, arbitrary user iterators, async/parallel iteration, and dimensional ranges remain future;
- D17 authorizes bare generic parameters plus payload enums/patterns and ordinary `Result`/`Optional`; interfaces/bounds, higher-kinded generics, specialization, and related future type-system features remain future;
- Stage-3 feature IDs/provenance, named outputs, sketch IDs, GeometryGraph IDs, operation-local lineage, and raw face/edge integer indices are not Stage-4 persistent topology identity;
- Stage-4 reference resolution remains fail-closed under D7: `Resolved`, `Ambiguous`, or `Broken`; automatic fingerprint recovery remains disabled for the first implementation unless a later owner decision changes that policy;
- current CLI behavior is `cad build ...`; broader historical command examples are planning, not current implemented interface.

## D20 RuntimeBuiltin type closure — resolved by DL-21

D20 is resolved and is not a remaining transition blocker. DL-21 establishes the following normative state, implemented by AICAD-076A:

- approved AICAD standard nominal types such as `Point3`, `Axis3`, `Frame3`, and `Plane` may appear in always-seeded RuntimeBuiltin signatures;
- the always-seeded builtin environment must be type-closed: every nominal type required by an automatically seeded builtin signature is automatically available to signature checking;
- eager signature collection/type checking remains authoritative;
- the catalogue plus its required standard type declarations must be independently type-validatable;
- `with_geometry_types` may remain as an idempotent compatibility/composition helper rather than a prerequisite for builtin type correctness;
- the environment remains closed and first-party: arbitrary plugin/runtime type injection, host callbacks, and OCCT types in public/HIR signatures are not authorized;
- AICAD-076's scalar-decomposition signatures were temporary compatibility workarounds, not the intended long-term Safe CAD API architecture.

The earlier version of this transition record treated a stale transition-brief sentence as an authority conflict. That treatment is corrected here. No new D20 decision is being made, and `OWNER_DECISIONS.md`, DL-21, and production code require no change.

## Determinism and tolerance cleanup

D5/D19 language is reconciled to the current layered contract:

- AICAD-owned canonical semantic/compiler state is deterministic for identical inputs where canonical serialization is defined;
- exact OCCT B-rep bytes and kernel enumeration order are not cross-platform/cross-version identity;
- supported geometry equivalence uses the versioned D5 comparison profile plus exact semantic requirements where applicable;
- kernel-version changes are compatibility comparisons rather than deterministic identity.

The cleanup explicitly separates numerical categories that had been easy to conflate in older planning prose:

- D5 geometry-equivalence comparison tolerance;
- solver convergence tolerance;
- modeling/construction tolerance;
- approximation/discretization tolerance;
- verification/assertion tolerance;
- private representation-validity thresholds.

The D5 v1 constants remain the calibrated values already accepted/implemented by D19/AICAD-064A. This pass does not invent defaults for the other categories.

## RFC reconciliation and rationale preservation

The Stage-0 RFC packet is now labeled as accepted by DL-10 rather than perpetually `Draft (Stage 0)`. Current RFC text removes stale claims that D5, D10, D11, or D20 are still open and distinguishes accepted architectural intent from unsupported current syntax.

Because the current RFCs required substantial condensation/reconciliation, the exact pre-cleanup Stage-0-era RFC blobs are preserved under `rfcs/history/stage0/`. Those files are design-history snapshots only; their stale open-status statements are not current authority.

## Frozen planning authority clarified

`AGENTS.md` no longer names `docs/plan/` as the unconditional current design source of truth. It now uses the transition authority hierarchy above. `docs/plan/README.md` is marked as a frozen foundation/planning corpus retained at its historical path for reference stability. D14's resolved AICAD branding/file naming is stated at the entry point while historical working-name text remains preserved as history.

The rest of `docs/plan/` is intentionally not rewritten wholesale. Its future-looking syntax and architectural sketches remain available for design archaeology and later roadmap work.

The frozen post-100 audit is likewise not rewritten to retroactively know about later decisions. Its findings remain an accurate snapshot of its audit revision where applicable. Current transition/index material and the live owner decision log establish that D20 was subsequently resolved by DL-21.

## Still intentionally unresolved / future

This pass does not decide or implement:

- Stage-5 raw/unsafe geometry source/effect mechanism;
- source-visible kernel query evaluation architecture;
- scaling the D18 closed RuntimeBuiltin catalogue beyond the current mechanism;
- modeling/construction/approximation/verification tolerance defaults beyond already-approved profiles;
- general feature/provenance identity through arbitrary user functions/control flow;
- assembly identity, relation/solver, or configuration semantics;
- verification/test/requirement language and evidence schemas;
- D7 automatic fingerprint-recovery policy;
- D8 exact internal OCAF use;
- D12/D15 trusted/plugin/native extension boundaries;
- D13 final public/commercial distribution licensing policy;
- any Stage-5+ task promotion or AICAD-101+ assignment.

## Production/stage guard

No `crates/`, `native/`, `.github/`, test, benchmark, task-queue, or Stage-4 production implementation file is changed by this correction. `project/TASKS.yaml` and `project/OWNER_DECISIONS.md` remain unchanged. AICAD-080 remains todo. Stage 4 has not started.

## Validation limitations

A local `git diff --check` run was attempted for this correction, but the execution environment cannot resolve `github.com`, so a repository checkout/ref cannot be reached and the command cannot be truthfully reported as completed. The GitHub connector is used for authoritative remote compare/scope and patch/whitespace review instead.

No active documentation generator is configured for this documentation/specification surface, and this correction changes no production Rust/build path. Historical Stage-3 build/test results therefore remain historical evidence only, not fresh validation claims for this correction.

## Transition result

Normative specification cleanup, including the D20 stale-brief correction, is complete for owner review. Remaining transition work is now:

1. separately authorized Stage-4 CI/CD preparation;
2. final owner-reviewed Stage-4 initialization/authorization.

Until those occur: do not start AICAD-080, do not claim persistent semantic topology references as implemented, and do not auto-merge this transition branch.
