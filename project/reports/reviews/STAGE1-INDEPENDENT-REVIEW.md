# Stage-1 Independent Adversarial Gate Review

This is an independent, adversarial re-review of the Stage-1 exit gate. It
does not extend the roadmap, does not begin AICAD-038, does not mark
Stage 1 owner-approved, and does not modify `project/CURRENT_STAGE.md`.
It reconstructs and verifies Stage-1 evidence directly from the
repository and this session's own tool runs; it does not accept
`project/gates/stage-1-gate.md`'s PASS recommendation on the strength of
its own claims.

## Scope

- **Exact commit reviewed**: `a296891f045aa7a263e2751c1c0effc7d8d97336`
  (`origin/main`, `main`, merge of PR #7 — a `SESSION_HANDOFF.md`-only
  update on top of PR #6's merge `1777d03`, which carried all of Batch 1E,
  `AICAD-034`–`AICAD-037`). Working tree confirmed clean
  (`git status --short` empty) both before and after this review; `git
  fetch origin --prune` confirmed this is the current tip of `origin/main`
  before any work began.
- **No patch commit was made.** This review found no BLOCKER or MAJOR
  defect that was both (a) real and (b) safely patchable within the
  patch policy's constraints (see Findings). One long-standing open item
  (native-code sanitizer coverage, "G2") was *closed with new evidence*
  by this review (see "Sanitizer results" below), but that required no
  source change — only running tools already available in this
  environment. Consequently there is no post-patch commit and no
  second-pass re-review; all findings below apply directly to
  `a296891`.
- **Environment used for verification** (independently confirmed, not
  assumed from prior reports): Ubuntu 24.04 container, `g++`/`clang++`
  13.3.0, CMake 3.28.3, `cargo`/`rustc` 1.98.1 (`rust-toolchain.toml`),
  OCCT 7.6.3 (`libocct-*-dev 7.6.3+dfsg1-7.1build1`, verified via `dpkg
  -l`), `valgrind` 3.22.0. This matches the environment every Stage-1
  task report and the gate packet itself claim to have used.
- **Files/subsystems reviewed directly** (full read, not summarized from
  reports): `native/occt_bridge/include/aicad_occt_bridge.h` (670 lines,
  complete), `native/occt_bridge/src/aicad_occt_bridge.cpp` (1955 lines,
  complete), `crates/cad-occt-bridge/src/ffi.rs` (complete),
  `crates/cad-occt-bridge/src/lib.rs` (the full public API surface, Drop
  impls, and the concurrency regression test), `crates/cad-kernel-api/src/lib.rs`
  and `geometry.rs` (handle/error types, transform math),
  `native/occt_bridge/tests/abi_boundary_test.cpp`,
  `native/occt_bridge/tests/lifecycle_test.cpp`,
  `crates/cad-occt-bridge/tests/stage1_bracket.rs` (with an independent
  hand-derivation of its analytic volume formula — see "Geometry
  findings"), `crates/cad-occt-bridge/tests/adversarial_sweep.rs`,
  `rfcs/0002-geometry-runtime-kernel-abstraction.md`,
  `project/CURRENT_STAGE.md`, `project/TASKS.yaml`,
  `project/DECISION_LOG.md`, `project/OWNER_DECISIONS.md`,
  `project/SESSION_HANDOFF.md`, `project/gates/stage-1-gate.md` and all
  four batch checkpoints, `project/reports/AICAD-033.md`,
  `AICAD-034.md`(referenced)/`AICAD-035.md`/`AICAD-036.md` in full, and
  `.github/workflows/*.yml`. Every other non-kernel crate
  (`cad-references`, `cad-feature-graph`, `cad-diagnostics`, `cad-runtime`,
  etc. — 25 crates) was confirmed by direct inspection to be an unedited
  6-line placeholder stub with no logic, ruling out Stage-2+ scope creep
  by direct evidence rather than trusting the reports' own "no product
  code yet" claim.
- **What was NOT independently re-derived**: the historical ~47%/29-of-40
  concurrency failure rates reported for the pre-fix `BRepFilletAPI_MakeFillet`
  defect (AICAD-036) were not reproduced by reverting the fix, since doing
  so would require modifying source under audit; instead, this review
  verified the *fix* holds (20/20 fresh repeated runs, see below) and
  read the isolation-experiment table as historical evidence, consistent
  with the audit brief's "second-pass" scope (verify the current state,
  not re-manufacture a historical bug).

## Tests executed (fresh, in this session, from a clean rebuild)

```
$ rm -rf native/occt_bridge/build && cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
$ cmake --build native/occt_bridge/build -j$(nproc)
(18 targets, 0 errors)
$ ctest --test-dir native/occt_bridge/build --output-on-failure
100% tests passed, 0 tests failed out of 18

$ cargo fmt --all -- --check          # exit 0
$ cargo clippy --workspace --all-targets --all-features -- -D warnings   # exit 0, zero warnings
$ cargo test --workspace
  cad-kernel-api:            23 passed
  cad-occt-bridge (lib):     84 passed
  cad-occt-bridge (adversarial_sweep): 7 passed
  cad-occt-bridge (stage1_bracket):    3 passed
  (all 25 other crates: 0 passed — confirmed by direct source inspection
   to be unedited placeholder stubs, not silently-empty test suites)
```

All counts match `project/gates/stage-1-gate.md` §3 exactly. Additionally:

