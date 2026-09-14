//! `cad refs check` (`AICAD-095`), per `docs/plan/17_CLI_DIAGNOSTICS_
//! SCHEMA.md` §3 and `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md`
//! §12: builds a `.aicad` source file through the real
//! [`ParametricBuildSession`] pipeline (the same one `cad build` itself
//! uses, via `crate::build`) and reports the health
//! (`cad_query::check_reference_health`) of that build's own stable
//! reference set.
//!
//! # Why the reported reference set is currently always empty
//!
//! See `cad_query::health`'s own module doc comment: `.aicad` source has
//! no syntax yet to *declare* a persistent stable reference (`query {
//! ... }` blocks remain reserved/unimplemented, per
//! `rfcs/0003-semantic-references.md` §7), so a real build's own
//! reference set is, honestly, always empty today — this module supplies
//! [`cad_query::check_reference_health`] with the real (currently empty)
//! set a real program can produce, rather than inventing one (e.g.
//! treating every top-level binding as an implicit "explicit export"
//! would be exactly the kind of unapproved semantics this campaign's own
//! escalation rules forbid). This command is nonetheless real, working
//! machinery — parsing/lowering/type-checking/building the program
//! through the exact same [`ParametricBuildSession`] `cad build` itself
//! uses, and reporting real diagnostics on failure — not a stub that
//! merely prints a canned report; `cad_query::health`'s own test suite
//! separately proves the underlying aggregation logic against a real,
//! non-empty, mixed-outcome reference set.

use std::path::Path;

use cad_diagnostics::Diagnostic;
use cad_diagnostics::json::Json;
use cad_occt_bridge::OcctContext;
use cad_query::ReferenceHealthReport;
use cad_references::AnyRef;

use crate::build::environment_diagnostic;
use crate::parametric_build::ParametricBuildSession;

/// `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §13-style status, mirroring
/// `crate::build::BuildStatus`'s own two-outcome convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefsCheckStatus {
    Ok,
    Failed,
}

impl RefsCheckStatus {
    fn as_str(self) -> &'static str {
        match self {
            RefsCheckStatus::Ok => "ok",
            RefsCheckStatus::Failed => "failed",
        }
    }
}

/// The result of one `cad refs check` invocation.
#[derive(Debug)]
pub struct RefsCheckReport {
    pub status: RefsCheckStatus,
    pub diagnostics: Vec<Diagnostic>,
    /// `Some` exactly when `status` is [`RefsCheckStatus::Ok`] — a build
    /// failure has no reference set to report health for at all.
    pub health: Option<ReferenceHealthReport>,
}

impl RefsCheckReport {
    pub fn exit_code(&self) -> i32 {
        match self.status {
            RefsCheckStatus::Ok => 0,
            RefsCheckStatus::Failed => 1,
        }
    }

    /// Mirrors `crate::build::BuildReport::to_human_string`'s own
    /// rustc-style diagnostic block, followed by
    /// [`ReferenceHealthReport::to_human_string`]'s own plan-§12-shaped
    /// report when the build succeeded.
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
        match &self.health {
            Some(health) => out.push_str(&health.to_human_string()),
            None => out.push_str("refs check failed\n"),
        }
        out
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("status".to_string(), Json::str(self.status.as_str())),
            (
                "diagnostics".to_string(),
                Json::Array(self.diagnostics.iter().map(Diagnostic::to_json).collect()),
            ),
            (
                "health".to_string(),
                match &self.health {
                    Some(health) => health.to_json(),
                    None => Json::Null,
                },
            ),
        ])
    }
}

/// Runs `cad refs check <path>`: reads `path` off disk and runs
/// [`refs_check_source`] against its contents — the same split
/// `crate::build::run_build`/`build_source` already establishes, kept for
/// the same reason (`refs_check_source` takes an in-memory string for
/// testability; this is the only place this module touches the
/// filesystem for the source file itself). A read failure is reported
/// under the `PARSE` family, mirroring `run_build`'s own identical
/// `SOURCE_FILE_UNREADABLE` case exactly (a precondition of parsing:
/// there is no source text to parse at all).
pub fn run_refs_check(path: &Path) -> RefsCheckReport {
    let file = path.display().to_string();
    match std::fs::read_to_string(path) {
        Ok(source) => refs_check_source(&file, &source),
        Err(err) => RefsCheckReport {
            status: RefsCheckStatus::Failed,
            diagnostics: vec![environment_diagnostic(
                "PARSE",
                900,
                "parse",
                "SOURCE_FILE_UNREADABLE",
                &file,
                "",
                &format!("could not read '{file}': {err}"),
            )],
            health: None,
        },
    }
}

