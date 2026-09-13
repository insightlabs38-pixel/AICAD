# Stage-4 semantic-reference test infrastructure

`AICAD-079C` establishes test plumbing only. It does **not** implement a
semantic-reference type, query language, resolver, lineage algorithm, or
fingerprint fallback.

The frozen AICAD-079A corpus remains under
`project/benchmarks/stage4_semantic_reference/`. The CI harness at
`scripts/ci/semantic_ref_harness.py` validates that corpus and defines the
result contract future AICAD-080+ code can feed into the benchmark.

The authoritative outcome classes are:

- `RESOLVED_CORRECT` — exactly one intended entity was resolved;
- `AMBIGUOUS` — multiple candidates are reported with evidence;
- `BROKEN` — no valid intended target remains and a reason/evidence is reported;
- `SILENT_WRONG` — a wrong entity was selected without the required ambiguity/breakage; **catastrophic**;
- `KERNEL_FAILURE` — geometry itself failed independently of resolution;
- `UNRELATED_FAILURE` — harness/build infrastructure failed independently of resolution.

The first three preserve D7's fail-closed policy. Fingerprints may be evidence,
ranking input, or benchmark data, but they are not authoritative automatic
recovery in Stage 4 absent a later owner decision.

## Feeding future resolver results

A future resolver adapter may write JSON of this shape and run:

```sh
python scripts/ci/semantic_ref_harness.py grade path/to/results.json
```

```json
{
  "schema_version": 1,
  "cases": [
    {
      "id": "01_topology_split_merge",
      "outcome": "AMBIGUOUS",
      "evidence": ["candidate face A", "candidate face B"]
    }
  ]
}
```

The grader requires one result for every frozen case, rejects unknown outcome
classes, requires evidence for `AMBIGUOUS`/`BROKEN`, returns nonzero for any
ground-truth mismatch, and gives `SILENT_WRONG` a distinct catastrophic exit
path. Until a real resolver exists, CI runs only corpus validation, exact
fixture buildability, and the grader's self-test.

## Permanent silent-misselection regressions

Every silent wrong selection discovered during Stage 4 must be minimized and
committed under `tests/semantic_refs/regressions/` using the record contract in
that directory's README. Do not weaken or relabel a reproducer to make an
implementation pass.
