# Troubleshooting

## Start with structured diagnostics

AICAD compiler/runtime failures use structured diagnostic/report paths. Re-run the CLI with `--json` when machine-readable detail is useful.

## Face/edge integer selection changed after a geometry edit

Stage-3 operations such as some fillet/chamfer/extrude/revolve/shell paths may still use raw kernel-enumeration-order integer selectors. These are intentionally not durable semantic references. Stage-4 semantic-reference work is planned to address robust identity; it is not implemented during this transition.

## A foundation-plan example does not compile

`docs/plan/` contains the frozen long-term implementation plan and examples that span later stages. Use `docs/user/` and the current runtime builtin catalogue for the implemented surface.

## Need implementation/history detail

Use `docs/developer/` for current architecture. Use `project/reports/archive/`, `project/gates/archive/`, and `project/planning/` only when development-history evidence or roadmap context is relevant.