```
$ for i in $(seq 1 20); do cargo test -p cad-occt-bridge --lib \
    concurrent_fillet_of_a_concave_edge --release; done
20/20 runs: test result: ok. 1 passed; 0 failed
```
(the batch's own evidence was 15 runs in default/debug mode; this review
re-ran 20 more in `--release`, which exercises different optimizer
codegen for the same race, with the same result: 0 failures.)

An **original, independent adversarial concurrency probe** (not present
in the batch's own evidence) was written and run against the five
kernel operations the batch's own concurrency investigation did *not*
stress-test — `shell`, `offset`, `sweep`, `loft`, `tessellate`, and the
topology-exploration functions — using the same methodology AICAD-036
used to find the fillet defect (independent contexts on independent
`std::thread::scope` threads, validity-checked, not merely
success-checked). 8 threads × 40 rounds per operation (320 concurrent
builds per operation, 1920 total), release mode: **0 failures across
all six operations.** This is bounded, not exhaustive, evidence (see
"Threading findings" below for what it does and does not establish); the
probe file was not committed (no defect was found, so per the patch
policy nothing needed to be added — consistent with the project's own
precedent in AICAD-036 of not committing throwaway isolation
experiments).

### Sanitizer results (new evidence this review contributes)

`project/gates/stage-1-gate.md` §4 (item **G2**) and every batch
checkpoint since `STAGE1-A_KERNEL_BOUNDARY.md` have carried an open item:
*"no fresh valgrind run has been performed since Batch 1A."* This review
closes that gap with fresh evidence:

**Valgrind memcheck** (`--leak-check=summary --track-origins=yes`) on
all 9 substantive native test binaries (`lifecycle_test`,
`abi_boundary_test`, `box_cylinder_test`, `boolean_test`,
`fillet_chamfer_test`, `shell_offset_test`, `step_export_test`,
`step_import_test`, `tessellation_test`):
```
ERROR SUMMARY: 0 errors from 0 contexts   (every binary)
LEAK SUMMARY: definitely lost: 0 bytes / indirectly lost: 0 bytes /
              possibly lost: 0 bytes      (every binary)
```
A small "still reachable" residue (16–752 bytes) appears in every
binary — this is OCCT's/Tcl's own well-known static-initialization
allocation, already correctly identified as such in
`STAGE1-A_KERNEL_BOUNDARY.md` §6, not a leak in this bridge's own code.

**AddressSanitizer + UndefinedBehaviorSanitizer** (new — no prior batch
attempted this): rebuilt the full native test suite with GCC
(`-fsanitize=address,undefined -fno-omit-frame-pointer -O1`, clang's ASan
runtime was unavailable in this container so GCC's was used instead) and
re-ran the full `ctest` suite under it:
```
100% tests passed, 0 tests failed out of 18
```
with zero ASan/UBSan reports (no heap/stack buffer overflow, use-after-free,
double-free, signed-integer-overflow, null-pointer, or misaligned-access
diagnostics from any binary). This is genuinely new, stronger evidence
than any prior batch gathered — G2 should be considered **closed** by
this review, not merely re-flagged as still-open.

This does **not** close G1 (exception-containment reachability — see
below): sanitizers detect memory/UB defects, not whether a specific
`catch (const Standard_Failure&)` branch has ever actually executed.

## Kernel-boundary findings (Audit 1)

**No leakage found.** `grep -rlE` for every major OCCT type/class prefix
(`TopoDS_`, `BRepAlgoAPI`, `BRepPrimAPI`, `BRepFilletAPI`,
`BRepOffsetAPI`, `gp_Pnt`, `gp_Trsf`, `STEPControl`, `Standard_Failure`,
`TopExp`, `TopAbs_`, etc.) across every `.rs` file in `crates/` outside
`cad-occt-bridge` returned exactly one hit:
`crates/cad-kernel-api/src/lib.rs:115`, which is a **doc comment**
(`/// A single point-like topological entity (OCCT's \`TopoDS_Vertex\`,
kept nameless here...)`) explaining *why* the type is deliberately
unnamed — not a leaked type. `cad-kernel-api` carries
`#![forbid(unsafe_code)]` crate-wide and defines only opaque,
backend-independent `KernelVertex`/`KernelEdge`/.../`KernelShape`/
`KernelError` types, matching RFC-0002 §3 and `DECISION_LOG.md#DL-5`
exactly. `native/occt_bridge/include/aicad_occt_bridge.h` (the entire C
ABI) was read in full and contains no OCCT class/enum name anywhere —
only `uint64_t`/`uint32_t` handle fields and a project-defined status
enum. `cad-occt-bridge/src/ffi.rs` mirrors the header 1:1 with
`#[repr(C)]` structs of plain integers; nothing OCCT-shaped crosses into
`lib.rs`'s safe wrapper types (`OcctContext`, `Shape<'ctx>`,
`BoundingBox`, `ValidationReport`, `TriangleMesh` — all pure Rust value
types).

Kernel handles are correctly build/epoch-local: `KernelId{context_id,
slot, generation}` and the native `aicad_shape_handle_t` both encode a
generation counter that increments on every release (native
`ShapeTable::Release`), and `context_id` is a per-process monotonic
counter starting at 1 (0 reserved), so a stale or foreign-context handle
can never alias a live one by value alone. Lineage/history capture is
absent from Stage 1 by design (not attempted, not silently frozen) —
consistent with DL-5's "lineage evidence is a later, above-the-kernel
concern." No Stage-4 semantic-reference-resolution policy (fingerprinting,
OCAF-as-identity, etc.) has been implemented or implicitly decided:
`project/experiments/` contains only its own README placeholder, and
`crates/cad-references` is an unedited stub. **Audit 1: no findings.**

## FFI / ownership / lifetime findings (Audit 2)

Verified directly, not assumed: `native/occt_bridge/tests/abi_boundary_test.cpp`
and `lifecycle_test.cpp` exercise invalid-handle, stale-handle (including
"a released handle does NOT alias a newly-created shape that reuses the
same slot, via generation mismatch"), double-release (rejected as
`STALE_HANDLE`, not a crash or silent no-op), foreign-context rejection,
and wrong-thread rejection, each as a real runtime check verified against
a real call, not a design claim. Every one of the 1955 lines of
`aicad_occt_bridge.cpp` wraps its OCCT-calling body in
`try { ... } catch (const Standard_Failure&) { return
OPERATION_FAILED; } catch (...) { return INTERNAL; }` with zero
exceptions — confirmed by reading the file in full, not sampling. No
STL container, `std::string`, or OCCT-owned object crosses the `extern
"C"` boundary; only `#[repr(C)]`-compatible plain-old-data structs and
opaque pointers do (`file_path` is a raw null-terminated `const char*`;
arrays are caller-owned pointer+length pairs). `OcctContext`'s `Drop`
destroys the native context exactly once (Rust ownership guarantees this
runs at most once), and every `Shape<'ctx>` borrows `'ctx`, so the borrow
checker — not just the native `WRONG_THREAD`/context-liveness runtime
check — prevents a context from being dropped while a `Shape` it produced
is still alive. `Shape<'ctx>::drop` releases the shape and explicitly
discards the status (documented rationale: a release failure here would
indicate a bug this crate's own tests would catch directly, not
something to propagate mid-unwind).

The `tessellate`/`tessellation_get` two-call buffer-out-param protocol
(`crates/cad-occt-bridge/src/lib.rs:1006-1053`) is a genuine
buffer-overflow-by-contract API at the native layer (`out_vertices`/
`out_normals` must each be sized `9 * triangle_count` for the *same*
handle's *most recently* cached tessellation) — verified that the only
caller, `Shape::tessellate`, allocates exactly `vec![0.0; 9 *
triangle_count]` from the count `aicad_occt_tessellate` itself just
returned, immediately before the paired `_get` call, with no intervening
call that could invalidate the cache for that handle (contexts are
single-thread-affine, so no other thread can touch this handle's slot in
between). This is safe as actually used, but it is safe **only** because
`cad-occt-bridge` is the sole permitted caller of this native function —
a correct design given DL-5's crate-boundary contract, not an
accidentally-safe one.

**MINOR — double-context-destroy has no native test coverage.**
`abi_boundary_test.cpp` tests double-*release-shape* explicitly (line 75:
"releasing an already-released handle is rejected as STALE_HANDLE, not a
double-free") but there is no equivalent test for calling
`aicad_occt_context_destroy` twice on the same pointer. Reading
`CheckContext`/`context_destroy` directly: after `delete context`, a
second call reads `context->owning_thread` on freed memory before its
null-check even applies (the null-check only guards a null pointer, not
a dangling one) — a genuine use-after-free **at the raw C ABI level**.
This is not reachable through the safe Rust API (`OcctContext`'s Drop
runs at most once, by construction), so it is not a defect in
`cad-occt-bridge`'s actual public surface, only an untested edge of the
native ABI's own documented contract ("callers must not retain handles
past context destruction" — reasonably read to extend to the context
pointer itself, though the header does not say so as explicitly as it
does for shape handles). Recommend adding this as a documented
NOT-supported case (either an explicit doc-comment sentence, or ideally
a defensive "already destroyed" sentinel if cheap) during hardening mode.
Severity: **MINOR** (no live attack surface through the crate's own
public API; a documentation/test-coverage gap, not a functional defect).

**Audit 2 conclusion**: no BLOCKER or MAJOR finding. The generation-based
handle design is sound and independently verified; exception containment
is complete for every reachable path this review found.

## Threading findings (Audit 3)

`OcctContext` holds a raw pointer field and declares no `unsafe impl
Send`/`unsafe impl Sync` anywhere in the crate (confirmed by `grep -rn
"unsafe impl" crates/ native/` returning zero results workspace-wide) —
Rust's auto-trait rules therefore make `OcctContext` (and, transitively,
`Shape<'ctx>`, which borrows it) neither `Send` nor `Sync` automatically,
at the type level, matching the doc comment's claim and the "conservative
baseline" this audit is instructed to check for. The native
`CheckContext` reads `context->owning_thread` (a `std::thread::id` set
once at construction and never mutated again), so this check itself is
race-free even when called in violation of the affinity contract from a
foreign thread — exactly the situation it exists to catch cleanly rather
than corrupt.

Two real, non-per-context OCCT global-state hazards were found and fixed
by the batch itself, both verified by this review as still fixed:
`STEPControl_Writer`/`Reader`'s shared XSTEP session state (a single
`StepIoMutex`, AICAD-033/035) and `BRepFilletAPI_MakeFillet`'s internal
`ChFi3d` machinery (a single `FilletMutex`, AICAD-036) — the latter
reconfirmed by this review with 20 additional release-mode runs (0
failures) and reviewed line-by-line to confirm the lock genuinely wraps
the entire critical section (construction through `Build()`), not merely
part of it.

**MINOR (downgraded from an initial MAJOR concern by this review's own
new evidence) — concurrency coverage beyond fillet/STEP-I/O is bounded,
not systematic.** The batch's own investigation (AICAD-036) explicitly
disclosed that only `union`/`cut`/`intersect`/`chamfer` were
stress-tested alongside the fillet defect; `sweep`, `loft`, `shell`,
`offset`, `tessellate`, and the topology-exploration functions
(`edge_count`/`get_edge`/`face_count`/`get_face`/vertex and
adjacency queries) were never stress-tested for the same class of
OCCT-internal-global-state defect before this review. This review ran an
original 320-build-per-operation concurrent probe against exactly these
six untested surfaces and found zero failures — real, new evidence that
narrows the risk, but it is one bounded run, not a proof of absence: the
fillet defect itself required specific geometry (a concave/reentrant
edge on a multi-boolean shape, not a bare box) and repeated trials
(~47% failure rate, but only under that specific 3-way-concurrent
condition) to surface at all, so a single 40-round campaign against
simpler fixtures (bare boxes/cylinders) cannot rule out an
equally-narrow defect in, say, `BRepOffsetAPI_ThruSections` or
`BRepOffsetAPI_MakeOffsetShape` under a geometry/concurrency combination
neither this review nor the batch's own testing happened to try.
Recommend, as a Stage-1-hardening (not Stage-1-blocking) condition:
extend the concurrent-stress methodology AICAD-036 established to these
remaining operations using non-trivial (not bare-primitive) fixtures,
and commit the result as a permanent regression the way
`concurrent_fillet_of_a_concave_edge...` already is.

**Audit 3 conclusion**: no BLOCKER. The "conservative baseline" (contexts
not silently promised arbitrary concurrent shared access) holds at both
the type level and the runtime-check level. The one MAJOR-shaped concern
this review went looking for (undiscovered OCCT global-state races in
untested operations) came back with new negative evidence, not
confirmation, so it is recorded as MINOR/residual risk rather than a
blocking finding.

## Geometry correctness findings (Audit 4 / Audit 6)

`stage1_bracket.rs`'s `bracket_is_a_valid_exact_brep_matching_analytic_properties`
test was independently re-derived by hand, not merely re-run, to check
whether its "expected volume" formula is actually correct for the
geometry it claims to model, rather than trusting the code comment:

- Base flange `80×60×10` at origin, wall `80×10×60` at origin: the true
  3D overlap region is `x∈[0,80] ∩ y∈[0,10] ∩ z∈[0,10]` (the wall's Y
  extent intersected with the base's Z extent), volume
  `80·10·10=8000`; union volume `48000+48000-8000=88000` — matches the
  test's own `overlap_volume = BASE_DX * WALL_DY * BASE_DZ` exactly.
- The two base-flange hole pairs (`y=45`, radius 4, drilled along Z
  through 10 units of material) and the two wall hole pairs (`z=35`,
  radius 4, drilled along Y through 10 units of material, after a
  90°-about-X rotation this review confirmed maps `(x,y,z)→(x,z,−y)`,
  correctly turning the cylinder's default +Z axis onto +Y) are all,
  independently verified from their stated centers/radii, geometrically
  disjoint from each other, from the union overlap band, and from the
  fillet/chamfer regions — so the "4 clean cylindrical through-holes,
  each removing `π·r²·10` exactly" claim is not merely assumed, it is
  actually true for these specific placements.
- The fillet is applied to the edge where the wall's back face (plane
  `y=10`, normal +Y) meets the base's top face (plane `z=10`, normal
  +Z) — a true 90° dihedral angle (Y⊥Z), so `r²(1−π/4)` per unit length
  is the exact, not approximate, material added by rounding a
  right-angle reentrant corner. The chamfer edge (wall top face
  `z=60` meets wall front face `y=0`, both box faces, hence also
  exactly 90°) makes `d²/2` per unit length the exact material a
  symmetric chamfer removes. **Both formulas are correct for this exact
  geometry, not merely plausible-looking** — this is a real, checkable
  engineering derivation, not a renamed-primitive test with a fudge
  factor.
- The bounding box, exact mirror-symmetry-on-center-of-mass invariant
  (`com.x == BASE_DX/2` exactly, by construction, independent of the
  fillet/chamfer/hole volumes' own exact values), and the
  greater-than-primitive topology-count assertions (not exact-count
  assertions, correctly avoiding brittle enumeration-order dependence)
  were all reviewed and are sound.

This substantially satisfies Audit 6: the demonstration part is
genuinely nontrivial (union + 4 transformed-cylinder cuts including a
rotated axis + fillet + chamfer), and its verification is analytic/exact,
not render-only or success-only.

The bridge's own operation implementations (Audit 4, sampled across
every operation in the 670-line header and cross-checked against their
1955-line implementation) consistently distinguish construction success
from topological validity (`IsDone()` vs. a caller-invoked
`is_valid()`/`validate()`), correctly reject non-finite/non-positive
inputs before ever calling into OCCT, and use `TopExp::MapShapes`
(de-duplicated) rather than a raw `TopExp_Explorer` for every
"count"/"get" pair — the header/implementation comments show this was
empirically verified (e.g. "24 vs. the correct 12 for a box"), not
assumed, and this review has no reason to doubt that specific claim
given the same pattern is used consistently and correctly throughout.

`adversarial_sweep.rs` (independently re-read and re-run by this review,
not merely trusted from its own report) enforces a real contract
discipline: it explicitly refuses to assert that extreme-but-not-invalid
inputs succeed (an oversized fillet is allowed to fail), while asserting
`KernelError::Internal` must never occur for a merely-extreme input
(reserving `Internal` exclusively for genuine adapter/backend defects) —
this is exactly the AGENTS.md evidence-rule discipline the audit brief
asks reviewers to check for, and it is real, not aspirational: this
review re-ran it and confirms the assertions as written actually enforce
this (not, e.g., accidentally swallowing `Internal` inside a broader
`Err(_) => {}` arm — checked line-by-line).

**MINOR — sweep/loft-specific adversarial cases are documented but not
harness-tested.** The native header's own doc comments state (and the
implementation enforces) that a non-G1-continuous sweep spine or a
mismatched-section-count loft is rejected with `OperationFailed`, but
`adversarial_sweep.rs` does not include sweep/loft in its bounded
campaign (it covers box/cylinder dimensions, boolean tangency/scale,
fillet/chamfer magnitude, and shell/offset delta — not degenerate sweep
paths or loft section mismatches). This is a real, if narrow, coverage
gap relative to the audit brief's Audit 9 list ("degenerate loft
profiles," "degenerate sweep paths"). Non-blocking for Stage 1 (the
Stage-1 proof bracket does not use sweep/loft, and their happy-path
correctness is separately covered by native `sweep_loft_test.cpp` and
Rust unit tests), but recommended for hardening mode.

