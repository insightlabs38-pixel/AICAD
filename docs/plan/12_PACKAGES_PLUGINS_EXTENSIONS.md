# 12 — Packages, Plugins, Extension APIs, and Custom Skills

## 1. Philosophy

The compiler core should stay small. Specialized engineering knowledge belongs in versioned packages whenever it can be expressed using existing language/geometry capabilities.

Examples:

```text
std.fasteners
std.bearings
std.threads
std.gears
manufacturing.sheetmetal
manufacturing.injection_molding
robotics.cycloidal
robotics.harmonic_drive
electronics.pcb
aerospace.internal_rules
```

## 2. Package structure

```text
package/
  package.toml
  src/
  schema/
    api.json
  docs/
  examples/
  tests/
  ai/
    skill.md
  assets/
  plugins/       # optional binaries/WASM manifests
```

## 3. Package manifest fields

```text
name
version
language_version
license
authors/organization
repository
exports
dependencies
optional_dependencies
features
capabilities
plugin_kind
ai_skill
api_schema
assets
checksums
```

## 4. Package manager CLI

```text
cad add <package>
cad remove <package>
cad update [package]
cad lock
cad vendor
cad package search <query>
cad package inspect <package>
cad package test
cad publish
```

## 5. Lockfile

`cad.lock` records:

- exact package versions;
- content hashes;
- registry/source;
- feature flags;
- plugin binary/WASM hashes;
- solver/kernel adapter versions where needed.

The lockfile participates in reproducible build hashes.

## 6. Extension levels

### Level 1 — Pure source package

Uses only language + safe CAD APIs. Preferred default.

### Level 2 — WASM plugin

For custom algorithms that need speed or another compiled language but can operate through a sandboxed ABI.

Potential capabilities:

- feature generation;
- numerical algorithms;
- validators;
- import/export transformation;
- metadata processing.

### Level 3 — External process plugin

For heavy solvers/tools with structured request/response contracts.

### Level 4 — Trusted native plugin

Reserved for:

- geometry kernels;
- kernel-level importers/exporters;
- high-performance solvers requiring native integration.

Must be explicitly trusted because it can escape ordinary sandbox guarantees.

## 7. Capability manifest

```yaml
capabilities:
  geometry_safe: true
  geometry_unsafe: false
  filesystem_read:
    - project_assets
  filesystem_write: false
  network: false
  external_process: false
  native: false
```

The IDE/CI/agent runtime shows requested capabilities before installation/activation.

## 8. Plugin ABI principles

- versioned;
- schema-first;
- no raw kernel pointers across sandbox boundaries;
- content-addressed inputs/outputs where possible;
- cancellation support;
- resource accounting;
- structured errors;
- deterministic mode declaration;
- explicit nondeterminism/capabilities.

## 9. Package-defined semantic types

A package can define high-level types:

```aicad
struct CycloidalDriveSpec { ... }
fn cycloidal_drive(spec: CycloidalDriveSpec) -> Assembly { ... }
```

It can use low-level geometry internally without exposing it to users.

## 10. Package-defined validators

Example:

```aicad
@validator
fn validate_cycloidal_drive(drive: Assembly) -> ValidationReport { ... }
```

Package validators can hook into build profiles if explicitly enabled.

## 11. Package-defined UI

Packages may supply schema-based inspector metadata:

- parameter groups;
- labels/descriptions;
- enum choices;
- preview helpers;
- custom visual overlays.

Avoid arbitrary trusted frontend JavaScript for normal packages; use declarative UI schemas where possible.

## 12. Package-defined AI skill

Required for novel abstractions if the package expects AI use.

`ai/skill.md` should be concise and focus on:

- concept model;
- primary API;
- decision rules;
- common mistakes;
- validation requirements;
- how to discover full docs.

## 13. API schema

Machine-readable package schema should include:

```text
symbols
kind
signature
parameter names/types/defaults/docs
return type
errors
capability requirements
stability/deprecation
examples refs
semantic outputs
```

This makes agents and IDE completion precise without loading prose.

## 14. Package testing

Required before publishing:

- parser/type tests;
- deterministic build tests;
- example builds;
- geometry validity;
- semantic-reference health;
- skill learnability smoke test for AI-focused packages;
- API schema/doc consistency.

## 15. Registry governance

Long-term registry should support:

- signed packages;
- vulnerability/security advisories;
- yanked versions;
- verified organizations;
- package quality badges;
- deterministic build attestation;
- compatibility matrices.

## 16. Standard library policy

Keep compiler intrinsics minimal. Promote widely used stable packages into `std.*` only when:

- domain semantics are broadly applicable;
- API has proven stable;
- interoperability mapping is understood;
- quality/tests meet higher bar.

## 17. New feature: package capability simulation

Before installing an unfamiliar package, users/agents can run:

```text
cad package explain-capabilities pkg
```

and receive:

```text
Uses safe geometry only.
Reads embedded assets.
No network.
No native code.
Deterministic under default profile.
Ships 12 examples and 48 tests.
```

This supports safe AI-driven package discovery.
