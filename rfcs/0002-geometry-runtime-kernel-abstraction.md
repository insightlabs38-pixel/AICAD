# RFC-0002: Geometry/Runtime and Kernel Abstraction

- Status: Draft (Stage 0)
- Stage-0 build items covered (`docs/plan/15_IMPLEMENTATION_ROADMAP.md`
  Stage 0): "safe vs raw topology model", "source/canonical-state rules",
  "kernel abstraction contract".
- Owner rulings incorporated: DL-5 (kernel abstraction boundary and
  lineage exposure), DL-6 (OCCT/standards-derived content, development
  policy). See `project/DECISION_LOG.md`.
- Depends on: RFC-0001 (surface syntax/mutation semantics this RFC's
  examples assume).

## 1. Summary

This RFC freezes three things: (a) the layered architecture between
AICAD source and exact B-rep geometry, (b) the safety model for raw
topology access, and (c) the exact shape and constraints of the kernel
abstraction boundary — including AICAD's relationship to Open CASCADE
Technology (OCCT) as its first kernel backend.

## 2. Architecture layers (frozen, from `docs/plan/01_SYSTEM_ARCHITECTURE.md`)

```text
Source AST -> Typed HIR -> Engineering HIR -> Feature IR / dependency DAG
  -> Geometry IR -> Kernel call graph -> B-rep
```

Each IR layer exists so that: high-level features lower predictably;
optimization/caching works before touching the kernel; diagnostics can
point back to semantic concepts; alternative kernels remain possible; and
analysis tools can inspect models without materializing all geometry
(`01` §5). No layer may be collapsed into another as an implementation
shortcut without a new RFC.

