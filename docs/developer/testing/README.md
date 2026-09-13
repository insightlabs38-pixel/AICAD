# Testing and validation

Routine Rust verification uses the workspace tooling defined by repository policy, including formatting, Clippy where required by the active task/gate, and workspace/targeted tests.

Stage gates additionally rely on task-specific integration fixtures, benchmarks, deterministic reruns, schema checks, native-bridge tests, and explicit owner-gate evidence. Completed Stage-0..3 reports/gates are archived under `project/reports/archive/` and `project/gates/archive/` for regression archaeology.

Do not weaken or delete an existing test to make a transition/documentation change pass. This Stage-3→4 transition intentionally changes no Rust production behavior.
