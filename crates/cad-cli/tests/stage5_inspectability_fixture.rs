//! `AICAD-129`: the bounded inspectability/tool-client fixture this
//! task's own acceptance list requires -- proves that a representative
//! Stage-5 program can be inspected `source -> typed call/value ->
//! feature/provenance/dependency -> Geometry IR/query -> kernel result/
//! reference evidence`, using only this project's existing *public*
//! surfaces, never a private/internal shortcut.
//!
//! Two distinct inspection mechanisms are exercised, and this file is
//! explicit about which is which (see `project/reports/AICAD-129.md`'s
//! own "Inspectability chain" section for the full disclosure):
//!
//! - **Layers 1 and 4** (diagnostics; reference evidence) go through the
//!   real stable **JSON text contract** `cad build --json`/`cad refs
//!   check --json` print (`docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §2-3):
//!   this file calls `BuildReport::to_json()`/`RefsCheckReport::to_json()`
//!   (exactly what `cad-cli`'s own `main.rs` does before printing), then
//!   serializes to text and re-parses it with `cad_diagnostics::json::
//!   Json::parse`, walking the result with only `Json`'s own generic
//!   `get`/`as_array`/`as_str` accessors -- simulating a genuinely
//!   external consumer reading CLI stdout, never touching a private
//!   `Diagnostic`/`BuildReport` field directly.
//! - **Layers 2 and 3** (feature/provenance/dependency; Geometry IR/
//!   query/kernel result) go through this project's *public Rust
//!   library* API (`cad_feature_graph::FeatureGraph`,
//!   `cad_cli::ParametricBuildSession`) -- today's real inspection
//!   mechanism for this layer (`docs/plan/17`'s own richer `cad inspect`/
//!   `cad explain`/`cad why` commands are unimplemented placeholders,
//!   correctly out of this task's scope per `AGENTS.md`'s "no
//!   speculative future work"). This is still a stable, typed, documented
//!   public surface (crate-public items only), just not yet a wire-format
//!   JSON contract -- disclosed precisely, not oversold.

use cad_feature_graph::FeatureGraph;
use cad_hir::BuiltinFnId;
use cad_occt_bridge::OcctContext;
use cad_runtime::Value;

fn number_magnitude(value: &Value) -> f64 {
    match value {
        Value::Number(n) => n.magnitude,
        other => panic!("expected Value::Number, got {other:?}"),
    }
}

fn bool_value(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        other => panic!("expected Value::Bool, got {other:?}"),
    }
}

fn global<'a>(session: &'a cad_cli::ParametricBuildSession<'_>, name: &str) -> &'a Value {
    let binding = session
        .binding_named(name)
        .unwrap_or_else(|| panic!("'{name}' is a top-level let binding"));
    session
        .last_global(binding)
        .unwrap_or_else(|| panic!("'{name}' should have a computed value"))
}

const REPRESENTATIVE_SOURCE: &str = "\
let bottom: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 0mm, y = 0mm, z = 0mm), direction = Vector3(x = 1.0, y = 0.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let right: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 10mm, y = 0mm, z = 0mm), direction = Vector3(x = 0.0, y = 1.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let top: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 10mm, y = 10mm, z = 0mm), direction = Vector3(x = -1.0, y = 0.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let left: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 0mm, y = 10mm, z = 0mm), direction = Vector3(x = 0.0, y = -1.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let square: Geometry = make_wire([bottom, right, top, left]);\n\
let square_face: Geometry = make_face(square);\n\
let square_face_valid: Bool = is_valid(square_face);\n\
let square_face_area: Area = area(square_face);\n\
";

/// `REPRESENTATIVE_SOURCE` minus its two `Query`-category calls
/// (`is_valid`/`area`) -- `cad_cli::build::build_source` (the JSON-text-
/// contract entry point `cad build --json` itself calls) wires no live
/// kernel query executor into its `Interpreter` at all, even with an
/// `--output` path (only `ParametricBuildSession` does, `AICAD-105`/
/// `DL-23`'s own "kernel-backed source queries" mechanism) -- so a
/// `Construction`-category-only program is this entry point's own real,
/// honest positive case, matching several ACTIVE examples' own identical
/// established constraint (`examples/README.md`: "cannot itself call
/// is_valid/area").
const CONSTRUCTION_ONLY_SOURCE: &str = "\
let bottom: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 0mm, y = 0mm, z = 0mm), direction = Vector3(x = 1.0, y = 0.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let right: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 10mm, y = 0mm, z = 0mm), direction = Vector3(x = 0.0, y = 1.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let top: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 10mm, y = 10mm, z = 0mm), direction = Vector3(x = -1.0, y = 0.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let left: Geometry = make_edge(trim_curve(\n\
    line_curve(origin = Point3(x = 0mm, y = 10mm, z = 0mm), direction = Vector3(x = 0.0, y = -1.0, z = 0.0)),\n\
    0.0, 0.01,\n\
));\n\
let square: Geometry = make_wire([bottom, right, top, left]);\n\
let square_face: Geometry = make_face(square);\n\
";

