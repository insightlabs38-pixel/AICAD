# AICAD-004 — Create baseline CI: fmt, clippy, tests, native build smoke

## Objective
Add baseline continuous integration covering `cargo fmt --check`,
`cargo clippy -D warnings`, `cargo build`/`test`, and a native-bridge build
smoke check, per `project/TASKS.yaml` (AICAD-004).

## Dependencies checked
AICAD-003 (Rust workspace/toolchain policy) — complete, see
`project/reports/AICAD-003.md`; the workspace builds and passes fmt/clippy
cleanly, which this CI now enforces automatically.

## Decisions made and why

1. **GitHub Actions**, since the repository remote is
   `https://github.com/insightlabs38-pixel/AICAD` — no other CI system is
   referenced anywhere in the plan or existing repo files.
2. **Four separate jobs** (`fmt`, `clippy`, `build-and-test`,
   `native-build-smoke`) rather than one combined job, so a failure is
   immediately attributable and jobs can run in parallel.
3. **No explicit toolchain-install action.** `rust-toolchain.toml`
   (AICAD-003) already pins `channel = "1.98.1"`; GitHub's `ubuntu-latest`
   runners ship `rustup`, which auto-installs the pinned toolchain on first
   `cargo`/`rustup` invocation — the same behavior observed locally in
   `project/reports/AICAD-003.md`'s verification log. This avoids a second,
   independent place where the compiler version could drift from the
   pinned one.
4. **`native-build-smoke` is a real check, not a placeholder that always
   passes silently.** `native/occt_bridge` has no `CMakeLists.txt` yet
   (Stage 1 / AICAD-015 territory, not Stage 0) — per
   `project/CURRENT_STAGE.md` "Not allowed yet" and the AGENTS.md
   escalation trigger "work would expand into a later roadmap stage," the
   job must not itself build an OCCT bridge now. Instead it searches
   `native/` for any `CMakeLists.txt`, explicitly reports "no-op — expected
   before Stage 1" when none exists (current state), and will automatically
   start configuring/building the moment a `CMakeLists.txt` lands — no CI
   file change required at that point. This avoids both silently skipping
   the check forever and building something out of scope now.
5. **`concurrency: cancel-in-progress`** on the workflow+ref, a standard
   cost/latency optimization; not plan-mandated but low-risk and did not
   require its own decision record.

No escalation condition was triggered: CI/build tooling is explicitly
"autonomously allowed" per `AGENTS.md`, and the native smoke job is scoped
to detect/report rather than implement Stage-1 work.

## Files changed
- Added: `.github/workflows/ci.yml`.

## Verification (exact commands/results)
```
$ cmake --version
cmake version 3.28.3

$ find native -mindepth 1 -iname 'CMakeLists.txt'
(no output — confirms the native-build-smoke "no-op" branch is exercised
by the current repository state)

$ python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml')); print('YAML OK')"
YAML OK

$ cargo fmt --all -- --check
(exit code 0, no output)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
(exit code 0, zero warnings)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s

$ cargo test --workspace
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

The four CI job command lines above were run locally exactly as they
appear in `.github/workflows/ci.yml` (`native-build-smoke`'s find/branch
logic was exercised directly, matching the job's script). The workflow
itself will additionally run on the next push/PR to the repository, since
GitHub Actions execution requires a real push event on the remote.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**, zero warnings.
- Task-specific check: native-build-smoke logic verified locally to
  correctly no-op given the current repository state, and to be
  activation-ready for Stage 1.

## Limitations / follow-up
- The workflow has not yet executed on GitHub Actions itself (that
  requires a push/PR against the remote); its steps were verified locally
  against the same commands.
- No caching (e.g. `Swatinem/rust-cache`) was added — out of scope for a
  baseline CI and not required by the task; can be added later as a
  non-semantic optimization without an owner decision.
- `native-build-smoke` will need real inputs (install Open CASCADE
  dependencies on the runner, etc.) once AICAD-015 adds
  `native/occt_bridge/CMakeLists.txt` — tracked implicitly by that task,
  not duplicated here.
