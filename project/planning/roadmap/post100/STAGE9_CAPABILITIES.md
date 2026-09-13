# Stage 9 capability envelope — source-first engineering environment and evidence-driven visual authoring

**Planning status:** capability envelope only. This audit recommends an explicit split between **Stage 9A** and **Stage 9B**. The split is a roadmap proposal and requires owner approval; it does not remove visual authoring from the long-term product.

## Governing principle

**Source and AICAD-owned semantic state remain authoritative.**

Every editor, viewport, inspector, parameter control, selection operation, diff view, or future visual authoring action must resolve to the same semantic model used by compiler/CLI/verification. UI state may contain transient selection/camera/layout preferences, but it must not contain hidden engineering model state that cannot be reconstructed from authoritative source/project inputs.

---

# Stage 9A — source-first engineering environment

## Goal

Deliver a technically complete source-first CAD/engineering workspace: users can navigate, edit, build, inspect, measure, select, trace, compare and verify real models with immediate 3D semantic feedback. This is sufficient to make AICAD a coherent engineering environment even before conventional visual feature/sketch creation is implemented.

## Dependencies

### Compiler/language

- canonical AST/HIR/source spans and diagnostics;
- project/module/package resolution;
- stable semantic IDs and feature identities;
- incremental build/dirty propagation;
- source edit transactions with parse/type/semantic validation.

### Geometry/reference/provenance

- Stage4 semantic refs and resolver outcomes;
- Stage5 advanced geometry/query/lineage;
- stable source↔feature mappings through ordinary functions/parts/control flow;
- raw handles remain private/ephemeral.

### Assemblies/configurations

- Stage6 instance paths/IDs, relation/pose/DOF/config/BOM/interference structured APIs.

### Verification

- Stage7 requirement/test/case IDs;
- traceability graph;
- machine evidence/results;
- profile/config/build identity.

### Artifacts/interchange

- Stage8 project/native artifact/asset identity sufficient for opening/rebuilding portable projects.

## Required capability groups

### 1. LSP/editor integration

Core language services:

- syntax highlighting/token classification;
- parse/type/semantic diagnostics with stable codes and related spans;
- completion from actual scope/type information;
- go-to definition/references;
- symbol/project outline;
- hover/type/unit/semantic information;
- rename/refactor only where identity/semantic rules allow safe transformation;
- formatting/source organization if stable;
- code actions backed by structured diagnostics, not string matching.

The LSP/API should expose stable semantic service DTOs, not raw internal HIR structs as a compatibility promise.

### 2. Project navigation

Users should navigate by semantic engineering entities as well as files:

- modules/source declarations;
- parts/features;
- parameters;
- semantic references;
- assemblies/components/instances/interfaces;
- configurations;
- requirements/tests;
- external assets/package dependencies;
- generated/derived artifacts.

Navigation must clearly distinguish definition from instance and source declaration from derived geometry occurrence.

### 3. Incremental build control

The environment should:

- observe source/project edits;
- determine affected params/features/assemblies/verification cases using existing dependency semantics;
- rebuild/cancel superseded work deterministically;
- surface build stages/status without changing semantic ordering;
- retain valid caches only under accepted content/dependency keys;
- explain why a subject rebuilt or remained cached where feasible.

Avoid UI-specific incremental state that disagrees with CLI/build behavior.

### 4. Diagnostics workspace

A unified diagnostics experience should cover:

- parse/type/unit errors;
- geometry failures/healing warnings;
- semantic-reference ambiguous/broken outcomes;
- solver/assembly conflicts;
- configuration invalidity;
- verification failures/errors;
- artifact/interchange issues.

Diagnostics remain structured semantic objects. The UI groups/filters/renders them; it does not reinterpret the underlying result.

### 5. 3D viewport

Minimum useful viewport capabilities:

- exact/derived tessellated display from authoritative built geometry;
- parts and assemblies;
- configurations;
- camera/navigation/display modes;
- selected/hovered semantic subject highlighting;
- visibility/isolation as UI state unless saved explicitly through source/project semantics;
- measurement overlays;
- verification/diagnostic overlays;
- provenance/reference overlays where useful.

Rendering mesh IDs are never semantic IDs.

### 6. Selection

Selection is a critical boundary because CAD GUIs commonly leak transient topology identity.

Required behavior:

- viewport hit-testing maps render primitives → realized topology → semantic reference candidate(s);
- user selection stores/returns a semantic subject/reference when durable meaning is needed;
- raw topology hit handles may exist only as transient implementation details;
- ambiguous semantic resolution is shown to the user rather than silently choosing;
- selections can navigate to source and structured inspectors.