**Audit 4/6 conclusion**: no BLOCKER or MAJOR. The demonstration part and
its verification are genuine engineering evidence, independently
re-derivable and found correct by this review, not merely internally
self-consistent.

## Silent healing / failure semantics findings (Audit 5)

Searched the entire native and Rust kernel-adapter source for
healing/repair/sewing/tolerance-loosening/retry/fallback patterns. Found
exactly two matches, both of which are explicit statements that **no**
such capability exists yet ("no `heal` operation exists yet," in the
header's `validate()` doc comment, and the same disclosure in
`adversarial_sweep.rs`'s own header comment). No `catch`-then-return-original-shape
pattern, no automatic tolerance broadening, no silent retry-with-different-parameters
loop exists anywhere in the 1955-line implementation (verified by reading
every `catch` block — all 40+ of them follow the identical
`OPERATION_FAILED`/`INTERNAL` two-way mapping with no branch that
manufactures a success). **Audit 5: no findings — this is a clean area,
not merely an unexercised one.**

## STEP interoperability findings (Audit 7)

Read `AICAD-033.md` and `AICAD-035.md` in full and independently
classify the two disclosed verification layers per the audit brief's own
STRONG/MODERATE/WEAK taxonomy, rather than accepting the reports' own
"genuinely independent" framing at face value:

- **Layer 1 (export → re-import through this same bridge, checking
  volume/bbox/face-count/edge-count equality)**: correctly self-disclosed
  by both reports as **WEAK** — same AICAD bridge, same OCCT installation,
  both directions. This is the *only* layer that checks actual re-imported
  geometric properties (volume, bounding box) against the original.
- **Layer 2 (the pure-Python `steputils` parser, applied to the same
  STEP file)**: genuinely independent code with no shared implementation
  with OCCT — a real, different toolchain, satisfying the "different
  toolchain" half of the audit brief's STRONG criterion. However, what it
  actually checks is **exchange-structure/entity-count conformance only**
  (does the file parse as valid ISO-10303-21; do entity-type occurrence
  counts — `ADVANCED_FACE`, `EDGE_CURVE`, `VERTEX_POINT`, etc. — match
  this bridge's own topology counts). It performs **no** independent
  recomputation of volume, bounding box, or any other geometric property,
  and (both reports disclose this correctly) no EXPRESS/AP214
  schema-semantic validation (e.g. it never checks that a
  `CARTESIAN_POINT`'s coordinates are consistent with the `PLANE`/
  `CYLINDRICAL_SURFACE` referencing it).

**MAJOR (classification/framing finding, not a code defect) — the
composite "two disclosed layers" story in `project/gates/stage-1-gate.md`
§2.3/§8 can be read as stronger independent verification than the
underlying evidence actually supports, if a reader does not carefully
separate what each layer checks.** The gate packet's language ("verified
through two disclosed layers... a genuinely independent non-OCCT
structural parse") is not false — both individual-task reports (`AICAD-033.md`,
`AICAD-035.md`) are in fact careful and explicit about exactly this
distinction in their own "What this does/does not prove" sections — but
the *composite claim*, read at the gate-packet level alone, risks
leaving an owner with the impression that the bracket's re-imported
*geometry* (not just its file structure) has been independently
confirmed correct. It has not: **no tool used anywhere in Stage 1,
independent or otherwise, independently recomputes the bracket's volume,
bounding box, or coordinate data from the STEP file and compares it to
the original** — the only volume/bbox comparison (Layer 1) is the
non-independent one. This is precisely the audit brief's warning ("A WEAK
check alone must not be described as strong independent verification")
applied one level up: here it is not a WEAK check being mislabeled STRONG,
but a STRONG-for-structure / WEAK-for-geometry pair of checks whose
combination could be mis-read as fully independent geometric
verification. Recommend, as a Stage-2-or-hardening-mode condition: either
(a) extend the independent parser check to also independently recompute
at least one geometric invariant (e.g. parse `CARTESIAN_POINT` coordinates
for the bracket's known-flat faces and confirm they lie in the expected
planes), or (b) rephrase the gate packet's own top-level summary to state
explicitly, in one sentence, that geometric-fidelity round-trip
verification (as opposed to file-structure verification) is WEAK/
non-independent for all of Stage 1. This is classified MAJOR because it
affects how much confidence the owner should place in a specific,
citable Stage-1 claim, not because the underlying engineering work is
deficient — the individual task reports already contain the accurate,
narrower disclosure this finding asks to be surfaced more prominently.

**Audit 7 conclusion**: one MAJOR (framing/disclosure, not implementation),
correctable without any architecture or test change — a documentation
fix. Both individual STEP task reports (`AICAD-033.md`, `AICAD-035.md`)
were independently confirmed to already contain accurate, non-laundered
disclosure of exactly this limitation in their own text.

## Native robustness / fuzzing findings (Audit 8)

`AICAD-036.md`'s isolation-experiment table (union/cut alone, fillet on
convex edges, chamfer on convex/real-shape edges, concave fillet under
concurrency, the full bracket pipeline under concurrency, and a
zero-concurrency control) was read in full and is genuinely
evidence-based: specific thread/repetition counts, specific failure
counts, and a zero-concurrency control run (300/300 clean) that
correctly isolates the variable under test (concurrency, not the
algorithm itself). The fix (a single `std::mutex` scoped exactly to
`aicad_occt_fillet`, confirmed by the same isolation evidence not to be
needed for `chamfer`) is narrow and evidence-justified, not
"serialize everything to be safe." This review's own 20-repetition
re-run (release mode) and original 6-operation/1920-build concurrent
probe (see "Threading findings" above) corroborate the fix holds and add
new negative evidence for the previously-untested operations.

