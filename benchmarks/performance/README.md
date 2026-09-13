# Performance regression foundation

Performance evidence is intentionally measurement-first. No noisy
microbenchmark threshold blocks routine pull requests.

AICAD-079C adds `scripts/ci/performance_baseline.py` and a scheduled/manual
workflow that records coarse wall-clock measurements for **implemented**
capabilities only:

- feature/dependency-graph tests;
- Stage-3 in-process parametric incremental rebuild integration.

The JSON artifact is retained for controlled comparisons. Stage 4 has two
explicit extension points once real resolver behavior exists: reference
resolution and adversarial candidate-set scaling. AICAD-079C does not fake
those measurements before AICAD-080+ provides the code under test.

Future performance gates must be based on measured baselines and controlled
comparison methodology rather than aspirational thresholds.
