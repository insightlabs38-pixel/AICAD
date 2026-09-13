#!/usr/bin/env python3
"""Small, network-free workspace policy checks used by required CI."""
from __future__ import annotations

import json
import sys
from pathlib import Path

EXPECTED_LICENSE = "Apache-2.0"


def main() -> int:
    if len(sys.argv) != 2:
        raise SystemExit("usage: workspace_policy.py <cargo-metadata.json>")
    data = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    workspace = set(data["workspace_members"])
    failures: list[str] = []
    for package in data["packages"]:
        if package["id"] not in workspace:
            continue
        if package.get("license") != EXPECTED_LICENSE:
            failures.append(f"{package['name']}: license={package.get('license')!r}")
        if package.get("source") is not None:
            failures.append(f"{package['name']}: workspace package unexpectedly has source={package['source']}")
        for dep in package.get("dependencies", []):
            source = dep.get("source")
            if isinstance(source, str) and source.startswith("git+"):
                failures.append(f"{package['name']}: unreviewed git dependency {dep['name']} ({source})")
    if failures:
        print("workspace policy failures:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"workspace policy OK: {len(workspace)} workspace packages, license={EXPECTED_LICENSE}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
