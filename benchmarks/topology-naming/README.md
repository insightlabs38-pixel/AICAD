# benchmarks/topology-naming

**Stage-4 hard-gate benchmark.** Baseline parts perturbed (resize extrusion,
add/remove hole, change pattern count, change fillet radius, split a face,
merge coplanar faces, reorder independent feature branches, suppress
configuration feature), rebuilt, and checked for correct reference
resolution. Tracks: correct resolution / ambiguous-but-detected /
broken-but-detected / **silent-wrong-resolution (must approach zero)** /
reference durability distribution.

Populated by tasks AICAD-096 (fixture corpus), AICAD-097 (perturbation
runner), AICAD-098 (metrics), AICAD-099 (adversarial bug-hunt — every
silent-wrong reproducer found must be preserved here, not discarded).

Plan references: `docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §5.
