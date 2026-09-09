# 21 — Master Feature Inventory

Use this as the program-level checklist. `[ ]` indicates not implemented; change to `[x]` as implementation lands.

## A. Language foundation

- [ ] Familiar C/Rust/TypeScript-style grammar
- [ ] Variables / constants / parameters
- [ ] Functions / methods
- [ ] Loops (`for`, `while`, `loop`)
- [ ] Conditionals
- [ ] Pattern matching
- [ ] Recursion
- [ ] Closures
- [ ] Iterators
- [ ] Generators/yield
- [ ] Structs
- [ ] Enums
- [ ] Interfaces/traits
- [ ] Generics
- [ ] Modules/imports
- [ ] Result/error types
- [ ] Compile-time evaluation
- [ ] Typed/hygienic metaprogramming later
- [ ] Pure functions
- [ ] Deterministic runtime
- [ ] Resource budgets
- [ ] Capability model

## B. Engineering type system

- [ ] Dimensional quantities
- [ ] Unit literals/conversion
- [ ] Length/area/volume
- [ ] Angle
- [ ] Mass/density
- [ ] force/torque
- [ ] pressure/stress
- [ ] time/velocity/acceleration
- [ ] temperature with affine handling
- [ ] tolerances
- [ ] ranges
- [ ] distributions/uncertainty
- [ ] fit types
- [ ] parameter UI metadata
- [ ] rationale metadata

## C. High-level CAD

- [ ] Part
- [ ] Sketch
- [ ] Profiles
- [ ] Lines/arcs/circles/rectangles/slots/polygons
- [ ] Sketch constraints
- [ ] Plate/box/cylinder/cone/sphere/torus
- [ ] Extrude
- [ ] Revolve
- [ ] Sweep
- [ ] Loft
- [ ] Booleans
- [ ] Split
- [ ] Fillet
- [ ] Chamfer
- [ ] Shell
- [ ] Offset
- [ ] Draft
- [ ] Hole
- [ ] Pocket
- [ ] Boss
- [ ] Rib
- [ ] Thread
- [ ] Linear/rectangular/radial/path patterns
- [ ] Mirror/transform
- [ ] Bearing seat
- [ ] O-ring groove
- [ ] High-level gear package
- [ ] Enclosure scaffold
- [ ] Datums
- [ ] Material assignment

## D. Low-level geometry

- [ ] Point/vector/direction/frame/axis
- [ ] Transform/matrix/quaternion
- [ ] Analytic curves
- [ ] Bezier curves
- [ ] B-spline/NURBS curves
- [ ] Curve interpolation
- [ ] Curve trimming/offset
- [ ] Analytic surfaces
- [ ] Bezier surfaces
- [ ] B-spline/NURBS surfaces
- [ ] Surface revolution/extrusion
- [ ] Surface trimming/offset
- [ ] Projection
- [ ] Intersection
- [ ] Closest distance
- [ ] Curve/surface evaluation
- [ ] Curvature/tangent/normal
- [ ] Vertex/edge/wire/face/shell/solid construction
- [ ] Compounds
- [ ] Sewing
- [ ] Healing
- [ ] Validation
- [ ] Face replacement/removal
- [ ] Edge split
- [ ] Face merge
- [ ] Topology traversal
- [ ] Raw topology handles
- [ ] Geometry epochs/stale-handle protection
- [ ] `unsafe geometry`
- [ ] Validated promotion
- [ ] Backend-specific final escape hatch

## E. Semantic references / dependency engine

- [ ] `FaceRef` / `EdgeRef` / etc.
- [ ] Explicit semantic feature outputs
- [ ] Query criteria objects
- [ ] Geometry predicates
- [ ] Topology predicates
- [ ] Spatial predicates
- [ ] Ranking/disambiguation
- [ ] Ambiguity errors
- [ ] Feature lineage
- [ ] Split/merge/delete tracking
- [ ] Reference durability levels
- [ ] Reference health report
- [ ] Feature DAG
- [ ] Incremental invalidation
- [ ] Cache keys

## F. Assemblies

- [ ] Components/instances
- [ ] Shared instance geometry
- [ ] Local frames
- [ ] Coincident mate
- [ ] Concentric mate
- [ ] Parallel/perpendicular mate
- [ ] Distance/angle mate
- [ ] Lock mate
- [ ] Revolute joint
- [ ] Prismatic joint
- [ ] Cylindrical/spherical/planar joints
- [ ] Universal/helical/custom joints
- [ ] Gear/rack/screw coupling
- [ ] DOF analysis
- [ ] Conflict diagnostics
- [ ] Interfaces
- [ ] Mechanical/fluid/electrical/optical/thermal ports
- [ ] Configurations
- [ ] Variant rules
- [ ] Suppression/replacement
- [ ] Nested subassemblies
- [ ] Contacts
- [ ] Motion study
- [ ] Collision over motion
- [ ] BOM
- [ ] Large-assembly LOD/lazy loading

## G. Verification

