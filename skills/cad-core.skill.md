# cad-core.skill.md (Stage-3 draft)

**Status: initial draft, `AICAD-079`.** This is the "core skill" `docs/plan/
11_AI_NATIVE_SKILLS_AND_AGENT_PROTOCOL.md` §1-2 and `docs/plan/
15_IMPLEMENTATION_ROADMAP.md`'s Stage-3 exit gate both call for: teach a
capable coding model with no prior AICAD training enough to build an
*ordinary* mechanical part in the Safe CAD language, from this file alone.

**Scope discipline.** Every statement below describes behavior that
actually exists in this repository today, verified against the compiler/
runtime/CLI source and its own tests — never the aspirational full surface
`docs/plan/` sketches for later stages. Where a plan concept is not yet
implemented, this file says so explicitly rather than describing it as if
it worked (a model that trusts a false claim here will write code that
fails to compile, which is worse than the file staying silent). See
"What this skill deliberately does not cover" at the end.

## 1. The shape of a `.aicad` file

A file is a flat list of top-level items — no module/namespace system yet:

```aicad
param width: Length = 40mm;       // an overridable input
const HOLE_COUNT: Int = 4;        // a fixed compile-time constant
let doubled: Length = width * 2.0;// an ordinary immutable top-level binding
                                   // (`Length * Length` is an area, not a
                                   // `Length` — dimensional arithmetic is
                                   // checked, `UNIT-E110`/`E104`)

fn helper(x: Length) -> Length {
    var total: Length = 0mm; // `var` (mutable, reassignable) is a *statement*,
    total = total + x;       // only valid inside a function body's block —
    return total * 2.0;      // never at the top level or directly in a `part` body.
}

struct Rect {
    width: Length,
    height: Length,
}

enum Outcome<T, E> {
    Good(T),
    Bad(E),
}

part Bracket {
    // a part's own body is the same kind of item list, see §5.
}
```

- Comments are `//` to end of line.
- Every `param`/`let`/`const`/`fn`/`struct`/`enum`/`part` declared at the
  top level is visible to every other top-level item in the same file,
  regardless of declaration order (only the *value* of a `param`/`let`
  binding is unavailable before it is evaluated — see §5).
- `if`/`else`, `match` (with exhaustiveness checking), `while`, `loop`,
  `for x in <range or list>`, `break`, `continue`, and `return` all work
  as ordinary control flow inside a function or a `part` body.
- Numeric literals: `1`, `1.5`, integers/floats; `[e1, e2, ...]` is a
  `List<T>` literal; `a..b` / `a..=b` is a `Range<Int>`/`Range<UInt>`
  (exclusive/inclusive) usable directly in a `for` loop.

## 2. Types

Primitive scalar types: `Bool`, `Int`, `UInt`, `Float`, `Decimal`,
`String`, `Bytes`, `Char`.

**Every physical quantity is a typed engineering unit, never a bare
number.** A dimensioned literal is a number immediately followed by a
unit suffix, with no space:

```aicad
let a: Length = 80mm;   // also: nm, um, cm, m, km, in, ft
let b: Mass   = 2kg;    // also: mg, g, lbm
let c: Angle  = 45deg;  // also: rad
```

Arithmetic between incompatible dimensions is a compile-time type error
(`UNIT-E104`), not a silent unit-stripping cast — `5mm + 2kg` never
compiles. Convert between units of the *same* dimension with ordinary
arithmetic (`80mm` and `8cm` compare/add correctly; the checker tracks
dimension, not the literal spelling).

Generic containers already available: `List<T>`, `Range<Int>`/
`Range<UInt>`, `Optional<T>` (`Some(x)`/`None`), `Result<T, E>` (`Ok(x)`/
`Err(e)`) — ordinary generic enums from the standard prelude, not compiler
magic. Pattern-match them with `match`.

`struct`/`enum` declarations (optionally generic, e.g. `struct
Vector3<T> { x: T, y: T, z: T }`) work exactly like any modern language's:
construct with named-field syntax (`Point3(x = 1mm, y = 2mm, z = 3mm)`),
read a field with `.name`.

## 3. Geometry values and the standard geometry types

A `Geometry` value is an opaque handle to one node in the program's own
build graph — it carries **no** kernel-specific type or raw topology
handle into source (`project/DECISION_LOG.md#DL-15`). You can pass it to
another geometry builtin, bind it with `let`, or return it — nothing else.

These small geometry-vocabulary structs are seeded into every program
automatically (no import needed):

```aicad
struct Vector2<T> { x: T, y: T }
struct Vector3<T> { x: T, y: T, z: T }
struct Point2 { x: Length, y: Length }
struct Point3 { x: Length, y: Length, z: Length }
struct Axis3 { origin: Point3, direction: Vector3<Float> }
struct Frame3 { origin: Point3, x_axis: Vector3<Float>, y_axis: Vector3<Float>, z_axis: Vector3<Float> }
struct Plane { origin: Point3, normal: Vector3<Float> }
```

`direction`/`x_axis`/`y_axis`/`z_axis`/`normal` are plain `Vector3<Float>`
(unitless direction components, e.g. `Vector3(x = 0.0, y = 0.0, z = 1.0)`
for +Z) — not yet a dedicated unit-checked direction type.