`adversarial_sweep.rs`'s ~1150 bounded, fixed-seed cases (box/cylinder
scale extremes including `NaN`/`±∞`/`1e-300`/`1e12`; tangent/coincident/
overlapping/disjoint/mixed-scale-`1e-9`-to-`1e6` booleans; impossible
fillet/chamfer magnitudes including negative/zero/`NaN`/10×-oversized;
extreme shell/offset deltas; 500-composition transform chains; unusual
near-degenerate frames) were re-run by this review as part of the
fresh `cargo test --workspace` above (7/7 passed) — this is a real,
reproducible fuzz-adjacent harness, not a report-only claim.

**No hangs, no crashes, no `Internal` misclassifications observed in any
run performed by this review** (the fresh full test suite, the 20-run
fillet-concurrency repeat, the ASan/UBSan-instrumented `ctest` run, or
the original 6-operation concurrent probe). **Regressions added by this
review**: none (no defect was found that warranted one).

**Audit 8 conclusion**: no BLOCKER or MAJOR. See "Threading findings"
above for the one MINOR residual-coverage item (concurrency stress
breadth) and "Geometry correctness findings" for the one MINOR
sweep/loft-adversarial-coverage item.

## Adversarial cases findings (Audit 9)

Covered by the existing harness: zero/near-zero/tiny/huge/mixed-scale
dimensions, tangent/coincident/coplanar-ish booleans (the
`offset_fraction` sweep includes exactly `1.0` = tangent and
`1.0+1e-9` = just-past-tangent), impossible fillet/chamfer, extreme
shell/offset deltas, repeated transforms, unusual frames — all
independently re-run by this review, not merely read. **Not covered**
(consistent with the two MINOR findings above): degenerate/self-
intersecting sweep paths, degenerate loft profiles, open wires used
where a closed wire is expected (partially covered — `make_wire_from_edges`'s
own unit tests separately exercise this, but not inside the bounded
adversarial harness). No BLOCKER-level gap: every adversarial case this
review could construct from already-implemented Stage-1 operations
either matched documented behavior or was already covered.