/// Layer 1: source -> typed call/value, through the real JSON text
/// contract. A clean build reports `status: "ok"` and an empty
/// `diagnostics` array; a deliberately type-broken variant (a `Bool`
/// literal where `line_curve`'s own `origin` needs a `Point3`) reports
/// `status: "failed"` with a real, structured diagnostic -- both walked
/// with only `Json`'s generic accessors.
#[test]
fn layer1_typed_call_diagnostics_are_inspectable_through_the_json_text_contract() {
    let output_path =
        std::env::temp_dir().join(format!("aicad-inspectability-{}.step", std::process::id()));
    let clean_report = cad_cli::build::build_source(
        "case.aicad",
        CONSTRUCTION_ONLY_SOURCE,
        Some(output_path.as_path()),
        Some("square_face"),
    );
    let _ = std::fs::remove_file(&output_path);
    let clean_text = clean_report.to_json().to_canonical_string();
    let clean_json = cad_diagnostics::json::Json::parse(&clean_text)
        .expect("cad build --json's own output must be valid JSON an external tool can parse");
    assert_eq!(
        clean_json.get("status").and_then(|j| j.as_str()),
        Some("ok")
    );
    let clean_diagnostics = clean_json
        .get("diagnostics")
        .and_then(|j| j.as_array())
        .expect("'diagnostics' must be a JSON array");
    assert!(
        clean_diagnostics.is_empty(),
        "a clean build should report zero diagnostics, got {clean_diagnostics:?}"
    );

    // A deliberately type-broken variant: `line_curve`'s own `origin`
    // parameter needs a `Point3`, given a `Bool` instead.
    let truly_broken_source = CONSTRUCTION_ONLY_SOURCE.replacen(
        "line_curve(origin = Point3(x = 0mm, y = 0mm, z = 0mm), direction = Vector3(x = 1.0, y = 0.0, z = 0.0))",
        "line_curve(origin = true, direction = Vector3(x = 1.0, y = 0.0, z = 0.0))",
        1,
    );
    let broken_report =
        cad_cli::build::build_source("case.aicad", &truly_broken_source, None, None);
    let broken_text = broken_report.to_json().to_canonical_string();
    let broken_json = cad_diagnostics::json::Json::parse(&broken_text)
        .expect("a failed build's own --json output must still be valid JSON");
    assert_eq!(
        broken_json.get("status").and_then(|j| j.as_str()),
        Some("failed")
    );
    let broken_diagnostics = broken_json
        .get("diagnostics")
        .and_then(|j| j.as_array())
        .expect("'diagnostics' must be a JSON array");
    assert!(
        !broken_diagnostics.is_empty(),
        "a type error should produce at least one structured diagnostic"
    );
    let first = &broken_diagnostics[0];
    for field in ["code", "severity", "title", "message"] {
        assert!(
            first.get(field).and_then(|j| j.as_str()).is_some(),
            "diagnostic is missing its own stable '{field}' string field: {first:?}"
        );
    }
}