Major services and their crate homes (`01` §2, cross-referenced against
`docs/plan/22_REPOSITORY_WORK_PACKAGES.md` and each crate's own `README.md`):

| Service (`01` §2) | Crate(s) |
|---|---|
| Compiler frontend | `cad-lexer`, `cad-parser`, `cad-ast`, `cad-types`, `cad-units` |
| Language runtime | `cad-runtime` |
| Parametric engine | `cad-feature-graph`, `cad-references`, `cad-query` |
| Constraint subsystem | `cad-constraints` |
| Geometry runtime | `cad-geometry-api`, `cad-geometry-runtime` |
| Kernel adapter | `cad-kernel-api`, `cad-occt-bridge`, `native/occt_bridge` |
| Verification engine | `cad-requirements`, `cad-validation` |
| Artifact/interchange engine | `cad-artifact`, `cad-interchange` |
| Language service / IDE backend | `cad-lsp` |
| AI tool server | `cad-agent-tools` |

Recommended process boundaries (`01` §3 — IDE/frontend, compiler/language
service, geometry worker, untrusted package/plugin worker, external solver
worker, AI tool gateway) are adopted as a target, not frozen as a Stage-0
requirement; the exact process topology is implementation detail decided
when `cad-cli`/`cad-lsp` are built (Stage 2/9).

## 3. Kernel abstraction boundary (DL-5 / D6, resolved)

- `cad-kernel-api` is **kernel-neutral**: it defines domain-level geometry
  operations, opaque topology handles, validation, properties, traversal,
  and interchange primitives — never OCCT (or any other backend's)
  class/type names. Acceptable public concepts mirror
  `docs/plan/01_SYSTEM_ARCHITECTURE.md` §8: NURBS curve, planar face,
  manifold solid, tangent continuity, boolean union, semantic face
  reference. Unacceptable: `TopoDS_Face`, `BRepAlgoAPI_Fuse`, or any
  OCCT-specific numerical status enum, anywhere above `cad-occt-bridge`.
- **No OCCT class/type may cross the `cad-occt-bridge` boundary.**
  `cad-occt-bridge` is the only crate permitted to call `native/occt_bridge`
  or reference OCCT types at all; it translates them into
  `cad-kernel-api`'s neutral vocabulary before anything else in the
  workspace sees them.
- The operation set exposed by `cad-kernel-api`/`native/occt_bridge` is
  **intentionally minimal and capability-driven** — add operations only as
  the low-level language API (`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`)
  actually requires them, not as a comprehensive up-front OCCT wrapper.
  The representative bridge operation list in `01` §4 (`create_box` ...
  `export_step`) is the Stage-1 starting set, not a ceiling.
- For **topology-changing operations**, the adapter must expose whatever
  created/modified/deleted/split/merge lineage evidence the backend can
  produce for that operation, as local, operation-scoped data — the
  adapter does not itself persist or interpret lineage across operations.
- **Kernel topology handles are build/epoch-local.** They are not, and
  must never be treated as, persistent AICAD semantic references. A raw
  handle obtained in one geometry evaluation epoch is invalid after any
  topology-mutating operation on the affected shape (§4 below).
- **The semantic-reference layer (`cad-references`, RFC-0003) owns durable
  identity above the kernel.** The kernel adapter's per-operation lineage
  evidence is an *input* the semantic-reference layer consumes to build
  durable references; the kernel adapter itself stores no durable
  reference state.

This directly implements `docs/plan/01_SYSTEM_ARCHITECTURE.md` §8's
kernel-independence contract by fixing *how* it is enforced (crate
boundary + capability-driven minimal surface + lineage-evidence-not-storage)
rather than changing what it requires.

Kernel-specific diagnostics may still appear in a nested `backend_details`
field for debugging (`01` §8), but such a field is never the sole
explanation in a diagnostic surfaced to a user or agent (RFC-0005).

## 4. Raw topology safety model (frozen, from `00` §8 and `02` §16)

- `FaceRef`, `EdgeRef`, etc. (RFC-0003) are stable/semantic references that
  can survive regeneration when resolvable.
- `Face*`, `Edge*`, `Solid*`, `KernelShape*` are ephemeral raw handles,
  valid only within a **geometry evaluation epoch**.
- A topology-mutating operation starts a new epoch for the shapes it
  affects; using a stale raw handle after that point is a
  compile-time-checked-where-possible, otherwise runtime-trapped, error —
  never silently tolerated.
- Crossing an epoch boundary, or storing a raw handle in persistent
  project state, is an error.
- `unsafe geometry { ... }` blocks are required to obtain/use raw handles
  (`02` §16). Leaving such a block requires explicit `validate()` or
  `adopt_validated()` before the result is promoted to an ordinary
  `Solid`/`Part` value usable outside the block.
- Raw handles cannot be serialized or exported from public APIs without an
  explicit unsafe wrapper type; stable references must be recreated or
  exported through semantic naming/query logic (RFC-0003), never by
  smuggling a raw handle out.

This section changes nothing from the plan; it is frozen here because
`cad-kernel-api`/`cad-occt-bridge` (§3) and `cad-references` (RFC-0003)
both depend on this exact contract existing before their Stage 1/4 work
begins.

## 5. Canonical-state and cache model (frozen, from `01` §6-7)

- **Canonical:** the source tree, the lockfile, and external immutable
  assets (e.g. imported STEP files, referenced by content digest). RFC-0001
  §3 (DL-4) froze the project manifest name as `aicad.toml` but did not
  freeze a lockfile file name; the plan's own placeholder is `cad.lock`
  (`docs/plan/12_PACKAGES_PLUGINS_EXTENSIONS.md` §5). Harmonizing it to
  `aicad.lock` alongside `aicad.toml` is the natural choice but is **not**
  frozen by this RFC — treat the exact lockfile file name as open until
  `crates/cad-packages`/`crates/cad-cli` need it (Stage 2 CLI baseline at
  the earliest).
- **Cacheable, never canonical:** everything under `.aicad-cache/` — AST
  cache, typed-HIR cache, feature-DAG cache, BREP cache, display-mesh
  cache, analysis cache. This is already enforced mechanically by
  `.gitignore` (AICAD-003).
- Cache entries are content-addressed by: source digest, dependency
  digest, compiler version, kernel version, target configuration,
  relevant plugin/solver versions, and deterministic build options. This
  is why `rust-toolchain.toml` pins an exact compiler version
  (AICAD-003) — "compiler version" is a cache-key component, not a
  convenience.
- Incremental build strategy (`01` §7): each feature node records source
  inputs, parameter inputs, semantic references consumed, geometry
  outputs, topology lineage, and validation outputs; on change, only the
  dirty subgraph is recomputed and only affected tests/analyses rerun.
  This is frozen here as a contract that `cad-feature-graph` (Stage 3) and
  `cad-references` (Stage 4) must both honor; neither RFC-0002 nor
  RFC-0003 re-derives it independently.

This RFC does **not** define the cross-platform "equivalent geometry"
comparison tolerance mentioned in `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §3
rule 9 — that is `project/OWNER_DECISIONS.md` D5, still open.

## 6. Capability tiers (frozen, from `01` §9)

| Tier | Scope |
|---|---|
| A — Pure language | Math, collections, types; no filesystem/network/native calls. |
| B — Safe CAD | High-level/low-level geometry through validated operations. |
| C — Unsafe geometry | Raw topology handles, manual topology construction, healing, direct kernel-grade operations (§4's `unsafe geometry` blocks). |
| D — External capabilities | Filesystem, network, native solver, custom exporter, organization data. Must be declared and approved (package capability manifest, `docs/plan/12_PACKAGES_PLUGINS_EXTENSIONS.md` §7). |

Adopted as-is; no changes.

## 7. OCCT as an external dependency (DL-6 / D13 development policy, resolved)

- OCCT is treated strictly as a **third-party dependency behind
  `native/occt_bridge`**, never as code incorporated into AICAD's own
  source tree.
- Applicable OCCT licensing/notices are preserved; the architecture (§3)
  must permit compliant dynamic/shared-library distribution and must not
  depend on copying OCCT implementation code into AICAD-owned files.
- Standards-derived functionality (STEP AP242 mapping, GD&T semantics —
  Stage 12B, out of scope for this RFC) must be independently implemented;
  ISO/ASME copyrighted prose, tables, or figures must never be copied into
  AICAD source, docs, or test fixtures without an appropriate license.
- **This is a development-phase policy only.** A formal distribution/
  license review remains required before any public binary or commercial
  distribution policy is frozen — that review is a separate, still-open,
  future gate (`project/OWNER_DECISIONS.md` D13's remaining open item), not
  satisfied by this RFC.

## 8. Alternatives considered

- **A comprehensive/near-total OCCT wrapper** exposed through
  `cad-kernel-api` — rejected (DL-5); would violate kernel-independence
  and make a future non-OCCT kernel backend infeasible.
- **Persisting lineage inside the kernel adapter** rather than treating it
  as per-operation evidence consumed above — rejected (DL-5); durable
  identity belongs to the semantic-reference layer, not the kernel
  boundary (see also RFC-0003 §on D8).
- **Deferring all native-bridge work pending a full legal review** —
  rejected (DL-6); would block Stage 1 for a question the conservative
  dynamic-linkage-plus-notice-preservation policy already answers safely
  for development purposes.
- **Vendoring/statically incorporating OCCT source** — rejected outright
  (DL-6).

## 9. Open questions (intentionally not resolved here)

- `project/OWNER_DECISIONS.md` D5 (determinism-equivalence tolerance) —
  not defined by this RFC; needed before Stage 2's deterministic evaluator
  matures and before Stage 8/13 determinism benchmarks.
- `project/OWNER_DECISIONS.md` D13's remaining open item (formal
  distribution/license review before public/commercial release) — a
  future gate, not resolved here.
- The exact extent of any internal OCAF usage (`project/OWNER_DECISIONS.md`
  D8) is addressed in RFC-0003, not here, since it is about durable
  reference identity, not the kernel boundary itself.

## 10. Impact

- `crates/cad-kernel-api`, `crates/cad-occt-bridge`, `native/occt_bridge`:
  implement §3-4 and §7 starting Stage 1 (AICAD-015..019).
- `crates/cad-geometry-api`, `crates/cad-geometry-runtime`,
  `crates/cad-validation`: consume the kernel adapter per §2-3; must never
  re-expose OCCT types themselves either.
- `crates/cad-feature-graph`: implements the incremental-invalidation
  contract in §5 (Stage 3, AICAD-066..069).
- All crates: `.aicad-cache/` remains the only generated-state location
  (already enforced by `.gitignore`).