## Sanitizer / resource safety findings (Audit 10)

See "Sanitizer results" above (valgrind + ASan/UBSan, both clean,
performed fresh by this review). `cargo fmt`/`clippy -D warnings`/full
workspace build+test all reproduced clean from a genuinely fresh
rebuild (`rm -rf native/occt_bridge/build` first), not an incremental
build that might mask a stale-artifact false pass.

## Report claims vs. reality findings (Audit 11)

Sampled and cross-checked `AICAD-033.md`, `AICAD-035.md`, `AICAD-036.md`,
`project/gates/stage-1-gate.md`, and all four batch checkpoints against
the actual current source and this review's own fresh tool runs. Every
sampled test-count, command, and pass/fail claim reproduced exactly (18/18
native, 23+84+7+3 Rust, `cargo fmt`/`clippy` both clean). No instance of
"report says PASS but the test only asserts non-null" was found —
every test this review read in detail (`stage1_bracket.rs`,
`adversarial_sweep.rs`, `abi_boundary_test.cpp`, `lifecycle_test.cpp`)
asserts a specific, checkable property, not merely that a call returned
`OK`. The one framing issue found is the STEP-independence composite
claim already discussed under Audit 7 — a real finding, but about
emphasis/completeness of disclosure at the gate-packet level, not a
fabricated or reproducibility-broken claim (the individual task reports
underneath it are accurate).

