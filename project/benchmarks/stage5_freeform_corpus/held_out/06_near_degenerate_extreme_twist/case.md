# 06 (held out) — near-degenerate freeform patch, surface topology construction

**Category:** near-degenerate adversarial (docs/plan/16 §5) +
capability-gap disclosure (surface side).

**Construction:** a `bspline_surface` whose `u=0` edge is only 0.001mm
long (a genuine, constructible, but extremely thin sliver — not a literal
zero-length edge, which `line_curve`/`trim_curve` cannot express at all),
bounded by a real 4-edge wire, passed to `make_face_on_surface`.

**Expected/measured outcome:** the build fails before ever reaching
OCCT — `make_face_on_surface` rejects *any* Bezier/B-spline surface today
(`UNSUPPORTED_TOPOLOGY_CONSTRUCTION`, `crates/cad-runtime/src/
interp.rs`'s `surface_to_surface_spec`), so this fixture's own
near-degeneracy never actually reaches the kernel. This is still the
required adversarial evidence — "explicit structured rejection, never a
panic/hang/silent-wrong success" — just via a different, now-understood
mechanism than originally intended (see this corpus's `README.md`
"Discovered capability gap"). Confirmed by
`crates/cad-cli/tests/stage5_freeform_corpus.rs::
freeform_surface_topology_construction_is_explicitly_rejected_never_silent`.

**Follow-up:** once `make_face_on_surface` gains Bezier/B-spline surface
support (tracked in this corpus's own README and
`project/OWNER_DECISIONS.md`'s non-decision items), this fixture should
be revisited to exercise the originally-intended near-degenerate-geometry
question (does OCCT itself reject or numerically mishandle the sliver),
not merely the interface-level rejection.