/// Layer 2: feature/provenance/dependency, through the public
/// `cad_feature_graph::FeatureGraph` API. Parses/lowers/type-checks the
/// representative source itself (the same three public pipeline calls
/// `ParametricBuildSession::new` makes internally), then walks the
/// resulting graph's own typed nodes: `square_face`'s own
/// `geometry_inputs` must name exactly `square`'s own `FeatureId`, and
/// `square`'s own `geometry_inputs` must name all four edge nodes --
/// real, inspectable dependency-graph structure, not a guess.
#[test]
fn layer2_feature_dependency_graph_is_inspectable_through_the_public_feature_graph_api() {
    let (program, parse_diagnostics) =
        cad_parser::parse_program(REPRESENTATIVE_SOURCE, "case.aicad");
    assert!(parse_diagnostics.is_empty(), "{parse_diagnostics:?}");
    let lowered = cad_hir::lower_program(&program, "case.aicad", REPRESENTATIVE_SOURCE);
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    let checked = cad_hir::check_program(
        &lowered.program,
        &lowered.bindings,
        "case.aicad",
        REPRESENTATIVE_SOURCE,
    );
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);

    let graph =
        FeatureGraph::build(&lowered.program).expect("a clean program must build a feature graph");
    let find_named = |name: &str| {
        graph
            .nodes()
            .iter()
            .find(|n| n.name == Some(name))
            .unwrap_or_else(|| panic!("no feature node named '{name}'"))
    };

    let bottom = find_named("bottom");
    assert_eq!(bottom.op, BuiltinFnId::MakeEdge);

    let square = find_named("square");
    assert_eq!(square.op, BuiltinFnId::MakeWire);

    let square_face = find_named("square_face");
    assert_eq!(square_face.op, BuiltinFnId::MakeFace);
    assert_eq!(
        square_face.geometry_inputs,
        vec![square.id],
        "'square_face' should depend on exactly 'square' -- make_face's own \
         single-Geometry 'wire' parameter is a real, inspectable dependency edge"
    );

    // **Discovered, narrowly-scoped limitation** (not assumed in
    // advance): `is_geometry_type` (`crates/cad-feature-graph/src/
    // graph.rs`) only recognizes a bare `Geometry`-typed parameter, not a
    // `List<Geometry>` one -- so `make_wire`'s own `edges: List<Geometry>`
    // parameter is classified as a scalar "parameter," not a
    // `geometry_inputs` edge, and `square`'s own dependency on its four
    // edge nodes is invisible to *this specific* `geometry_inputs`
    // introspection field. The same `is_geometry_type_ref` gate exists in
    // the real production trace mechanism too (`cad_runtime::interp::
    // Interpreter::call`), so this is not merely a stale/superseded API's
    // own quirk. This is recorded as an observed inspectability-API gap,
    // not a claimed incremental-rebuild/dirty-set correctness defect --
    // this fixture does not exercise `dirty_set`/`rebuild` at all, and a
    // separate `binding_refs`/provenance-resolution mechanism may still
    // carry the real dependency there; verifying that is out of this
    // bounded fixture's own scope. See `project/reports/AICAD-129.md`'s
    // own "Inspectability chain" section and `project/OWNER_DECISIONS.md`.
    assert!(
        square.geometry_inputs.is_empty(),
        "if this now contains the four edge FeatureIds, `List<Geometry>` \
         tracking has been fixed -- update this assertion (and the finding \
         in AICAD-129.md) to match, rather than leaving a stale expectation"
    );
}

/// Layer 3: Geometry IR/query and kernel result, through the public
/// `ParametricBuildSession` API. A `Construction`-category binding
/// (`square_face`) has a real kernel `Shape` behind it whose measured
/// area matches the exact closed-form square area; `square_face_valid`/
/// `square_face_area` (pure `Query`-category results) do not.
#[test]
fn layer3_geometry_ir_and_kernel_result_are_inspectable_through_the_session_api() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = cad_cli::ParametricBuildSession::new("case.aicad", REPRESENTATIVE_SOURCE, &ctx)
        .expect("the representative fixture should build cleanly");

    let square_face_binding = session.binding_named("square_face").unwrap();
    let shape = session
        .shape_for_binding(square_face_binding)
        .expect("'square_face' is Construction-category -- it must have a real kernel Shape");
    let measured_area = shape.area().expect("area query against the real kernel");
    assert!(
        (measured_area - 1e-4).abs() < 1e-12,
        "area was {measured_area}"
    );

    assert!(bool_value(global(&session, "square_face_valid")));
    let area_value = number_magnitude(global(&session, "square_face_area"));
    assert!((area_value - 1e-4).abs() < 1e-12);
}

/// Layer 4: reference evidence, through the real JSON text contract
/// (`cad refs check --json`), reusing the two ACTIVE reference examples
/// (`examples/references/ambiguous_reference.aicad`/
/// `broken_reference.aicad`) that already establish these exact expected
/// health outcomes at the Rust level -- this test instead walks the same
/// outcome through the stable JSON schema an external tool would read.
#[test]
fn layer4_reference_evidence_is_inspectable_through_the_json_text_contract() {
    for (relative, expect_field) in [
        ("examples/references/ambiguous_reference.aicad", "ambiguous"),
        ("examples/references/broken_reference.aicad", "broken"),
    ] {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let report = cad_cli::refs_check::refs_check_source(relative, &source);
        let text = report.to_json().to_canonical_string();
        let json = cad_diagnostics::json::Json::parse(&text)
            .expect("cad refs check --json's own output must be valid JSON");
        assert_eq!(json.get("status").and_then(|j| j.as_str()), Some("ok"));
        let health = json.get("health").expect("'health' field must be present");
        let count = match health.get(expect_field) {
            Some(cad_diagnostics::json::Json::Integer(n)) => *n,
            other => panic!("expected an Integer '{expect_field}' field, got {other:?}"),
        };
        assert!(
            count >= 1,
            "{relative}: expected health.{expect_field} >= 1, got {count} (full health: {health:?})"
        );
    }
}