**MINOR — `project/reports/AICAD-037.md` does not exist**, although
`project/TASKS.yaml`'s own `AICAD-037` entry specifies
`report: project/reports/AICAD-037.md`. `SESSION_HANDOFF.md` explains
this was a deliberate substitution ("this file + `project/gates/stage-1-gate.md`"
doubles as the report), which is a reasonable call given the gate packet
is itself AICAD-037's own deliverable, but it is an undocumented
deviation from the ticket's own stated `report:` path — a future
mechanical check that expects every `AICAD-NNN.md` report file to exist
would flag this as missing. Recommend adding a one-line stub at that
path pointing to the gate packet, purely for tooling consistency; this
finding does not affect the substance of any evidence.

## Architectural scope-creep findings (Audit 12)

Every one of the 25 non-kernel crates (`cad-references`,
`cad-feature-graph`, `cad-diagnostics`, `cad-runtime`, `cad-compiler`,
`cad-constraints`, `cad-query`, `cad-hir`, `cad-ast`, `cad-lexer`,
`cad-parser`, `cad-types`, `cad-units`, `cad-validation`,
`cad-geometry-api`, `cad-geometry-runtime`, `cad-interchange`,
`cad-artifact`, `cad-provenance`, `cad-requirements`, `cad-assemblies`,
`cad-configurations`, `cad-packages`, `cad-lsp`, `cad-agent-tools`,
`cad-cli`) was directly confirmed (not sampled) to be an unedited 6-line
placeholder (`//! ... placeholder crate created by AICAD-002/AICAD-003
repository scaffolding. No implementation yet.`) with zero test targets,
matching `cargo test --workspace`'s own "0 passed; 0 failed" for each.
`skills/`, `editor/`, `examples/` (beyond the Stage-0 paper example) are
README-only directories. `project/experiments/` (the OCAF-prototyping
area D8 explicitly reserves) contains only its own README, confirming
D8's "internal OCAF usage is prototype-driven, not yet decided" has not
been silently resolved either way. No parser/language semantics, Stage-2
evaluator behavior, feature-DAG semantics, persistent semantic
references, geometry-fingerprint logic, sketch semantics, or solver
semantics exist anywhere in the repository outside `docs/plan/`
(design documents, correctly not implementation) and `rfcs/` (frozen
design decisions, correctly not implementation). **Audit 12: no
findings** — Stage 1 has not silently frozen or pre-implemented any
later-stage architecture.

