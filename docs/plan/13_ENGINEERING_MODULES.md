# 13 — Engineering Modules: Drawings, PMI/GD&T, DFM, Simulation, Optimization, and Uncertainty

## 1. Principle

The core compiler should not become a monolithic implementation of every engineering discipline. These features belong above the common semantic model and should use pluggable solver/backend adapters where practical.

However, the **language-level concepts** must be first-class enough that they participate in dependency tracking, version control, requirements, AI inspection, and export.

---

# Part A — Materials and physical properties

## 2. Material type

```aicad
material Aluminum6061T6 {
    density: 2.70g/cm^3;
    elastic_modulus: 68.9GPa;
    poisson_ratio: 0.33;
    yield_strength: 276MPa;
    ultimate_strength: 310MPa;
    thermal_conductivity: ...;
    expansion_coefficient: ...;
}
```

Material data should support provenance/source/version metadata.

## 3. Material assignment

```aicad
@material(Aluminum6061T6)
part Bracket { ... }
```

Allow body/component-level overrides.

## 4. Mass properties

Core deterministic queries:

```text
volume(shape)
surface_area(shape)
mass(shape)
center_of_mass(shape)
inertia_tensor(shape, frame?)
principal_axes(shape)
bounding_box(shape)
```

---

# Part B — Drawings and documentation

## 5. Drawing object

```aicad
drawing BracketDrawing {
    sheet: A3;
    units: mm;
    standard: ISO;
    ...
}
```

## 6. Sheet parameters

```text
size: A0/A1/A2/A3/A4/custom
orientation: portrait/landscape
units
projection: first_angle/third_angle
scale
title_block
template
revision
```

## 7. View types

```text
orthographic
isometric
section
partial_section
detail
auxiliary
broken
exploded assembly
```

View signature:

```text
view(source, orientation, scale=auto, position=auto, hidden_lines=standard)
```

## 8. Drawing dimensions

Dimensions should reference semantic model parameters/entities, not arbitrary drawing pixels.

```aicad
dimension width = model.width;
dimension hole_d = mounting_hole.diameter;
```

Support:

- linear;
- angular;
- radial;
- diameter;
- ordinate;
- baseline/chain;
- hole callouts;
- thread callouts;
- fit/tolerance dimensions.

## 9. Drawing automation

Rules can place views/dimensions automatically, but output remains editable and source-backed. Provide collision/overlap checks for annotations.

---

# Part C — PMI and GD&T

## 10. Datums

```aicad
datum A = housing.base_face;
datum B = housing.center_axis;
```

## 11. Geometric tolerance frame

Conceptual syntax:

```aicad
gdt position {
    target: mounting_holes;
    tolerance: diameter(0.10mm);
    references: [A, B];
    material_condition: MMC;
}
```

Support standard semantic categories progressively:

```text
form: straightness, flatness, circularity, cylindricity
orientation: parallelism, perpendicularity, angularity
location: position, concentricity/coaxiality where applicable, symmetry
profile: line/surface profile
runout: circular/total
```

Do not encode a standard incorrectly: implement against licensed/authoritative standards work during the engineering stage. The language should preserve version/standard identifiers.

## 12. Surface finish

```aicad
@surface_finish(Ra=1.6um, process="ground")
FaceRef bearing_surface;
```

---

# Part D — Manufacturing semantics / DFM

## 13. Manufacturing process declaration

```aicad
manufacturing bracket {
    process: cnc_3axis;
    material: Aluminum6061T6;
    stock: box([100mm, 70mm, 15mm]);
}
```

## 14. Generic DFM constraint parameters

```text
minimum_wall
minimum_feature
minimum_internal_radius
maximum_aspect_ratio
allowed_undercuts
required_draft
surface_finish capability
tolerance capability
stock/envelope
setup count
machine axes
tool access
```

## 15. CNC module

Inputs:

- machine class/axes;
- stock;
- setup frames;
- tool library or min cutter diameter;
- reach/holder constraints;
- internal corner requirements;
- hole depth ratios;
- tolerance capabilities.

Checks:

- inaccessible surfaces;
- impossible undercuts;
- corner radius below tool capability;
- excessive deep/narrow pockets;
- setup-count estimate;
- stock containment;
- thin walls.