### 7. Semantic inspection

Inspector panes should expose, as applicable:

- type/unit/value;
- feature identity and parameters;
- dependency edges/cache status;
- semantic outputs;
- reference recipe/outcome/durability;
- topology lineage;
- source spans;
- provenance;
- instance path/local/world pose;
- mate/joint/DOF/conflict evidence;
- configuration status;
- requirement/test/evidence links;
- external asset/interchange provenance.

This is one of AICAD's differentiators: the UI should reveal the semantic system rather than hiding it behind a feature tree alone.

### 8. Measurement

Viewport-driven measurements should call the same Stage5 query services used by source/verification:

- distance/closest points;
- length/area/volume;
- angle;
- radius/diameter/curvature where supported;
- coordinate/frame readouts;
- assembly clearance/interference where supported.

Measurement UI reports units, relevant tolerance/approximation/evaluation evidence, and semantic subjects. It must not create a second geometry-query implementation with different numerical policy.

### 9. Source → geometry navigation

From source declaration/expression/diagnostic/reference:

- identify corresponding semantic feature/subject(s);
- frame/highlight them in viewport;
- show configuration/instance context if multiple realizations exist;
- explain when source maps to no currently active geometry (suppressed branch/config/etc.).

This depends directly on GAP-003 being solved in Stage5; top-level-builtin-only source mapping is insufficient.

### 10. Geometry → source navigation

From selection:

- resolve semantic ref/feature/instance;
- navigate to defining/producing source;
- display generated/modified lineage if no single source span owns the resulting topology;
- expose ambiguity/broken mapping explicitly.

Do not navigate via transient edge/face enumeration indices.

### 11. Feature DAG inspection

A developer/advanced-user graph view should support:

- semantic feature nodes;
- geometry dependencies;
- source/function call provenance;
- param dependencies;
- cache/dirty state;
- selected subject focus;
- reasons for invalidation;
- link to semantic refs and verification coverage.

The graph UI is a consumer of the feature service, not authority for graph mutations.

### 12. Semantic-reference inspection

Show:

- recipe;
- requested subject kind/cardinality;
- resolver stages/candidates;
- lineage evidence;
- durability class;
- current resolved/ambiguous/broken state;
- fingerprint evidence where policy permits;
- source consumers.

This is particularly useful for debugging parametric model robustness.

### 13. Provenance inspection

At minimum distinguish:

- source/feature dependency provenance;
- topology lineage;
- external asset provenance;
- verification/build evidence provenance;
- later collaboration/actor provenance if implemented.

Do not collapse all provenance concepts into one vague “history” list.

### 14. Parameter editing as source transaction

A parameter inspector may offer sliders/text fields/dropdowns, but committed engineering changes must be source/project transactions:

1. identify authoritative parameter declaration;
2. validate typed/unit-aware new value;
3. produce deterministic source edit or explicit override mechanism if such overrides are first-class authoritative state;
4. reparse/typecheck/rebuild;
5. show diagnostics if invalid;
6. retain normal version-control/diff behavior.

A slider must never mutate a hidden in-memory model that diverges from source.

### 15. Model diff/review

Leverage semantic identity/provenance to show more than text diffs:

- parameter changes;
- feature additions/removals/modifications;
- semantic-reference outcome changes;
- geometry measurement/fingerprint deltas;
- assembly/config/BOM changes;
- requirement/test status changes;
- external asset changes;
- visual overlays where useful.

Text diff remains available and authoritative source changes remain reviewable.

## Likely core abstractions for 9A

1. **SemanticService** — versioned query boundary over symbols/types/features/refs/assemblies/verification subjects.
2. **BuildSession / BuildSnapshot** — immutable identity for one resolved source/project/config/profile build, consumable by viewport/inspectors.
3. **SemanticSelection** — stable subject/reference + build/instance context, distinct from render hit ID.
4. **SourceTransaction** — validated source/project edit operation with preconditions and resulting diagnostics/build identity.
5. **NavigationTarget** — source ↔ semantic subject mapping with multiplicity/ambiguity.
6. **ModelDiff** — semantic change representation built from stable identities/evidence.

Do not expose private AST/HIR/kernels directly as these protocols unless the owner intentionally freezes them as public APIs.

## Stage 9A gate shape

A Stage9A gate should demonstrate a realistic project through an integrated environment:

