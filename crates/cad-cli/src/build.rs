//! `cad build`'s own pipeline: parse -> lower -> type check -> execute
//! top-level -> (if requested) dispatch the resulting `GeometryGraph`
//! into a real kernel context and export STEP. See this crate's own
//! module doc comment for the exact scope this task covers.

use cad_diagnostics::json::Json;
use cad_diagnostics::{Diagnostic, Severity};
use std::path::{Path, PathBuf};

/// The `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §13 build-output status
/// this task's pipeline can actually determine — `"ok"`/`"failed"`, per
/// whether any collected diagnostic is `Severity::Error`. A clean build
/// with only warnings (none of this stage's phases currently emit any,
/// but a future phase might) is still `Ok`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildStatus {
    Ok,
    Failed,
}

impl BuildStatus {
    fn as_str(self) -> &'static str {
        match self {
            BuildStatus::Ok => "ok",
            BuildStatus::Failed => "failed",
        }
    }
}

/// The result of one `cad build` invocation — the subset of `docs/plan/
/// 17_CLI_DIAGNOSTICS_SCHEMA.md` §13's build-output schema this stage's
/// pipeline populates (see this crate's own module doc comment for why
/// the rest of that schema is intentionally omitted, not stubbed).
#[derive(Debug)]
pub struct BuildReport {
    pub status: BuildStatus,
    pub diagnostics: Vec<Diagnostic>,
    /// Paths this build actually wrote (only ever populated when
    /// `--output` was given and a geometry node was both present and
    /// successfully exported).
    pub artifacts: Vec<PathBuf>,
}

impl BuildReport {
    /// The process exit code this report implies: `0` for
    /// [`BuildStatus::Ok`], `1` for [`BuildStatus::Failed`] — the same
    /// two-outcome convention every phase below already uses internally.
    pub fn exit_code(&self) -> i32 {
        match self.status {
            BuildStatus::Ok => 0,
            BuildStatus::Failed => 1,
        }
    }

    /// Renders this report as one human-readable rustc-style block per
    /// diagnostic (`error[CODE]: title\n  message\n  --> file:line:col`),
    /// followed by a one-line summary. `cad_diagnostics::Diagnostic` has
    /// no `Display` impl of its own (RFC-0005 only froze the JSON shape),
    /// so this is this crate's own small, self-contained formatter.
    pub fn to_human_string(&self) -> String {
        let mut out = String::new();
        for diagnostic in &self.diagnostics {
            out.push_str(&format!(
                "{}[{}]: {}\n",
                diagnostic.severity.as_str(),
                diagnostic.code.as_string(),
                diagnostic.title
            ));
            out.push_str(&format!("  {}\n", diagnostic.message));
            if let Some(source) = &diagnostic.source {
                out.push_str(&format!(
                    "  --> {}:{}:{}\n",
                    source.file, source.start.line, source.start.column
                ));
            }
        }
        match self.status {
            BuildStatus::Ok => out.push_str("build succeeded\n"),
            BuildStatus::Failed => {
                let errors = self
                    .diagnostics
                    .iter()
                    .filter(|d| d.severity == Severity::Error)
                    .count();
                out.push_str(&format!("build failed ({errors} error(s))\n"));
            }
        }
        for artifact in &self.artifacts {
            out.push_str(&format!("wrote {}\n", artifact.display()));
        }
        out
    }

    /// Renders this report per `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md`
    /// §13's own build-output schema, restricted to the fields this
    /// task's pipeline actually populates.
    pub fn to_json(&self) -> Json {
        Json::object([
            ("status".to_string(), Json::str(self.status.as_str())),
            (
                "diagnostics".to_string(),
                Json::Array(self.diagnostics.iter().map(Diagnostic::to_json).collect()),
            ),
            (
                "artifacts".to_string(),
                Json::Array(
                    self.artifacts
                        .iter()
                        .map(|p| Json::str(p.display().to_string()))
                        .collect(),
                ),
            ),
        ])
    }
}

fn has_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == Severity::Error)
}

