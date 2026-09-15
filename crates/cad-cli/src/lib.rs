//! `cad-cli` (`AICAD-061`) — the `cad build` command: reads one `.aicad`
//! source file through the complete Stage-2 pipeline (parse -> lower ->
//! type check -> execute top-level -> dispatch any constructed geometry
//! into the kernel), and reports every diagnostic collected along the way
//! in either human-readable or JSON form (`docs/plan/17_CLI_DIAGNOSTICS_
//! SCHEMA.md` §2's `cad build --json` option, §11's diagnostic schema).
//!
//! ## Scope (this task's own boundary)
//!
//! `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` sketches a large, aspirational
//! CLI surface (`cad new`/`test`/`inspect`/`docs`/`import`/`package`/
//! `debug`/`diff`/...) spanning every later Stage. `AICAD-061`'s own task
//! title is narrower: "Create cad-cli build command with human and JSON
//! diagnostics." This crate therefore implements exactly `cad build <path>
//! [--json] [--output <path>] [--name <binding>[.<field>]]` (`--name`
//! added by `AICAD-079`, see below) — none of the other command groups
//! §2-9 name, and only the subset of §13's build-output schema this
//! stage's pipeline can actually populate (`status`/`diagnostics`/
//! `artifacts`; `configuration`/`changed_features`/`verification`/
//! `resource_usage` are Stage 3+ concepts with no backing crate yet —
//! omitted entirely rather than emitted as empty placeholders, per
//! `AGENTS.md`'s "No speculative future work"). `AICAD-095` adds this
//! crate's second command, `cad refs check <path> [--json]` — §3's
//! reference-health verification command — via [`cli::parse_command`]/
//! [`refs_check`]; see that module's own doc comment for why its own
//! reported reference set is currently always empty. `AICAD-097` adds
//! [`perturbation`] — a deterministic perturbation-case runner (`docs/
//! plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §5's own "topological naming
//! benchmark" methodology) — as a library module only, with no CLI
//! subcommand of its own yet.
//!
//! `AICAD-063`'s own end-to-end proof (a full bracket-shaped fixture with
//! parameters/derived expressions/control flow, verified by exact B-rep
//! properties and STEP re-import) is a separate, later task; this crate
//! only needs to prove the *pipeline plumbing* works for whatever program
//! it is given, not demonstrate that fixture itself.
//!
//! ## Which geometry gets exported
//!
//! When `--output` is given without `--name`, this crate still exports
//! the **last** `Geometry`-producing node appended to the interpreter's
//! own accumulated `GeometryGraph` (i.e. the most recently constructed
//! shape) — the original Stage-2 default, unchanged, for a program with
//! no declared named outputs at all.
//!
//! `AICAD-079` (Stage 3, "named semantic outputs baseline") added
//! `--name <binding>[.<field>]`: an explicit selection, by the exact
//! declared name of a top-level `let`/`const`/`param` binding or one
//! named field of a top-level `part`'s own executed outputs (`AICAD-071`),
//! of exactly which `Geometry` value to export — the `explicit` durability
//! level (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §11), not a
//! query or topology search. See `build::resolve_named_output`'s own doc
//! comment. This is the "build target" designation this doc comment
//! itself once called out as missing; the old positional default remains
//! available (and is still what an unnamed `--output` uses) precisely so
//! no already-passing Stage-2/Stage-3 caller changes behavior.
//!
//! No third-party (crates.io) dependency is introduced here, consistent
//! with every other Stage-2 crate's own zero-external-dependency policy —
//! argument parsing is hand-rolled (`ParsedArgs`), and diagnostics are
//! rendered directly from `cad_diagnostics::Diagnostic`/`Json`, reusing
//! `Diagnostic::to_json` rather than a third-party serializer.

pub mod build;
pub mod cli;
pub mod metrics;
pub mod parametric_build;
pub mod perturbation;
pub mod reference_replay;
pub mod refs_check;

pub use build::{BuildReport, BuildStatus, run_build};
pub use cli::{
    ArgsError, Command, ParsedArgs, RefsCheckArgs, parse_args, parse_command, parse_refs_check_args,
};
pub use parametric_build::{ParametricBuildSession, RebuildOutcome};
pub use perturbation::{PerturbationCase, PerturbationRun, RunOutcome, run_case};
pub use reference_replay::FeatureLineageIndex;
pub use refs_check::{RefsCheckReport, RefsCheckStatus, refs_check_source, run_refs_check};
