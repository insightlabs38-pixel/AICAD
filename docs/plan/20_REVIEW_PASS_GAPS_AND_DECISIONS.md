# 20 — Review Pass: Completeness, Gaps, Risks, and Decisions Still to Lock

This file is the deliberate second-pass review requested after assembling the full plan.

## 1. Completeness review result

The package covers every major idea developed in the conversation:

- AI-friendly symbolic CAD rather than GUI navigation;
- source as an importable/exportable real engineering model;
- high-level ordinary CAD layer;
- low-level near-kernel geometry layer;
- one unified language rather than separate DSLs;
- Turing completeness;
- loops/conditionals/functions/recursion;
- raw pointer-like topology escape hatch;
- semantic/stable references;
- query-based geometry selection;
- units/types/tolerances;
- assemblies/configurations/kinematics;
- interfaces/components;
- executable requirements/tests;
- compiler validation/diagnostics;
- package/plugin ecosystem;
- core + advanced AI skill files;
- custom skills for custom packages;
- source/GUI bidirectionality;
- REPL/debug/time travel;
- STEP/interchange;
- parametric reconstruction;
- drawings/PMI/GD&T;
- DFM/simulation/optimization/uncertainty;
- semantic Git/provenance;
- deterministic builds/security/budgets;
- AI learnability benchmarks;
- staged implementation roadmap.

No major conversation feature was intentionally dropped.

## 2. New features added during review

### 2.1 Raw topology epochs/lifetimes

**Why added:** A raw `Face*` or `Edge*` becomes invalid after topology changes. Treating raw handles like persistent pointers would create silent bugs.

**Decision:** Raw handles are geometry-epoch-scoped and cannot become persistent design state. Promotion requires validation and semantic exports.

### 2.2 Export fidelity reports

**Why added:** STEP/glTF/STL/etc. preserve different subsets of geometry/metadata/PMI/kinematics. Users and AI need to know what was lost.

**Decision:** Exports can emit structured fidelity reports.

### 2.3 Reference health reports

**Why added:** Stable naming is existentially important and should be measurable.

**Decision:** `cad refs check` reports explicit/lineage/query/raw/broken references.

### 2.4 Model context packets for AI

**Why added:** Large CAD projects will exceed useful model context.

**Decision:** `cad context <entity> --for-agent` emits a compact deterministic subset of relevant state.

### 2.5 Verification coverage + mutation testing

**Why added:** AI can generate superficial tests that all pass without protecting important behavior.

**Decision:** Track semantic verification coverage and optionally mutate dimensions/features to test requirement sensitivity.

### 2.6 Plugin capability simulation

**Why added:** AI agents may discover/install packages; users need predictable security properties.

**Decision:** Package manifests expose capabilities and can be inspected before activation.

### 2.7 Mechanical ports beyond geometric mates

**Why added:** Assemblies eventually need typed fluid/electrical/thermal/optical interfaces, not only coincident faces.

**Decision:** Make connector/port semantics extensible through packages.

## 3. Highest technical risks

### Risk A — Persistent topological naming

This is the largest foundational CAD risk. A simplistic query system can still pick the wrong face after topology changes.

Mitigation:

- explicit feature outputs;
- lineage tracking;
- query criteria;
- ambiguity-as-error;
- reference durability levels;
- aggressive regeneration benchmark;
- no silent arbitrary fallback.

Do not claim this solved until the benchmark proves it.

### Risk B — Geometry kernel robustness

Booleans, fillets, offsets, shelling, and healing can fail on difficult cases.

Mitigation:

- isolate backend;
- structured backend-independent errors;
- shape validation;
- healing profiles;
- alternative algorithms/backends later;
- geometry regression corpus.

### Risk C — Constraint solver robustness

Sketch/assembly/nonlinear constraints are difficult and can have multiple/unstable solutions.

Mitigation:

- separate constraint IR from solver;
- track solution branch/seed;
- report DOF/conflicts;
- start with common constraints;
- plugin solver architecture.

### Risk D — Bidirectional GUI/source edits

Arbitrary visual edits cannot always map cleanly back to a simple source patch.

Mitigation:

- source-aware structured edit operations;
- show derived/non-editable dimensions honestly;
- provide "extract override" or refactoring rather than silently rewriting expressions;
- visual graph edits operate on AST/HIR nodes.

### Risk E — Turing completeness and performance

Loops/recursion can create pathological geometry.

Mitigation:

- execution/resource budgets;
- lazy iterators;
- pure-function caching;
- geometry cost accounting;
- cancellation.

### Risk F — Language becoming too large

CAD domain breadth can cause hundreds of compiler intrinsics.

Mitigation:

- compiler core has small mathematical/topological primitive set;
- ordinary semantic features live in standard packages;
- low-level DSL is the extensibility escape hatch;
- every new intrinsic must pass the "could this be a library?" test.

### Risk G — AI skill sprawl

Thousands of packages could flood context.

Mitigation:

- progressive disclosure;
- exact schemas separate from prose skill;
- package triggers/discovery;
- compact context packets;
- load skills only when required.

## 4. Decisions to prototype before freezing

### 4.1 Exact surface syntax

Choose between:

- Rust-like braces/semicolons;
- TypeScript-like;
- Python-like indentation.

Recommendation remains Rust/TypeScript-like because it makes nested geometry and typed declarations explicit and is familiar to coding models.

### 4.2 Mutation semantics

Test whether users prefer:

```aicad
body = cut(body, hole);
```

or:

```aicad
body.cut(hole);
```

The compiler can lower both, but too many overlapping idioms can harm style consistency. Likely support functional core + ergonomic builder/method sugar.

### 4.3 Sketch representation

Decide whether sketch entities are explicit named source objects or a declarative block that lowers to them. Strongly favor explicit objects internally, with concise block syntax as sugar.

### 4.4 Package plugin runtime

Prototype WASM and external-process plugins before committing to a single extension ABI.

### 4.5 OCAF use vs custom semantic graph

OCAF concepts are useful, but the language likely needs a kernel-independent semantic-reference database above OCAF. Prototype both and avoid storing essential semantics only in OCAF structures.

### 4.6 File extension/branding

`.aicad`, `.cadl`, `CAD-IR`, etc. are placeholders.

## 5. Features deliberately deferred rather than missing

- electrical schematic/ECAD authoring;
- architecture/BIM;
- full CAM toolpaths;
- photorealistic rendering;
- topology optimization solver implementation;
- cloud PLM;
- real-time multi-user editing;
- certified standards database;
- native mobile CAD UI;
- domain-specific aerospace/medical validation.

These can integrate later but should not dilute the mechanical-CAD foundation.

## 6. Suggested first vertical demonstration

To validate the broad architecture without building every module, build a compact but demanding example:

**Parametric motor/gearbox enclosure assembly** containing:

- two custom parametric parts;
- one vendor STEP component;
- high-level features;
- one low-level spline/sweep feature;
- named semantic refs;
- loops/patterns;
- conditional configuration;
- mates + revolute joint;
- clearance requirement;
- wall-thickness test;
- mass requirement;
- STEP assembly export;
- source/viewport linking;
- AI creation/modification from core + geometry skill.

This tests almost every architectural thesis without requiring simulation/GD&T first.

## 7. Quality assessment

The plan is architecturally coherent if development respects the dependency gates. The main danger is not a missing feature; it is implementing attractive high-level features before semantic references, verification, and the low-level geometry escape hatch are mature enough.

The recommended discipline is:

> Expand vertical product capability only after the underlying abstraction proves it can survive regeneration, exact geometry, and debugging.