/// One `--name <binding>[.<field>]` resolution failure (`AICAD-079`) —
/// always an `EXPORT`-family error, since it only ever affects which
/// geometry `--output` exports, never the build's own diagnostics.
enum NamedOutputError {
    /// No top-level binding named `binding` exists (a plain `let`/`const`/
    /// `param`, or a `part`) in this program at all.
    UnknownBinding { binding: String },
    /// `binding` resolved, but to a value that is neither `Geometry` nor a
    /// `part` instance (e.g. a `Length` or a plain `struct`) — there is no
    /// field to look inside, so `.field` (if any) is meaningless.
    NotGeometryOrPart { binding: String, kind: &'static str },
    /// `binding` is a `part`, but it has no field named `field` at all.
    NoSuchField { binding: String, field: String },
    /// `binding.field` resolved to a value that is not `Geometry`.
    FieldNotGeometry {
        binding: String,
        field: String,
        kind: &'static str,
    },
    /// `binding` is a `part` and no `.field` was given, but it exposes
    /// zero `Geometry`-typed fields — there is nothing to export.
    NoGeometryFields { binding: String },
    /// `binding` is a `part` and no `.field` was given, but it exposes
    /// more than one `Geometry`-typed field — `explicit` durability
    /// (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §11) requires
    /// naming exactly one, never guessing.
    AmbiguousFields {
        binding: String,
        candidates: Vec<String>,
    },
}

impl NamedOutputError {
    fn diagnostic(&self, file: &str, source: &str) -> Diagnostic {
        let message = match self {
            NamedOutputError::UnknownBinding { binding } => {
                format!("no top-level binding named '{binding}' in this program")
            }
            NamedOutputError::NotGeometryOrPart { binding, kind } => format!(
                "'{binding}' is a {kind}, not a Geometry value or a part instance; --name \
                 can only select Geometry-typed named outputs"
            ),
            NamedOutputError::NoSuchField { binding, field } => {
                format!("part '{binding}' has no named output '{field}'")
            }
            NamedOutputError::FieldNotGeometry {
                binding,
                field,
                kind,
            } => format!("'{binding}.{field}' is a {kind}, not a Geometry value"),
            NamedOutputError::NoGeometryFields { binding } => {
                format!("part '{binding}' exposes no Geometry-typed named outputs to export")
            }
            NamedOutputError::AmbiguousFields {
                binding,
                candidates,
            } => format!(
                "part '{binding}' exposes more than one Geometry-typed named output ({}); \
                 use --name {binding}.<field> to pick one",
                candidates.join(", ")
            ),
        };
        environment_diagnostic(
            "EXPORT",
            2,
            "export",
            "NAMED_OUTPUT_NOT_RESOLVED",
            file,
            source,
            &message,
        )
    }
}

/// Resolves `--name <binding>[.<field>]` (`AICAD-079`) against the
/// program's own top-level bindings and the values [`cad_runtime::interp::
/// Interpreter::run_top_level`] already produced for them, to exactly one
/// [`cad_geometry_api::GeomId`] to export.
///
/// This is deliberately the *only* named-output selection Stage 3
/// implements: an ordinary lookup by the exact declared name of a
/// top-level `let`/`const`/`param` binding, or one named field of a
/// top-level `part`'s own already-executed outputs (`AICAD-071`) — the
/// `explicit` durability level in `docs/plan/06_REFERENCES_QUERIES_
/// FEATURE_DAG.md` §11 ("Feature exported the entity by semantic name").
/// It performs no query, no topology/geometry-fingerprint search, and no
/// lineage tracking across rebuilds — that is Stage-4 semantic-reference
/// resolution (`AGENTS.md`'s own Stage-3 boundary: "Stage-4 persistent
/// topology identity" is explicitly out of scope here).
fn resolve_named_output(
    interpreter: &cad_runtime::interp::Interpreter<'_>,
    bindings: &[cad_hir::ids::Binding],
    name: &str,
) -> Result<cad_geometry_api::GeomId, NamedOutputError> {
    let (binding_name, field_name) = match name.split_once('.') {
        Some((head, tail)) => (head, Some(tail)),
        None => (name, None),
    };

    let binding = bindings
        .iter()
        .find(|b| b.name == binding_name)
        .ok_or_else(|| NamedOutputError::UnknownBinding {
            binding: binding_name.to_string(),
        })?;
    let value = interpreter
        .global(binding.id)
        .ok_or_else(|| NamedOutputError::UnknownBinding {
            binding: binding_name.to_string(),
        })?;

    match value {
        cad_runtime::value::Value::Geometry(id) => {
            // A bare `Geometry`-typed binding has no field of its own —
            // a trailing `.field` on it is simply a name that does not
            // resolve, reported the same way an unknown part field is.
            if let Some(field) = field_name {
                return Err(NamedOutputError::NoSuchField {
                    binding: binding_name.to_string(),
                    field: field.to_string(),
                });
            }
            Ok(*id)
        }
        cad_runtime::value::Value::Part { fields, .. } => {
            if let Some(field) = field_name {
                let (_, field_value) =
                    fields.iter().find(|(n, _)| n == field).ok_or_else(|| {
                        NamedOutputError::NoSuchField {
                            binding: binding_name.to_string(),
                            field: field.to_string(),
                        }
                    })?;
                match field_value {
                    cad_runtime::value::Value::Geometry(id) => Ok(*id),
                    other => Err(NamedOutputError::FieldNotGeometry {
                        binding: binding_name.to_string(),
                        field: field.to_string(),
                        kind: other.kind_name(),
                    }),
                }
            } else {
                let geometry_fields: Vec<&str> = fields
                    .iter()
                    .filter_map(|(n, v)| match v {
                        cad_runtime::value::Value::Geometry(_) => Some(n.as_str()),
                        _ => None,
                    })
                    .collect();
                match geometry_fields.as_slice() {
                    [] => Err(NamedOutputError::NoGeometryFields {
                        binding: binding_name.to_string(),
                    }),
                    [only] => {
                        let (_, field_value) = fields
                            .iter()
                            .find(|(n, _)| n == only)
                            .expect("the field name just collected above is present in `fields`");
                        match field_value {
                            cad_runtime::value::Value::Geometry(id) => Ok(*id),
                            _ => unreachable!("already filtered to Geometry-typed fields above"),
                        }
                    }
                    many => Err(NamedOutputError::AmbiguousFields {
                        binding: binding_name.to_string(),
                        candidates: many.iter().map(|s| s.to_string()).collect(),
                    }),
                }
            }
        }
        other => Err(NamedOutputError::NotGeometryOrPart {
            binding: binding_name.to_string(),
            kind: other.kind_name(),
        }),
    }
}

/// Runs the full `cad build` pipeline against `source`, as if it were the
/// contents of `file`. Exposed separately from [`run_build`] (which reads
/// `file` off disk) so tests can exercise the pipeline against an in-memory
/// string with no filesystem dependency.
///
/// `output_name` is `--name`'s own optional `<binding>[.<field>]` argument
/// (`AICAD-079`) — when given, [`resolve_named_output`] alone decides what
/// `output` exports; when omitted, the pre-existing Stage-2 default
/// applies unchanged (the last geometry-producing node created anywhere in
/// the program's shared `GeometryGraph`), so every already-passing
/// Stage-2/Stage-3 caller is unaffected.
pub fn build_source(
    file: &str,
    source: &str,
    output: Option<&Path>,
    output_name: Option<&str>,
) -> BuildReport {
    let mut diagnostics = Vec::new();

    let (program, parse_diagnostics) = cad_parser::parse_program(source, file);
    diagnostics.extend(parse_diagnostics);
    if has_error(&diagnostics) {
        return BuildReport {
            status: BuildStatus::Failed,
            diagnostics,
            artifacts: Vec::new(),
        };
    }

    let lowered = cad_hir::lower_program(&program, file, source);
    diagnostics.extend(lowered.diagnostics.clone());
    if has_error(&diagnostics) {
        return BuildReport {
            status: BuildStatus::Failed,
            diagnostics,
            artifacts: Vec::new(),
        };
    }

    let checked = cad_hir::check_program(&lowered.program, &lowered.bindings, file, source);
    diagnostics.extend(checked.diagnostics);
    if has_error(&diagnostics) {
        return BuildReport {
            status: BuildStatus::Failed,
            diagnostics,
            artifacts: Vec::new(),
        };
    }

    let mut interpreter =
        cad_runtime::interp::Interpreter::new(&lowered.program, &lowered.bindings, file, source);
    if let Err(diagnostic) = interpreter.run_top_level(&lowered.program) {
        diagnostics.push(*diagnostic);
        return BuildReport {
            status: BuildStatus::Failed,
            diagnostics,
            artifacts: Vec::new(),
        };
    }

    let mut artifacts = Vec::new();
    if let Some(output_path) = output {
        let target = match output_name {
            Some(name) => resolve_named_output(&interpreter, &lowered.bindings, name)
                .map(Some)
                .unwrap_or_else(|err| {
                    diagnostics.push(err.diagnostic(file, source));
                    None
                }),
            None => {
                let graph = interpreter.geometry_graph();
                graph
                    .nodes()
                    .iter()
                    .rev()
                    .find(|node| node.kind.produces_geometry())
                    .map(|node| node.id)
            }
        };
        let graph = interpreter.geometry_graph();
        if let Some(id) = target {
            match cad_occt_bridge::OcctContext::new() {
                Ok(ctx) => match cad_geometry_runtime::dispatch_graph(graph, &ctx) {
                    Ok(results) => match &results[id.index() as usize] {
                        cad_geometry_runtime::NodeResult::Shape(shape) => {
                            match shape.export_step(output_path) {
                                Ok(()) => artifacts.push(output_path.to_path_buf()),
                                Err(err) => diagnostics.push(export_step_diagnostic(
                                    file,
                                    source,
                                    &err.to_string(),
                                )),
                            }
                        }
                        other => diagnostics.push(export_step_diagnostic(
                            file,
                            source,
                            &format!("internal error: expected a Shape result, got {other:?}"),
                        )),
                    },
                    Err(dispatch_err) => diagnostics.push(dispatch_err.to_diagnostic(file, source)),
                },
                Err(err) => {
                    diagnostics.push(export_step_diagnostic(file, source, &err.to_string()))
                }
            }
        }
    }

    let status = if has_error(&diagnostics) {
        BuildStatus::Failed
    } else {
        BuildStatus::Ok
    };
    BuildReport {
        status,
        diagnostics,
        artifacts,
    }
}

/// A minimal environment-level diagnostic under `family`/`number` for a
/// failure with no more specific `Diagnostic` of its own to reuse and no
/// meaningful span (a build/environment-level failure, not a specific
/// program location — reported at file offset 0).
fn environment_diagnostic(
    family: &'static str,
    number: u16,
    category: &'static str,
    title: &'static str,
    file: &str,
    source: &str,
    message: &str,
) -> Diagnostic {
    let code = cad_diagnostics::DiagnosticCode::new(
        family,
        cad_diagnostics::SeverityLetter::Error,
        number,
    )
    .expect("family/number given here are always a valid diagnostic code");
    let line_index = cad_ast::LineIndex::new(source);
    let start = line_index.line_column(source, 0);
    Diagnostic::new(code, Severity::Error, category, title, message.to_string())
        .expect("every code built here carries the 'E' severity letter")
        .with_source(cad_diagnostics::SourceSpan {
            file: file.to_string(),
            start: cad_diagnostics::Position::new(start.line, start.column),
            end: cad_diagnostics::Position::new(start.line, start.column),
        })
}

/// A `EXPORT`-family diagnostic for a STEP-export/kernel-context failure
/// (`OcctContext::new`'s and `Shape::export_step`'s own `KernelError`
/// carries no source span of its own).
fn export_step_diagnostic(file: &str, source: &str, message: &str) -> Diagnostic {
    environment_diagnostic(
        "EXPORT",
        1,
        "export",
        "STEP_EXPORT_FAILED",
        file,
        source,
        message,
    )
}

/// Reads `path` off disk and runs [`build_source`] against its contents.
/// The only way this crate touches the filesystem for the source file
/// itself (`build_source` takes an in-memory string for testability). A
/// read failure is reported under the `PARSE` family (a precondition of
/// parsing: there is no source text to parse at all).
pub fn run_build(path: &Path, output: Option<&Path>, output_name: Option<&str>) -> BuildReport {
    let file = path.display().to_string();
    match std::fs::read_to_string(path) {
        Ok(source) => build_source(&file, &source, output, output_name),
        Err(err) => BuildReport {
            status: BuildStatus::Failed,
            diagnostics: vec![environment_diagnostic(
                "PARSE",
                // A high, deliberately-out-of-range number: `cad-lexer`/
                // `cad-parser` own the low end of the `PARSE` family for
                // program-level diagnostics (`PARSE-E001`..); this is a
                // CLI/environment-level failure (no source text exists to
                // parse at all), kept in the same family only because no
                // more specific family fits, chosen well clear of that
                // range specifically to avoid colliding with a future
                // `cad-lexer`/`cad-parser` code — `project/
                // DECISION_LOG.md#DL-18` now makes any such collision a
                // real stability violation, not merely a style concern.
                900,
                "parse",
                "SOURCE_FILE_UNREADABLE",
                &file,
                "",
                &format!("could not read '{file}': {err}"),
            )],
            artifacts: Vec::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_parse_error_stops_the_pipeline_and_is_reported() {
        let report = build_source("test.aicad", "let x = ;", None, None);
        assert_eq!(report.status, BuildStatus::Failed);
        assert!(!report.diagnostics.is_empty());
        assert!(report.artifacts.is_empty());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code.as_string().starts_with("PARSE-")),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn a_type_error_stops_the_pipeline_before_execution() {
        let report = build_source("test.aicad", "let x = box(1mm, 2kg, 3mm);", None, None);
        assert_eq!(report.status, BuildStatus::Failed);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code.as_string() == "TYPE-E418"),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn a_clean_program_with_no_output_requested_succeeds_with_no_artifacts() {
        let report = build_source("test.aicad", "let x = 5mm + 2cm;", None, None);
        assert_eq!(report.status, BuildStatus::Ok);
        assert!(report.diagnostics.is_empty());
        assert!(report.artifacts.is_empty());
    }

    #[test]
    fn a_program_constructing_geometry_exports_step_when_output_is_requested() {
        let output_path = std::env::temp_dir().join("aicad_cli_build_test_output.step");
        let _ = std::fs::remove_file(&output_path);
        let report = build_source(
            "test.aicad",
            "let piece = box(10mm, 10mm, 10mm);",
            Some(&output_path),
            None,
        );
        assert_eq!(report.status, BuildStatus::Ok, "{:?}", report.diagnostics);
        assert_eq!(report.artifacts, vec![output_path.clone()]);
        assert!(output_path.exists());
        let contents = std::fs::read_to_string(&output_path).unwrap();
        assert!(
            contents.contains("ISO-10303"),
            "not a STEP file:\n{contents}"
        );
        let _ = std::fs::remove_file(&output_path);
    }

    #[test]
    fn a_program_with_no_geometry_and_output_requested_produces_no_artifact() {
        let output_path = std::env::temp_dir().join("aicad_cli_build_test_no_geometry.step");
        let _ = std::fs::remove_file(&output_path);
        let report = build_source("test.aicad", "let x = 5mm;", Some(&output_path), None);
        assert_eq!(report.status, BuildStatus::Ok);
        assert!(report.artifacts.is_empty());
        assert!(!output_path.exists());
    }

    #[test]
    fn human_and_json_rendering_both_reflect_a_failed_build() {
        let report = build_source("test.aicad", "let x = ;", None, None);
        let human = report.to_human_string();
        assert!(human.contains("error["));
        assert!(human.contains("build failed"));
        let json = report.to_json();
        assert_eq!(json.get("status").and_then(Json::as_str), Some("failed"));
        assert!(
            !json
                .get("diagnostics")
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn json_rendering_reflects_a_successful_build_with_an_artifact() {
        let output_path = std::env::temp_dir().join("aicad_cli_build_test_json_artifact.step");
        let _ = std::fs::remove_file(&output_path);
        let report = build_source(
            "test.aicad",
            "let piece = box(5mm, 5mm, 5mm);",
            Some(&output_path),
            None,
        );
        let json = report.to_json();
        assert_eq!(json.get("status").and_then(Json::as_str), Some("ok"));
        let artifacts = json.get("artifacts").unwrap().as_array().unwrap();
        assert_eq!(artifacts.len(), 1);
        let _ = std::fs::remove_file(&output_path);
    }

    // --- AICAD-079: `--name` named-output export selection --------------

    #[test]
    fn name_selects_a_plain_top_level_binding_even_when_it_is_not_the_last_geometry_node() {
        let output_path = std::env::temp_dir().join("aicad_079_named_plain_binding.step");
        let _ = std::fs::remove_file(&output_path);
        // `scrap` is constructed *after* `wanted`, so the pre-`AICAD-079`
        // "last geometry node" default would export the wrong shape here.
        let report = build_source(
            "test.aicad",
            "let wanted = box(10mm, 10mm, 10mm); let scrap = box(1mm, 1mm, 1mm);",
            Some(&output_path),
            Some("wanted"),
        );
        assert_eq!(report.status, BuildStatus::Ok, "{:?}", report.diagnostics);
        assert_eq!(report.artifacts, vec![output_path.clone()]);
        let contents = std::fs::read_to_string(&output_path).unwrap();
        assert!(contents.contains("ISO-10303"));
        let _ = std::fs::remove_file(&output_path);
    }

    #[test]
    fn name_selects_one_named_field_of_a_part() {
        let output_path = std::env::temp_dir().join("aicad_079_named_part_field.step");
        let _ = std::fs::remove_file(&output_path);
        let report = build_source(
            "test.aicad",
            "part Bracket { \
                 let scrap: Geometry = box(1mm, 1mm, 1mm); \
                 let body: Geometry = box(10mm, 10mm, 10mm); \
             }",
            Some(&output_path),
            Some("Bracket.body"),
        );
        assert_eq!(report.status, BuildStatus::Ok, "{:?}", report.diagnostics);
        assert_eq!(report.artifacts, vec![output_path.clone()]);
        let _ = std::fs::remove_file(&output_path);
    }

    #[test]
    fn name_with_no_field_resolves_a_part_s_sole_geometry_field_automatically() {
        let output_path = std::env::temp_dir().join("aicad_079_named_part_sole_field.step");
        let _ = std::fs::remove_file(&output_path);
        let report = build_source(
            "test.aicad",
            "part Bracket { \
                 let width: Length = 10mm; \
                 let body: Geometry = box(width, width, width); \
             }",
            Some(&output_path),
            Some("Bracket"),
        );
        assert_eq!(report.status, BuildStatus::Ok, "{:?}", report.diagnostics);
        assert_eq!(report.artifacts, vec![output_path.clone()]);
        let _ = std::fs::remove_file(&output_path);
    }

    #[test]
    fn name_with_no_field_on_a_part_with_two_geometry_fields_is_ambiguous() {
        let output_path = std::env::temp_dir().join("aicad_079_named_ambiguous.step");
        let _ = std::fs::remove_file(&output_path);
        let report = build_source(
            "test.aicad",
            "part Bracket { \
                 let left: Geometry = box(1mm, 1mm, 1mm); \
                 let right: Geometry = box(2mm, 2mm, 2mm); \
             }",
            Some(&output_path),
            Some("Bracket"),
        );
        assert_eq!(report.status, BuildStatus::Failed);
        assert!(report.artifacts.is_empty());
        assert!(!output_path.exists());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code.as_string() == "EXPORT-E002"
                    && d.message.contains("left")
                    && d.message.contains("right")),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn name_referring_to_an_unknown_binding_is_reported_and_fails_the_build() {
        let output_path = std::env::temp_dir().join("aicad_079_named_unknown.step");
        let _ = std::fs::remove_file(&output_path);
        let report = build_source(
            "test.aicad",
            "let piece = box(10mm, 10mm, 10mm);",
            Some(&output_path),
            Some("nonexistent"),
        );
        assert_eq!(report.status, BuildStatus::Failed);
        assert!(report.artifacts.is_empty());
        assert!(!output_path.exists());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code.as_string() == "EXPORT-E002" && d.message.contains("nonexistent")),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn name_referring_to_an_unknown_field_on_a_part_is_reported() {
        let output_path = std::env::temp_dir().join("aicad_079_named_unknown_field.step");
        let _ = std::fs::remove_file(&output_path);
        let report = build_source(
            "test.aicad",
            "part Bracket { let body: Geometry = box(10mm, 10mm, 10mm); }",
            Some(&output_path),
            Some("Bracket.nope"),
        );
        assert_eq!(report.status, BuildStatus::Failed);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code.as_string() == "EXPORT-E002" && d.message.contains("nope")),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn name_referring_to_a_non_geometry_binding_is_reported() {
        let output_path = std::env::temp_dir().join("aicad_079_named_non_geometry.step");
        let _ = std::fs::remove_file(&output_path);
        let report = build_source(
            "test.aicad",
            "let x: Length = 5mm;",
            Some(&output_path),
            Some("x"),
        );
        assert_eq!(report.status, BuildStatus::Failed);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code.as_string() == "EXPORT-E002" && d.message.contains("Number")),
            "{:?}",
            report.diagnostics
        );
    }
}