- [ ] Unified constraint IR
- [ ] Hard/soft constraints
- [ ] Solver conflict sets
- [ ] Requirements
- [ ] Requirement metadata/severity/source
- [ ] CAD unit tests
- [ ] Geometry assertions
- [ ] Approximate assertions
- [ ] Parameterized tests
- [ ] Contracts
- [ ] Invariants
- [ ] Build profiles
- [ ] Verification evidence
- [ ] Verification coverage
- [ ] Mutation testing

## H. Artifact/interchange

- [ ] Plain source modules
- [ ] `.aicad` deterministic bundle
- [ ] Manifest
- [ ] Lockfile
- [ ] BREP cache
- [ ] STEP part export
- [ ] STEP assembly export
- [ ] STEP import
- [ ] STEP metadata/PMI progressive mapping
- [ ] 3MF
- [ ] STL
- [ ] DXF
- [ ] glTF/GLB
- [ ] Healing report
- [ ] Export fidelity report
- [ ] External asset digests/provenance
- [ ] Round-trip tests
- [ ] Parametric reconstruction
- [ ] Reconstruction confidence/evidence

## I. Human IDE

- [ ] Code editor
- [ ] 3D viewport
- [ ] Language server
- [ ] Source -> geometry highlight
- [ ] Geometry -> source/reference
- [ ] Parameter inspector
- [ ] Bidirectional dimension edits
- [ ] Visual sketch editor
- [ ] Visual feature creation
- [ ] Feature DAG view
- [ ] REPL
- [ ] Geometry debugger
- [ ] Time travel
- [ ] Inspector
- [ ] Code actions/refactors
- [ ] Optional visual node mode
- [ ] Semantic diff/overlay
- [ ] "Explain selection"

## J. AI-native layer

- [ ] Core skill
- [ ] Geometry skill
- [ ] Engineering skill/profile
- [ ] Runtime docs search
- [ ] Exact API schema
- [ ] Build tool
- [ ] Test tool
- [ ] Inspect tool
- [ ] Query tool
- [ ] Explain tool
- [ ] Render tool
- [ ] Diff tool
- [ ] Export tool
- [ ] Package/skill discovery
- [ ] Agent context packets
- [ ] Learnability benchmark
- [ ] Unsupported API hallucination tracking
- [ ] AI source-style lints
- [ ] Project/org/team skills

## K. Packages/plugins

- [ ] Package manifest
- [ ] Dependency resolver
- [ ] Registry
- [ ] Lockfile integration
- [ ] Pure source packages
- [ ] WASM plugins
- [ ] External process plugins
- [ ] Trusted native plugins
- [ ] Capability declarations
- [ ] API schemas
- [ ] Package AI skills
- [ ] Examples/tests contract
- [ ] Package-defined validators
- [ ] Declarative package UI metadata
- [ ] Signing/advisories/yanking later
- [ ] Capability simulation

## L. Engineering modules

- [ ] Material database interface
- [ ] Mass/inertia properties
- [ ] Drawings/sheets/views
- [ ] Dimensions/callouts
- [ ] Drawing automation
- [ ] Datums
- [ ] PMI
- [ ] GD&T
- [ ] Surface finish
- [ ] Fits/tolerances
- [ ] CNC DFM
- [ ] Additive DFM
- [ ] Injection molding DFM
- [ ] Sheet metal
- [ ] Structural simulation adapter
- [ ] Thermal adapter
- [ ] CFD/EM plugin schema later
- [ ] Optimization
- [ ] Pareto/design-space exploration
- [ ] Tolerance stack
- [ ] Reliability probability assertions
- [ ] Cost model plugin
- [ ] Sustainability plugin

## M. Collaboration/security/scale

- [ ] Text diff
- [ ] AST diff
- [ ] Semantic/geometry diff
- [ ] Semantic merge
- [ ] Git diff/merge drivers
- [ ] Provenance
- [ ] AI provenance policy
- [ ] Deterministic builds
- [ ] Build attestation
- [ ] Sandboxing
- [ ] Geometry DoS protection
- [ ] Unsafe review report
- [ ] Package security
- [ ] Geometry fingerprint
- [ ] Incremental compilation
- [ ] Parallel DAG evaluation
- [ ] Large-assembly streaming
- [ ] Distributed build workers later

## N. CLI/tooling

- [ ] `cad build/check/fmt`
- [ ] `cad test/requirements/validate`
- [ ] `cad refs check`
- [ ] `cad unsafe list`
- [ ] `cad inspect/query/explain/why/history`
- [ ] `cad docs/search/schema/skill`
- [ ] `cad import/export/reconstruct/fidelity`
- [ ] package commands
- [ ] `cad debug/repl/trace/profile`
- [ ] `cad diff/merge-check/provenance/attest`
- [ ] `cad doctor`
- [ ] JSON structured output for CI/agents

## O. Testing/release infrastructure

- [ ] Parser/compiler tests
- [ ] Runtime tests
- [ ] Geometry operation suite
- [ ] Golden fixtures
- [ ] Topological naming benchmark
- [ ] Low-level completeness benchmark
- [ ] Assembly benchmark
- [ ] Interoperability benchmark
- [ ] Determinism benchmark
- [ ] Performance benchmark
- [ ] Fuzzing
- [ ] Property testing
- [ ] Security testing
- [ ] AI learnability benchmark
- [ ] Human usability benchmark
- [ ] Compatibility/migration testing
