#!/usr/bin/env python3
"""Stage-4 semantic-reference benchmark plumbing.

AICAD-079C intentionally does not import or implement a resolver. This tool
validates the frozen AICAD-079A corpus now and defines the result contract that
AICAD-080+ can feed once real resolver output exists.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CORPUS = ROOT / "project/benchmarks/stage4_semantic_reference"
REGRESSIONS = ROOT / "tests/semantic_refs/regressions"

CANONICAL = {
    "correct_resolved_reference": "RESOLVED_CORRECT",
    "explicit_ambiguity": "AMBIGUOUS",
    "explicit_broken_reference": "BROKEN",
    "silent_wrong_resolution": "SILENT_WRONG",
    "kernel_failure": "KERNEL_FAILURE",
    "unrelated_build_failure": "UNRELATED_FAILURE",
}
OUTCOMES = frozenset(CANONICAL.values())
EXPECTED_RE = re.compile(r"\*\*Expected classification:\*\* `([^`]+)`")

REQUIRED_CATEGORIES = {
    "01_topology_split_merge",
    "02_disappearing_entity",
    "03_symmetric_candidates",
    "04_pattern_count_change",
    "05_boolean_topology_change",
    "06_fillet_viability",
    "07_operation_reordering",
    "08_upstream_suppression",
    "09_changing_region",
    "10_near_degenerate",
}

@dataclass(frozen=True)
class Case:
    case_id: str
    split: str
    expected: str
    directory: Path


def discover_cases() -> list[Case]:
    cases: list[Case] = []
    for split in ("public", "held_out"):
        for directory in sorted((CORPUS / split).glob("[0-9][0-9]_*")):
            if not directory.is_dir():
                continue
            case_file = directory / "case.md"
            baseline = directory / "baseline.aicad"
            perturbed = directory / "perturbed.aicad"
            missing = [p.name for p in (case_file, baseline, perturbed) if not p.is_file()]
            if missing:
                raise AssertionError(f"{directory}: missing {', '.join(missing)}")
            text = case_file.read_text(encoding="utf-8")
            match = EXPECTED_RE.search(text)
            if not match:
                raise AssertionError(f"{case_file}: expected-classification metadata missing")
            legacy = match.group(1)
            if legacy not in CANONICAL:
                raise AssertionError(f"{case_file}: unknown classification {legacy!r}")
            cases.append(Case(directory.name, split, CANONICAL[legacy], directory))
    ids = {case.case_id for case in cases}
    if ids != REQUIRED_CATEGORIES:
        missing = sorted(REQUIRED_CATEGORIES - ids)
        extra = sorted(ids - REQUIRED_CATEGORIES)
        raise AssertionError(f"frozen corpus IDs changed: missing={missing}, extra={extra}")
    if len(cases) != len(ids):
        raise AssertionError("duplicate semantic-reference case id")
    return cases


def verify_held_out_manifest() -> None:
    held_out = CORPUS / "held_out"
    manifest = held_out / "MANIFEST.sha256"
    if not manifest.is_file():
        raise AssertionError("held-out checksum manifest is missing")
    for line_no, line in enumerate(manifest.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        digest, rel = line.split(maxsplit=1)
        rel = rel.lstrip("* ")
        path = held_out / rel
        if not path.is_file():
            raise AssertionError(f"held-out manifest line {line_no}: missing {rel}")
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != digest:
            raise AssertionError(
                f"held-out manifest line {line_no}: {rel} changed ({actual} != {digest})"
            )


def validate_regressions() -> None:
    if not REGRESSIONS.is_dir():
        raise AssertionError("semantic-reference regression directory is missing")
    for path in sorted(REGRESSIONS.glob("*.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        required = {
            "schema_version",
            "id",
            "model",
            "perturbation",
            "intended_target",
            "actual_outcome",
            "failure_classification",
            "evidence",
        }
        missing = sorted(required - data.keys())
        if missing:
            raise AssertionError(f"{path}: missing regression fields {missing}")
        if data["schema_version"] != 1:
            raise AssertionError(f"{path}: unsupported schema_version")
        if data["failure_classification"] != "SILENT_WRONG":
            raise AssertionError(f"{path}: permanent regression must record SILENT_WRONG")
        if not isinstance(data["evidence"], list) or not data["evidence"]:
            raise AssertionError(f"{path}: evidence must be a non-empty list")


def grade_results(path: Path, cases: list[Case]) -> dict[str, object]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if payload.get("schema_version") != 1 or not isinstance(payload.get("cases"), list):
        raise AssertionError("result document must be schema_version=1 with a cases array")
    expected = {case.case_id: case for case in cases}
    actual: dict[str, dict[str, object]] = {}
    for result in payload["cases"]:
        case_id = result.get("id")
        outcome = result.get("outcome")
        if case_id not in expected:
            raise AssertionError(f"result references unknown case {case_id!r}")
        if case_id in actual:
            raise AssertionError(f"duplicate result for {case_id}")
        if outcome not in OUTCOMES:
            raise AssertionError(f"{case_id}: unknown outcome {outcome!r}")
        if outcome in {"AMBIGUOUS", "BROKEN"} and not result.get("evidence"):
            raise AssertionError(f"{case_id}: {outcome} requires evidence")
        actual[case_id] = result
    missing = sorted(expected.keys() - actual.keys())
    if missing:
        raise AssertionError(f"missing results for {missing}")

    counts = {name: 0 for name in sorted(OUTCOMES)}
    mismatches: list[str] = []
    silent_wrong: list[str] = []
    for case_id, result in actual.items():
        outcome = str(result["outcome"])
        counts[outcome] += 1
        if outcome == "SILENT_WRONG":
            silent_wrong.append(case_id)
        if outcome != expected[case_id].expected:
            mismatches.append(case_id)
    return {
        "schema_version": 1,
        "counts": counts,
        "mismatches": sorted(mismatches),
        "silent_wrong": sorted(silent_wrong),
    }


def command_validate() -> int:
    cases = discover_cases()
    verify_held_out_manifest()
    validate_regressions()
    split_counts = {split: sum(c.split == split for c in cases) for split in ("public", "held_out")}
    print(json.dumps({"status": "ok", "cases": len(cases), "splits": split_counts}, sort_keys=True))
    return 0


def command_grade(results: Path) -> int:
    summary = grade_results(results, discover_cases())
    print(json.dumps(summary, sort_keys=True))
    return 2 if summary["silent_wrong"] else (1 if summary["mismatches"] else 0)


def command_self_test() -> int:
    cases = discover_cases()
    good = {
        "schema_version": 1,
        "cases": [
            {"id": case.case_id, "outcome": case.expected, "evidence": ["self-test synthetic evidence"]}
            for case in cases
        ],
    }
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        good_path = tmp_path / "good.json"
        good_path.write_text(json.dumps(good), encoding="utf-8")
        good_summary = grade_results(good_path, cases)
        assert not good_summary["mismatches"] and not good_summary["silent_wrong"]

        bad = json.loads(json.dumps(good))
        bad["cases"][0]["outcome"] = "SILENT_WRONG"
        bad_path = tmp_path / "bad.json"
        bad_path.write_text(json.dumps(bad), encoding="utf-8")
        bad_summary = grade_results(bad_path, cases)
        assert bad_summary["silent_wrong"] == [cases[0].case_id]
    print(json.dumps({"status": "ok", "self_test": "silent-wrong gate exercised"}))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("validate")
    grade = sub.add_parser("grade")
    grade.add_argument("results", type=Path)
    sub.add_parser("self-test")
    args = parser.parse_args()
    if args.command == "validate":
        return command_validate()
    if args.command == "grade":
        return command_grade(args.results)
    return command_self_test()


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, json.JSONDecodeError, OSError, ValueError) as exc:
        print(f"semantic-ref harness error: {exc}", file=sys.stderr)
        raise SystemExit(2)
