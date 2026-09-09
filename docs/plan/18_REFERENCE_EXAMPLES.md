# 18 — Reference Source Examples

These examples are intentionally illustrative. Final syntax may change, but all capabilities should survive the language design.

# Example 1 — Parametric mounting plate

```aicad
part MountingPlate {
    param width: Length = 120mm;
    param height: Length = 80mm;
    param thickness: Length = 4mm;
    param corner_radius: Length = 8mm;
    param edge_inset: Length = 10mm;
    param mounting_d: Length = 5.5mm;

    let base = plate(
        size=[width, height],
        thickness=thickness,
        corner_radius=corner_radius
    );

    let holes = corner_points(
        size=[width, height],
        inset=edge_inset
    ).map(|p| hole(
        center=p,
        diameter=mounting_d,
        depth=ThroughAll
    ));

    body = cut(base, holes);

    expose top_face = query body.faces {
        planar;
        normal ~= +Z;
        largest(area);
    };

    test "valid plate" {
        assert valid(body);
        assert holes.count() == 4;
    }
}
```

# Example 2 — Conditional configuration

```aicad
enum MotorSize { NEMA17, NEMA23 }

configuration Product {
    motor: MotorSize = NEMA17;
}

part MotorMount {
    let wall = match Product.motor {
        NEMA17 => 3mm,
        NEMA23 => 4mm,
    };

    let pattern = if Product.motor == NEMA17 {
        nema17_pattern()
    } else {
        nema23_pattern()
    };

    ...
}
```

# Example 3 — Low-level custom retaining hook

```aicad
fn retaining_hook() -> Solid {
    let profile = bspline_curve(
        degree=3,
        control_points=[
            [0mm, 0mm, 0mm],
            [1.2mm, 0.4mm, 0mm],
            [2.1mm, 1.8mm, 0mm],
            [1.4mm, 3.2mm, 0mm]
        ],
        knots=[0.0, 1.0],
        multiplicities=[4,4]
    );

    let path = circle_curve(
        center=[0mm,0mm,0mm],
        normal=+Z,
        radius=7.5mm
    ).trim(0deg, 38deg);

    return sweep(
        profile=profile,
        path=path,
        orientation=minimum_twist
    );
}

part Housing {
    let base = enclosure(...);
    let hook = retaining_hook();
    body = union(base, hook);
}
```

# Example 4 — Raw topology escape hatch

```aicad
part ImportedRepair {
    var body = import_step("damaged.step");

    unsafe geometry {
        let raw: KernelShape* = raw_shape(body);
        let suspect: Face* = raw_face(body, 17);
        let repaired = specialized_repair(raw, suspect);

        body = adopt_validated(
            repaired,
            semantic_outputs={
                "mounting_face": query_result(...)
            },
            validation=strict
        );
    }

    test "repair valid" {
        assert valid(body);
        assert closed(body);
    }
}
```

# Example 5 — Assembly with interfaces

```aicad
interface MotorMount {
    mounting_face: FaceRef;
    output_axis: AxisRef;
}

assembly Actuator {
    instance housing = Housing();
    instance motor = NEMA17();
    instance shaft = OutputShaft();

    mate coincident(
        motor.mounting_face,
        housing.motor_face
    );

    mate concentric(
        motor.output_axis,
        shaft.axis
    );

    joint output {
        type: revolute;
        parent: housing.output_axis;
        child: shaft.axis;
        limits: -180deg..180deg;
    }

    test "one output DOF" {
        assert degrees_of_freedom(self) == 1;
    }
}
```

# Example 6 — Executable requirement

```aicad
requirement R_CLEARANCE {
    description: "PCB must retain enclosure clearance";
    severity: blocker;
    assert clearance(pcb, housing.inner_faces) >= 1.5mm;
}
```

# Example 7 — Geometry query

```aicad
let bearing_bores = body.faces()
    .where(|f| f.cylindrical)
    .where(|f| f.radius ~= 11mm within 0.01mm)
    .where(|f| f.axis ~= shaft.axis within 0.1deg);

assert bearing_bores.count() == 2;
```

# Example 8 — Optimization

```aicad
optimize BracketOptimization {
    variables {
        Bracket.wall: 1.5mm..5mm;
        Bracket.rib_height: 4mm..20mm;
    }

    minimize mass(Bracket);

    subject_to {
        safety_factor(Bracket, LC1) >= 2.0;
        max_displacement(Bracket, LC1) <= 0.4mm;
        min_wall(Bracket) >= 1.5mm;
    }
}
```

# Example 9 — Drawing

```aicad
drawing BracketA3 {
    sheet: A3;
    projection: third_angle;
    units: mm;

    view front = view(Bracket, orientation=front, scale=2:1);
    view top = view(Bracket, orientation=top, align_to=front);

    dimension overall_width = Bracket.width;
    dimension mounting_hole = Bracket.mounting_hole.diameter;

    datum A = Bracket.mounting_face;
}
```

# Example 10 — Custom package + AI skill concept

```text
cad add robotics.cycloidal
```

```aicad
import robotics.cycloidal::{CycloidalDrive};

let drive = CycloidalDrive(
    reduction_ratio=19,
    eccentricity=1.2mm,
    thickness=8mm
);

assert drive.validation().passed;
```

The model should learn this package from its package skill/schema rather than requiring language retraining.