## Findings summary (all severities)

| # | Severity | Area | File(s) | Summary |
|---|---|---|---|---|
| 1 | MAJOR | STEP independence framing | `project/gates/stage-1-gate.md` §2.3/§8 | The composite "two disclosed layers" claim can be read as independently verifying re-imported *geometry*, but only file-*structure*/entity-counts are independently verified; the only geometry (volume/bbox) comparison is the non-independent self-round-trip. Individual task reports already disclose this accurately; the gate-packet-level summary does not surface it as sharply. |
| 2 | MINOR | FFI test coverage | `native/occt_bridge/src/aicad_occt_bridge.cpp` (`aicad_occt_context_destroy`), `tests/abi_boundary_test.cpp` | Double-context-destroy is untested (unlike double-shape-release, which is tested) and is a genuine native use-after-free if a caller violates the documented single-destroy contract. Not reachable through the safe Rust API. |
| 3 | MINOR | Concurrency coverage breadth | `adversarial_sweep.rs`, AICAD-036 investigation scope | `sweep`/`loft`/`shell`/`offset`/`tessellate`/topology-exploration were never stress-tested for the fillet-class OCCT-global-state defect before this review; this review's own bounded probe found nothing, but one bounded run cannot prove absence of a narrow, geometry-specific race. |
| 4 | MINOR | Adversarial-case coverage | `adversarial_sweep.rs` | Degenerate/non-G1 sweep spines and mismatched-section-count lofts are documented and correctly rejected by the implementation but are not exercised by the bounded adversarial harness. |
| 5 | MINOR | Process/documentation | `project/TASKS.yaml` (AICAD-037 `report:` path), `project/reports/` | `project/reports/AICAD-037.md` does not exist even though the ticket specifies that path; `SESSION_HANDOFF.md` explains the substitution but the file itself is silently absent. |
| 6 | NOTE | Sanitizer coverage (now closed) | native test suite | G2 (no valgrind since Batch 1A) is closed by this review's fresh valgrind + new ASan/UBSan runs, both clean. Recorded here for traceability, not as an open item. |
| 7 | NOTE | Exception-path coverage | native bridge, all operations | G1 (no confirmed genuine `Standard_Failure` throw reached by any test) remains open; this review did not find a new way to trigger one either, consistent with the batch's own honest disclosure. Non-blocking. |
| 8 | NOTE | Determinism/performance baselines | `benchmarks/performance/` | Not yet established; correctly out of Stage-1's own ticket scope (`OWNER_DECISIONS.md` D5 remains open and non-blocking for Stage 1, per RFC-0002 §9). |

No BLOCKER findings.

## Patches applied

