# examples/brackets

Example library category: bracket parts, including the Stage-1 "complex
bracket" fixture referenced by the Stage-1 exit gate.

Plan references: `docs/plan/15_IMPLEMENTATION_ROADMAP.md` Stage 1;
`docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §1.

- `stage2_mounting_plate.aicad` — the Stage-2 end-to-end proof fixture
  (`AICAD-063`).
- `stage3_l_bracket.aicad` — a Stage-3 ordinary-part example (`AICAD-079`):
  an L-shaped mounting bracket built from `box`/`union`/`fillet`/`hole`/
  `mirror`, exposing two independently named `Geometry` outputs
  (`LBracket.body`, `LBracket.mirrored`) — see `crates/cad-cli/tests/
  stage3_ordinary_parts.rs` for its build/validity proof.
