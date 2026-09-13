#!/usr/bin/env python3
"""Run coarse, controlled timing baselines for implemented AICAD capabilities.

No pass/fail performance threshold is defined here. Stage 4 can append real
resolver benchmarks after AICAD-080+ provides resolver behavior.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COMMANDS = [
    {
        "name": "feature_graph_tests",
        "argv": ["cargo", "test", "-p", "cad-feature-graph", "--locked"],
        "capability": "graph traversal/dependency state",
    },
    {
        "name": "stage3_incremental_rebuild",
        "argv": [
            "cargo", "test", "-p", "cad-cli", "--test",
            "stage3_parametric_incremental_rebuild", "--locked", "--", "--test-threads=1",
        ],
        "capability": "implemented incremental rebuild",
    },
]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    results = []
    for entry in COMMANDS:
        start = time.perf_counter()
        completed = subprocess.run(entry["argv"], cwd=ROOT, check=False)
        elapsed = time.perf_counter() - start
        results.append({
            "name": entry["name"],
            "capability": entry["capability"],
            "seconds": round(elapsed, 6),
            "exit_code": completed.returncode,
        })
        if completed.returncode != 0:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps({"schema_version": 1, "results": results}, indent=2) + "\n")
            return completed.returncode
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({
        "schema_version": 1,
        "threshold_policy": "measurement-only; no noisy CI threshold",
        "stage4_extension_points": ["reference resolution", "adversarial candidate sets"],
        "results": results,
    }, indent=2) + "\n", encoding="utf-8")
    print(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
