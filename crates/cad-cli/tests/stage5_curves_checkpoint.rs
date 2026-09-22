//! `AICAD-112` (Checkpoint A): proves the Stage-5 curve surface
//! (`AICAD-109`-`111`) through the real production path
//! (`ParametricBuildSession`, exactly like `stage5_kernel_backed_queries.rs`
//! proves the kernel-backed query family), with an *independent* exact
//! analytic check — not merely "it built" — and direct evidence that curve
//! construction/evaluation never touches the kernel at all (`AGENTS.md`'s
//! evidence rule: no OCCT-specific classes escape the adapter; here there
//! is no kernel adapter call in the first place).
//!
//! The central proof: a rational quadratic Bezier curve (built from
//! `bezier_curve`/`evaluate_curve`, the closed-form-vs-numerical NURBS core
//! `AICAD-110` implements) reproduces the exact, independently-computable
//! 45-degree point of a unit circle — the same standard textbook identity
//! `cad_geometry_api::curve`'s own unit tests already check at the Rust
//! level, re-verified here through parsed `.aicad` source, HIR lowering,
//! type-checking, and interpretation, with **no** `OcctContext::create_*`/
//! `Shape` call anywhere in the path (`ParametricBuildSession::
//! shape_for_binding` returns `None` for every curve-typed binding).

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_runtime::Value;

const SOURCE: &str = "\
let half_sqrt2: Float = 0.70710678118654757;\n\
let arc: Curve = bezier_curve(\n\
    control_points = [\n\
        Point3(x = 1m, y = 0m, z = 0m),\n\
        Point3(x = 1m, y = 1m, z = 0m),\n\
        Point3(x = 0m, y = 1m, z = 0m),\n\
    ],\n\
    weights = [1.0, half_sqrt2, 1.0],\n\
);\n\
let midpoint: CurveEvaluation = evaluate_curve(arc, 0.5);\n\
let midpoint_x: Length = midpoint.point.x;\n\
let midpoint_y: Length = midpoint.point.y;\n\
";

fn session(ctx: &OcctContext) -> ParametricBuildSession<'_> {
    ParametricBuildSession::new("test.aicad", SOURCE, ctx)
        .expect("the fixture should build cleanly through the real production path")
}

fn number_magnitude(value: &Value) -> f64 {
    match value {
        Value::Number(n) => n.magnitude,
        other => panic!("expected Value::Number, got {other:?}"),
    }
}

#[test]
fn rational_bezier_arc_reproduces_the_exact_45_degree_unit_circle_point() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    let x = session
        .binding_named("midpoint_x")
        .and_then(|b| session.last_global(b))
        .expect("'midpoint_x' is a top-level let binding");
    let y = session
        .binding_named("midpoint_y")
        .and_then(|b| session.last_global(b))
        .expect("'midpoint_y' is a top-level let binding");
    let (x, y) = (number_magnitude(x), number_magnitude(y));
    // Independent check: this is the exact 45-degree point of a real unit
    // circle, not merely "whatever this code happens to compute" — both
    // coordinates equal 1/sqrt(2), and x^2 + y^2 == 1 exactly.
    assert!((x - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
    assert!((y - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
    assert!((x * x + y * y - 1.0).abs() < 1e-9);
}

#[test]
fn curve_construction_and_evaluation_never_touch_the_kernel() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);
    for name in ["arc", "midpoint", "midpoint_x", "midpoint_y"] {
        let binding = session
            .binding_named(name)
            .unwrap_or_else(|| panic!("'{name}' is a top-level let binding"));
        assert!(
            session.shape_for_binding(binding).is_none(),
            "'{name}' is a Curve/CurveEvaluation/Length value — it must never have a kernel \
             Shape, proving no OCCT call happened to produce it"
        );
    }
}
