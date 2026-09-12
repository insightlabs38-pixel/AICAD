# AICAD-063: Implement end-to-end source -> exact bracket -> STEP slice

## Status: COMPLETE

## Objective

Per `project/TASKS.yaml` and the active campaign brief's "END-TO-END
STAGE-2 PROOF" section: prove the complete chain `.aicad` source ->
lexer/parser -> binding -> units/type checking -> typed HIR -> ordinary
execution/control flow -> Geometry IR -> Stage-1 kernel API -> exact
bracket -> valid STEP, for a program exercising meaningful combinations
of parameters, engineering units, derived expressions, functions,
ordinary control flow, and geometry operations — with no demo-specific
interpreter shortcut, verified by exact B-rep/analytic evidence (validity,
bounds, dimensions, volume, center of mass, solid count, topology
sanity, STEP re-import), not a render-only check.

## Base commit

`890403d` ("AICAD-062: Create Tree-sitter grammar for supported syntax
subset"), the newest `origin/claude/aicad-stage2-dev` HEAD at session
start. Batch S2-12 (`AICAD-062`) was already complete; this session
executes Batch S2-13 (`AICAD-063`), its own single-task batch, per the
fixed batch order.

## What was implemented

### The fixture: `examples/brackets/stage2_mounting_plate.aicad`

Per `docs/plan/22_REPOSITORY_WORK_PACKAGES.md`'s own repository layout
(`examples/brackets/`), a real, buildable `.aicad` program — a
parameterized mounting plate:

- **Parameters with engineering units** (`param plate_dx: Length =
  80mm;`, ... nine total: plate `dx`/`dy`/`dz`, hole margin/radius, boss
  radius/height, fillet radius, chamfer distance).
- **Derived expressions**: `hole_center_x`'s `margin + spacing * i`
  (`spacing = span / (count - 1)`); `bracket`'s `dx - margin * 2`,
  `dx / 2`, `dy / 2`; `make_through_hole`'s `thickness + overshoot * 2`
  and unary `-overshoot` — all through ordinary `Length`/`Int`
  dimensional arithmetic (`Length * Int -> Length`, `Length / Int ->
  Length`), never hand-converted to a bare float.
- **Functions**: `hole_center_x`, `make_through_hole`,
  `clamp_fillet_radius`, `rounded_plate`, `bracket` — five functions,
  `bracket` calling all four others, each with explicit typed
  parameters/return types.
- **Ordinary control flow**: `if` (`hole_center_x`'s `count <= 1` guard;
  `bracket`'s `HOLE_COUNT > 0` guard around the hole loop), `while`
  (`clamp_fillet_radius`'s radius-halving safety clamp — genuinely
  executes its body once for the fixture's own default parameters, not
  merely present-but-untaken: `2mm` fails `r*2 > plate_dz/3 (≈3.33mm)` on
  the first check, so the loop runs once and settles at `1mm`), `for`
  (`0..HOLE_COUNT` over the hole-cutting loop), `match` (`BOSS_STYLE`,
  an ordinary non-geometry `Int` selector, mirroring
  `tests/parser/corpus/positive/control_flow.aicad`'s own
  `0 => {...} _ => {}` idiom), and `return`.
- **Geometry operations**: `box`, `cylinder`, `transform` (translation),
  `union`, `cut`, `fillet`, `chamfer` — every one an ordinary
  runtime-backed standard function call (`docs/API/safe-cad-api.md`,
  `project/DECISION_LOG.md#DL-15`), called with plain function-call
  syntax, never a special AST node or host callback.

**Scope decision — no rotation, hence no L-shaped wall.** Stage 2's
`transform` is deliberately translate-only (`DL-15`, `docs/API/
safe-cad-api.md`): there is no source-level vector/axis/rotation type to
name a rotation unambiguously yet. `AICAD-034`'s Stage-1 raw-API bracket
used rotation (`Transform::rotation`) to drill holes through a wall whose
thickness ran along Y. Rather than add rotation to the language merely
to make this fixture more visually bracket-like (`AGENTS.md`: "no
speculative future work", "the smallest correct change"), this fixture
uses a flat plate with a central boss instead of an L-shaped wall — a
real, common parametric-CAD shape that every feature (boss placement,
hole axis) can express with translation alone. A future task adding
source-level rotation (its own signature, e.g. `rotate`, per
`docs/API/safe-cad-api.md`'s own note) is free to build a closer
Stage-1-style L-bracket; this is not silently guessed at here.

**Edge selection — raw indices, found empirically, not guessed.**
`fillet`/`chamfer` take a plain `List<Int>` of raw edge indices per
`docs/API/safe-cad-api.md` (`DL-15`'s own deliberate Stage-2 scope: no
semantic-reference layer exists until Stage 4, `DECISION_LOG.md#DL-9`).
The fixture's `[9]` (front-top edge of the plain base box) and `[12]`
(back-top edge, after the first fillet) were **not guessed** — they were
determined by directly reproducing the identical construction sequence
against `cad-occt-bridge`'s own API (a temporary, never-committed
scratch harness: `ctx.create_box(dx, dy, dz)`, enumerate
`edge_count()`/`get_edge(i)`, search by `bounding_box()` for the edge
whose box matches the known analytic front-top/back-top edge — the exact
technique `crates/cad-occt-bridge/src/lib.rs`'s own
`chamfer_single_edge_matches_analytic_volume` test already established),
then hard-coding the resulting indices. Verified stable across 5
repeated runs and across two different candidate fillet radii (`1mm`
pre-clamp-discovery and `2mm`) — `BRepPrimAPI_MakeBox`'s own edge
enumeration order depends on its construction algorithm, not the
magnitudes passed to it, so these indices are expected to generalize to
any positive box dimensions (documented in the fixture's own header
comment, not asserted as a universal kernel guarantee beyond what was
actually tested).

### The proof: `crates/cad-cli/tests/stage2_end_to_end.rs`

Three tests, all driving `cad_cli::build::build_source` — **the literal
`cad build` pipeline**, the same code path `crates/cad-cli/src/main.rs`
runs for a real user (parse -> lower -> type check -> execute top-level
-> dispatch `GeometryGraph` into a real `OcctContext` -> export STEP) —
never a bypassed/alternate path, and never a `call_by_name` invocation of
an otherwise-uncalled function the way `cad-geometry-runtime`'s own
smaller `AICAD-060`-era pipeline proof did (that test's own module
comment explicitly flags itself as "not a substitute for `AICAD-063`'s
own full end-to-end gate"). Because `Interpreter::run_top_level` only
evaluates top-level `let`/`const`/`param` item *values* (not `fn` bodies
unless called), the fixture's geometry-producing statement is a
**top-level `let mounting_plate = bracket(...);`** — the only way `cad
build`'s own real pipeline ever reaches source-level geometry today (no
source-level "build target" designation syntax exists yet, per
`AICAD-061`'s own documented default: "the last `Geometry`-producing node
appended" — here, unambiguously, this one call).

1. **`stage2_bracket_fixture_builds_to_a_valid_exact_brep_and_step`**:
   build succeeds with **zero diagnostics** (not merely `status: Ok`);
   writes a real STEP file; re-imports it through a **second, independent**
   `OcctContext` (proving the file is valid on disk, not merely
   self-consistent in the exporting process's own memory, mirroring
   `cad-geometry-runtime`'s own dispatch-pipeline test); asserts:
   - `is_valid()` and a fully-zero `validate()` report;
   - **closed-form volume** (hand-derived in the test file's own module
     doc comment: `80·60·10 − 80·1²·(1−π/4) − 80·2²/2 + π·8²·12 −
     2·π·4²·10 ≈ 49230.265 mm³ ≈ 4.9230265e-5 m³`), matched to the
     kernel's own computed volume to well inside a 0.1% relative
     tolerance (observed: exact to ~13 significant figures — the
     fillet/chamfer/boss/hole geometry genuinely does not overlap
     anywhere, so no inclusion-exclusion correction was needed, and the
     closed form is not an approximation);
   - **bounding box** `(0,0,0)`–`(0.08,0.06,0.022)` within a `1e-4` m
     tolerance (the boss raises the part's own height above the bare
     plate's `plate_dz`; the same small fillet/chamfer curve-
     approximation noise `stage1_bracket.rs` already documented is
     observed here too, at the same ~1e-7 m magnitude);
   - **exact mirror-symmetry invariant on `center_of_mass().x`**: every
     feature (both holes, the full-length fillet, the full-length
     chamfer, the centered boss) is placed mirror-symmetrically about
     `x = plate_dx/2 = 0.04`, so `com.x` must equal `0.04` exactly
     regardless of the fillet/chamfer/boss/hole volumes' own values —
     held to `1e-6` (observed: exact to 15 significant figures),
     mirroring `stage1_bracket.rs`'s own "exact mirror-symmetry
     invariant" technique; `y`/`z` are only range-checked (the fillet
     removes less material than the chamfer and the boss adds mass
     above the plate, so the true centroid is not at the geometric
     center in those axes, and deriving it exactly was judged
     unnecessary complexity for this task's own evidence requirements,
     exactly the same judgment call `stage1_bracket.rs` itself made for
     `y`/`z`);
   - **solid count via the exported STEP text**: exactly one
     `MANIFOLD_SOLID_BREP(` entity, zero `MANIFOLD_SOLID_BREP_WITH_VOIDS(`
     entities (the through-holes are open to the exterior, not enclosed
     internal cavities), one `CLOSED_SHELL(` — `cad-occt-bridge`'s Rust
     API exposes no direct "solid count" query (confirmed by inspection:
     no such function exists anywhere in `crates/cad-occt-bridge/src/
     lib.rs`, and `BRepAlgoAPI_Fuse`/`Cut` always yield a `TopAbs_COMPOUND`,
     never a bare `TopAbs_SOLID`, per that file's own `union`/`cut` doc
     comments), so this is the same STEP-content technique
     `stage1_bracket.rs` already uses ("contains a manifold-solid/
     brep-with-voids entity") generalized to an exact count rather than
     mere presence — no new native/Rust API was added solely to expose a
     "solid count" query, per `AGENTS.md`'s "smallest correct change" and
     "no speculative future work" (a genuine source-visible
     `solid_count`/`shell_count` geometry query, if ever needed by a
     later task, is its own signature decision, not silently added here);
   - **nontrivial topology**: `face_count() > 6`, `edge_count() > 12`,
     `vertex_count() > 8` (the bare box's own counts) — deliberately not
     exact counts, per `AGENTS.md`'s topological-naming guidance
     (`stage1_bracket.rs`'s own identical judgment call).
2. **`stage2_bracket_fixture_is_deterministic_across_independent_runs`**:
   the same source, built three times independently (each its own fresh
   `Interpreter`/`OcctContext`/output file), produces the same
   `is_valid`/volume result every time (`1e-9` relative tolerance across
   runs) — `D5` Level-1/Level-2 evidence (`project/DECISION_LOG.md#DL-12`):
   not byte-identical B-rep, but the same semantic/numerical verification
   outcome, guarding against nondeterminism entering through iteration
   order, hashing, or environment-dependent behavior anywhere in this
   fixture's own pipeline.
3. **`a_dimensionally_invalid_variant_of_the_fixture_is_rejected_before_execution`**:
   appending a dimensionally invalid `5mm + 2kg` to the fixture fails
   type checking (`UNIT-E104` `DIMENSIONAL_ARITHMETIC_ERROR`, reused
   verbatim from `cad-units` per that crate's established pattern) and
   never reaches execution/dispatch/STEP export (`status: Failed`, no
   artifact written) — the pipeline's ordinary error path still holds at
   this fixture's own larger scale, not just `cad-cli`'s existing small
   smoke tests.

All three tests were additionally run 10x in `cargo test`'s default
parallel mode (`AICAD-036`'s own fillet-concurrency defect is already
fixed by a process-wide mutex; this task's own fixture also calls
`fillet` and independently re-verifies no regression under parallel
execution) with 0 failures.

## Files changed

- Added: `examples/brackets/stage2_mounting_plate.aicad` (the fixture).
- Added: `crates/cad-cli/tests/stage2_end_to_end.rs` (the proof — 3
  tests; no existing file was modified).
- Added: this report.
- `project/TASKS.yaml`: `AICAD-063` status `todo` -> `done`.

No existing crate's source was changed — `cad-cli`'s own `build_source`
pipeline (`AICAD-061`) and every upstream phase it calls
(`cad-parser`/`cad-hir`/`cad-runtime`/`cad-geometry-api`/
`cad-geometry-runtime`/`cad-occt-bridge`) already implemented everything
this fixture needed; this task is a pure proof/fixture addition.

## Exact commands and results

```
$ cargo test -p cad-cli --test stage2_end_to_end
running 3 tests
test stage2_bracket_fixture_is_deterministic_across_independent_runs ... ok
test stage2_bracket_fixture_builds_to_a_valid_exact_brep_and_step ... ok
test a_dimensionally_invalid_variant_of_the_fixture_is_rejected_before_execution ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ for i in $(seq 1 10); do cargo test -p cad-cli --test stage2_end_to_end; done
(10/10 runs, default parallel mode: test result: ok. 3 passed; 0 failed)

$ cargo run -p cad-cli --bin cad-cli -- build examples/brackets/stage2_mounting_plate.aicad --output /tmp/smoke.step
build succeeded
wrote /tmp/smoke.step
$ head -3 /tmp/smoke.step
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Open CASCADE Model'),'2;1');

$ cd tree-sitter-aicad && tree-sitter parse ../examples/brackets/stage2_mounting_plate.aicad
(parses cleanly: zero (ERROR ...)/(MISSING ...) nodes — confirms AICAD-062's
shared-syntax guarantee holds for this fixture too)

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.38s
(clean, zero warnings)

$ cargo test --workspace
Every crate: test result: ok, 0 failed anywhere.
cad-cli: 15 (unchanged, AICAD-061's own) + 3 (new, this task's stage2_end_to_end.rs)
All other crates: unchanged from the AICAD-062 session's own baseline
(cad-hir 197, cad-runtime 89, cad-occt-bridge 84, cad-parser 119,
cad-units 75, cad-lexer 29, cad-kernel-api 23, cad-types 14,
cad-geometry-api 17, cad-ast 7+19, ...).
```

Environment: unchanged from the `AICAD-062` session's own record — Rust
1.98.1 (edition 2024), OCCT 7.6.3, CMake 3.28.3, GCC/G++ 13.3.0, Ubuntu
24.04.4 LTS x86_64; `tree-sitter` CLI 0.27.0 / Node v22.22.2 available and
used only for the confirmatory parse above (not a new Cargo workspace
dependency).

## Numerical precision finding (not a defect — consistent with AICAD-034's own prior finding)

Post-fillet/chamfer curve-approximation noise on the overall bounding box
is observed at the same ~1e-7 m magnitude `stage1_bracket.rs`'s own
"Numerical precision finding" section already documented for a
structurally similar (box + fillet + chamfer) construction — the `1e-4`
m bounding-box tolerance used here is, like that file's own, comfortably
above the observed noise floor, not an arbitrarily loosened check.

## Decisions made (autonomous, within AGENTS.md's allowed scope)

1. **Flat plate + central boss instead of an L-shaped wall** (no
   rotation available in Stage-2 `transform`) — see "Scope decision"
   above. Does not touch public syntax/semantics; uses only the already-
   approved `docs/API/safe-cad-api.md` catalogue as-is.
2. **Proof lives in `crates/cad-cli/tests/`, driving `build_source`
   directly**, not a new top-level `tests/` harness or a `cad-geometry-
   runtime` test — because `cad-cli`'s own `build_source` *is* the
   literal, unmodified `cad build` pipeline the campaign brief's exit
   gate describes, and no other crate owns "run the whole pipeline
   against one file and export STEP" as its own public API.
3. **Raw edge indices (`[9]`/`[12]`) determined by a temporary,
   never-committed discovery harness**, not hand-guessed and not
   committed as a permanent test fixture of their own — consistent with
   `docs/API/safe-cad-api.md`'s own documented Stage-2 limitation (raw
   kernel-enumeration-order selection, pending Stage 4's semantic-
   reference layer, `DECISION_LOG.md#DL-9`).
4. **No new `solid_count`/`shell_count` geometry query was added** to
   `cad-occt-bridge`/`cad-geometry-api` merely to make this task's own
   "solid count" verification point more elegant — the STEP-text
   technique `stage1_bracket.rs` already established was generalized
   instead (see "solid count via the exported STEP text" above). A real
   source-visible solid/shell-count query, if a later task needs one, is
   its own signature decision under `docs/API/safe-cad-api.md`'s existing
   "Deliberately not exposed as Stage-2 source functions" policy, not
   silently added here.

None of these required an `OWNER_DECISIONS.md` entry — none changes
public syntax, typed-unit/affine semantics, the canonical-determinism
contract, the kernel abstraction boundary, or weakens any existing gate/
test (AGENTS.md's "autonomously allowed": "implementation inside approved
interfaces/RFCs", "regression tests for discovered bugs" class of work).

## Regressions

None. Every pre-existing test in every crate still passes unchanged (see
`cargo test --workspace` output above). No existing source file was
modified.

## Known limitations

- The fixture does not exercise `intersect` (the eighth catalogue entry)
  — `union`/`cut`/`fillet`/`chamfer`/`transform`/`box`/`cylinder` are all
  exercised; adding an `intersect` call purely for catalogue-completeness
  was judged unnecessary scope creep for this task's own acceptance
  criterion ("AICAD source ... compiles through HIR/Geometry IR to valid
  exact bracket and verified STEP"), which does not require exercising
  every catalogue entry.
- As documented above, no L-shaped wall (would require source-level
  rotation, not yet available).
- The fixture's own `param` values are fixed defaults — `cad build` has
  no `--param` override mechanism yet (`AICAD-061`'s own documented
  limitation, unchanged); this task's "parameters" requirement is
  satisfied by the `param` *declarations and their typed-unit defaults*
  actually executing through `Interpreter::run_top_level`, not by
  proving an override mechanism that does not exist yet.
- `center_of_mass()`'s `y`/`z` components are range-checked, not
  asserted exactly — see the "exact mirror-symmetry invariant" discussion
  above (the same judgment call `stage1_bracket.rs` itself made).

## Unresolved questions

None requiring owner escalation. No public syntax/semantics changed, no
gate/test weakened, no kernel-specific type introduced above the
adapter, no ambiguous semantic reference silently resolved, and no work
expanded into Stage 3.

## Next action

Per the fixed batch order, the next invocation executes **Batch S2-14**
(`AICAD-064`, "Prepare the Stage-2 owner gate packet" — its own
single-task batch, no new roadmap feature development). `AICAD-065` and
all Stage-3 work remain forbidden until a later explicit owner approval.
