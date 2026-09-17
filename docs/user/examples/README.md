# Examples

The user-facing `examples/` tree is a maintained product surface. Its ACTIVE examples use current supported syntax and are registered with the automated active-example integration test.

See the repository-level [`examples/README.md`](../../../examples/README.md) for the complete ACTIVE inventory and expected reference-health outcomes.

Recommended learning path:

1. `examples/getting_started/simple_box.aicad` — smallest part and named output.
2. `examples/parametric/derived_plate.aicad` — typed parameters and derived values.
3. `examples/brackets/stage2_mounting_plate.aicad` — control flow plus parametric modeling.
4. `examples/brackets/stage3_l_bracket.aicad` — multi-feature part and mirror.
5. `examples/plates/stage3_bearing_mount.aicad` — pocket, transform, radial pattern, realistic mechanical part.
6. `examples/enclosures/stage3_enclosure.aicad` — shell and hole.
7. `examples/references/` — persistent, ambiguous, and broken semantic-reference cases.

Historical filenames such as `stage2_*` and `stage3_*` are retained because tests and evidence reference them; ACTIVE status means the syntax is still current and automatically validated.

Stage-0 paper examples and future freeform/verification placeholders are archived under `project/archive/examples/` rather than presented as current usage.
