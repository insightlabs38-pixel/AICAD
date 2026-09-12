# Stage-2 Safe CAD source API

Status: Stage 2, authoritative for the current Stage-2 window.
Owner ruling: `project/DECISION_LOG.md#DL-15` (resolving
`project/OWNER_DECISIONS.md#D18`).
Implementing crate/task: `AICAD-060`, `crates/cad-hir/src/builtins.rs`
(the machine-readable catalogue this document renders), `crates/
cad-runtime/src/interp.rs` (`Interpreter::dispatch_builtin`), `crates/
cad-geometry-runtime/src/dispatch.rs` (the kernel dispatcher).

## What this document is

This is the human-readable rendering of `cad_hir::builtins::catalogue`, the
single authoritative signature table both the type checker (`cad_hir::
typeck::Checker::collect_signatures`, via ordinary `resolve_type_ref`) and
the runtime (`cad_runtime::interp::Interpreter::dispatch_builtin`) consume.
If this document and that catalogue ever disagree, the catalogue is correct
and this document is stale — update this document to match it, never the
reverse.

Per `DL-15`: **this catalogue is not the Geometry IR.**
`cad_geometry_api::ir::GeometryOp`/`GeometryQuery` is an internal,
backend-independent execution representation that may be refactored,
split, combined, or extended without automatically changing this public
source-level surface. Stage 2 exposes only the operations needed to prove
the Stage-2 language-to-geometry slice (`AICAD-063`'s bracket proof) — not
a 1:1 mirror of every `GeometryOp`/`GeometryQuery` variant the dispatcher
already implements.

## Mechanism: runtime-backed standard functions, not a compiler intrinsic

Every function below is called with **ordinary function-call syntax** —
`box(10mm, 10mm, 10mm)`, `cut(a, b)`. No new grammar, no new import syntax,
no `unsafe geometry { ... }` block. Each resolves through ordinary lexical
name binding, is type-checked through the exact same call-checking
machinery as an AICAD-defined `fn`, and executes through ordinary
`HirExpr::Call` — the only difference from an AICAD-source function is
that its implementation is provided by the runtime
(`cad_hir::hir::FunctionImplementation::RuntimeBuiltin`) rather than a
`HirBlock` the interpreter executes. This is **not** a compiler intrinsic
in the `project/DECISION_LOG.md#DL-7` sense (no new syntax/typing/lowering
rule unavailable to an ordinary function); `DL-7`'s RFC-gated intrinsic
process is unrelated and unweakened by this mechanism.

`BuiltinFnId` (`cad_hir::builtins::BuiltinFnId`) is a **closed** set — there
is no way for a package, a plugin, or AICAD source itself to register a new
runtime-backed function. Adding one requires adding a variant to that enum
and a corresponding dispatch arm in `Interpreter::dispatch_builtin`, i.e. a
compiler-team change, not an extension point exposed to users.

## The `Geometry` type

Every function below consumes and/or produces a `Geometry` value — a
single, opaque, nominal Stage-2 type (`cad_hir::typeck::CheckedType::
Geometry`). Stage 2 does not distinguish solid/wire/face/edge at the
type-checker level: every Safe CAD function uses this one type, and
whether a particular combination is geometrically sensible (e.g. calling
`fillet` on a face rather than a solid) is left to kernel-time validation
(a `cad_geometry_runtime::dispatch::DispatchError::Kernel`), not caught at
compile time. A `Geometry` value's only representation is
`cad_runtime::value::Value::Geometry(cad_geometry_api::GeomId)` — an opaque
SSA-node index into the interpreter's own accumulated `GeometryGraph`.
**It never carries an OCCT object, a raw kernel topology pointer, or
persistent identity derived from one** — realizing it into an actual
kernel `Shape` happens later, and separately, when a caller (e.g. the
eventual `cad-cli` build command) runs `cad_geometry_runtime::dispatch::
dispatch_graph` against a real `cad_occt_bridge::OcctContext`.

## Catalogue

| Function | Signature | Maps to |
|---|---|---|
| `box` | `(dx: Length, dy: Length, dz: Length) -> Geometry` | `GeometryOp::Box` |
| `cylinder` | `(radius: Length, height: Length) -> Geometry` | `GeometryOp::Cylinder` |
| `transform` | `(target: Geometry, dx: Length, dy: Length, dz: Length) -> Geometry` | `GeometryOp::Transform` (translation only — see below) |
| `union` | `(a: Geometry, b: Geometry) -> Geometry` | `GeometryOp::Union` |
| `cut` | `(a: Geometry, b: Geometry) -> Geometry` | `GeometryOp::Cut` |
| `intersect` | `(a: Geometry, b: Geometry) -> Geometry` | `GeometryOp::Intersect` |
| `fillet` | `(target: Geometry, edges: List<Int>, radius: Length) -> Geometry` | `GeometryOp::Fillet` |
| `chamfer` | `(target: Geometry, edges: List<Int>, distance: Length) -> Geometry` | `GeometryOp::Chamfer` |

### `box(dx, dy, dz) -> Geometry`

An axis-aligned box, corner at the origin, spanning `[0, dx] × [0, dy] ×
[0, dz]` (`OcctContext::create_box`'s own convention).

### `cylinder(radius, height) -> Geometry`

A capped cylinder, axis along +Z (`OcctContext::create_cylinder`'s own
convention).

### `transform(target, dx, dy, dz) -> Geometry` — deliberately translate-only

**Stage-2 narrowing, not the full `cad_kernel_api::Transform`.** The kernel
adapter's `Transform` can also rotate and compose arbitrary rigid motions
(`Transform::rotation`, `Transform::compose`, `Transform::from_frames`),
but no AICAD source-level vector/axis/rotation type exists yet to name a
rotation unambiguously (there is no `Vector3`/`Axis3`/angle-around-axis
surface syntax in Stage 2). Per `DL-15`'s own instruction ("if an exact
signature cannot be derived unambiguously... document the ambiguity and
escalate rather than guessing"), this function is scoped to the one
unambiguous case: a rigid translation by `(dx, dy, dz)`. A future task
introducing source-level rotation needs its own signature (a distinct
function name, e.g. `rotate`, rather than overloading `transform` — no
such name is reserved by this document) once AICAD gains a way to name an
axis/angle unambiguously in source; that is out of scope here and not
silently guessed at.

### `union`/`cut`/`intersect(a, b) -> Geometry`

Ordinary Boolean operations; `cut` is `a - b`.

### `fillet(target, edges, radius) -> Geometry` / `chamfer(target, edges, distance) -> Geometry`

`edges` is a plain `List<Int>` of raw edge indices — exactly the `usize`
selector `cad_geometry_api::ir::EdgeIndex` already is, carried as an
ordinary integer list rather than any new "edge reference" type. This
exposes no kernel handle to source, only integers the author picks (today,
by knowledge of the target's own topology-enumeration order — the same
"raw, ephemeral, kernel-enumeration-order" selection `cad_geometry_api::ir`
already documents as a known Stage-2 limitation, not a durable semantic
reference; the Stage-4 semantic-reference layer is expected to supersede
this with stable identity-preserving selection, per `project/
DECISION_LOG.md#DL-9`).

## Deliberately not exposed as Stage-2 source functions

Per `DL-15`, the following do **not** become ordinary Stage-2 source
functions merely because a corresponding `GeometryOp`/`GeometryQuery`/
kernel operation already exists:

- `export_step` / `import_step` — STEP export for the `AICAD-063` gate may
  remain part of the build/output pipeline rather than an arbitrary
  source-level I/O operation, avoiding accidental file-I/O/effect semantics
  in the language. (`ImportStep`/`ExportStep` remain available to a
  build-pipeline caller directly via `cad_geometry_api::ir::GeometryOp::
  ImportStep`/`GeometryQuery::ExportStep` and `cad_geometry_runtime::
  dispatch`.)
- `tessellate` — a display/export-pipeline concern, not ordinary modeling.
- Low-level edge/wire construction (`LineEdge`, `CircleWire`,
  `WireFromEdges`, `MakeFace`, `Extrude`, `Revolve`, `Sweep`, `Loft`,
  `Shell`, `Offset`) and low-level raw topology traversal — not required by
  the Stage-2 slice this catalogue is scoped to prove; may be added by a
  later task if a concrete Stage-2 (or later) requirement needs one, each
  evaluated on its own signature the same way this document evaluates
  `transform`'s.
- `validate`/`adopt_validated` and other raw/unsafe-tier operations — these
  belong to RFC-0002 §4's `unsafe geometry { ... }` Tier-C mechanism, kept
  semantically distinct from this Tier-B catalogue.
- Geometry queries (`is_valid`, `volume`, `area`, `bounding_box`,
  `center_of_mass`) — `DL-15` explicitly does not require these to become
  source-visible in Stage 2. They may use this same runtime-backed-function
  architecture later; if letting ordinary source control flow depend on a
  kernel-evaluated query result needs a materially different execution/
  evaluation model (e.g. because it would require dispatching to the
  kernel *during* interpretation rather than after it, as this task's own
  two-phase design assumes), that is its own future architecture decision,
  not something folded silently into this one.

## Determinism / resource accounting

Every call listed above is charged against `cad_runtime::interp::
Interpreter`'s ordinary `ResourceBudget` (recursion-depth accounting) via
`enter_call`/`exit_call`, exactly like an AICAD-source function call — it
does not bypass execution budgets merely because its implementation is
runtime-provided (`DL-15`). Building a `GeometryGraph` node is itself a
pure, deterministic, in-memory operation (an ordinary `Vec` append with
structural validation, per `cad_geometry_api::ir`'s own D5 evidence) — no
kernel call, no nondeterminism, happens during this phase. The later,
separate `cad_geometry_runtime::dispatch::dispatch_graph` phase is where
real kernel calls happen, governed by `project/DECISION_LOG.md#DL-12`'s
own layered determinism policy (Level 2: same locked kernel environment,
semantic/numerical verification, no byte-identical B-rep requirement).