None. No finding above met the patch-policy bar of "a clear
implementation bug, safely patchable within already-approved
architecture" — findings 2–5 are coverage/documentation gaps
appropriate for hardening-mode work or a trivial doc fix, not defects
with a failing-test-first fix cycle available; finding 1 is a
recommendation about how a claim is framed, not a code change. Per the
patch policy, coverage gaps that would require *new* test infrastructure
or campaign design (findings 2–4) are exactly the kind of work the
active scheduled-task brief already assigns to "Stage-1 hardening mode,"
not to an audit session — adding them here would exceed this review's
own scope of verifying, not extending, Stage 1.

## Second-pass requirement

Not triggered — no BLOCKER or MAJOR-and-safely-patchable finding exists.
(Finding 1, the one MAJOR, is a framing/disclosure recommendation with no
safe code patch available inside this review's own scope — rephrasing an
owner-facing gate document's summary language is itself an editorial/
communication judgment call for the gate's author or the owner, not a
"fix the failing test" cycle this review can run through the patch
policy's five-step loop.)

## Known kernel limitations (carried forward, independently reconfirmed)

- `G1`: exception containment's `catch (const Standard_Failure&)`
  branches remain unproven reachable by a genuine OCCT throw (open).
- Sweep/loft: no variable-section sweep, no explicit trihedron control,
  ruled-only loft (documented capability boundary, not a defect).
- Shell/offset: OCCT's own documented limitations (>3-edge vertices,
  offset-vs-curvature limits, C0 BSpline unsupported) apply as-is; no
  attempt was made to work around them, correctly.
- No wall-clock/subprocess watchdog exists for adversarial/fuzz test
  runs; the CI job timeout is the only backstop. Reasonable at Stage 1's
  current fuzz-campaign scale (~1150 + this review's own ~1920 cases,
  seconds of wall time), worth revisiting if campaigns grow substantially.
- No STEP EXPRESS/AP214 schema-semantic conformance check exists at any
  layer (see Audit 7 finding).
- No determinism/performance baseline exists (`OWNER_DECISIONS.md` D5,
  correctly still open and non-blocking for Stage 1).

## Unresolved owner decisions

Unchanged from `project/gates/stage-1-gate.md` §7, independently
re-verified against `project/OWNER_DECISIONS.md`'s own current index:
D3, D5, D10, D11, D12, D15 fully open; D7, D8, D13 partially resolved
with an explicitly-tracked residual item; D1, D2, D4, D6, D9, D14 fully
resolved. None of the open items were found by this review to have been
silently resolved, assumed, or worked around by any Stage-1 code — this
was checked directly (e.g. D8/OCAF via `project/experiments/`'s empty
state, D5/determinism via the absence of any benchmark tolerance
constant anywhere in the kernel adapter) rather than taken on faith from
the gate packet's own §7.

## Recommendation

**PASS WITH CONDITIONS.**

The Stage-1 kernel substrate is architecturally correct against its own
frozen contract (RFC-0002 §3/§4, DL-5): this review found zero OCCT-type
leakage above the adapter boundary, a sound generation-based handle
design with no aliasing defect, complete exception containment across
every one of 1955 lines of native implementation, a type-level
(`!Send`/`!Sync`) and runtime-checked single-thread-affine concurrency
contract that holds up under fresh adversarial testing (including this
review's own original 1920-call concurrent probe against previously-
untested operations), zero silent healing/repair anywhere in the
codebase, a genuinely nontrivial and correctly-verified demonstration
part (independently re-derived by hand, not merely re-run), and zero
Stage-2+ scope creep (directly confirmed across all 25 other crates).
Fresh valgrind and — new to this review — ASan/UBSan runs are both
clean, closing the batch's own long-standing G2 item with stronger
evidence than existed before this review.

The reason this is not an unconditional PASS is Finding 1 (STEP
verification framing) and the cluster of MINOR coverage gaps (Findings
2–4): none of these indicate the kernel substrate is unsafe to build on,
but they mean the owner should not read Stage 1's own gate packet as
having independently verified *geometric* round-trip fidelity through
STEP (only file-structural fidelity is independently verified), and
should expect the concurrency-safety and sweep/loft-adversarial evidence
to still be bounded rather than exhaustive when Stage 2 begins exercising
this kernel substrate more heavily.

**Conditions recommended before or during Stage 2** (advisory; this
review does not authorize Stage 2 or record an owner decision):

1. Before treating Stage 1's STEP evidence as covering geometric (not
   just structural) round-trip fidelity, either extend independent
   verification to recompute at least one geometric invariant outside
   OCCT, or explicitly narrow the claim in `project/gates/stage-1-gate.md`'s
   own summary language (Finding 1).
2. During Stage-1 hardening mode (already scheduled per
   `SESSION_HANDOFF.md`), extend the concurrent-stress methodology to
   `sweep`/`loft`/`shell`/`offset`/`tessellate`/topology-exploration using
   non-trivial fixtures, and add sweep/loft-specific degenerate-input
   cases to the adversarial harness (Findings 2–4).
3. Add the missing `project/reports/AICAD-037.md` stub for tooling
   consistency (Finding 5) — trivial, non-blocking.

This recommendation is advisory only. Per `AGENTS.md` and
`project/CURRENT_STAGE.md`, Stage 2 (`AICAD-038` onward) remains blocked
until the owner records a Stage-1 pass decision in
`project/DECISION_LOG.md`. This review does not update
`project/CURRENT_STAGE.md`, does not begin `AICAD-038`, and does not
mark Stage 1 owner-approved.