## 16. Additive module

Inputs:

- process (FDM/SLA/SLS/etc.);
- layer height;
- nozzle/beam/material;
- build orientation;
- support threshold;
- min wall/feature;
- shrink/compensation model.

Checks:

- overhangs;
- unsupported islands;
- trapped powder/resin;
- wall thickness;
- build volume;
- anisotropy-aware orientation hints.

## 17. Injection molding module

Inputs/checks:

- draft;
- nominal wall;
- wall variation;
- ribs/boss ratios;
- undercuts;
- parting line;
- ejection direction;
- gate/cooling hints later;
- sink/warp risk adapters later.

## 18. Sheet metal module

First-class semantics:

```text
base flange
edge flange
bend
hem
jog
rip
relief
unfold/refold
flat pattern
```

Parameters:

```text
thickness
bend radius
K-factor/bend allowance table
bend angle
grains/material
relief type
```

This is likely a major standard-library package rather than core compiler syntax.

---

# Part E — Simulation/CAE

## 19. Simulation object

```aicad
simulation LoadCase1 {
    model: bracket;
    type: structural_static;
    ...
}
```

## 20. Common simulation fields

```text
model/configuration
solver/backend
materials
mesh policy
contacts
boundary conditions
loads
convergence settings
outputs
assertions
```

## 21. Structural analysis

Loads:

```text
force
torque
pressure
gravity
acceleration
bearing load
remote load
```

Boundary conditions:

```text
fixed
pinned
symmetry
displacement
rotation
spring/support
```

Outputs:

```text
stress
strain
displacement
reaction force
safety factor
buckling factor
modal frequency
```

## 22. Thermal

Inputs:

- temperature BC;
- heat flux;
- convection;
- heat generation;
- contact conductance.

Outputs:

- temperature;
- heat flow;
- gradients;
- thermal expansion coupling.

## 23. CFD/electromagnetic

Treat as later plugin schemas. Preserve common geometry-selection and parameter concepts but avoid prematurely inventing solver-specific semantics.

---

# Part F — Optimization

## 24. Optimization syntax

```aicad
optimize bracket {
    variables {
        wall: 1.5mm..5mm;
        rib_height: 3mm..20mm;
    }

    minimize mass(bracket);

    subject_to {
        safety_factor(bracket, LC1) >= 2.0;
        displacement(bracket, LC1) <= 0.4mm;
    }
}
```

## 25. Optimization components

- continuous variables;
- integer/discrete variables;
- configuration variables;
- multiple objectives;
- hard/soft constraints;
- evaluation budget;
- surrogate-model plugin;
- solver selection;
- Pareto front output;
- reproducible random seed.

## 26. Design-space exploration

```aicad
explore {
    objectives: [mass, cost];
    produce: pareto_front;
}
```

Results preserve full configuration/source parameter vectors and can be promoted into named variants.

---

# Part G — Uncertainty and tolerance analysis

## 27. Distribution types

```text
Normal
Uniform
Triangular
Empirical
Discrete
```

Example:

```aicad
let shaft_d = Normal(mean=10mm, sigma=0.008mm);
```

## 28. Tolerance stack

Support deterministic worst-case and statistical methods:

```text
worst_case
RSS
Monte Carlo
solver/plugin custom
```

Example:

```aicad
tolerance_analysis axial_stack {
    output: clearance(faceA, faceB);
    method: monte_carlo(samples=100000, seed=42);
}
```

## 29. Reliability assertions

```aicad
assert probability(clearance > 0.2mm) >= 0.999;
```

Results must include uncertainty/confidence and method details.

---

# Part H — Cost and sustainability metadata

## 30. Optional future modules

Useful additions discovered during review:

### Cost model

```text
material cost
machine time
setup count
purchased components
process rate tables
```

### Sustainability

```text
material mass/waste
process energy estimates
recyclability metadata
transport assumptions
```

These should be plugins with explicit data sources, not hard-coded claims in the compiler.

---

# Part I — Requirement linkage

Every downstream engineering object should be traceable:

```text
requirement -> design parameter/entity -> simulation/DFM/drawing -> result/evidence
```

The platform's advantage is not merely running analyses; it is preserving the semantic graph connecting why geometry exists to how it was verified.
