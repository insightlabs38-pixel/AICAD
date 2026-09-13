# AICAD developer documentation

This directory describes the **current implemented system**, not the chronology of how it was built.

- [`architecture/`](architecture/) — repository/layer architecture and source-of-truth boundaries.
- [`compiler-runtime/`](compiler-runtime/) — frontend → HIR/type system → runtime flow.
- [`geometry/`](geometry/) — source modeling API, Geometry IR/runtime, and geometry boundaries.
- [`kernel/`](kernel/) — kernel-neutral API and OCCT adapter boundary.
- [`parametrics/`](parametrics/) — parameters, feature DAG, cache keys, dirty propagation, provenance.
- [`constraints/`](constraints/) — current solver-independent sketch constraint architecture.
- [`testing/`](testing/) — workspace/test/gate expectations.
- [`contributing/`](contributing/) — repository workflow and where development evidence belongs.

Historical rationale may be linked from `project/reports/archive/`, `project/gates/archive/`, accepted RFCs, and the decision log when needed, but those archives are not routine developer reading.
