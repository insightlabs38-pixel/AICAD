# AICAD-061: Create cad-cli build command with human and JSON diagnostics

## Status: COMPLETE

## Objective

Per `project/TASKS.yaml`: implement `cad-cli`'s `build` command with both
human-readable and JSON diagnostic output, per `docs/plan/
17_CLI_DIAGNOSTICS_SCHEMA.md` §2 (`cad build [path] [--json] ...`) and §11
(the diagnostic JSON schema, already implemented by `cad_diagnostics::
Diagnostic::to_json`/`AICAD-038`).

## Scope decision

`docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` sketches a large, aspirational
CLI surface spanning every later stage (`new`/`test`/`inspect`/`docs`/
`import`/`package`/`debug`/`diff`/...). This task's own title is narrower:
only the `build` command. This crate therefore implements exactly `cad
build <path> [--json] [--output <path>]` — no other command group, and
only the subset of §13's build-output schema (`status`/`diagnostics`/
`artifacts`) this stage's pipeline can actually populate (`configuration`/
`changed_features`/`reference_health`/`verification`/`resource_usage` are
Stage 3+ concepts with no backing crate yet — omitted entirely, per
`AGENTS.md`'s "No speculative future work", rather than emitted as fake
empty placeholders).

`AICAD-063`'s own end-to-end proof (a full bracket-shaped fixture,
verified by exact B-rep properties and STEP re-import) is separate and
later; this task only needed to prove the pipeline plumbing works for
whatever program it is given.

## What was implemented

Populated the previously-empty `AICAD-002`/`AICAD-003` placeholder
`crates/cad-cli`:

- **`build` module** (`crates/cad-cli/src/build.rs`): `build_source(file,
  source, output)` runs the complete pipeline — `cad_parser::parse_program`
  -> `cad_hir::lower_program` -> `cad_hir::check_program` ->
  `cad_runtime::interp::Interpreter::run_top_level` -> (if `output` is
  given and the interpreter's own `geometry_graph()` has any
  geometry-producing node) `cad_geometry_runtime::dispatch_graph` against
  a real `cad_occt_bridge::OcctContext`, then `Shape::export_step` on the
  **last** geometry node constructed (documented as this task's own narrow
  default — no `.aicad` source-level "build target" designation syntax
  exists yet; a later task introducing one should replace this rule, not
  extend it). Stops and reports at the first phase producing an
  `Severity::Error` diagnostic, never running a later phase against an
  already-broken program. Returns a `BuildReport { status, diagnostics,
  artifacts }`.
- `BuildReport::to_human_string()` — a small, self-contained rustc-style
  formatter (`cad_diagnostics::Diagnostic` has no `Display` impl of its
  own; RFC-0005 only froze the JSON shape).
- `BuildReport::to_json()` — reuses `Diagnostic::to_json` verbatim per
  diagnostic, wrapped in the `status`/`diagnostics`/`artifacts` subset of
  §13's own schema.
- **`cli` module** (`crates/cad-cli/src/cli.rs`): hand-rolled argument
  parsing (`build <path> [--json] [--output <path>]`) — no third-party
  crate, matching every other Stage-2 crate's zero-external-dependency
  policy.
- **`main.rs`**: a thin binary wiring `cli::parse_args`/`build::run_build`
  together and setting the process exit code from `BuildReport::
  exit_code()`.
- Two new diagnostic codes: `EXPORT-E001` (`STEP_EXPORT_FAILED` — the
  `EXPORT` family, reserved since `AICAD-038`, was unused until now) and a
  deliberately out-of-range `PARSE-E900` (`SOURCE_FILE_UNREADABLE` — a
  CLI/environment-level failure, kept out of `cad-lexer`/`cad-parser`'s own
  low-numbered `PARSE-E001`.. range to avoid colliding with an existing
  program-level diagnostic meaning).

## Tests

15 tests in `crates/cad-cli`: argument-parsing (7, `cli.rs` — bare
invocation, flag combinations in either order, every error case) and
pipeline (8, `build.rs`) — a parse error stops the pipeline; a type error
(`box(1mm, 2kg, 3mm)`) stops it before execution; a clean program with no
`--output` succeeds with zero artifacts; a program constructing geometry
(`box(...)`) with `--output` given actually writes a real STEP file
(verified by reading it back and checking for the `ISO-10303` header — not
merely that a file exists) through a real `OcctContext`; a program with no
geometry and `--output` given produces no artifact (never a spurious
empty/error export); human and JSON rendering both correctly reflect a
failed build; JSON rendering correctly reflects a successful build with an
artifact path.

Additionally smoke-tested the actual compiled binary directly (`cargo run
-p cad-cli --bin cad-cli -- build ...`), not only the library's own unit
tests:

```
$ cargo run -p cad-cli --bin cad-cli -- build /tmp/smoke.aicad --output /tmp/smoke.step
(source: "let piece = box(10mm, 10mm, 10mm);")
... [OCCT STEP writer log] ...
build succeeded
wrote /tmp/smoke.step
$ head -3 /tmp/smoke.step
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Open CASCADE Model'),'2;1');

$ cargo run -p cad-cli --bin cad-cli -- build /tmp/smoke_bad.aicad --json
(source: "let x = box(1mm, 2kg, 3mm);")
{"status":"failed","diagnostics":[{"code":"TYPE-E418","severity":"error",
"category":"type-check","title":"ARGUMENT_TYPE_MISMATCH","message":
"expected type Length, found Mass", ...}],"artifacts":[]}
```

## Exact commands and results

```
$ cargo build -p cad-cli --bin cad-cli
Finished, clean.

$ cargo test -p cad-cli
15 passed; 0 failed.

$ cargo fmt --all -- --check
(clean, after one `cargo fmt --all` pass)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.22s
(clean, zero warnings)

$ cargo test --workspace
Every crate: test result: ok, 0 failed anywhere.
cad-cli: 15 (new), all other crates unchanged from the AICAD-060 session's
own baseline.
```

## Known limitations

- Only `cad build` is implemented — every other command group `docs/plan/
  17_CLI_DIAGNOSTICS_SCHEMA.md` sketches (`test`/`inspect`/`docs`/
  `import`/`package`/`debug`/`diff`/...) is out of this task's scope, per
  its own title.
- The build-output JSON is a strict subset of §13's schema
  (`status`/`diagnostics`/`artifacts` only) — see scope decision above.
- "Which geometry gets exported" uses a documented narrow default (the
  last-constructed node) since no source-level build-target designation
  syntax exists yet.
- `--configuration`/`--profile`/`--target`/`--budget` (§2's other `cad
  build` options) are not implemented — none of the Stage-2 infrastructure
  they'd need (`cad-configurations`, resource-budget profiles beyond the
  interpreter's own recursion limit, non-STEP export targets) exists yet.

## Unresolved questions

None.
