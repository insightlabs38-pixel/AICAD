# Persistent semantic references

Raw face/edge enumeration is not durable identity: a topology-changing edit can renumber, split, merge, create, or delete entities. AICAD therefore owns persistent semantic reference recipes above the geometry kernel.

## Source syntax

Current `.aicad` syntax is:

```text
query <name> : <EntityKind> in <scope> {
    <clause>();
    ...
}
```

`EntityKind` is one of `Vertex`, `Edge`, `Wire`, `Face`, `Shell`, or `Solid`. Scope is mandatory for source-declared references.

A real current example:

```aicad
part Wall {
    let base: Geometry = box(60mm, 30mm, 10mm);
    let bored_a: Geometry = hole(
        base,
        Axis3(
            origin = Point3(x = 15mm, y = 15mm, z = 0mm - 1mm),
            direction = Vector3(x = 0.0, y = 0.0, z = 1.0),
        ),
        6mm,
        12mm,
    );

    query hole_wall : Face in Wall.bored_a {
        generated_by(bored_a);
        cylindrical();
        unique();
    }
}
```

The scope limits the candidate universe to `Wall.bored_a`; it is not a geometric hint. If required scope cannot be resolved, resolution fails closed instead of silently widening to the whole session.

## Current clause vocabulary

Current source lowering supports:

- lineage/topology: `generated_by(name)`, `modified_by(name)`, `descended_from(name)`, `convex()`, `concave()`, `manifold()`, `nonmanifold()`;
- surface family: `planar()`, `cylindrical()`, `conical()`, `spherical()`, `toroidal()`, `bspline()`;
- comparisons: `radius(cmp, magnitude)`, `length(cmp, magnitude)` where `cmp` is `eq`, `lt`, `lte`, `gt`, or `gte`;
- directions: `normal(x, y, z[, tolerance])`, `axis(x, y, z[, tolerance])`;
- ranking: `first()`, `largest(area|radius)`, `smallest(area|radius)`;
- cardinality: `unique()`, `expect_count(n)`.

Unknown or malformed clauses produce structured diagnostics. Do not use plan-only syntax for predicates that have not yet been promoted into source lowering.

## Fail-closed outcomes

Resolution has exactly three semantic outcomes:

- **Resolved** — the reference contract yields the required candidate count;
- **Ambiguous** — multiple candidates remain where the contract requires fewer;
- **Broken** — no valid candidate/scope satisfies the reference contract.

AICAD does not pick an arbitrary "best" candidate from a genuine ambiguity.

Cardinality is part of the recipe. `unique()` requires exactly one candidate. `expect_count(n)` makes the expected count explicit.

## Regeneration and lineage

A persistent reference is replayed against the current `ParametricBuildSession` state after a parameter edit/rebuild. Feature/provenance identity is AICAD-owned; kernel `Generated`/`Modified`/deletion lineage is resolver evidence, not semantic identity by itself.

Raw topology handles are epoch/context-bound and become stale across regeneration even when some underlying geometry is reused. They are not persistent references.

## Reference health

Run:

```sh
cargo run -p cad-cli -- refs check model.aicad
```

The command checks the model's actual source-declared references and summarizes resolved, ambiguous, and broken outcomes. Use `--json` for machine-readable output.

The maintained examples include:

- `examples/references/hole_wall_reference.aicad` — resolves and is replayed after a parameter edit by CI;
- `examples/references/ambiguous_reference.aicad` — deliberately ambiguous;
- `examples/references/broken_reference.aicad` — deliberately broken.

## Fingerprints are not automatic repair

AICAD can compute geometric-fingerprint evidence for diagnostics/ranking/experiments, but automatic fingerprint recovery is disabled. A reference is not silently "repaired" from geometric similarity.

## Current source-language gaps

Some predicates already exist at the Rust/query-evaluator level but cannot yet be expressed by the current minimal source clause grammar because they require nested references, richer spatial operands, or missing dimensional syntax. Area source construction and the remaining query vocabulary are planned Stage-5-prelude work. Do not treat those planned forms as current syntax.
