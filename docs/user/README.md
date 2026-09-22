# AICAD User Guide

AICAD is source-first: `.aicad` source and AICAD-owned semantic state are authoritative, while exact B-rep/STEP outputs are derived artifacts.

AICAD is currently pre-1.0. Stage 5 is complete; assemblies/configurations are planned Stage-6 capabilities and are not implemented yet.

## Start here

- [Getting started](getting-started/) — prerequisites, build, first model, and output inspection.
- [Modeling](modeling/) — typed parametric modeling, supported geometry, advanced geometry/topology, and persistent semantic references.
- [CLI](cli/) — current command surface and structured output.
- [Current limitations](current-limitations.md) — unsupported, deferred, numerical, usability, and next-stage boundaries.
- [Maintained examples](../../examples/) — executable current syntax grouped by use case.

## Current capability baseline

The supported product surface includes typed engineering values, source-defined parametrics and incremental dependencies, exact OCCT-backed B-rep operations behind kernel-neutral APIs, persistent fail-closed semantic references, advanced curves/surfaces, trimmed geometry, geometric queries, topology construction/healing/inspection, controlled raw geometry, raw-to-safe adoption, lineage/provenance, and maintained examples.

Persistent semantic references are deliberately separate from topology-local integer selectors. A source reference resolves to `Resolved`, `Ambiguous`, or `Broken`; ambiguity is not silently guessed through.

For implementation architecture and contribution/testing internals, use the [developer guide](../developer/). Internal planning, gates, and historical evidence live under [`project/`](../../project/) and are not user documentation.