## 4. The geometry builtin catalogue (exact, current signatures)

Every one of these is an ordinary function call — no special syntax:

```text
box(dx: Length, dy: Length, dz: Length) -> Geometry
cylinder(radius: Length, height: Length) -> Geometry
plate(width: Length, depth: Length, thickness: Length) -> Geometry
transform(target: Geometry, dx: Length, dy: Length, dz: Length) -> Geometry   // translate only
union(a: Geometry, b: Geometry) -> Geometry
cut(a: Geometry, b: Geometry) -> Geometry
intersect(a: Geometry, b: Geometry) -> Geometry
fillet(target: Geometry, edges: List<Int>, radius: Length) -> Geometry
chamfer(target: Geometry, edges: List<Int>, distance: Length) -> Geometry
extrude(target: Geometry, face: Int, direction: Vector3<Float>, distance: Length) -> Geometry
revolve(target: Geometry, face: Int, axis: Axis3, angle: Angle) -> Geometry
hole(target: Geometry, axis: Axis3, diameter: Length, depth: Length) -> Geometry
pocket(target: Geometry, frame: Frame3, width: Length, length: Length, depth: Length) -> Geometry
mirror(target: Geometry, plane: Plane) -> Geometry
linear_pattern(target: Geometry, direction: Vector3<Float>, count: Int, spacing: Length) -> Geometry
radial_pattern(target: Geometry, axis: Axis3, count: Int, angle: Angle) -> Geometry
shell(target: Geometry, removed_faces: List<Int>, thickness: Length) -> Geometry
```

Notes that matter for getting a first build right:

- **`box`/`plate` are always corner-at-origin.** Place them elsewhere with
  `transform`.
- **`fillet`/`chamfer`/`extrude`/`revolve`/`shell`'s `edges`/`face`/
  `removed_faces` are raw, kernel-enumeration-order integers**, not a
  persistent name — see §6 before using these.
- **`hole`/`pocket`/`mirror`/`linear_pattern`/`radial_pattern` never take a
  raw index** — they place a tool by an absolute `Axis3`/`Frame3`/`Plane`
  (world coordinates) and then boolean-cut/union it, so they stay valid no
  matter what `fillet`/`chamfer`/`shell` already did to `target`. Prefer
  these whenever the plan gives you a choice.
- `hole`'s `depth` is measured along `axis.direction` starting at
  `axis.origin` — for a clean through-hole, start `axis.origin` slightly
  *before* the surface (e.g. `1mm` below it) and make `depth` the material
  thickness plus that overshoot on both ends, so the cut has no coincident
  end faces.
- `radial_pattern(target, axis, count, angle)` returns the union of
  `count` copies of `target`, evenly spaced across `angle` (e.g. `count =
  6, angle = 360deg` is a 6-hole bolt circle at 60° spacing) — the
  original, unrotated `target` is one of the `count` copies.
- `shell`'s Safe CAD `thickness` is always a positive "hollow inward by
  this much" — you never write the kernel's own signed convention.
- There is **no Sketch/Profile source-level type yet.** `extrude`/
  `revolve`'s `profile` is always "a face of an already-built solid,
  selected by raw index" — never a 2D sketch. If a task needs a
  non-primitive 2D profile, it cannot be built today; say so rather than
  inventing sketch syntax (there is none to invent into).

## 5. `part` — grouping a design and naming its outputs

```aicad
part Bracket {
    param width: Length = 80mm;
    param height: Length = 40mm;
    let body: Geometry = box(width, height, 10mm);
}
```

