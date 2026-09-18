# AICAD-108: Add kernel-neutral advanced geometry value, result, report, and execution-IR families

## Status

Done.

## Objective

Establish the kernel-neutral AICAD value/IR families Stage-5 curve, surface,
multi-result-query, topology, raw-handle, adoption, and operation-report
work (`AICAD-109`-`126`) needs to exist before any of it can be
implemented, per `project/DECISION_LOG.md#DL-5`/`DL-24`/`DL-26` and
`project/OWNER_DECISIONS.md#D25`'s "advanced geometry value/IR families"
item. Construction, evaluation, and kernel dispatch for every family added
here are explicitly out of scope — each later task in the fixed batch order
owns that.

## Base / resulting commit

Base: `f70f858` (`origin/claude/aicad-stage5-dev`, AICAD-107).

## What already existed (no new type needed)

Before writing anything, checked what D22's four distinct layers (safe
value / persistent reference / raw handle / kernel-native object) already
had:

- **Kernel-native object** — `cad_kernel_api::{KernelId, KernelVertex,
  KernelEdge, KernelWire, KernelFace, KernelShell, KernelSolid,
  KernelShape, KernelCurve, KernelSurface}` (`AICAD-017`) already fully
  covers this, kernel-neutral, no OCCT type crossing it.
- **Raw/unsafe handle** — `cad_references::raw_handle::{Epoch,
  EpochCounter, RawHandle<T>}` (`AICAD-093`) already implements D22's raw
  tier *generically*: `RawHandle<cad_kernel_api::KernelShape>` (or any
  other kernel-neutral payload) is a real, usable Stage-5 raw topology
  handle today, with no new type required.
- **Persistent semantic reference** — `cad_references::{AnyRef,
  ReferenceRecipe, ConstructionStrategy, ...}` (Stage 4) already covers
  this tier in full.
- **Safe semantic value** — `cad_runtime::value::Value::Geometry(GeomId)` +
  `cad_geometry_api::GeometryGraph` (Stage 2) already covers this tier.

So this task's real gap was narrower than "invent four tiers": the
*curve/surface/multi-result-query/topology-kind/adoption-outcome/
operation-report* **value shapes** that later tasks will build features
against — genuinely missing — not the D22 identity-tier architecture
itself, which was already sound and unchanged by this task.

## What was added

All new, all pure data (no kernel call, no `.aicad` source wiring, no
`GeometryOp`/`GeometryQuery` variant consuming them yet — each module's own
doc comment states exactly which later task owns that):

- **`cad_geometry_api::curve::AnalyticCurve`** (new) — `Line`/`Circle`/
  `Arc`/`Ellipse`, built from `cad_kernel_api::{Point3, Direction3}` +
  `Quantity`. `AICAD-109`'s own job to wire construction/evaluation/kernel
  dispatch.
- **`cad_geometry_api::surface::AnalyticSurface`** (new) — `Plane`/
  `Cylinder`/`Cone`/`Sphere`/`Torus`, same treatment. `AICAD-113`-`116`'s job.
- **`cad_geometry_api::query_result::{QueryOutcome<T>, QueryFailure}`**
  (new) — the multi-solution query result shape: `Solutions(Vec<T>)`
  (including a legitimately empty `Vec` — a real "no solutions" answer,
  never conflated with failure) or `Failed(QueryFailure)` (`Degenerate` /
  `NumericallyUnstable` / `Unsupported` — never silently absorbed into an
  empty solution set). `AICAD-117`/`118`'s job to instantiate for real
  intersection/projection/distance queries.
- **`cad_geometry_api::operation_report::OperationReport<T>`** (new) — an
  owned, `'ctx`-independent snapshot of what a topology-changing operation
  did to its own inputs (`created`/`modified`/`deleted`/`split`/`merged`),
  generalizing the *shape* of what `cad_occt_bridge::Lineage<'ctx>`
  (`AICAD-086`) already exposes as a *live query* for exactly five
  operations, into something any future operation can build once and hand
  off without holding a kernel context open. `AICAD-119`-`124`'s job to
  produce real ones.
