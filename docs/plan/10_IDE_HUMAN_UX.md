# 10 — Human IDE, Visual CAD UX, REPL, and Debugging

## 1. Product principle

Humans should not have to choose between "code CAD" and "normal CAD." The IDE should provide text, structured inspectors, visual manipulation, and feature/assembly views over the **same semantic model**.

No hidden model state should exist solely in the GUI.

## 2. Primary workspace

Recommended layout:

```text
+------------------------+----------------------------------+
| Project / feature DAG  | 3D viewport                      |
|                        |                                  |
| Source modules         |                                  |
+------------------------+----------------------------------+
| Code editor            | Inspector / params / constraints |
|                        |                                  |
+------------------------+----------------------------------+
| Diagnostics / tests / build / REPL / timeline            |
+-----------------------------------------------------------+
```

Every pane is optional/dockable; the product can offer simplified workspaces for code-first and visual-first users.

## 3. Source <-> geometry mapping

### Source to geometry

Selecting:

```aicad
housing.mounting_face
```

highlights the face and displays:

```text
semantic id
current geometry
feature that created it
features consuming it
requirements/tests using it
drawing dimensions using it
manufacturing/analysis dependencies
reference durability
```

### Geometry to source

Clicking a face in viewport provides:

- semantic path if one exists;
- strongest query that currently identifies it;
- creating/modifying features;
- "copy semantic reference";
- "create named reference";
- jump to source.

## 4. Bidirectional parameter editing

Dragging or editing a visual dimension must generate a structured source edit, e.g.:

```diff
- param width = 80mm;
+ param width = 92mm;
```

If a value is derived:

```aicad
width = base_width * scale;
```

the UI should show that the dimension is derived and offer:

- edit upstream parameter;
- convert to independent parameter;
- override in configuration (if permitted).

Never silently replace an expression with a literal.

## 5. Visual feature creation

Visual commands should produce normal source constructs:

```text
select face -> Hole -> set diameter/depth
```

becomes a source transaction using a semantic reference or stable query.

The user should be able to preview the generated source diff before commit in advanced mode.

## 6. Sketch editor

A traditional-looking 2D sketch interface can exist, but it edits sketch declarations/constraints in source.

Features:

- snapping;
- constraint inference;
- remaining DOF visualization;
- constraint glyphs;
- equation-driven dimensions;
- construction geometry;
- semantic entity naming;
- show source for selected entity;
- solver conflict explanation.

## 7. Feature DAG view

Render actual dependency graph, not merely a linear history list.

Each node shows:

- feature name/type;
- status;
- cached/dirty state;
- build duration/resource cost;
- requirements affected;
- reference warnings;
- provenance (human/AI/package/import).

Allow filtering to a linear "timeline" projection for familiarity.

## 8. Geometry debugger

Commands/actions:

```text
break before feature
break after feature
step feature
continue
inspect variable
inspect geometry
highlight ref/query
show raw topology
show lineage
show dependencies
```

Example:

```text
cad debug housing.cadl
break after usb_cut
inspect housing
next
```

## 9. Time-travel geometry

Because builds are deterministic and feature-DAG based:

```text
show model @ feature(base_shell)
show model @ commit abc123
show reference history housing.usb_face
```

Useful for finding where a defect or topology ambiguity was introduced.

## 10. REPL

The REPL should share the language runtime and active project context:

```text
> volume(housing)
184.7cm^3

> min_wall(housing)
1.73mm

> housing.faces().where(|f| f.planar).count()
18

> highlight(housing.mounting_face)
```

REPL changes are ephemeral unless explicitly applied as a source patch.

## 11. Inspector

For selected semantic entities show:

### Part/body

- bounds;
- volume;
- area;
- mass/material;
- topology counts;
- validity;
- configuration;
- provenance.

### Face

- type;
- area;
- normal/axis;
- curvature;
- adjacency;
- semantic name;
- tolerance;
- lineage.

### Feature

- parameters;
- inputs;
- outputs;
- build result;
- dependencies;
- diagnostics.

### Assembly instance

- definition;
- transform;
- mates/joints;
- free DOF;
- BOM metadata;
- configuration.

## 12. LSP-like language service

Implement:

- completion;
- signature help;
- hover docs;
- type information;
- dimensional errors;
- go to definition;
- find references;
- rename symbol;
- code actions;
- format;
- semantic highlighting;
- inline diagnostics;
- package docs/discovery;
- refactoring.

## 13. Code actions

Examples:

```text
Convert raw face index to semantic query
Name selected reference
Extract feature into function
Extract component into package
Add explicit units
Generate requirement from measurement
Generate regression test
Replace repeated geometry with pattern
Promote literal to parameter
```

These are valuable for both humans and AI-generated source cleanup.

## 14. Visual programming mode

Optional node graph:

```text
[Sketch] -> [Extrude] -> [Shell]
                         /     \
                     [Holes] [Ribs]
```

Nodes are an editable AST/HIR projection, not another storage format.

## 15. Diff/review UI

Show source diff plus semantic/geometry impact:

```text
wall: 2.0 -> 2.4mm
mass: +11.2g
bounding box: unchanged
requirements: 28/28 pass
new topology: +8 faces
reference health: unchanged
```

Optionally overlay old/new geometry.

## 16. AI UX

AI should be a normal client of the compiler and project, not a magical separate model.

Human-facing AI controls:

- propose patch;
- explain geometry;
- generate feature/function;
- fix failed test;
- generate tests;
- inspect reference breakage;
- search package ecosystem;
- reconstruct imported part;
- explain design rationale/provenance.

Every AI change should be reviewable as source + semantic diff.

## 17. Accessibility and non-programmer mode

A visual-first user should be able to:

- create standard parts;
- edit parameters;
- place mates;
- create sketches;
- inspect requirements;
- export drawings;

without hand-writing code. The code remains available and canonical underneath.

## 18. New feature: "explain selection"

Selecting any viewport entity can ask the runtime, not an LLM, for structured explanation:

```text
This face is `housing.mounting_face`.
Created by `base_extrude`.
Modified by `draft_1`.
Used by mates M1/M3, datum A, drawing view Front, and requirement R-14.
Reference durability: explicit.
```

An AI can then turn that structured explanation into prose if desired.
