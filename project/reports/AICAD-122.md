# AICAD-122: Implement the controlled raw/unsafe geometry tier with epoch-bound handles

## Status

Done. First task of Batch S5-07.

## Objective

Per `project/DECISION_LOG.md#DL-24` (D22): give AICAD a real, source-reachable
raw/unsafe geometry tier — AICAD-owned, opaque at the source/HIR boundary,
kernel-neutral in public semantics, and bound to an explicit owning
build-session epoch, so a stale or foreign-session handle fails
deterministically rather than silently misbehaving.

## Base / resulting commit

Base: `5e65e00` (`origin/claude/aicad-stage5-dev`, AICAD-121).

## What already existed (no new epoch/handle mechanism needed)

`AICAD-093`/`108` already built both halves D22's raw tier needs:
`cad_references::raw_handle::{RawHandle, EpochCounter, Epoch}` (generic,
kernel-neutral, epoch-bound) and `cad_kernel_api::topology::ClassifiedShape`
(a lifetime-free `KernelShape` + its own classified `TopologyKind`). This
task's real gap was narrower: wiring a concrete instantiation through the
source/HIR/runtime boundary and giving `.aicad` source an explicit,
auditable way to enter and read the tier.

## What was implemented

- **`cad_geometry_api::raw::RawGeometry`** (new, `crates/cad-geometry-api`
  now depends on `cad-references`) — `RawHandle<ClassifiedShape>`, the
  concrete raw value type. `RawHandle<T>` gained `PartialEq`/`Eq`
  (bounded on `T`) so `Value` (which must be `PartialEq`) can hold one.
- **`cad_runtime::value::Value::Raw(RawGeometry)`** — a new, un-boxed
  (small, `Copy`-sized payload) `Value` variant. Never constructed from
  source syntax directly.
- **`CheckedType::Raw`** (`cad-hir::typeck`) — a fourth single-opaque-
  nominal type alongside `Geometry`/`Curve`/`Surface`, resolved from the
  bare source name `"Raw"` the identical way. Structurally distinct from
  `Geometry`: passing one where the other is expected is a `TYPE-E418`
  compile error, not a runtime surprise.
- **Two new `RuntimeBuiltin`s** (`cad_hir::builtins`):
  - `enter_raw(target: Geometry) -> Raw` — the sole, explicit entry point.
    `Query`-category (`GeometryQuery::EnterRaw`, new): materializes
    `target`, classifies its `TopologyKind` (`Shape::topology_kind`,
    already existed), and mints the result into a `RawGeometry` against
    the calling session's *current* `EpochCounter`.
  - `raw_topology_kind_of(raw: Raw) -> String` — reads the classified kind
    back, re-checking `raw`'s minting epoch against the session's
    *current* epoch first. New `BuiltinCategory::Raw`: no
    `GeometryGraph`/`GeometryQuery` node, no kernel call (the kind was
    already captured at entry) — pure epoch-checked data access.
