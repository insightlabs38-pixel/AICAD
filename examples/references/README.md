# Persistent-reference examples

- `hole_wall_reference.aicad` — expected `Resolved`; CI also changes `hole_diameter`, rebuilds, and verifies that the same source reference resolves against regenerated geometry.
- `ambiguous_reference.aicad` — expected `Ambiguous`; all six planar box faces satisfy a `unique()` query, so AICAD refuses to choose.
- `broken_reference.aicad` — expected `Broken`; a plain box has no cylindrical face.

Run any case with:

```sh
cargo run -p cad-cli -- refs check examples/references/<file>.aicad
```

These examples use only current source syntax. See `docs/user/modeling/persistent-references.md`.
