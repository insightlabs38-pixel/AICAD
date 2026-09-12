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
//! diagnostics." This crate therefore implements exactly `cad build
//! <path> [--json] [--output <path>]` — none of the other command groups
//! §2-9 name, and only the subset of §13's build-output schema this
//! stage's pipeline can actually populate (`status`/`diagnostics`/
//! `artifacts`; `configuration`/`changed_features`/`reference_health`/
//! `verification`/`resource_usage` are Stage 3+ concepts with no backing
//! crate yet — omitted entirely rather than emitted as empty placeholders,
//! per `AGENTS.md`'s "No speculative future work").
//!
//! `AICAD-063`'s own end-to-end proof (a full bracket-shaped fixture with
//! parameters/derived expressions/control flow, verified by exact B-rep
//! properties and STEP re-import) is a separate, later task; this crate
//! only needs to prove the *pipeline plumbing* works for whatever program
//! it is given, not demonstrate that fixture itself.
//!
//! ## Which geometry gets exported
//!
//! A `.aicad` program has no source-level "this is the part to build"
//! designation yet (no such syntax exists in Stage 2). When `--output` is
//! given, this crate exports the **last** `Geometry`-producing node
//! appended to the interpreter's own accumulated `GeometryGraph` (i.e. the
//! most recently constructed shape) — a documented, narrow default for
//! this task's own scope, not a claim that it generalizes to a real
//! multi-part project; a later task introducing an explicit "build
//! target" designation should replace this rule rather than extend it.
//!
//! No third-party (crates.io) dependency is introduced here, consistent
//! with every other Stage-2 crate's own zero-external-dependency policy —
//! argument parsing is hand-rolled (`ParsedArgs`), and diagnostics are
//! rendered directly from `cad_diagnostics::Diagnostic`/`Json`, reusing
//! `Diagnostic::to_json` rather than a third-party serializer.

pub mod build;
pub mod cli;

pub use build::{BuildReport, BuildStatus, run_build};
pub use cli::{ArgsError, ParsedArgs, parse_args};