- **`Interpreter::epoch_counter`/`with_epoch_counter`** (mirrors
  `with_query_executor`'s existing injection pattern exactly) — `None` by
  default; both raw-tier builtins fail cleanly
  (`RuntimeError::RawTierUnavailable`, `RUNTIME-E144`) rather than minting
  an epoch-less (never-stale) handle. A presented stale/foreign handle
  fails with `RuntimeError::RawHandleStale` (`RUNTIME-E145`, wraps
  `StaleHandle`'s own message).
- **`cad-geometry-runtime`**: `NodeResult::Classified`/`QueryOutcome::
  Classified` (new variants, following `Bool`/`Number`/`Point`/`Text`'s
  own precedent) carry the `ClassifiedShape` result out of
  `dispatch_query`/`OcctQueryExecutor` — no native bridge changes needed
  (`Shape::handle()`/`topology_kind()` already existed).
- **`ParametricBuildSession::rebuild`** now chains
  `.with_epoch_counter(&self.epoch)` alongside the existing
  `.with_query_executor(&query_executor)` — completing the real session
  integration `AICAD-093`'s own report named as future work, for the raw
  tier specifically.

## Tolerance domain

None used — this task performs no geometric construction/comparison, only
identity/epoch bookkeeping.

## Lineage / semantic-reference implications

None: `enter_raw` classifies an existing shape without changing topology,
and a `RawGeometry` has no conversion to/from `AnyRef`/`ReferenceRecipe` (by
design — see `cad_references::raw_handle`'s own "not a second semantic-
reference system"). Stage-4 reference machinery is untouched.

## Verification

```
cargo fmt --all -- --check                                                     # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings           # clean
cargo test -p cad-references -p cad-kernel-api -p cad-occt-bridge \
            -p cad-runtime -p cad-cli                                          # 0 failed
cargo test --workspace                                                        # 1751 passed, 0 failed
```

No native bridge (`native/occt_bridge`) source was touched by this task, so
its own CMake/CTest suite was not re-run (unaffected surface).

New tests (14): `cad-geometry-api::raw` (3, `RawGeometry`-level epoch
proofs); `cad-hir::typeck` (2, `Raw`/`Geometry` cross-type rejection);
`cad-runtime::interp` (6: round-trip within one epoch, missing-executor,
missing-epoch-counter, wrong-context rejection, post-rebuild-advance
rejection — the latter two via `Interpreter::call_by_name`'s own
`Vec<Value>` argument injection, since no `.aicad` source can persist a
value across separate runs); `cad-cli` `stage5_raw_geometry.rs` (3, real
`OcctContext`/`ParametricBuildSession` production-path proof, including the
wrong-context and dropped/rebuilt-owner adversarial cases against two real
independent sessions/rounds).

## Adversarial evidence (AICAD-122's own acceptance criteria)

- **Stale epoch**: `raw_topology_kind_of_rejects_a_handle_after_its_own_
  counter_advances` (`cad-runtime`) and `a_raw_handle_captured_before_a_
  rebuild_is_rejected_by_the_next_round` (`cad-cli`, real production path).
- **Wrong context**: `raw_topology_kind_of_rejects_a_handle_minted_by_a_
  different_epoch_counter` (`cad-runtime`) and `a_raw_handle_from_one_
  session_is_rejected_by_a_different_sessions_epoch` (`cad-cli`, two real
  independent `OcctContext`/`ParametricBuildSession` pairs).
- **Dropped/rebuilt owner**: the `cad-cli` post-`rebuild()` test above —
  the captured Rust `Value::Raw` itself never changes; only the session's
  own current epoch does.
- **Handle reuse**: the same `cad-cli` test asserts rejection twice against
  the same stale handle (not a one-time consumption).
- **Raw passed where safe/persistent identity is required**: `cad-hir`'s
  two type-level tests (`a_raw_value_cannot_be_passed_where_geometry_is_
  expected`, and the converse) — enforced structurally by `CheckedType::
  Raw` being a distinct nominal type, not merely documented convention.

## Examples policy

`enter_raw`/`raw_topology_kind_of` are **not** added to `examples/topology/
topology_construction_basics.aicad`. That file is exercised only through
`cad_cli::build::build_source` (structural build, no real kernel context) —
`AICAD-121` already established this same boundary for its own
`Query`-category inspection builtins (`face_count`/`topology_kind_of`/...),
which are likewise absent from that file and instead proven in a dedicated
`crates/cad-cli/tests/` file against a real `OcctContext`
(`stage5_topology_inspection.rs`). `enter_raw` is `Query`-category for the
identical reason (a real kernel call), so it follows the identical,
already-established precedent: `stage5_raw_geometry.rs` is that dedicated
file for the raw tier.

## Limitations / follow-up

- `RawGeometry` wraps a whole classified shape only — no per-entity (face/
  edge/vertex) raw handle yet. `AICAD-123` (raw editing) is expected to
  extend this as its own construction needs dictate, not guessed here.
- `raw_topology_kind_of` is the only raw-tier read builtin. Further
  inspection/editing/adoption operations are `AICAD-123`/`124`'s own job
  per the fixed batch order.
- `RawGeometry`'s own `Debug` output includes its internal
  `ClassifiedShape`/`Epoch` fields (diagnostic-only; no OCCT type is
  reachable through it either way, so this is not a kernel-neutrality
  concern, only worth noting since D22 says "opaque at the source/HIR
  boundary" — that opacity is enforced at the `.aicad` source/type level,
  not by hiding Rust-internal `Debug` output from embedding code).

## Next dependency

`AICAD-123` (functional raw topology editing with lineage/change evidence)
depends on `AICAD-120` (satisfied) and `AICAD-122` (this task, satisfied).
