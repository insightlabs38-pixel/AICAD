# Stage-0 paper/spec example — concept walkthrough

Companion to `stage0_paper_example.aicad` (AICAD-012). Maps every concept
required by the Stage-0 exit gate
(`docs/plan/15_IMPLEMENTATION_ROADMAP.md`: "parameters, function, loop,
conditional, sketch, extrusion, semantic query, low-level geometry,
assembly, constraint, and test") to its exact location in the example, and
to the RFC/plan section it is drawn from. This table is the direct input
to AICAD-013's contradiction review.

| Required concept | Location in `stage0_paper_example.aicad` | Source (RFC / plan section) |
|---|---|---|
| Parameters | `Bracket`'s five `param` declarations | RFC-0004 §3-4 (dimensional types, unit literals); `docs/plan/02_LANGUAGE_AND_COMPILER.md` §4 |
| Function | `corner_points` (`pure fn`) and `retaining_hook` (`fn`) | RFC-0001 §7 grammar; `docs/plan/02_LANGUAGE_AND_COMPILER.md` §6 |
| Loop | `for p in corner_points(...) { ... }` inside `Bracket` | RFC-0001 §7; `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §15 |
| Conditional | `let wall = if Product.motor == MotorSize.NEMA17 { 3mm } else { 4mm };` | RFC-0001 §7; `docs/plan/07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md` §10 (configuration-driven conditional, same pattern as Example 2 in `docs/plan/18_REFERENCE_EXAMPLES.md`) |
| Sketch | `sketch(plane = XY) { let r = rectangle(...); symmetric(...); }` | `docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3 (`sketch`, `rectangle`), §4 (sketch constraints) |
| Extrusion | `let base = extrude(profile, distance = thickness);` | `docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3 (`extrude`) |
| Semantic query | `expose mounting_face = query body.faces { planar; normal ~= +Z within 0.1deg; largest(area); };` and `count(query body.faces { cylindrical; ... })` inside the test; also `Motor`'s `mounting_face` export | RFC-0003 §4 (explicit semantic exports), §6 (query model), verbatim from `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §4-6 |
| Low-level geometry | `retaining_hook`: `bspline_curve`, `circle_curve(...).trim(...)`, `sweep`, `unsafe geometry { raw_shape, heal, adopt_validated }` | RFC-0002 §4 (raw-topology safety model); `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §3, §5 |
| Assembly | `assembly ActuatorStage0 { instance bracket = Bracket(); instance motor = Motor(); ... }` | `docs/plan/07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md` §3-6 |
| Constraint | Three distinct constraint domains, deliberately: (1) algebraic — `constraint { width >= 40mm; height >= 30mm; mount_inset < width / 2; }`; (2) sketch — `symmetric(r.left_edge, r.right_edge, about = y_axis);`; (3) assembly — `mate coincident(...)`, `mate concentric(...)` | `docs/plan/08_CONSTRAINTS_REQUIREMENTS_TESTS.md` §2 (all three are named constraint domains sharing one conceptual layer) |
| Test | `test "bracket is valid with four bolt holes" { ... }` inside `Bracket`; `test "assembly has exactly one rotational degree of freedom" { ... }` inside the assembly | `docs/plan/08_CONSTRAINTS_REQUIREMENTS_TESTS.md` §8-10 |

Additional constructs used, present in the plan but not required by the
exit gate's literal list, included because they arise naturally in a
realistic design and exercise more of the frozen RFCs:

- `configuration`/`enum` (drives the conditional) —
  `docs/plan/07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md` §10.
- `component` (external/vendor geometry, `Motor`) —
  `docs/plan/07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md` §3;
  `import_step` per RFC-0002's kernel-independence contract (the imported
  shape is queried through ordinary `query`/`datum_axis`, never through a
  backend-specific type).
- `joint` (revolute) and `requirement` — `docs/plan/07` §6 and
  `docs/plan/08` §7, exercised alongside `mate` to show assemblies compose
  multiple constraint/verification concepts, not just mates.
- `.aicad` extension convention — RFC-0001 §3 (DL-4).
- No mutation of `body` in place anywhere: every `body = ...` and
  `hook = ...` is an explicit rebind of a `var`, per RFC-0001 §5 (DL-2).
  `body.cut(hole(...))` is used once, deliberately, to show the sugar form
  still requires the surrounding `body = ...` rebind to take effect —
  exactly the pattern RFC-0001 §5 specifies.

## Contradiction-review notes carried into AICAD-013

One genuine plan-level underspecification surfaced while writing this
example (not introduced by it): `datum_axis(...)` returns a `Datum`
(`docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3), while `mate concentric(a,
b)` is documented as taking "axes/cylinders"
(`docs/plan/07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md` §5). The plan never
states whether a `Datum` implicitly coerces to the axis reference a mate
expects, or whether a separate accessor is needed. This example follows
the same informality the plan's own reference examples use (e.g.
`docs/plan/18_REFERENCE_EXAMPLES.md` Example 5's `motor.output_axis` /
`shaft.axis` passed directly to `mate concentric` without stating their
exact type). This is **not** a contradiction this example introduces; it
is an existing plan gap, tracked as a note for AICAD-013 rather than
resolved here (resolving it would be a type-system design decision outside
this task's scope, and outside every currently-open
`project/OWNER_DECISIONS.md` item — it may warrant its own future entry if
it blocks real implementation work in Stage 3/6).

A second gap, found during AICAD-013's review, is squarely inside the
already-open `project/OWNER_DECISIONS.md` D3 (sketch entity/object model):
this example's sketch constraint,
`symmetric(r.left_edge, r.right_edge, about = y_axis);`, presumes a
`rectangle(...)`'s `Profile` result exposes its sides as named,
field-accessible sub-entities (`r.left_edge`, `r.right_edge`). Nothing in
`docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3-4 documents such fields —
`rectangle` is only documented as returning an opaque `Profile`. This
phrasing is a plausible reading consistent with D3's "explicit named
source objects" lean (`docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md`
§4.3) but must **not** be read as resolving D3 — it is exactly the
open question D3 already tracks, illustrated concretely rather than newly
discovered. No change was made to the example itself: the alternative
(building the rectangle from four explicitly-named `line(...)` calls with
manual coincidence constraints) would only relocate the same open question
to "how do independently-drawn lines close into one profile," not resolve
it, while making the example considerably longer for no gain in rigor.
