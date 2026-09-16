#!/usr/bin/env python3
"""Audit/fix the narrow AICAD-079C -> Stage-4 queue transition metadata.

This intentionally does not redesign AICAD-080..100. It enforces only the
transition dependency, two stale D7/AICAD-079A metadata corrections found
by the AICAD-079C audit, and the AICAD-099A remediation task inserted
between AICAD-099 and AICAD-100 (mirroring AICAD-064A's own precedent
between AICAD-064 and AICAD-065).
"""
from __future__ import annotations

import argparse
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TASKS = ROOT / "project/TASKS.yaml"

TASK_079C = """- id: AICAD-079C
  stage: transition
  title: Stage-4 CI/CD infrastructure expansion and transition finalization
  status: done
  depends_on:
  - AICAD-079B
  plan_references:
  - project/gates/stage-3-gate.md
  - project/planning/transitions/stage3-to-stage4/README.md
  - project/benchmarks/stage4_semantic_reference/README.md
  - docs/developer/testing/README.md
  acceptance:
  - Stage-4 layered CI covers fast required checks, exact-geometry integration, AICAD-owned determinism, semantic-reference
    corpus/harness plumbing, and appropriately scheduled/manual hardening without implementing semantic-reference behavior.
  - The AICAD-079A frozen corpus is consumable by the resolver-independent Stage-4 harness, with explicit
    RESOLVED_CORRECT/AMBIGUOUS/BROKEN/SILENT_WRONG/KERNEL_FAILURE/UNRELATED_FAILURE outcome classes and a permanent
    minimized-regression policy for every discovered SILENT_WRONG result.
  - Platform, performance, dependency/supply-chain, release-build, failure-artifact, and branch-protection foundations are
    documented and bounded to currently supported/evidenced capability.
  - Stage-3 -> Stage-4 governance records the transition complete pending owner review/merge and Stage 4 ready but not yet
    implemented; AICAD-080 remains todo and no Stage-5/AICAD-101+ work is promoted.
  required_checks:
  - cargo fmt --all -- --check
  - cargo clippy --workspace --all-targets --all-features -- -D warnings
  - cargo test --workspace
  - native OCCT bridge CMake build plus CTest
  - python scripts/ci/semantic_ref_harness.py validate
  - python scripts/ci/semantic_ref_harness.py self-test
  - python scripts/ci/stage4_task_audit.py --check
  report: project/reports/AICAD-079C.md
  escalate_if:
  - a stage gate/benchmark/test would need weakening
  - kernel-specific types would leak above the adapter
  - semantic-reference ambiguity would require arbitrary fallback
  - any production Stage-4 semantic-reference implementation would be required to complete this infrastructure task
  - work would expand into Stage 5 or AICAD-101+
"""


def task_section(text: str, task_id: str) -> tuple[int, int, str]:
    marker = f"- id: {task_id}\n"
    start = text.find(marker)
    if start < 0:
        raise AssertionError(f"missing {task_id}")
    next_start = text.find("\n- id: ", start + len(marker))
    end = len(text) if next_start < 0 else next_start + 1
    return start, end, text[start:end]


def replace_section(text: str, task_id: str, section: str) -> str:
    start, end, _ = task_section(text, task_id)
    return text[:start] + section.rstrip() + "\n" + text[end:]


