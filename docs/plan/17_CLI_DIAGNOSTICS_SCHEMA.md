# 17 — CLI, Structured Diagnostics, and Tooling Contracts

## 1. CLI philosophy

Every important IDE/AI action should have a deterministic non-GUI equivalent. The CLI becomes the stable automation boundary for CI and a reference implementation for agent tools.

## 2. Project commands

```text
cad new <name>
cad init
cad build [path]
cad clean
cad fmt
cad check
cad run <module/function>
```

### `cad build`

Options:

```text
--configuration <name/key-values>
--profile fast|standard|release|audit
--target step|brep|gltf|...
--output <path>
--json
--budget <profile>
```

## 3. Verification commands

```text
cad test [filter]
cad requirements
cad validate
cad refs check
cad unsafe list
```

Useful options:

```text
--configuration
--all-configurations
--profile
--changed-since <commit>
--json
```

## 4. Inspection commands

```text
cad inspect <symbol/ref>
cad query <expression>
cad explain <diagnostic/ref/parameter>
cad why <symbol>
cad history <ref>
cad context <symbol> --for-agent
```

## 5. Documentation commands

```text
cad docs <symbol>
cad search <terms>
cad schema <symbol>
cad skill <package/core>
```

## 6. Import/export commands

```text
cad import <file>
cad export <format> [target]
cad reconstruct <file>
cad fidelity <artifact>
```

## 7. Package commands

```text
cad add
cad remove
cad update
cad lock
cad vendor
cad package search
cad package inspect
cad package test
cad publish
```

## 8. Debug commands

```text
cad debug <target>
cad repl
cad trace <feature>
cad profile <target>
```

## 9. Collaboration commands

```text
cad diff [refs]
cad merge-check
cad provenance
cad attest
```

## 10. Diagnostic code taxonomy

Recommended families:

```text
PARSE-E###
TYPE-E###
UNIT-E###
RUNTIME-E###
BUDGET-E###
GEOM-E###
TOPO-E###
REF-E###
CONSTRAINT-E###
ASM-E###
TEST-E###
REQ-E###
IMPORT-E###
EXPORT-E###
DFM-E/W###
SIM-E/W###
PKG-E###
SEC-E###
```

Warnings use `W`, errors `E`, information `I` if needed.

## 11. Diagnostic schema

```json
{
  "code": "REF-E102",
  "severity": "error",
  "category": "reference",
  "title": "AMBIGUOUS_REFERENCE",
  "message": "Reference resolved to 2 faces; expected 1.",
  "source": {
    "file": "src/housing.cadl",
    "start": {"line": 42, "column": 8},
    "end": {"line": 46, "column": 2}
  },
  "entity": "housing.mounting_face",
  "expected": {"count": 1},
  "observed": {"count": 2},
  "candidates": [],
  "suggestions": [],
  "backend_details": null
}
```

## 12. Suggestion contract

A suggestion must distinguish:

```text
machine_applicable   safe structured patch
likely_fix           supported by deterministic analysis but needs review
informational        conceptual options only
```

AI agents must not treat informational suggestions as guaranteed solutions.

## 13. Build output schema

```json
{
  "status": "failed",
  "build_id": "...",
  "configuration": {},
  "changed_features": [],
  "artifacts": [],
  "diagnostics": [],
  "reference_health": {},
  "verification": {},
  "resource_usage": {}
}
```

## 14. Query output schema

Entity result should expose stable semantic information first:

```json
{
  "semantic_ref": "housing.top_face",
  "entity_type": "Face",
  "geometry": {
    "surface": "Plane",
    "area": "641.2mm^2",
    "normal": [0,0,1]
  },
  "lineage": {
    "created_by": "base_extrude",
    "modified_by": ["draft"]
  },
  "durability": "explicit"
}
```

Raw backend IDs can appear only in debug sections.

## 15. Exit codes

Define stable exit semantics for CI:

```text
0 success
1 build/type/geometry failure
2 test/requirement failure
3 invalid CLI/project configuration
4 resource/security policy violation
5 internal compiler/kernel failure
```

Exact numbering can change before 1.0 but must be documented.

## 16. New feature: `cad doctor`

Checks environment:

- compiler/kernel versions;
- package lock integrity;
- plugin availability;
- solver availability;
- cache health;
- external exporter support;
- project compatibility.

Useful for humans, CI, and AI agents before debugging phantom source errors.
