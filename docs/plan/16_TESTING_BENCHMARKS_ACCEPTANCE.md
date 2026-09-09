# 16 — Testing, Benchmarks, Acceptance Gates, and Quality Strategy

## 1. Testing philosophy

This project can appear to work while being fundamentally unreliable. A rendered model is not sufficient evidence. Testing must cover syntax, semantics, numerical geometry, topology identity, interoperability, AI learnability, and engineering verification.

## 2. Test layers

### Compiler unit tests

- lexer/tokenization;
- parser;
- AST round-trip/formatter;
- type checking;
- unit dimensional algebra;
- generics;
- control flow;
- compile-time evaluation;
- capability checks;
- diagnostic spans.

### Runtime tests

- loops/recursion;
- iterators/generators;
- deterministic execution;
- budget enforcement;
- error/result behavior;
- sandbox restrictions.

### Geometry unit tests

For every geometry operation test:

- valid nominal case;
- boundary/degenerate case;
- invalid input;
- tolerance-sensitive case;
- deterministic properties;
- topology/provenance outputs;
- cancellation/resource failure where applicable.

## 3. Geometry correctness methods

Do not over-rely on serialized B-rep byte equality.

Use combinations of:

- validity/manifold checks;
- expected topology classes/count ranges;
- volume/area/length;
- bounding boxes;
- center of mass/inertia;
- exact known analytic dimensions;
- Hausdorff/sample-distance comparison where appropriate;
- section/intersection comparisons;
- semantic output resolution;
- independent round-trip/import checks.

## 4. Golden fixture categories

Create fixtures for:

```text
primitives
sketch constraints
extrusions/revolves
fillets/chamfers
shells/draft
patterns
freeform loft/sweep
NURBS trimming
booleans
healing
imported dirty STEP
assemblies
kinematics
configurations
requirements
raw topology/unsafe blocks
```

## 5. Topological naming benchmark

This should be a dedicated benchmark suite.

For each baseline part:

1. bind downstream features to semantic refs;
2. perturb upstream dimensions/features;
3. rebuild;
4. check intended entity selection;
5. check ambiguity is reported rather than silently misresolved.

Perturbations:

- resize extrusion;
- add/remove hole;
- change pattern count;
- change fillet radius;
- split a face;
- merge coplanar faces;
- reorder independent feature branches;
- suppress configuration feature.

Metrics:

```text
correct reference resolution
ambiguous-but-detected
broken-but-detected
silent wrong resolution   <- must approach zero
reference durability distribution
```

## 6. Low-level completeness benchmark

Maintain a corpus of advanced geometry tasks. Any time the solution requires changing compiler intrinsics, classify why.

Target categories:

- spline-defined retaining hook;
- variable-section sweep;
- twisted loft;
- custom impeller blade surface;
- lattice cell construction;
- cam profile;
- nonstandard gear tooth;
- manifold built from manually trimmed surfaces;
- repair of open imported shell.

The goal is not 100% of all mathematics; it is to show that the low-level DSL is general enough that the core language does not need feature proliferation.

## 7. Assembly benchmark

Fixture families:

- hinge;
- slider;
- gearbox;
- linear actuator;
- simple robot arm;
- nested assembly with repeated fasteners;
- vendor STEP parts;
- configuration replacement.

Validate:

- solved pose;
- DOF count;
- mate conflicts;
- collision;
- BOM counts;
- configuration behavior.

## 8. Interoperability benchmark

For each supported format/toolchain:

```text
source -> export -> independent import -> compare
```

Track:

- exact/near geometry;
- assemblies;
- units;
- names;
- colors;
- PMI subset;
- metadata;
- fidelity report correctness.

## 9. Determinism benchmark

Run the same build repeatedly and across supported platforms.

Compare:

- semantic build graph;
- normalized geometry fingerprint;
- test results;
- artifact metadata excluding nondeterministic fields.

Document platform/kernel differences instead of hiding them.

## 10. Performance benchmark

Track:

```text
parse/type time
full build time
incremental parameter edit time
B-rep operation time
mesh generation time
memory
feature count
face count
assembly instance count
cache hit rate
```

Benchmark scales:

- tiny part;
- normal part;
- complex freeform part;
- 100-component assembly;
- 10k-instance assembly;
- stress synthetic model.

Do not set aspirational numbers until early prototypes establish realistic baselines; set regression thresholds once measured.

## 11. Fuzzing

Fuzz:

- parser;
- unit/type expressions;
- serialized manifests;
- query expressions;
- importers;
- geometry operation parameter boundaries.

Geometry fuzzing must enforce budgets to avoid pathological kernel hangs.

## 12. Property-based tests

Examples:

- rigid transform preserves volume;
- mirror preserves volume/mass;
- union volume is bounded appropriately;
- applying inverse transform returns equivalent shape;
- unit conversion round-trips;
- query result respects predicate;
- deterministic pure function returns same result.

## 13. Security tests

- infinite loop budget;
- recursion explosion;
- geometry explosion;
- plugin capability escape attempts;
- malformed archive/path traversal;
- malicious imported files;
- stale raw handle access;
- native plugin trust boundary.

## 14. AI learnability benchmark

Always test with at least one **fresh general-purpose coding model not assumed to know the language**.

Task classes:

```text
learn core skill
create simple part
create parametric part
modify existing part
use semantic refs
fix compiler error
fix geometry error
fix test failure
create assembly
use unfamiliar package skill
advanced geometry after loading geometry skill
```

Metrics:

- skill tokens;
- tool calls;
- compile iterations;
- final tests passed;
- semantic reference quality;
- unsafe use frequency;
- unsupported API hallucination rate;
- manual intervention rate.

## 15. Human usability benchmark

Test both code-first and visual-first engineers on:

- create bracket;
- modify parameter;
- identify why fillet fails;
- replace vendor component;
- review a change;
- resolve reference ambiguity;
- create drawing;
- run requirement suite.

Measure task success and error classes, not only subjective preference.

## 16. Release gates

### Alpha foundation

- parser/compiler stable enough for examples;
- exact geometry backend;
- basic STEP export;
- no known silent unit conversion bugs.

### Parametric alpha

- feature DAG;
- semantic refs baseline;
- regeneration tests;
- core skill initial benchmark.

### Technical preview

- advanced low-level DSL;
- assemblies;
- requirements/tests;
- IDE/source mapping;
- package skeleton.

### Beta

- strong reference benchmark;
- interoperability corpus;
- package ecosystem;
- AI tool protocol;
- deterministic release profiles;
- security budgets.

### 1.0 candidate

Require:

- documented language stability policy;
- migration tooling;
- zero known silent wrong-reference bugs in release corpus;
- broad geometry validity suite;
- core AI learnability target met;
- IDE source/visual round-trip stable;
- reproducible lockfile builds;
- import/export fidelity documentation;
- core packages tested.

## 17. New feature: mutation testing for requirements

To determine whether engineering tests actually detect relevant mistakes, automatically perturb parameters/features within bounded ranges and see whether expected requirements fail.

Example:

```text
reduce wall 2.4 -> 0.8mm
remove mounting hole
shift connector 2mm
```

If all tests still pass, verification coverage may be weak.

This is analogous to software mutation testing and is especially valuable for AI-generated verification suites.