A `part`'s own top-level `let`/`const`/`param`-with-default items are
evaluated once, in source order, and become that part's **named
outputs** — every one of them, not just the `Geometry`-typed ones. This
*is* "semantic output naming" at Stage 3: an ordinary declared name, never
a query or a topology search (`docs/plan/06_REFERENCES_QUERIES_
FEATURE_DAG.md` §11's `explicit` durability level). A `param` with no
default contributes no output (there is nothing to evaluate yet).

Give the solid you actually want built a clear name — `body` is a strong
convention, but any name works, and a part may expose more than one
`Geometry` field (e.g. a mirrored twin) under distinct names.

**Not yet supported:** parameterized part instantiation (`Bracket()`
call syntax — a `part` is declared, not called), and `.`-syntax
source-level field access (`Bracket.body` inside another `.aicad`
expression). The only way to read a part's own named output today is
`cad build`'s own `--name` flag (§7) — there is no in-language way yet.

## 6. Raw-index selection: use it, but narrowly

`fillet`/`chamfer`/`extrude`/`revolve`/`shell` need a face/edge picked out
of `target`'s *current* topology, by the exact integer position the
kernel happens to enumerate it at. This index:

- is **not** a persistent semantic reference — it can silently point at a
  different face/edge after almost any upstream edit (a changed
  dimension, an added feature earlier in the chain);
- is only knowable by actually building the shape and enumerating it (or
  by reusing a mapping someone already verified that way) — never guess.

Until Stage 4 lands proper semantic references, keep raw-index selection
safe by construction:

1. **Prefer the non-index builtins** (`hole`/`pocket`/`mirror`/
   `linear_pattern`/`radial_pattern`) for anything they can express —
   they never go stale.
2. **When you must use a raw index, select it on the plainest, least-
   modified shape you can** — ideally a freshly constructed `box`/
   `cylinder`, before any cut/union/pattern touches it. A `box(dx, dy,
   dz)`'s own face order is fixed and already verified in this
   repository: face `0` = `x=0`, `1` = `x=dx`, `2` = `y=0`, `3` = `y=dy`,
   `4` = `z=0`, `5` = `z=dz`; see `examples/brackets/
   stage2_mounting_plate.aicad` and `examples/enclosures/
   stage3_enclosure.aicad` for edge indices worked the same way.
3. **Never assume a raw index survives a prior `fillet`/`chamfer`/`shell`
   on the same shape** — that operation's own output has new, unverified
   topology of its own.

## 7. Build/test loop

```text
cad build <path.aicad> [--json] [--output <path.step>] [--name <binding>[.<field>]]
```

- With no `--output`, `cad build` only runs the program and reports
  diagnostics (parse/type/unit/runtime errors) — nothing is exported.
- `--output <path>` exports one `Geometry` value to STEP. Without
  `--name`, it exports whichever `Geometry`-producing node was
  constructed *last* anywhere in the program (the original, `part`-naive
  Stage-2 default) — fragile for anything beyond a single-shape script.
- `--name <binding>` or `--name <binding>.<field>` (`AICAD-079`) exports
  a specific named output explicitly: `<binding>` is a top-level `let`/
  `const`/`param` name or a `part`'s own name, `<field>` (only meaningful
  for a `part`) picks one of its named outputs. If `<binding>` is a `part`
  with exactly one `Geometry`-typed output, `<field>` may be omitted; with
  more than one, omitting it is an error naming every candidate rather
  than guessing.
- `--json` renders the same report (`status`/`diagnostics`/`artifacts`)
  as machine-readable JSON instead of human text.

There is no `cad test`/`cad inspect`/`cad docs`/`cad query` yet — `docs/
plan/11_AI_NATIVE_SKILLS_AND_AGENT_PROTOCOL.md` §4's larger tool protocol
is aspirational, later-stage scope. `cad build`'s own diagnostics are the
only structured feedback available today.

## 8. Reading a diagnostic

Every error/warning is `FAMILY-<E|W|I><NNN>` plus a human message — never
a bare string. Families you will actually see today: `PARSE` (syntax),
`TYPE` (type-checking, including argument-count/type mismatches), `UNIT`
(dimensional-arithmetic errors, e.g. `UNIT-E104`), `RUNTIME` (e.g.
unbound name, recursion budget, invalid geometry argument), `GEOM`
(invalid geometry-graph construction), `EXPORT` (STEP-export/`--name`
resolution failures). Fix the *first* diagnostic first — later ones in
the same report are frequently consequences of it, not independent bugs.

## 9. When not to use raw topology

There is no *raw kernel handle* exposed to Safe CAD source at all today
(no OCCT-specific type ever appears in a `.aicad` program,
`project/DECISION_LOG.md#DL-15`) — so "avoid raw topology" is not yet a
choice you can get wrong by reaching for the wrong API. The one
topology-adjacent risk that does exist is §6's raw-index selection: treat
every `fillet`/`chamfer`/`extrude`/`revolve`/`shell` index as disposable,
re-derived evidence, never a name you can rely on across an edit.

## 10. What this skill deliberately does not cover

Not implemented in this repository yet — do not write source that assumes
any of the following exist, and say so rather than guessing at syntax:

- **Sketch/2D-profile source syntax** (`sketch { ... }` blocks, sketch
  constraints reachable from `.aicad` source). The constraint-solver/
  profile-lowering machinery exists internally (`AICAD-072`-`075`) but has
  no grammar/lowering integration yet — `extrude`/`revolve` only ever take
  an existing solid's own face (§4).
- **Assemblies, configurations, mates/joints, package/plugin imports.**
- **`requirement`/`test`/`constraint`/`expose`/`query`/`assembly`/`mate`/
  `instance`** are reserved keywords with no execution semantics behind
  them yet.
- **`cad test`/`cad inspect`/`cad docs`/`cad search`/`cad query`/`cad
  render`/`cad diff`/`cad export`/`cad packages`** and every other tool
  `docs/plan/11_AI_NATIVE_SKILLS_AND_AGENT_PROTOCOL.md` §4 describes —
  `cad build` (§7) is the only command that exists.
- **Persistent semantic references** (`FaceRef`/`EdgeRef`/.../ a query
  language) — Stage 4 scope, not yet built; §6's raw-index discipline is
  the only mitigation available today.

See `examples/brackets/stage3_l_bracket.aicad`, `examples/plates/
stage3_bearing_mount.aicad`, and `examples/enclosures/
stage3_enclosure.aicad` for complete, currently-building ordinary parts
written against exactly this skill file's own rules — and `project/
benchmarks/stage3_core_skill/` for the task briefs those examples answer.