- **`cad_geometry_api::adoption::{AdoptionOutcome<T>, AdoptionEvidence,
  AdoptionRejection}`** (new) — D22's explicit raw-to-safe promotion
  contract: `Adopted { value, evidence }` (evidence names which checks
  were actually run — never a bare "trust me" flag) or
  `Rejected(StaleHandle | ValidationFailed | Unsupported)`. `AICAD-122`-
  `124`'s job to wire real validation.
- **`cad_kernel_api::topology::{TopologyKind, ClassifiedShape}`** (new) —
  the six-way kernel-neutral kind tag (`Vertex`/`Edge`/`Wire`/`Face`/
  `Shell`/`Solid`) a future inspection operation pairs with an otherwise-
  unclassified `KernelShape`, plus `From<KernelFace>`/etc. conversions for
  every already-classified handle type. Deliberately an independent
  duplicate of `cad_references::EntityKind`'s six-way spelling, not a
  shared type — `cad-kernel-api` and `cad-references` are deliberately
  unconnected crates (neither depends on the other, `DL-5`), and coupling
  them to share one enum would be a larger architectural change than this
  task's own scope; mirrors `cad_feature_graph::cache`'s own established
  small-duplication-over-cross-layer-coupling precedent. `AICAD-119`-
  `121`'s job to produce real classified shapes.

## Architecture / determinism evidence

- Every new type derives `Debug`+`Clone`+`PartialEq` (+`Eq`+`Hash` where
  the fields allow — `TopologyKind`/`ClassifiedShape`/`OperationReport`/
  `AdoptionEvidence`/`AdoptionRejection`; curve/surface/`QueryOutcome`
  carry `f64` fields transitively and so stay `PartialEq`-only, matching
  `cad_kernel_api::Point3`/`Direction3`'s own identical precedent) —
  satisfies "deterministic and inspectable" directly, with a test per
  module proving equality/inequality/`Debug`-non-emptiness.
- No new dependency on `cad-occt-bridge` anywhere (`cad-geometry-api`'s and
  `cad-kernel-api`'s own `Cargo.toml` are unchanged) — the architectural
  boundary is enforced by the dependency graph itself, the same
  enforcement mechanism `cad-kernel-api`'s own pre-existing module doc
  comment already relies on ("No OCCT... type or enum value appears
  anywhere in this crate").
- `OperationReport<T>`/`AdoptionOutcome<T>`/`QueryOutcome<T>`/
  `ClassifiedShape` are all generic over (or paired with) a caller-supplied
  kernel-neutral payload type, never a native pointer or OCCT handle —
  serialization/caching a value built from these types can never
  accidentally persist a native handle as semantic identity, since none of
  these types can hold one in the first place (no OCCT type is importable
  from either crate).

## Verification

```
cargo fmt --all -- --check                                                     # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings           # clean
cargo test -p cad-types -p cad-geometry-api -p cad-geometry-runtime -p cad-kernel-api   # all ok
cargo test --workspace                                                        # 82 test-result blocks, 0 failed
python3 scripts/ci/semantic_ref_harness.py validate                            # ok
python3 scripts/ci/semantic_ref_harness.py self-test                           # ok
```

New tests: `cad-geometry-api::curve` (+4), `::surface` (+4), `::query_result`
(+5), `::operation_report` (+4), `::adoption` (+5); `cad-kernel-api::topology`
(+4).

## Limitations / follow-up

- No family here is wired into `GeometryOp`/`GeometryQuery`, `.aicad`
  source, or kernel dispatch — by design (each later task's own job; see
  each module's own doc comment for the exact dependency).
- `AnalyticCurve::Arc`'s angle-reference convention is left unspecified
  (documented on the variant) — `AICAD-109`'s own construction concern.
- `TopologyKind` intentionally duplicates `cad_references::EntityKind`'s
  six-way spelling rather than sharing it, to avoid a new cross-layer
  dependency this task's own scope does not justify (documented above).

## Next dependency

Batch S5-03 (`AICAD-109`-`112`, curves + Checkpoint A) depends on
`AICAD-108`.