def fix(text: str) -> str:
    if "- id: AICAD-079C\n" not in text:
        start, _, _ = task_section(text, "AICAD-080")
        text = text[:start] + TASK_079C + text[start:]

    _, _, s080 = task_section(text, "AICAD-080")
    s080 = s080.replace("  - AICAD-079B\n", "  - AICAD-079C\n", 1)
    if "  - project/benchmarks/stage4_semantic_reference/README.md\n" not in s080:
        anchor = "  - docs/plan/22_REPOSITORY_WORK_PACKAGES.md\n"
        s080 = s080.replace(
            anchor,
            anchor
            + "  - project/benchmarks/stage4_semantic_reference/README.md\n"
            + "  - tests/semantic_refs/README.md\n",
            1,
        )
    text = replace_section(text, "AICAD-080", s080)

    _, _, s092 = task_section(text, "AICAD-092")
    s092 = re.sub(
        r"  title: .*\n",
        "  title: Implement geometry-fingerprint evidence/ranking/benchmark support without automatic recovery\n",
        s092,
        count=1,
    )
    s092 = s092.replace(
        "  - Fallback is visibly classified as weak/query_geometric and never masquerades as explicit/lineage/strong resolution.\n",
        "  - Fingerprint information is available only as diagnostic evidence, candidate-ranking input, and benchmark/experiment data;\n"
        "    it never converts Ambiguous/Broken into Resolved automatically in the first Stage-4 implementation.\n"
        "  - Any proposal to enable automatic fingerprint recovery requires a later explicit owner decision supported by Stage-4\n"
        "    benchmark evidence; D7/DL-8 remains authoritative.\n",
        1,
    )
    if "  - project/DECISION_LOG.md#DL-8\n" not in s092:
        anchor = "  - docs/plan/22_REPOSITORY_WORK_PACKAGES.md\n"
        s092 = s092.replace(
            anchor,
            anchor
            + "  - project/DECISION_LOG.md#DL-8\n"
            + "  - tests/semantic_refs/README.md\n",
            1,
        )
    text = replace_section(text, "AICAD-092", s092)

    _, _, s096 = task_section(text, "AICAD-096")
    s096 = re.sub(
        r"  title: .*\n",
        "  title: Extend the frozen AICAD-079A topology-reference corpus for resolver execution\n",
        s096,
        count=1,
    )
    s096 = s096.replace(
        "  - Corpus includes extrusion resize, add/remove hole, pattern count, fillet radius, split, merge, branch reorder, and\n"
        "    suppression perturbations.\n",
        "  - The frozen AICAD-079A public/held-out corpus remains the baseline; Stage-4 resolver execution extends coverage without\n"
        "    rewriting held-out ground truth or recreating the baseline corpus.\n"
        "  - Coverage includes extrusion resize, add/remove hole, pattern count, fillet radius, split, merge, branch reorder, and\n"
        "    suppression perturbations, reusing the AICAD-079A harness/outcome taxonomy where applicable.\n",
        1,
    )
    if "  - project/benchmarks/stage4_semantic_reference/README.md\n" not in s096:
        anchor = "  - docs/plan/22_REPOSITORY_WORK_PACKAGES.md\n"
        s096 = s096.replace(
            anchor,
            anchor
            + "  - project/benchmarks/stage4_semantic_reference/README.md\n"
            + "  - tests/semantic_refs/README.md\n",
            1,
        )
    text = replace_section(text, "AICAD-096", s096)
    return text


def check(text: str) -> None:
    for task_id in (
        "AICAD-079C",
        *[f"AICAD-{n:03d}" for n in range(80, 101)],
        "AICAD-099A",
    ):
        count = text.count(f"- id: {task_id}\n")
        if count != 1:
            raise AssertionError(f"{task_id}: expected exactly one task entry, found {count}")

    _, _, s079c = task_section(text, "AICAD-079C")
    assert "  stage: transition\n" in s079c
    assert "  status: done\n" in s079c
    assert "  - AICAD-079B\n" in s079c

    _, _, s080 = task_section(text, "AICAD-080")
    assert "  - AICAD-079C\n" in s080
    assert "  - AICAD-079B\n" not in s080
    assert "project/benchmarks/stage4_semantic_reference/README.md" in s080

    _, _, s092 = task_section(text, "AICAD-092")
    assert "without automatic recovery" in s092
    assert "never converts Ambiguous/Broken into Resolved automatically" in s092
    assert "project/DECISION_LOG.md#DL-8" in s092

    _, _, s096 = task_section(text, "AICAD-096")
    assert "Extend the frozen AICAD-079A" in s096
    assert "without\n    rewriting held-out ground truth" in s096

    _, _, s099a = task_section(text, "AICAD-099A")
    assert "  - AICAD-099\n" in s099a
    assert "Scoped candidate-universe resolution" in s099a

    _, _, s100 = task_section(text, "AICAD-100")
    assert "  - AICAD-099A\n" in s100
    assert "Stage-4 hard-gate packet for owner review" in s100


def main() -> int:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", action="store_true")
    group.add_argument("--fix", action="store_true")
    args = parser.parse_args()

    original = TASKS.read_text(encoding="utf-8")
    if args.fix:
        updated = fix(original)
        check(updated)
        if updated != original:
            TASKS.write_text(updated, encoding="utf-8")
            print("updated project/TASKS.yaml")
        else:
            print("project/TASKS.yaml already up to date")
    else:
        check(original)
        print("Stage-4 task metadata audit OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
