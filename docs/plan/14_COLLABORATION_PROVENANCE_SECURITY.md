# 14 — Collaboration, Semantic Version Control, Provenance, Reproducibility, and Security

## 1. Semantic diffs

Plain text already improves review, but CAD needs domain-aware impact reporting.

Example:

```diff
Part Housing

- wall = 2.0mm
+ wall = 2.4mm

+ feature rear_reinforcement_rib

USBMount:
- clearance = 0.40mm
+ clearance = 0.65mm

Impact:
mass              +11.2 g
volume            +4.1 cm^3
bounding box      unchanged
requirements      28/28 pass
reference health  unchanged
```

## 2. Diff levels

```text
text diff
AST diff
parameter diff
feature DAG diff
semantic entity diff
geometry/property diff
requirement/test diff
assembly/BOM diff
drawing/PMI diff
provenance diff
```

Review UI can progressively disclose these levels.

## 3. Semantic merge

If branches edit independent DAG nodes, auto-merge may be safe.

Conflict types:

- same parameter divergent edits;
- delete-vs-modify feature;
- semantic reference changed differently;
- assembly mate conflict;
- package/configuration conflict;
- geometry result conflict despite syntactic independence.

A semantic merge should rebuild and run required validation after combining changes.

## 4. Git integration

Store source as normal text. Provide custom diff/merge drivers:

```text
cad git diff
cad git merge-driver
```

The `.aicad` bundle should not replace source repositories; it is an exchange/archive artifact.

## 5. Provenance

Track at minimum:

```text
created_by: human | AI | package | import | optimizer | reconstruction
actor/tool identifier
model/tool version when AI-generated
source request/task identifier where available
commit/build ID
timestamp (metadata only, excluded from deterministic geometry hash)
verification status
review/approval state
```

Do not require personal identity in source; organizations choose policy.

## 6. Feature-level provenance

Each feature can retain:

```text
origin
creation commit
last semantic modification
AI/human status
rationale
linked requirement
package version
```

## 7. AI governance

Example policy:

```text
AI-created safety-critical features require human approval.
AI-created raw/unsafe geometry requires geometry review.
Release builds fail if required AI provenance is missing.
```

Policies should be project/org configuration, not hard-coded moral judgments in the language.

## 8. Deterministic builds

Build identity includes:

```text
source digests
asset digests
compiler version
language version
package lockfile
kernel version
solver/plugin versions
configuration
build profile
numerical/tolerance policy
```

Exclude nondeterministic metadata such as wall-clock timestamp from the geometry identity.

## 9. Lockfiles

`cad.lock` pins dependencies and relevant backend versions. Release builds should reject unresolved floating dependencies unless policy explicitly allows them.

## 10. Build attestations

Optional release artifact:

```json
{
  "build_id": "...",
  "source_digest": "...",
  "compiler": "...",
  "kernel": "...",
  "packages": {},
  "configuration": "...",
  "verification": "passed",
  "artifact_digests": {}
}
```

Signing/organization attestation can be added later.

## 11. Sandbox model

Untrusted source/packages run with denied-by-default access to:

- network;
- arbitrary filesystem;
- subprocesses;
- native libraries;
- environment secrets.

Geometry operations are allowed through controlled API capabilities.

## 12. Resource budgets

Enforce:

```text
CPU time
wall time
memory
geometry operation count
face/edge count
mesh triangle count
solver iterations
external process quota
```

Return a precise budget diagnostic instead of crashing/hanging.

## 13. Geometry denial-of-service risks

Potential adversarial/accidental cases:

- enormous patterns;
- pathological booleans;
- recursive geometry explosion;
- millions of tiny faces;
- extremely tight tolerances;
- malformed imported B-reps;
- mesh tessellation explosion.

Add guardrails and per-operation cost accounting.

## 14. Unsafe geometry review

`unsafe geometry` sections are searchable and auditable:

```text
cad unsafe list
```

Output:

```text
3 unsafe blocks
2 package-owned
1 project-owned
all validated
```

Release policy can require tests for each unsafe block.

## 15. Package security

Registry/security support:

- signed package metadata;
- content hashes;
- capability declaration;
- native/WASM distinction;
- vulnerability advisories;
- yanked versions;
- reproducible package builds where possible.

## 16. External asset provenance

Imported STEP/vendor assets store:

- source path/URI metadata;
- content digest;
- original units;
- healing transformations;
- optional supplier/version info;
- license/usage metadata if known.

## 17. Project policies

Example:

```yaml
policy:
  allow_network: false
  allow_native_plugins:
    - approved.solver
  require_lockfile: true
  require_release_tests: true
  max_raw_topology_blocks: 20
  ai:
    require_provenance: true
    require_review_for_unsafe: true
```

## 18. New feature: deterministic geometry fingerprint

Produce a backend-normalized fingerprint derived from:

- topology class/counts;
- canonicalized geometric descriptors;
- mass properties;
- bounding volumes;
- sampled geometric signature;

Use it for regression detection and cache verification. Do **not** claim bit-identical B-rep files across kernel/platform versions unless actually guaranteed.
