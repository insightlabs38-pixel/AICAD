//! `AICAD-079`: proves the illustrative source snippets in `skills/
//! cad-core.skill.md` actually parse/type-check/execute cleanly — a skill
//! file teaching a fresh model incorrect syntax would be actively
//! harmful, so its own examples get the same pipeline proof as any other
//! Safe CAD source (`AGENTS.md`'s evidence rule).

#[test]
fn the_declaration_overview_snippet_builds_cleanly() {
    let source = "\
param width: Length = 40mm;
const HOLE_COUNT: Int = 4;
let doubled: Length = width * 2.0;

fn helper(x: Length) -> Length {
    var total: Length = 0mm;
    total = total + x;
    return total * 2.0;
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
}
";
    let report = cad_cli::build::build_source("skill_snippet_1.aicad", source, None, None);
    assert_eq!(
        report.status,
        cad_cli::BuildStatus::Ok,
        "{:?}",
        report.diagnostics
    );
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
}

#[test]
fn the_part_named_outputs_snippet_builds_and_exposes_body() {
    let source = "\
part Bracket {
    param width: Length = 80mm;
    param height: Length = 40mm;
    let body: Geometry = box(width, height, 10mm);
}
";
    let output_path = std::env::temp_dir().join("aicad_079_skill_doc_part_snippet.step");
    let _ = std::fs::remove_file(&output_path);
    let report = cad_cli::build::build_source(
        "skill_snippet_5.aicad",
        source,
        Some(&output_path),
        Some("Bracket.body"),
    );
    assert_eq!(
        report.status,
        cad_cli::BuildStatus::Ok,
        "{:?}",
        report.diagnostics
    );
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    assert!(output_path.exists());
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn the_geometry_type_construction_snippet_builds_cleanly() {
    // The exact record-construction shapes §3/§4 of the skill doc show.
    let source = "\
fn f() -> Geometry {
    return hole(
        box(20mm, 20mm, 10mm),
        Axis3(
            origin = Point3(x = 5mm, y = 5mm, z = -1mm),
            direction = Vector3(x = 0.0, y = 0.0, z = 1.0),
        ),
        4mm,
        12mm,
    );
}
";
    let report = cad_cli::build::build_source("skill_snippet_geometry.aicad", source, None, None);
    assert_eq!(
        report.status,
        cad_cli::BuildStatus::Ok,
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn the_unit_mismatch_example_is_really_rejected_as_documented() {
    // The skill doc's own §2 claim: "`5mm + 2kg` never compiles."
    let report = cad_cli::build::build_source(
        "skill_snippet_unit_error.aicad",
        "let x = 5mm + 2kg;",
        None,
        None,
    );
    assert_eq!(report.status, cad_cli::BuildStatus::Failed);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code.as_string() == "UNIT-E104"),
        "{:?}",
        report.diagnostics
    );
}
