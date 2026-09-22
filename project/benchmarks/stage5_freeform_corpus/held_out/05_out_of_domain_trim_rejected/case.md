# 05 (held out) — evaluation outside a trim loop

**Category:** numerical/domain adversarial (feeds `AICAD-128`'s "explicit
ambiguity/no-solution" failure class).

**Construction:** identical to `public/04_trimmed_freeform_patch`, but
`evaluate_surface(trimmed, 0.95, 0.95)` — distance ~0.636 from the trim
center `(0.5, 0.5)`, well outside the `r=0.3` loop.

**Expected/measured outcome:** the whole build fails —
`RuntimeError::SurfaceEvaluationFailed` wrapping
`cad_geometry_api::QueryFailure::OutOfDomain`
(`crates/cad-runtime/src/interp.rs`'s `EvaluateSurface` dispatch arm).
Confirmed by `crates/cad-cli/tests/stage5_freeform_corpus.rs::
evaluating_outside_a_trim_loop_fails_the_build_explicitly_never_silently`.
Never a silently-returned out-of-region point.
