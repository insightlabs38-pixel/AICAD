# 07 — Assemblies, Components, Interfaces, Kinematics, and Configurations

## 1. Assembly design philosophy

Assemblies must be first-class semantic programs, not merely collections of solids with manually assigned XYZ transforms.

An assembly should express:

- components and instances;
- local coordinate frames;
- semantic mechanical interfaces;
- mates and joints;
- degrees of freedom;
- contacts/collisions;
- subassemblies;
- configurations/variants;
- purchased/vendor parts;
- BOM identity;
- kinematic relationships.

## 2. Core types

```text
Component<T>
Instance<T>
Assembly
Subassembly
Interface
Port/Connector
Mate
Joint
Contact
RigidTransform
Configuration
Variant
BOMItem
```

## 3. Component declaration

```aicad
part MotorBracket implements MotorMount {
    ...
    expose mounting_face = ...;
    expose output_axis = ...;
}
```

External component:

```aicad
component Bearing608 {
    source: import_step("608.step");
    metadata {
        manufacturer: "...";
        part_number: "...";
    }
    expose axis = detect_cylindrical_axis(...);
    expose outer_seat = ...;
}
```

## 4. Instance API

Proposed signature:

```text
instance(component, name?, configuration?, initial_transform?, suppressed=false) -> Instance
```

Parameters:

| Parameter | Type | Meaning |
|---|---|---|
| `component` | `Component|Part|Assembly` | Definition to instantiate |
| `name` | `String?` | Stable instance name |
| `configuration` | `ConfigurationSelection?` | Variant parameters |
| `initial_transform` | `Transform?` | Seed pose, not final mate semantics |
| `suppressed` | `Bool` | Configuration-driven suppression |

## 5. Mate types

Baseline high-level mates:

| Mate | Parameters | Effect |
|---|---|---|
| `coincident(a,b)` | points/planes/faces | coincident placement |
| `concentric(a,b)` | axes/cylinders | shared axis |
| `parallel(a,b)` | planes/axes | parallel orientation |
| `perpendicular(a,b)` | planes/axes | 90° relation |
| `distance(a,b,value)` | entities + Length | fixed separation |
| `angle(a,b,value)` | entities + Angle | fixed angle |
| `tangent(a,b)` | surfaces/curves | tangency |
| `lock(a,b)` | frames/components | remove all relative DOF |
| `gear_ratio(a,b,ratio)` | rotational joints | coupled rotation |
| `rack_pinion(rot,lin,pitch)` | joints | rotary-linear coupling |
| `screw(rot,lin,pitch)` | joints | helical coupling |

## 6. Joint types

| Joint | Core parameters |
|---|---|
| `fixed` | parent frame, child frame |
| `revolute` | parent axis, child axis, angular limits, zero position |
| `prismatic` | axis, linear limits, zero |
| `cylindrical` | axis + linear/rotational limits |
| `spherical` | center + angular limits |
| `planar` | plane + in-plane limits |
| `universal` | intersecting axes + limits |
| `helical` | axis + pitch + limits |
| `custom` | equations/constraint set |

Example:

```aicad
joint hinge {
    type: revolute;
    parent: chassis.hinge_axis;
    child: arm.hinge_axis;
    limits: -20deg..115deg;
    home: 0deg;
}
```

## 7. DOF analysis

The assembly solver must report:

- total free DOF;
- DOF per component/subassembly;
- constraints removing each DOF;
- redundant/over-constraining mates;
- unresolved mate references;
- inconsistent mate cycles.

Example diagnostic:

```text
ASM-E034 OVERCONSTRAINED_ASSEMBLY
Component: gearbox.output_shaft
Redundant constraint: mate M17
Conflict set: M03, M11, M17
Residual: 0.18 mm
```

## 8. Interfaces

Interfaces make assemblies reusable and type-aware.

```aicad
interface MotorMount {
    mounting_face: FaceRef;
    output_axis: AxisRef;
    hole_pattern: Pattern;
}

interface BearingInterface {
    bore_axis: AxisRef;
    seat_diameter: Length;
    seat_face: FaceRef;
}
```

Functions can accept interface-constrained components:

```aicad
fn attach_motor<T: MotorMount>(motor: T, housing: Part) -> MateGroup { ... }
```

## 9. Mechanical connectors/ports

Add typed ports for reusable subsystems:

```text
MechanicalPort
FluidPort
ElectricalPort
OpticalPort
ThermalInterface
```

This extends assemblies beyond geometric mating and allows packages to encode compatibility.

Example:

```aicad
port hydraulic_A: FluidPort {
    standard: "SAE-J1926";
    size: ...;
    axis: ...;
    sealing_face: ...;
}
```

## 10. Configuration system

Configurations must be ordinary typed values rather than a separate hidden UI database.

```aicad
enum MotorSize { NEMA17, NEMA23 }
enum HousingMaterial { Plastic, Aluminum }

configuration Product {
    motor: MotorSize = NEMA17;
    ratio: Int = 10;
    material: HousingMaterial = Aluminum;
}
```

Then:

```aicad
let wall = match Product.material {
    Plastic => 3mm,
    Aluminum => 2mm,
};
```

## 11. Configuration constraints

```aicad
configuration_rules {
    require ratio in [5, 10, 20];
    forbid Product.motor == NEMA23 && Product.material == Plastic;
}
```

The compiler can enumerate valid combinations for product-family generation.

## 12. Suppression and replacement

```aicad
if config.has_encoder {
    instance encoder = ...;
}
```

or declaratively:

```aicad
@suppress_if(!config.has_encoder)
instance encoder = ...;
```

Replacement should preserve declared interfaces when compatible.

## 13. Nested assemblies

Subassemblies should expose interfaces just like parts:

```aicad
assembly Gearbox implements RotaryPowerUnit {
    ...
    expose input_axis = motor.output_axis;
    expose output_axis = shaft.axis;
    expose mount_face = housing.base_face;
}
```

## 14. Contact model

Start simple:

```text
interference-only
no-penetration static contact
clearance contact
```

Later add friction/contact solver integrations.

Contact declaration:

```aicad
contact gear_mesh {
    a: gearA.teeth;
    b: gearB.teeth;
    mode: no_penetration;
}
```

## 15. Kinematic evaluation

API:

```text
pose(assembly, state) -> AssemblyPose
sweep_joint(joint, range, samples) -> MotionStudy
interference_over_motion(assembly, study) -> CollisionReport
workspace(component, joints, ranges) -> Volume/PointCloud
```

## 16. BOM semantics

Every physical instance may carry:

```text
part_number
revision
make_buy
manufacturer
supplier_part_number
quantity rule
material
finish
mass
cost source
```

BOM generation must be configuration-aware and deduplicate identical definitions.

## 17. Large-assembly strategy

Design now for later scalability:

- instance geometry is shared, not duplicated;
- transforms remain cheap;
- use level-of-detail meshes for viewport;
- lazy-load external parts;
- hierarchical bounding volumes for collision;
- build/test only changed subassemblies;
- permit lightweight/suppressed representation.