/// Builds `source` (as if it were the contents of `file`) and reports the
/// health of its own reference set — see this module's own doc comment
/// for exactly what that means today. A build failure is reported as
/// `Failed` with the real collected diagnostics, exactly `crate::build::
/// build_source`'s own convention — this never trades a build failure for
/// a fabricated "no references" success report.
pub fn refs_check_source(file: &str, source: &str) -> RefsCheckReport {
    let ctx = match OcctContext::new() {
        Ok(ctx) => ctx,
        Err(err) => {
            return RefsCheckReport {
                status: RefsCheckStatus::Failed,
                diagnostics: vec![environment_diagnostic(
                    "EXPORT",
                    900,
                    "export",
                    "KERNEL_CONTEXT_UNAVAILABLE",
                    file,
                    source,
                    &format!("could not create a kernel context: {err}"),
                )],
                health: None,
            };
        }
    };

    let session = match ParametricBuildSession::new(file, source, &ctx) {
        Ok(session) => session,
        Err(diagnostics) => {
            return RefsCheckReport {
                status: RefsCheckStatus::Failed,
                diagnostics,
                health: None,
            };
        }
    };

    // No `.aicad` source syntax exists yet to declare a persistent stable
    // reference -- see this module's own doc comment. A real reference
    // set can only ever be empty today, so `check_reference_health` is
    // never actually asked to resolve anything and can never itself fail.
    let references: Vec<AnyRef> = Vec::new();
    let health = cad_query::check_reference_health(&references, &session)
        .expect("an empty reference set never calls the resolver");

    RefsCheckReport {
        status: RefsCheckStatus::Ok,
        diagnostics: Vec::new(),
        health: Some(health),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_valid_program_reports_an_all_zero_healthy_reference_set() {
        let report = refs_check_source("test.aicad", "let x = box(1mm, 1mm, 1mm);\n");
        assert_eq!(report.status, RefsCheckStatus::Ok);
        assert!(report.diagnostics.is_empty());
        let health = report.health.as_ref().unwrap();
        assert_eq!(health.total, 0);
        assert_eq!(health.resolved, 0);
        assert_eq!(health.ambiguous, 0);
        assert_eq!(health.broken, 0);
    }

    #[test]
    fn a_parse_error_is_reported_as_a_failed_check_with_no_health_report() {
        let report = refs_check_source("test.aicad", "let x = ;\n");
        assert_eq!(report.status, RefsCheckStatus::Failed);
        assert!(!report.diagnostics.is_empty());
        assert!(report.health.is_none());
    }

    #[test]
    fn an_unreadable_path_is_reported_as_a_failed_check() {
        let report = run_refs_check(Path::new("/nonexistent/path/does/not/exist.aicad"));
        assert_eq!(report.status, RefsCheckStatus::Failed);
        assert!(!report.diagnostics.is_empty());
        assert!(report.health.is_none());
    }

    /// The only real-disk exercise of [`run_refs_check`] itself (every
    /// other test uses [`refs_check_source`] directly, matching
    /// `crate::build`'s own `build_source`/`run_build` split) — a plain
    /// `std::fs`-written fixture, no third-party crate, matching every
    /// other `cad-cli` test's own temp-file convention
    /// (`std::env::temp_dir()`).
    #[test]
    fn run_refs_check_reads_a_real_file_off_disk() {
        let path = std::env::temp_dir().join("aicad_095_refs_check_real_file.aicad");
        std::fs::write(&path, "let x = box(1mm, 1mm, 1mm);\n").unwrap();
        let report = run_refs_check(&path);
        assert_eq!(report.status, RefsCheckStatus::Ok);
        assert_eq!(report.health.unwrap().total, 0);
    }

    #[test]
    fn human_and_json_rendering_of_a_healthy_report_match_plan_12s_shape() {
        let report = refs_check_source("test.aicad", "let x = box(1mm, 1mm, 1mm);\n");
        assert_eq!(
            report.to_human_string(),
            "0 semantic references\n\
             0 explicit/lineage\n\
             0 strong queries\n\
             0 geometric fallbacks\n\
             0 ambiguous\n\
             0 broken\n"
        );
        let json = report.to_json();
        assert_eq!(json.get("status").unwrap(), &Json::str("ok"));
        assert!(json.get("health").unwrap().get("total").is_some());
    }
}