- open/rebuild project from Stage8 artifact/source;
- editor/LSP diagnostics and navigation;
- viewport of advanced part + configurable assembly;
- viewport selection → semantic ref/instance → source;
- source → selected geometry;
- measurement through shared query service;
- feature DAG/reference/provenance inspection;
- parameter edit committed as source transaction and incremental rebuild;
- verification failure shown on semantic subject and navigated to requirement/source;
- semantic diff between two revisions/configurations;
- no hidden model state needed to reproduce the engineering result with CLI.

Performance targets should be benchmark-driven (interactive latency for representative projects), not arbitrary before Stage8/9 evidence exists.

---

# Stage 9B — visual authoring, conditional on evidence

## Why it is separate

A full traditional CAD GUI contains several large product programs: sketch creation/constraint editing, feature dialogs, direct manipulation, assembly mating, configuration editors, drawing tools, potentially surface modeling and more. Requiring all of that for the first “IDE” gate would:

- delay the semantic/source-first environment;
- create pressure for UI-only state and command histories;
- duplicate existing source/API semantics;
- make Stage9 success depend on broad UX parity before usage evidence identifies the highest-value workflows.

Separating 9B is a sequencing decision, not a claim that visual authoring is unimportant.

## Evidence that should justify promoting 9B work

At least one or more of:

1. usability studies show source editing is the dominant adoption blocker for a target workflow;
2. repeated user telemetry/interviews identify a visual task with high frequency and clear semantic mapping;
3. visual construction measurably reduces errors/time for a target engineering flow;
4. source transaction APIs are mature enough to guarantee every visual action has a stable auditable source representation;
5. downstream users require mixed source/visual collaboration;
6. a particular visual workflow (e.g. sketch constraint placement or assembly mating) provides materially better discoverability than text without creating hidden state.

“Other CAD tools have it” is insufficient evidence by itself.

## Candidate 9B capability families

These are candidates, **not guaranteed Stage9B scope**:

- visual sketch entity creation/editing;
- constraint creation/manipulation;
- feature creation dialogs/manipulators;
- semantic topology selection to construct feature references;
- freeform control-point/curve/surface editing;
- assembly component placement and mate/joint authoring;
- configuration/variant editor;
- requirement/test authoring helpers;
- drawing/annotation authoring if Stage12/domain priorities pull it forward.

Prioritize by evidence, not parity checklist.

## Non-negotiable visual-authoring architecture

Every committed authoring action must:

1. operate on semantic subjects, not render/topology indices as durable identity;
2. produce a validated **source/project transaction**;
3. show the exact source/project change or an intelligible semantic diff;
4. pass normal parser/type/compiler/reference/verification machinery;
5. be undoable by reverting source/project transaction/history, not by replaying hidden kernel state;
6. preserve D2 value semantics and D6 kernel neutrality;
7. never embed solver-native mate/constraint state as source authority;
8. never make visual state mandatory to build the project headlessly.

## Source generation/editing policy

Visual authoring will need a source-edit strategy. Options should be evaluated with real source examples:

- edit existing declaration/expression while preserving formatting/comments where feasible;
- generate named declarations into explicit source regions/modules;
- use an AST-aware source transformation preserving user intent;
- use canonical formatting for generated fragments but avoid rewriting unrelated source;
- present conflicts when a requested visual transformation cannot be represented unambiguously.

Do not store visual operations as an opaque command log that only the GUI can interpret.

## Major 9B risks

- semantic refs replaced by transient viewport picks;
- generated source becomes unreadable and users stop treating source as authoritative;
- visual editing rewrites unrelated code;
- GUI maintains a hidden model and source becomes an export format;
- solver-specific assembly manipulators encode backend assumptions;
- feature dialogs force high-level API shape around current UI instead of semantic design;
- direct modeling pressures Stage5 raw topology into persistent identity.

The gate for any promoted 9B capability should contain explicit tests against these failure modes.

## What Stage 9 should not pre-commit to

- complete parity with SolidWorks/Fusion/Onshape/etc.;
- a particular desktop/web rendering/UI framework years in advance;
- one feature-tree metaphor as the semantic model;
- a command-history architecture as model authority;
- full drawing/PMI/simulation UI before their engineering modules exist;
- collaborative real-time editing before Stage13 collaboration semantics justify it.

## Recommendation

Officially rename/reframe the roadmap to:

- **Stage 9A — Source-first engineering environment**: mandatory semantic IDE/viewer/inspection/edit-transaction capabilities.
- **Stage 9B — Evidence-driven visual authoring**: prioritized after 9A based on measured workflow need, with every visual edit lowering to authoritative source/project semantics.

This preserves the full product capability while preventing the GUI from becoming a second hidden source of truth or an unconditional multi-year gate.
