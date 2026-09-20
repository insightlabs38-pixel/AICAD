//! The Stage-2 Safe CAD standard-function catalogue (`project/
//! DECISION_LOG.md#DL-15`, resolving `project/OWNER_DECISIONS.md#D18`).
//!
//! `DL-15` authorizes a general, non-geometry-specific mechanism —
//! compiler/runtime-owned "standard functions" — and requires that
//! mechanism be described by "a single authoritative declaration/
//! catalogue" feeding both binding/type checking and runtime dispatch.
//! [`BuiltinFnId`] is that catalogue's closed identity set (never a
//! serialized function pointer, never user-extensible — "a closed
//! runtime-owned mechanism", `DL-15`), and [`catalogue`] is the
//! authoritative signature table `crate::lower::Lowerer` reads to seed
//! every program with these names already bound (mirroring how
//! `crate::prelude::with_prelude` makes `Result`/`Optional` resolvable
//! everywhere, but at the HIR layer instead of AST source text — see this
//! module's own doc comment "Why HIR-level, not source-text, seeding"),
//! and `cad_runtime::interp::Interpreter::call` reads (via `crate::hir::
//! FunctionImplementation::RuntimeBuiltin`) to decide which native
//! behavior to run instead of executing a `HirBlock`.
//!
//! # Not a compiler intrinsic (`DL-15`)
//!
//! A `BuiltinFnId` names an ordinary callable: it resolves through
//! ordinary lexical scope, is type-checked through the exact same
//! `cad_hir::typeck::Checker::collect_signatures`/call-checking machinery
//! as any user-defined `fn`, and is invoked through ordinary
//! `HirExpr::Call`/`HirCallee::Fn` — nothing about the *language* changes
//! to support it. `project/DECISION_LOG.md#DL-7`'s compiler-intrinsic RFC
//! gate is untouched and unweakened; a genuine future compiler intrinsic
//! (a construct requiring new syntax/typing/lowering rules no ordinary
//! function could express) still needs its own RFC.
//!
//! # Why HIR-level, not source-text, seeding
//!
//! `crate::prelude::with_prelude` sources `Result`/`Optional` as literal
//! AICAD source text specifically because both are expressible in
//! ordinary, already-existing declaration syntax (`enum ... { ... }`) —
//! `crate::prelude`'s own doc comment: "byte-for-byte what a user could
//! paste into their own file." A `BuiltinFnId` has no such AICAD-source
//! spelling: there is no grammar production for "declare a function with
//! no body" (`cad_ast::item::Item::Fn` always carries a real block), and
//! `DL-15` explicitly forbids inventing new declaration/import syntax to
//! create one ("No new geometry-specific call syntax is introduced"; "No
//! new import syntax is authorized"). So unlike the prelude, these
//! declarations are built directly as HIR nodes by `crate::lower::
//! Lowerer::seed_builtins`, using this module's [`catalogue`] as the
//! single source of truth for names/signatures — never a second,
//! independently-typed copy in the type checker (`DL-15`: "The type
//! checker must not duplicate geometry signatures independently from the
//! runtime catalogue").
//!
//! # Stage-2 catalogue scope
//!
//! `DL-15`'s own text: "Stage 2 should expose only the Safe CAD operations
//! required to prove the Stage-2 language-to-geometry slice... Do NOT
//! expose every existing `GeometryOp` merely because the dispatcher
//! already implements it." This module's [`catalogue`] therefore covers
//! exactly `box`/`cylinder`/`transform`/`union`/`cut`/`intersect`/
//! `fillet`/`chamfer` (Stage 2) plus `plate` (Stage 3, `AICAD-071`) — not a
//! 1:1 mirror of `cad_geometry_api::ir::GeometryOp`'s full 18-variant set.
//! See `docs/API/safe-cad-api.md` for the full human-readable
//! specification (including the deliberate narrowings this module encodes:
//! `transform` is translate-only, `fillet`/`chamfer` select edges by a
//! plain `List<Int>` of raw indices rather than any new "edge reference"
//! type, and `plate` is corner-at-origin with no `corner_radius`/`frame`
//! parameter yet — see [`BuiltinFnId::Plate`]'s own doc comment).

use crate::types::HirTypeRef;
use cad_ast::Span;

/// The closed set of compiler/runtime-owned standard functions Stage 2
/// defines (`project/DECISION_LOG.md#DL-15`). Adding a variant here is
/// the only way a new runtime-backed function can ever exist — there is
/// no dynamic registration, no plugin hook, and no way for AICAD source
/// itself to define one, matching `DL-15`'s "no arbitrary native callback
/// facility" requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinFnId {
    /// `box(dx: Length, dy: Length, dz: Length) -> Geometry`.
    Box,
    /// `cylinder(radius: Length, height: Length) -> Geometry`.
    Cylinder,
    /// `transform(target: Geometry, dx: Length, dy: Length, dz: Length) ->
    /// Geometry` — a rigid translation. Stage-2 deliberately narrows the
    /// full `cad_kernel_api::Transform` (which can also rotate/compose)
    /// to translation only: no AICAD source-level vector/axis/rotation
    /// type exists yet to name a rotation unambiguously, and `DL-15`
    /// requires escalating rather than guessing an ambiguous signature.
    /// See `docs/spec/safe-cad-api.md` for the full rationale.
    Transform,
    /// `union(a: Geometry, b: Geometry) -> Geometry`.
    Union,
    /// `cut(a: Geometry, b: Geometry) -> Geometry`.
    Cut,
    /// `intersect(a: Geometry, b: Geometry) -> Geometry`.
    Intersect,
    /// `fillet(target: Geometry, edges: List<Int>, radius: Length) ->
    /// Geometry`. `edges` is a plain list of raw edge indices (the exact
    /// `usize` selection `cad_geometry_api::ir::EdgeIndex` already is),
    /// carried as an ordinary `List<Int>` value rather than a new
    /// "edge reference" type — this exposes no OCCT/kernel handle to
    /// source, only integers, matching `DL-15`'s "does NOT expose a raw
    /// kernel topology pointer."
    Fillet,
    /// `chamfer(target: Geometry, edges: List<Int>, distance: Length) ->
    /// Geometry`. See [`BuiltinFnId::Fillet`]'s own doc comment.
    Chamfer,
    /// `plate(width: Length, depth: Length, thickness: Length) ->
    /// Geometry` (`AICAD-071`, Stage 3). A rectangular plate — dispatches
    /// to the identical `GeometryOp::Box` construction `box` itself uses
    /// (`dx = width, dy = depth, dz = thickness`), since a flat rectangular
    /// plate has no geometry `box` does not already cover; `plate` exists
    /// as its own catalogue entry purely so Safe CAD source has the
    /// domain-meaningful name `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s
    /// `plate` feature specifies, not because a new `GeometryOp` variant
    /// is needed. Deliberately narrower than that plan section's own
    /// `plate` signature (`size: Vector2<Length>`, `center: Point3`,
    /// `corner_radius: Length`, `frame: Frame3`, `centered: Bool`): no
    /// rounded-corner construction op exists in `cad_geometry_api::ir`
    /// (`corner_radius` would need low-level wire/face construction,
    /// explicitly out of Stage-2/3 scope so far — `cad_hir::builtins`'s own
    /// module doc comment, "Stage-2 catalogue scope"), and no builtin
    /// consumes the new `AICAD-070` `Frame3`/`Point3` geometry types yet
    /// (Stage-3's own axis/frame/rotation foundation is `AICAD-075A`'s
    /// task, not this one's — matches `Transform`'s own precedent
    /// "deliberately translate-only" narrowing for the identical reason:
    /// escalating rather than guessing an ambiguous signature). A plate is
    /// always corner-at-origin, matching `box`'s own existing convention;
    /// `center`/`centered`/`frame` placement is left to a future
    /// `transform` call, exactly as any other Safe CAD solid.
    Plate,
    /// `extrude(target: Geometry, face: Int, direction: Vector3<Float>,
    /// distance: Length) -> Geometry` (`AICAD-076`, Stage 3). Extrudes
    /// one face of an already-built `target` solid outward by `distance`
    /// along `direction`, returning the new standalone prism (not merged
    /// with `target` -- compose with `union` for a boss). `face` is a
    /// raw kernel-enumeration-order index, mirroring `Fillet`/`Chamfer`'s
    /// own established `List<Int>` raw-index-selection precedent.
    /// Deliberately narrower than `docs/plan/
    /// 04_HIGH_LEVEL_MODELING_API.md`'s own `extrude(profile: Profile|
    /// Sketch|FaceRef, ...)` signature: no source-level `Sketch`/
    /// `Profile` construction exists yet (`cad_hir::sketch` has no
    /// grammar/lowering integration — see that module's own doc
    /// comment), so "an existing solid's own face" is the only
    /// source-visible profile source today. Uses `AICAD-075A`'s
    /// `cad_runtime::spatial::direction3_from_value` for `direction`.
    Extrude,
    /// `revolve(target: Geometry, face: Int, axis: Axis3, angle: Angle)
    /// -> Geometry` (`AICAD-076`, re-typed by `AICAD-076A`). Revolves one
    /// face of an already-built `target` solid about `axis` by `angle`,
    /// returning the new standalone solid of revolution. Same
    /// face-selection/profile-source narrowing as [`BuiltinFnId::
    /// Extrude`]'s own doc comment. `axis` is a real `Axis3` value,
    /// converted via `AICAD-075A`'s `cad_runtime::spatial::
    /// axis3_from_value` — the standard type environment note below
    /// explains why this is now safe to reference directly.
    Revolve,
    /// `hole(target: Geometry, axis: Axis3, diameter: Length, depth:
    /// Length) -> Geometry` (`AICAD-076`, re-typed by `AICAD-076A`). Cuts
    /// a cylindrical hole of `diameter`/`depth` along `axis` out of
    /// `target` — dispatches to `GeometryOp::Cylinder`, `GeometryOp::
    /// Transform` (placed via `Frame3::from_z`/`Transform::from_frames`,
    /// `AICAD-075A`), and `GeometryOp::Cut`, no new `GeometryOp` variant
    /// needed (mirrors [`BuiltinFnId::Plate`]'s own "domain-meaningful
    /// name over existing ops" precedent). `axis` is a real `Axis3`
    /// value, converted via `cad_runtime::spatial::axis3_from_value`.
    /// Deliberately narrower than `docs/plan/
    /// 04_HIGH_LEVEL_MODELING_API.md`'s own `hole` signature: `depth` is
    /// a plain `Length` picked by the caller (no `ThroughAll` -- querying
    /// `target`'s own extent along `axis` to compute one automatically is
    /// a separate, not-yet-built capability), and no counterbore/
    /// countersink/thread metadata yet.
    Hole,
    /// `pocket(target: Geometry, frame: Frame3, width: Length, length:
    /// Length, depth: Length) -> Geometry` (`AICAD-076`, re-typed by
    /// `AICAD-076A`). Cuts a `width` x `length` x `depth` rectangular
    /// pocket out of `target`, corner-at-`frame`'s-origin extending along
    /// `frame`'s own `x`/`y`/`z` axes (matching `box`'s own
    /// corner-at-origin convention, relocated by `frame` via
    /// `Transform::from_frames`) -- dispatches to `GeometryOp::Box` +
    /// `GeometryOp::Transform` + `GeometryOp::Cut`, no new `GeometryOp`
    /// variant needed. `frame` is a real `Frame3` value, converted via
    /// `cad_runtime::spatial::frame3_from_value`. Deliberately narrower
    /// than `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s own
    /// `pocket(profile: Profile|Sketch, ...)` signature: narrowed from an
    /// arbitrary profile to a rectangle, mirroring `plate`'s own
    /// already-accepted box-only narrowing of the general `Profile`
    /// concept.
    Pocket,
    /// `mirror(target: Geometry, plane: Plane) -> Geometry` (`AICAD-077`).
    /// Mirrors `target` across `plane` — a true reflection (determinant
    /// -1), dispatching to the new `GeometryOp::Mirror`/`Shape::mirror`
    /// kernel capability (`project/OWNER_DECISIONS.md#D20`'s own "Next
    /// dependency" note: mirror cannot be expressed as a `Transform`,
    /// which is proper-rigid-only). `plane` is a real `Plane` value,
    /// converted via `cad_runtime::spatial::plane3_from_value` — safe to
    /// reference directly per the type-closed standard environment
    /// `AICAD-076A` established. Deliberately narrower than `docs/plan/
    /// 04_HIGH_LEVEL_MODELING_API.md`'s own `mirror(item: Shape|Feature,
    /// plane: Plane|FaceRef, merge: Bool)` signature: no `FaceRef` plane
    /// source (no source-level face-reference type exists yet, same gap
    /// `Extrude`/`Revolve`'s own `face: Int` raw-index narrowing
    /// documents) and no `merge` option (the caller composes `union`
    /// itself if a merged result is wanted, exactly like `extrude`'s own
    /// "returns the new standalone prism, compose with `union` for a
    /// boss" precedent).
    Mirror,
    /// `linear_pattern(target: Geometry, direction: Vector3<Float>,
    /// count: Int, spacing: Length) -> Geometry` (`AICAD-077`). Returns
    /// the union of `count` copies of `target`, the first left in place
    /// and each subsequent one translated along `direction` (normalized)
    /// by an additional `spacing`, i.e. positioned at `0, spacing,
    /// 2*spacing, ..., (count-1)*spacing`. No new `GeometryOp` variant —
    /// built entirely from the already-existing `GeometryOp::Transform`/
    /// `GeometryOp::Union`, mirroring `Hole`/`Pocket`'s own "domain-
    /// meaningful name over existing ops" precedent, just looped `count`
    /// times. Deliberately narrower than `docs/plan/
    /// 04_HIGH_LEVEL_MODELING_API.md`'s own `linear_pattern` signature:
    /// `item` is a built `Geometry` value (no `Feature|Shape|Fn` source
    /// polymorphism — no `Feature`/`FeatureGroup` concept exists yet, per
    /// this catalogue's own established "a builtin returns `Geometry`,
    /// not a provenance-bearing feature handle" precedent) and no
    /// `centered` option (the caller's own choice of `direction`'s sign
    /// already controls which way the pattern extends from `target`'s
    /// unmoved first copy).
    LinearPattern,
    /// `radial_pattern(target: Geometry, axis: Axis3, count: Int, angle:
    /// Angle) -> Geometry` (`AICAD-077`). Returns the union of `count`
    /// copies of `target`, the first left in place and each subsequent
    /// one rotated about `axis` by an additional `angle / count`, i.e.
    /// positioned at angular offsets `0, angle/count, 2*angle/count, ...,
    /// (count-1)*angle/count` — the standard "full circle divided evenly"
    /// convention for a closed pattern (`angle = 360deg` places `count`
    /// evenly spaced copies with no duplicate at the seam). `axis` is a
    /// real `Axis3` value, the same axis representation `revolve`/`hole`
    /// already share (`AICAD-075A`'s own integration requirement). No new
    /// `GeometryOp` variant — built from `GeometryOp::Transform`/
    /// `GeometryOp::Union`, exactly like [`BuiltinFnId::LinearPattern`].
    /// Deliberately narrower than `docs/plan/
    /// 04_HIGH_LEVEL_MODELING_API.md`'s own `radial_pattern` signature: no
    /// `include_endpoint` option (this builtin's own fixed "divide `angle`
    /// into `count` equal steps, never repeating the `angle`-degree
    /// position" convention already matches that option's own documented
    /// `false` default), matching [`BuiltinFnId::LinearPattern`]'s own
    /// narrowing rationale otherwise.
    RadialPattern,
    /// `shell(target: Geometry, removed_faces: List<Int>, thickness:
    /// Length) -> Geometry` (`AICAD-078`). Hollows `target` to a uniform
    /// `thickness`, opening the given `removed_faces`. `removed_faces` is
    /// a plain `List<Int>` of raw, kernel-enumeration-order face indices
    /// — the same raw-index-selection convention `fillet`/`chamfer`
    /// already established for `edges` (`Fillet`/`Chamfer`'s own doc
    /// comments), applied here to faces instead, matching `cad_geometry_api::
    /// ir::GeometryOp::Shell`'s own `removed_faces: Vec<FaceIndex>` shape
    /// directly. Unlike `Fillet`/`Chamfer`'s `edges`, an empty
    /// `removed_faces` list is legitimate (a fully closed shell) — see
    /// `GeometryOp::Shell`'s own doc comment; this builtin does not reject
    /// it. No new `GeometryOp` variant or kernel capability was needed:
    /// `GeometryOp::Shell`/`Shape::shell`/`aicad_occt_shell` have existed
    /// since `AICAD-026`/`AICAD-059`/`AICAD-060` — this is purely the
    /// missing Safe CAD catalogue entry over an already-complete
    /// capability, the one dress-up feature `cad_hir::builtins`'s own
    /// "Stage-2 catalogue scope" note left out alongside `fillet`/
    /// `chamfer`. `thickness` is always hollowed *inward* (cavity removes
    /// material) — `cad_runtime::interp::Interpreter::dispatch_builtin`'s
    /// own `Shell` arm negates the evaluated magnitude before building the
    /// `GeometryOp::Shell` node, since `Shape::shell`'s own already-
    /// established convention is "negative thickness hollows inward,
    /// positive builds material outward" (`crates/cad-occt-bridge`'s own
    /// `shell_hollowed_box_matches_analytic_volume` test). This matches
    /// `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s own `inward: Bool =
    /// true` default with no separate parameter needed for it.
    Shell,
    /// `is_valid(target: Geometry) -> Bool` (`AICAD-105`,
    /// `project/DECISION_LOG.md#DL-23`/`DL-25`). A kernel-backed query, not
    /// a construction op — see [`BuiltinCategory::Query`]'s own doc
    /// comment. Dispatches to `GeometryQuery::IsValid`, demand-materialized
    /// through `cad_runtime::query_exec::KernelQueryExecutor` so its real
    /// result can drive ordinary source control flow immediately.
    IsValid,
    /// `volume(target: Geometry) -> Volume` (`AICAD-105`). Dispatches to
    /// `GeometryQuery::Volume`. See [`BuiltinFnId::IsValid`]'s own doc
    /// comment for the query-dispatch mechanism.
    Volume,
    /// `area(target: Geometry) -> Area` (`AICAD-105`). Dispatches to
    /// `GeometryQuery::Area`. See [`BuiltinFnId::IsValid`]'s own doc
    /// comment for the query-dispatch mechanism.
    Area,
    /// `line_curve(origin: Point3, direction: Vector3<Float>) -> Curve`
    /// (`AICAD-109`). Builds a `cad_geometry_api::curve::AnalyticCurve::
    /// Line` — a [`BuiltinCategory::Value`] builtin: pure data assembly, no
    /// `GeometryGraph` node and no kernel call (`AnalyticCurve`'s own doc
    /// comment: "constructing one is pure data assembly, never a kernel
    /// call").
    LineCurve,
    /// `circle_curve(center: Point3, normal: Vector3<Float>, radius:
    /// Length) -> Curve` (`AICAD-109`). Builds a validated
    /// `AnalyticCurve::Circle` (`cad_geometry_api::curve::AnalyticCurve::
    /// circle`) — rejects a non-finite/non-positive `radius`. See
    /// [`BuiltinFnId::LineCurve`]'s own doc comment for the category.
    CircleCurve,
    /// `arc_curve(center: Point3, normal: Vector3<Float>, radius: Length,
    /// start_angle: Angle, end_angle: Angle) -> Curve` (`AICAD-109`).
    /// Builds a validated `AnalyticCurve::Arc` — rejects a non-finite/
    /// non-positive `radius` or a `start_angle >= end_angle`. See
    /// [`BuiltinFnId::LineCurve`]'s own doc comment for the category.
    ArcCurve,
    /// `ellipse_curve(center: Point3, normal: Vector3<Float>,
    /// major_direction: Vector3<Float>, major_radius: Length, minor_radius:
    /// Length) -> Curve` (`AICAD-109`). Builds a validated
    /// `AnalyticCurve::Ellipse` — rejects a non-finite/non-positive radius,
    /// `major_radius < minor_radius`, or a `major_direction` not
    /// perpendicular to `normal`. See [`BuiltinFnId::LineCurve`]'s own doc
    /// comment for the category.
    EllipseCurve,
    /// `evaluate_curve(curve: Curve, u: Float) -> CurveEvaluation`
    /// (`AICAD-109`). Evaluates any [`BuiltinFnId::LineCurve`]/
    /// [`BuiltinFnId::CircleCurve`]/[`BuiltinFnId::ArcCurve`]/
    /// [`BuiltinFnId::EllipseCurve`]-constructed `Curve` at parameter `u`
    /// via `cad_geometry_api::curve::AnalyticCurve::evaluate` — a
    /// closed-form computation (see that method's own doc comment for each
    /// family's parameter convention), never a kernel call. A degenerate
    /// curve or an out-of-domain `u` (an `Arc`'s own restricted range) is a
    /// structured `RuntimeError`, never a silently wrong point. See
    /// [`BuiltinFnId::LineCurve`]'s own doc comment for the category.
    EvaluateCurve,
    /// `bezier_curve(control_points: List<Point3>, weights: List<Float>)
    /// -> Curve` (`AICAD-110`). Builds a validated `cad_geometry_api::
    /// curve::AnalyticCurve::Bezier` (`AnalyticCurve::bezier`) — rejects
    /// fewer than 2 control points, or (when `weights` is non-empty) a
    /// length mismatch or a non-positive/non-finite weight. An empty
    /// `weights` list means a plain (non-rational) Bezier — `docs/plan/
    /// 05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`'s own `weights: List<Float>?`
    /// spelling is an *optional* parameter, which the closed `RuntimeBuiltin`
    /// catalogue (`cad_hir::builtins::BuiltinFnSpec`) has no mechanism for
    /// yet (no existing catalogue entry has ever needed one — every
    /// existing optional-shaped signature in `docs/plan` was already
    /// narrowed away, e.g. `plate`'s own missing `corner_radius`); an
    /// empty list plays that same "absent" role using a mechanism the
    /// catalogue already fully supports. See [`BuiltinFnId::LineCurve`]'s
    /// own doc comment for the category.
    BezierCurve,
    /// `bspline_curve(degree: Int, control_points: List<Point3>, knots:
    /// List<Float>, multiplicities: List<Int>, weights: List<Float>,
    /// periodic: Bool) -> Curve` (`AICAD-110`). Builds a validated
    /// `AnalyticCurve::BSpline` (`AnalyticCurve::bspline`) — see that
    /// constructor's own doc comment for the full validation list.
    /// `periodic: true` is rejected (`CurveConstructionError::
    /// UnsupportedPeriodic`) rather than silently ignored — a documented
    /// `AICAD-110` scope limitation, not a default value standing in for
    /// an unsupported case. `weights` follows [`BuiltinFnId::BezierCurve`]'s
    /// own "empty list means non-rational" convention. See
    /// [`BuiltinFnId::LineCurve`]'s own doc comment for the category.
    BSplineCurve,
    /// `trim_curve(curve: Curve, u0: Float, u1: Float) -> Curve`
    /// (`AICAD-111`). Builds a validated `AnalyticCurve::Trimmed`
    /// (`AnalyticCurve::trim`) — rejects a non-finite `u0`/`u1`,
    /// `u0 >= u1`, or (when `curve` already has a bounded
    /// `AnalyticCurve::domain`) a `[u0, u1]` that is not a sub-range of
    /// it. See [`BuiltinFnId::LineCurve`]'s own doc comment for the
    /// category.
    TrimCurve,
    /// `offset_curve(curve: Curve, distance: Length, normal:
    /// Vector3<Float>) -> Curve` (`AICAD-111`). Exact for `Line`/`Circle`/
    /// `Arc` (`AnalyticCurve::offset`); every other family reports
    /// `CurveOperationError::UnsupportedFamily` (exact offsetting is not,
    /// in general, expressible in the same family — see that error
    /// variant's own doc comment). `normal` is always required (the
    /// catalogue has no optional-parameter mechanism — see
    /// [`BuiltinFnId::BezierCurve`]'s own doc comment) even though only a
    /// `Line` offset actually consumes it. See [`BuiltinFnId::LineCurve`]'s
    /// own doc comment for the category.
    OffsetCurve,
    /// `closest_point_on_curve(curve: Curve, point: Point3) ->
    /// List<ClosestPointResult>` (`AICAD-111`). Every point on `curve`
    /// closest to `point` (`AnalyticCurve::closest_point`) — **every**
    /// local-minimum solution found, never an arbitrary single one, per
    /// `AGENTS.md`'s "ambiguity is an error, never an arbitrary
    /// selection": a target equidistant from more than one point on the
    /// curve returns every one of them as a separate list element. A
    /// genuinely degenerate query (e.g. a circle's own center) is a
    /// structured `RuntimeError`, never a silently empty list standing in
    /// for "could not tell." See [`BuiltinFnId::LineCurve`]'s own doc
    /// comment for the category.
    ClosestPointOnCurve,
    /// `interpolate_curve(points: List<Point3>, tolerance: Length) ->
    /// Curve` (`AICAD-111`). Fits an *exact* interpolating cubic B-spline
    /// through `points` (`cad_geometry_api::curve::interpolate`) —
    /// `tolerance` bounds only the achieved numerical residual the linear
    /// solve itself may leave, never a target approximation error (this
    /// builtin interpolates exactly, it does not least-squares-fit). See
    /// [`BuiltinFnId::LineCurve`]'s own doc comment for the category.
    InterpolateCurve,
    /// `plane_surface(origin: Point3, normal: Vector3<Float>) -> Surface`
    /// (`AICAD-113`). Builds a `cad_geometry_api::surface::AnalyticSurface::
    /// Plane` — a [`BuiltinCategory::Value`] builtin, mirroring
    /// [`BuiltinFnId::LineCurve`]'s own "pure data assembly, never a kernel
    /// call" category exactly, applied to the surface-family counterpart of
    /// `Curve`.
    PlaneSurface,
    /// `cylinder_surface(axis: Axis3, radius: Length) -> Surface`
    /// (`AICAD-113`). Builds a validated `AnalyticSurface::Cylinder`
    /// (`AnalyticSurface::cylinder`) — rejects a non-finite/non-positive
    /// `radius`. See [`BuiltinFnId::PlaneSurface`]'s own doc comment for the
    /// category.
    CylinderSurface,
    /// `cone_surface(axis: Axis3, half_angle: Angle) -> Surface`
    /// (`AICAD-113`). Builds a validated `AnalyticSurface::Cone` — rejects a
    /// `half_angle` outside `(0, pi/2)`. See [`BuiltinFnId::PlaneSurface`]'s
    /// own doc comment for the category.
    ConeSurface,
    /// `sphere_surface(center: Point3, radius: Length) -> Surface`
    /// (`AICAD-113`). Builds a validated `AnalyticSurface::Sphere` —
    /// rejects a non-finite/non-positive `radius`. See
    /// [`BuiltinFnId::PlaneSurface`]'s own doc comment for the category.
    SphereSurface,
    /// `torus_surface(axis: Axis3, major_radius: Length, minor_radius:
    /// Length) -> Surface` (`AICAD-113`). Builds a validated
    /// `AnalyticSurface::Torus` — rejects a non-finite/non-positive radius
    /// or `minor_radius >= major_radius` (Stage-5's initial "ring torus
    /// only" scope — see `SurfaceConstructionError::
    /// MinorNotLessThanMajor`'s own doc comment). See
    /// [`BuiltinFnId::PlaneSurface`]'s own doc comment for the category.
    TorusSurface,
    /// `evaluate_surface(surface: Surface, u: Float, v: Float) ->
    /// SurfaceEvaluation` (`AICAD-113`). Evaluates any surface builtin's
    /// constructed `Surface` at `(u, v)` via `cad_geometry_api::surface::
    /// AnalyticSurface::evaluate` — a closed-form computation (see that
    /// method's own doc comment for each family's parameter convention),
    /// never a kernel call. A genuine parametrization singularity (e.g. a
    /// sphere's own pole) or an out-of-domain `(u, v)` is a structured
    /// `RuntimeError`, never a silently wrong point/normal. See
    /// [`BuiltinFnId::PlaneSurface`]'s own doc comment for the category.
    EvaluateSurface,
    /// `bezier_surface(control_points: List<List<Point3>>, weights:
    /// List<List<Float>>) -> Surface` (`AICAD-114`). Builds a validated
    /// `cad_geometry_api::surface::AnalyticSurface::Bezier`
    /// (`AnalyticSurface::bezier`) — `control_points[i]` is one row along
    /// `u`, `control_points[i][j]` the control point at `(i, j)`; rejects a
    /// non-rectangular net, fewer than 2 rows/columns, or (when `weights`
    /// is non-empty) a shape mismatch or a non-positive/non-finite weight.
    /// An empty `weights` list means a plain (non-rational) surface,
    /// mirroring [`BuiltinFnId::BezierCurve`]'s own "no optional-parameter
    /// mechanism" convention. See [`BuiltinFnId::PlaneSurface`]'s own doc
    /// comment for the category.
    BezierSurface,
    /// `bspline_surface(degree_u: Int, degree_v: Int, control_points:
    /// List<List<Point3>>, knots_u: List<Float>, multiplicities_u:
    /// List<Int>, knots_v: List<Float>, multiplicities_v: List<Int>,
    /// weights: List<List<Float>>, periodic_u: Bool, periodic_v: Bool) ->
    /// Surface` (`AICAD-114`). Builds a validated `AnalyticSurface::BSpline`
    /// (`AnalyticSurface::bspline`) — see that constructor's own doc
    /// comment for the full validation list, applied once per direction.
    /// `periodic_u`/`periodic_v: true` is rejected
    /// (`SurfaceConstructionError::UnsupportedPeriodic`), mirroring
    /// [`BuiltinFnId::BSplineCurve`]'s own identical scope limitation. See
    /// [`BuiltinFnId::PlaneSurface`]'s own doc comment for the category.
    BSplineSurface,
    /// `trim_surface(base: Surface, outer: Curve, holes: List<Curve>,
    /// tolerance: Length) -> Surface` (`AICAD-115`). Builds a validated
    /// `cad_geometry_api::surface::AnalyticSurface::Trimmed`
    /// (`AnalyticSurface::trim`) — `outer`/each element of `holes` is a
    /// `Curve` read as a closed loop in `base`'s own `(u, v)` parameter
    /// plane (`cad_geometry_api::surface::TrimLoop::new`, `tolerance` its
    /// own `project/DECISION_LOG.md#DL-26` modeling/construction
    /// tolerance). Rejects an unclosed/non-planar/degenerate loop, a hole
    /// oriented the same way as the outer boundary, or a loop sample
    /// outside `base`'s own valid domain — see `AnalyticSurface::trim`'s
    /// own doc comment for the full validation. See
    /// [`BuiltinFnId::PlaneSurface`]'s own doc comment for the category.
    TrimSurface,
    /// `offset_surface(surface: Surface, distance: Length) -> Surface`
    /// (`AICAD-116`). Exact for `Plane`/`Cylinder`/`Cone`/`Sphere`/`Torus`
    /// (`cad_geometry_api::surface::AnalyticSurface::offset`); every other
    /// family reports `SurfaceOperationError::UnsupportedFamily` (exact
    /// offsetting of a Bezier/B-spline/trimmed surface is not, in general,
    /// expressible in the same family — see that error variant's own doc
    /// comment). See [`BuiltinFnId::PlaneSurface`]'s own doc comment for the
    /// category.
    OffsetSurface,
    /// `intersect_curves(a: Curve, b: Curve, tolerance: Length) ->
    /// List<CurveIntersectionResult>` (`AICAD-117`,
    /// `cad_geometry_api::intersect_curves`). Every transversal crossing
    /// point, in a deterministic order — genuinely zero is a real answer,
    /// never a failure; a coincident/overlapping pair (or a tangency with
    /// no well-formed finite point) is a structured `RuntimeError`, never a
    /// silently empty or arbitrary result. See [`BuiltinFnId::LineCurve`]'s
    /// own doc comment for the category.
    IntersectCurves,
    /// `intersect_curve_surface(curve: Curve, surface: Surface, tolerance:
    /// Length) -> List<CurveSurfaceIntersectionResult>` (`AICAD-117`,
    /// `cad_geometry_api::intersect_curve_surface`). Mirrors
    /// [`BuiltinFnId::IntersectCurves`]'s own cardinality/failure
    /// convention for the curve/surface case.
    IntersectCurveSurface,
    /// `intersect_surfaces(a: Surface, b: Surface, tolerance: Length) ->
    /// List<Curve>` (`AICAD-117`, `cad_geometry_api::intersect_surfaces`).
    /// Exact only for `Plane`-`Plane`/`Plane`-`Sphere`/`Sphere`-`Sphere`
    /// (every other pair's intersection curve is not representable by
    /// `Curve`'s own closed family set — a structural, not merely
    /// narrow-effort, limit, reported `RuntimeError::Unsupported`); `0` or
    /// `1` list elements (never more — every supported pair's intersection
    /// is a single connected curve). See [`BuiltinFnId::LineCurve`]'s own
    /// doc comment for the category.
    IntersectSurfaces,
    /// `project_point_to_surface(surface: Surface, point: Point3) ->
    /// List<SurfaceProjectionResult>` (`AICAD-117`,
    /// `cad_geometry_api::surface::AnalyticSurface::project_point`). The
    /// surface-family counterpart of [`BuiltinFnId::ClosestPointOnCurve`] —
    /// every local-minimum-distance point, never an arbitrary single one.
    ProjectPointToSurface,
    /// `distance_curve_curve(a: Curve, b: Curve) -> List<DistanceResult>`
    /// (`AICAD-117`, `cad_geometry_api::distance_curve_curve`). The
    /// achieved minimum distance and a witness point on each curve; more
    /// than one element only when genuinely tied (e.g. two parallel skew
    /// lines). See [`BuiltinFnId::LineCurve`]'s own doc comment for the
    /// category.
    DistanceCurveCurve,
    /// `distance_curve_surface(curve: Curve, surface: Surface) ->
    /// List<DistanceResult>` (`AICAD-117`,
    /// `cad_geometry_api::distance_curve_surface`). Mirrors
    /// [`BuiltinFnId::DistanceCurveCurve`] for the curve/surface case;
    /// `RuntimeError::Unsupported` for a `Surface::Trimmed` or a genuinely
    /// unsupported `Line`-vs-family combination (see
    /// `cad_geometry_api::query`'s own module doc comment).
    DistanceCurveSurface,
    /// `distance_surface_surface(a: Surface, b: Surface) ->
    /// List<DistanceResult>` (`AICAD-117`,
    /// `cad_geometry_api::distance_surface_surface`). Mirrors
    /// [`BuiltinFnId::DistanceCurveCurve`] for the surface/surface case;
    /// `RuntimeError::Unsupported` when both surfaces are unbounded
    /// (`Plane`/`Cylinder`/`Cone`) or either is `Surface::Trimmed`.
    DistanceSurfaceSurface,
    /// `make_vertex(point: Point3) -> Geometry` (`AICAD-119`). Pushes a
    /// `GeometryOp::MakeVertex` node — a [`BuiltinCategory::Construction`]
    /// builtin, the base case of the vertex->edge->wire->face->shell->
    /// solid pipeline this batch completes.
    MakeVertex,
    /// `make_edge(curve: Curve) -> Geometry` (`AICAD-119`). Materializes an
    /// already-constructed `Curve` value into real kernel topology by
    /// pushing the matching existing `GeometryOp` — `Line` must already be
    /// `trim_curve`-bounded (an infinite `Line` has no two endpoints to
    /// build an edge from) and maps to `GeometryOp::LineEdge`; `Arc` maps
    /// to `GeometryOp::ArcEdge` (its 3 defining points obtained via
    /// `AnalyticCurve::evaluate`, reusing that already-tested trigonometry
    /// rather than re-deriving it here); a full `Circle` maps to
    /// `GeometryOp::CircleWire`, producing a closed *wire* rather than an
    /// open edge — disclosed here, not silently pretended uniform, since a
    /// full circle has no natural single start/end point for OCCT's own
    /// edge model. `Ellipse`/`Bezier`/`BSpline`/any other `Trimmed` base
    /// is `RuntimeError::Unsupported` (no matching kernel construction op
    /// exists yet for those families — a structural limit, not a missing
    /// case check).
    MakeEdge,
    /// `make_wire(edges: List<Geometry>) -> Geometry` (`AICAD-119`).
    /// Pushes `GeometryOp::WireFromEdges` — this exact op has existed since
    /// `AICAD-022`; this is its first source-language exposure, following
    /// [`BuiltinFnId::Plate`]'s own "domain-meaningful name over an
    /// existing op" precedent (here, the op simply had no builtin name
    /// yet at all).
    MakeWire,
    /// `make_face(wire: Geometry) -> Geometry` (`AICAD-119`). Pushes
    /// `GeometryOp::MakeFace` — a *planar* face inferred from `wire`'s own
    /// geometry, no holes, no explicit surface (see
    /// [`BuiltinFnId::MakeFaceOnSurface`] for the holes-and-explicit-
    /// surface-capable general form). Like [`BuiltinFnId::MakeWire`], this
    /// exposes an op that has existed since `AICAD-023` under its first
    /// source-language name.
    MakeFace,
    /// `make_face_on_surface(surface: Surface, outer: Geometry, holes:
    /// List<Geometry>) -> Geometry` (`AICAD-119`). Materializes an
    /// already-constructed `Surface` value into a new
    /// `GeometryOp::MakeFaceOnSurface` node bounded by the already-real
    /// kernel wire `outer` (plus `holes`) — bounded to the same 5
    /// elementary quadric families `cad_geometry_api::ir::SurfaceSpec`
    /// covers (`Plane`/`Cylinder`/`Cone`/`Sphere`/`Torus`);
    /// `RuntimeError::Unsupported` for `Bezier`/`BSpline`/`Trimmed` (no
    /// matching kernel construction op exists yet for those families,
    /// mirroring [`BuiltinFnId::MakeEdge`]'s own identical structural
    /// limit on the curve side). Always builds on `outer`'s forward
    /// orientation — no source-level control over
    /// `cad_geometry_api::ir::FaceOrientation::Reversed` yet (a narrower
    /// scope than the underlying op, not a missing capability at the
    /// kernel layer).
    MakeFaceOnSurface,
    /// `make_shell(faces: List<Geometry>) -> Geometry` (`AICAD-119`).
    /// Pushes `GeometryOp::MakeShell` — a structural container only, no
    /// sewing/gap-closing (`AICAD-120`'s job): faces that do not already
    /// share identical edges produce an open/non-manifold shell under
    /// `is_valid`, not a silently repaired one.
    MakeShell,
    /// `make_solid(shell: Geometry, voids: List<Geometry>) -> Geometry`
    /// (`AICAD-119`). Pushes `GeometryOp::MakeSolid` — `shell` need not be
    /// closed for this call to succeed; see that op's own doc comment for
    /// why construction success here is even less evidence of validity
    /// than usual (`is_valid`/`volume` afterward are the real evidence).
    MakeSolid,
    /// `compound(shapes: List<Geometry>) -> Geometry` (`AICAD-119`). Pushes
    /// `GeometryOp::Compound` — groups any mix of already-built kinds
    /// (vertex/edge/wire/face/shell/solid) with no closure/connectivity
    /// requirement to fail.
    Compound,
}

/// The category/effect metadata `project/DECISION_LOG.md#DL-23` requires
/// the closed `RuntimeBuiltin` catalogue to carry as it scales
/// (`project/DECISION_LOG.md#DL-23`'s own "category/effect metadata"
/// requirement). Every entry is exactly one of:
///
/// - [`BuiltinCategory::Construction`]: builds/extends the caller's
///   `cad_geometry_api::ir::GeometryGraph` (a `GeometryOp` node) and
///   returns a `Geometry` value — pure with respect to the kernel (no
///   kernel call happens until a later dispatch phase materializes the
///   whole graph).
/// - [`BuiltinCategory::Query`]: pushes a `GeometryQuery` node and — per
///   `project/DECISION_LOG.md#DL-25`'s demand-materialization policy —
///   synchronously executes it through `cad_runtime::query_exec::
///   KernelQueryExecutor` during evaluation, returning an ordinary typed
///   AICAD value a program can immediately branch on. This is a real
///   effect (a kernel call happens now, not at some later dispatch phase),
///   which is exactly why this category exists as its own metadata rather
///   than being folded into `Construction`.
/// - [`BuiltinCategory::Value`] (`AICAD-109`): computes an ordinary typed
///   AICAD value directly, with **no** `GeometryGraph`/`GeometryQuery` node
///   and **no** kernel call at all — e.g. analytic curve construction/
///   evaluation, which is closed-form (`cad_geometry_api::curve::
///   AnalyticCurve`'s own doc comment: "pure data assembly, never a kernel
///   call"). Distinct from `Query`: nothing here demand-materializes
///   through a live kernel context, so this category never touches
///   `cad_runtime::query_exec::KernelQueryExecutor` or the query budget —
///   `project/DECISION_LOG.md#DL-23`'s own "category... metadata" is
///   explicitly extensible for exactly this kind of scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinCategory {
    Construction,
    Query,
    Value,
}

impl BuiltinFnId {
    /// This builtin's category — see [`BuiltinCategory`]'s own doc
    /// comment. A `match` here (not a lookup table) so the compiler
    /// enforces every [`BuiltinFnId::ALL`] entry has exactly one category,
    /// the same exhaustiveness guarantee `cad_runtime::interp::
    /// Interpreter::dispatch_builtin`'s own match already gives every
    /// entry exactly one dispatch path.
    pub fn category(self) -> BuiltinCategory {
        match self {
            BuiltinFnId::Box
            | BuiltinFnId::Cylinder
            | BuiltinFnId::Transform
            | BuiltinFnId::Union
            | BuiltinFnId::Cut
            | BuiltinFnId::Intersect
            | BuiltinFnId::Fillet
            | BuiltinFnId::Chamfer
            | BuiltinFnId::Plate
            | BuiltinFnId::Extrude
            | BuiltinFnId::Revolve
            | BuiltinFnId::Hole
            | BuiltinFnId::Pocket
            | BuiltinFnId::Mirror
            | BuiltinFnId::LinearPattern
            | BuiltinFnId::RadialPattern
            | BuiltinFnId::Shell
            | BuiltinFnId::MakeVertex
            | BuiltinFnId::MakeEdge
            | BuiltinFnId::MakeWire
            | BuiltinFnId::MakeFace
            | BuiltinFnId::MakeFaceOnSurface
            | BuiltinFnId::MakeShell
            | BuiltinFnId::MakeSolid
            | BuiltinFnId::Compound => BuiltinCategory::Construction,
            BuiltinFnId::IsValid | BuiltinFnId::Volume | BuiltinFnId::Area => {
                BuiltinCategory::Query
            }
            BuiltinFnId::LineCurve
            | BuiltinFnId::CircleCurve
            | BuiltinFnId::ArcCurve
            | BuiltinFnId::EllipseCurve
            | BuiltinFnId::EvaluateCurve
            | BuiltinFnId::BezierCurve
            | BuiltinFnId::BSplineCurve
            | BuiltinFnId::TrimCurve
            | BuiltinFnId::OffsetCurve
            | BuiltinFnId::ClosestPointOnCurve
            | BuiltinFnId::InterpolateCurve
            | BuiltinFnId::PlaneSurface
            | BuiltinFnId::CylinderSurface
            | BuiltinFnId::ConeSurface
            | BuiltinFnId::SphereSurface
            | BuiltinFnId::TorusSurface
            | BuiltinFnId::EvaluateSurface
            | BuiltinFnId::BezierSurface
            | BuiltinFnId::BSplineSurface
            | BuiltinFnId::TrimSurface
            | BuiltinFnId::OffsetSurface
            | BuiltinFnId::IntersectCurves
            | BuiltinFnId::IntersectCurveSurface
            | BuiltinFnId::IntersectSurfaces
            | BuiltinFnId::ProjectPointToSurface
            | BuiltinFnId::DistanceCurveCurve
            | BuiltinFnId::DistanceCurveSurface
            | BuiltinFnId::DistanceSurfaceSurface => BuiltinCategory::Value,
        }
    }
}

// --- The standard type environment (`AICAD-076A`, `project/DECISION_LOG.md#DL-21`) ---
//
// `AICAD-075A` built `cad_hir::geometry_types::{Axis3, Frame3, Plane}` and
// `cad_runtime::spatial::{axis3_from_value, frame3_from_value,
// plane3_from_value}` specifically so a builtin like `revolve`/`hole`/
// `pocket` could take a real `Axis3`/`Frame3` *value* — the design
// `revolve`/`hole`/`pocket` use above. `AICAD-076` first attempted this and
// found a genuine, repository-wide architecture gap: `crate::lower::
// Lowerer::seed_builtins` seeds *every* `BuiltinFnId` into *every* compiled
// program's global scope unconditionally, and `cad_hir::typeck::Checker::
// collect_signatures` eagerly resolves every seeded function's own
// parameter types up front — including a function's own type that is never
// actually called. Before `AICAD-076A`, a `HirTypeRef::Named` reference to
// a `cad_hir::geometry_types` struct that was not itself in scope (true for
// almost every existing program, since `with_geometry_types` composition
// was optional) failed eagerly with `UNKNOWN_TYPE_NAME` for *that other*
// program — confirmed empirically: `AICAD-076`'s own first draft broke 149
// previously-passing `cad-hir` tests unrelated to Stage-3 modeling.
//
// `project/OWNER_DECISIONS.md#D20`/`project/DECISION_LOG.md#DL-21` resolved
// this: the always-seeded builtin environment must be type-closed, so
// `crate::lower::Lowerer::seed_standard_types` (called by `crate::lower::
// lower_program` unconditionally, alongside `seed_builtins`) now seeds
// `cad_hir::geometry_types::GEOMETRY_TYPES_SOURCE`'s struct declarations
// into every compiled program's global scope too — `Axis3`/`Frame3`/
// `Point3`/`Plane` now always resolve, with no caller composition required.
// `AICAD-076`'s original scalar-decomposed `revolve`/`hole`/`pocket`
// signatures were an approved *temporary* compatibility workaround, not the
// long-term pattern; `AICAD-076A` migrated them back to the real `Axis3`/
// `Frame3`-typed signatures above, which is what should be used from here
// on for any new struct-typed builtin (`AICAD-077`'s own planned
// `mirror(target, plane: Plane)` included) — `extrude`'s `direction:
// Vector3<Float>` was never affected by any of this (a `HirTypeRef::Generic`
// reference to a user-defined generic struct resolves to `None` silently
// when unresolved, not eagerly, per `Checker::resolve_generic_type_
// application`'s own `?`-early-return — the asymmetry that made the
// original gap possible in the first place).

impl BuiltinFnId {
    /// Every catalogue entry, in a fixed, stable order (declaration order
    /// above) — used both by `crate::lower::Lowerer::seed_builtins` (to
    /// seed bindings) and by this module's own tests.
    pub const ALL: [BuiltinFnId; 56] = [
        BuiltinFnId::Box,
        BuiltinFnId::Cylinder,
        BuiltinFnId::Transform,
        BuiltinFnId::Union,
        BuiltinFnId::Cut,
        BuiltinFnId::Intersect,
        BuiltinFnId::Fillet,
        BuiltinFnId::Chamfer,
        BuiltinFnId::Plate,
        BuiltinFnId::Extrude,
        BuiltinFnId::Revolve,
        BuiltinFnId::Hole,
        BuiltinFnId::Pocket,
        BuiltinFnId::Mirror,
        BuiltinFnId::LinearPattern,
        BuiltinFnId::RadialPattern,
        BuiltinFnId::Shell,
        BuiltinFnId::IsValid,
        BuiltinFnId::Volume,
        BuiltinFnId::Area,
        BuiltinFnId::LineCurve,
        BuiltinFnId::CircleCurve,
        BuiltinFnId::ArcCurve,
        BuiltinFnId::EllipseCurve,
        BuiltinFnId::EvaluateCurve,
        BuiltinFnId::BezierCurve,
        BuiltinFnId::BSplineCurve,
        BuiltinFnId::TrimCurve,
        BuiltinFnId::OffsetCurve,
        BuiltinFnId::ClosestPointOnCurve,
        BuiltinFnId::InterpolateCurve,
        BuiltinFnId::PlaneSurface,
        BuiltinFnId::CylinderSurface,
        BuiltinFnId::ConeSurface,
        BuiltinFnId::SphereSurface,
        BuiltinFnId::TorusSurface,
        BuiltinFnId::EvaluateSurface,
        BuiltinFnId::BezierSurface,
        BuiltinFnId::BSplineSurface,
        BuiltinFnId::TrimSurface,
        BuiltinFnId::OffsetSurface,
        BuiltinFnId::IntersectCurves,
        BuiltinFnId::IntersectCurveSurface,
        BuiltinFnId::IntersectSurfaces,
        BuiltinFnId::ProjectPointToSurface,
        BuiltinFnId::DistanceCurveCurve,
        BuiltinFnId::DistanceCurveSurface,
        BuiltinFnId::DistanceSurfaceSurface,
        BuiltinFnId::MakeVertex,
        BuiltinFnId::MakeEdge,
        BuiltinFnId::MakeWire,
        BuiltinFnId::MakeFace,
        BuiltinFnId::MakeFaceOnSurface,
        BuiltinFnId::MakeShell,
        BuiltinFnId::MakeSolid,
        BuiltinFnId::Compound,
    ];
}

fn named(name: &str) -> HirTypeRef {
    HirTypeRef::Named {
        name: name.to_string(),
        span: Span::new(0, 0),
    }
}

fn list_of(elem: &str) -> HirTypeRef {
    list_of_ref(named(elem))
}

/// `List<elem>` for an already-built `elem` type reference (`AICAD-114`) —
/// [`list_of`]'s own general form, needed for a nested `List<List<Point3>>`
/// control-net/weight-grid parameter (`bezier_surface`/`bspline_surface`),
/// which `list_of`'s `&str`-only signature cannot express.
fn list_of_ref(elem: HirTypeRef) -> HirTypeRef {
    HirTypeRef::Generic {
        name: "List".to_string(),
        args: vec![elem],
        span: Span::new(0, 0),
    }
}

/// `Vector3<elem>` (`AICAD-076`) — `cad_hir::geometry_types`'s own
/// generic `Vector3<T>` struct, referenced here exactly as `list_of`
/// already references `List<T>`. Only resolvable when the compiling
/// program has `cad_hir::geometry_types::with_geometry_types` applied
/// (see that module's own doc comment) — [`BuiltinFnId::Extrude`]/
/// [`BuiltinFnId::Revolve`]/[`BuiltinFnId::Hole`]/[`BuiltinFnId::Pocket`]
/// are the first catalogue entries with this requirement.
fn vector3_of(elem: &str) -> HirTypeRef {
    HirTypeRef::Generic {
        name: "Vector3".to_string(),
        args: vec![named(elem)],
        span: Span::new(0, 0),
    }
}

/// `Vector3<Float>` — the concrete instantiation every existing struct-
/// typed builtin parameter already uses for a plain direction/vector
/// (`extrude`/`revolve`/`hole`/... above), reused unchanged for the new
/// `AICAD-109` curve builtins.
fn direction3() -> HirTypeRef {
    vector3_of("Float")
}

/// One catalogue entry: a runtime-backed function's name and signature,
/// expressed in exactly the same syntactic `HirTypeRef` vocabulary an
/// ordinary parsed `fn` declaration would carry (`"Length"` ->
/// `HirTypeRef::Named`, `"List<Int>"` -> `HirTypeRef::Generic`, ...) — so
/// `cad_hir::typeck::Checker::resolve_type_ref` resolves every parameter/
/// return type exactly as it would for user-written source, with no
/// separate builtin-specific type-resolution path.
pub struct BuiltinFnSpec {
    pub id: BuiltinFnId,
    pub name: &'static str,
    /// Parameter name/type pairs, in declaration order.
    pub params: Vec<(&'static str, HirTypeRef)>,
    pub return_ty: HirTypeRef,
}

/// The authoritative Stage-2 Safe CAD signature table — see module doc
/// comment. Mirrors `docs/spec/safe-cad-api.md` exactly; that document is
/// the human-readable rendering of this same data, not an independent
/// second source of truth.
pub fn catalogue() -> Vec<BuiltinFnSpec> {
    vec![
        BuiltinFnSpec {
            id: BuiltinFnId::Box,
            name: "box",
            params: vec![
                ("dx", named("Length")),
                ("dy", named("Length")),
                ("dz", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Cylinder,
            name: "cylinder",
            params: vec![("radius", named("Length")), ("height", named("Length"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Transform,
            name: "transform",
            params: vec![
                ("target", named("Geometry")),
                ("dx", named("Length")),
                ("dy", named("Length")),
                ("dz", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Union,
            name: "union",
            params: vec![("a", named("Geometry")), ("b", named("Geometry"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Cut,
            name: "cut",
            params: vec![("a", named("Geometry")), ("b", named("Geometry"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Intersect,
            name: "intersect",
            params: vec![("a", named("Geometry")), ("b", named("Geometry"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Fillet,
            name: "fillet",
            params: vec![
                ("target", named("Geometry")),
                ("edges", list_of("Int")),
                ("radius", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Chamfer,
            name: "chamfer",
            params: vec![
                ("target", named("Geometry")),
                ("edges", list_of("Int")),
                ("distance", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Plate,
            name: "plate",
            params: vec![
                ("width", named("Length")),
                ("depth", named("Length")),
                ("thickness", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Extrude,
            name: "extrude",
            params: vec![
                ("target", named("Geometry")),
                ("face", named("Int")),
                ("direction", vector3_of("Float")),
                ("distance", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Revolve,
            name: "revolve",
            params: vec![
                ("target", named("Geometry")),
                ("face", named("Int")),
                ("axis", named("Axis3")),
                ("angle", named("Angle")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Hole,
            name: "hole",
            params: vec![
                ("target", named("Geometry")),
                ("axis", named("Axis3")),
                ("diameter", named("Length")),
                ("depth", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Pocket,
            name: "pocket",
            params: vec![
                ("target", named("Geometry")),
                ("frame", named("Frame3")),
                ("width", named("Length")),
                ("length", named("Length")),
                ("depth", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Mirror,
            name: "mirror",
            params: vec![("target", named("Geometry")), ("plane", named("Plane"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::LinearPattern,
            name: "linear_pattern",
            params: vec![
                ("target", named("Geometry")),
                ("direction", vector3_of("Float")),
                ("count", named("Int")),
                ("spacing", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::RadialPattern,
            name: "radial_pattern",
            params: vec![
                ("target", named("Geometry")),
                ("axis", named("Axis3")),
                ("count", named("Int")),
                ("angle", named("Angle")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Shell,
            name: "shell",
            params: vec![
                ("target", named("Geometry")),
                ("removed_faces", list_of("Int")),
                ("thickness", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::IsValid,
            name: "is_valid",
            params: vec![("target", named("Geometry"))],
            return_ty: named("Bool"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Volume,
            name: "volume",
            params: vec![("target", named("Geometry"))],
            return_ty: named("Volume"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Area,
            name: "area",
            params: vec![("target", named("Geometry"))],
            return_ty: named("Area"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::LineCurve,
            name: "line_curve",
            params: vec![("origin", named("Point3")), ("direction", direction3())],
            return_ty: named("Curve"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::CircleCurve,
            name: "circle_curve",
            params: vec![
                ("center", named("Point3")),
                ("normal", direction3()),
                ("radius", named("Length")),
            ],
            return_ty: named("Curve"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::ArcCurve,
            name: "arc_curve",
            params: vec![
                ("center", named("Point3")),
                ("normal", direction3()),
                ("radius", named("Length")),
                ("start_angle", named("Angle")),
                ("end_angle", named("Angle")),
            ],
            return_ty: named("Curve"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::EllipseCurve,
            name: "ellipse_curve",
            params: vec![
                ("center", named("Point3")),
                ("normal", direction3()),
                ("major_direction", direction3()),
                ("major_radius", named("Length")),
                ("minor_radius", named("Length")),
            ],
            return_ty: named("Curve"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::EvaluateCurve,
            name: "evaluate_curve",
            params: vec![("curve", named("Curve")), ("u", named("Float"))],
            return_ty: named("CurveEvaluation"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::BezierCurve,
            name: "bezier_curve",
            params: vec![
                ("control_points", list_of("Point3")),
                ("weights", list_of("Float")),
            ],
            return_ty: named("Curve"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::BSplineCurve,
            name: "bspline_curve",
            params: vec![
                ("degree", named("Int")),
                ("control_points", list_of("Point3")),
                ("knots", list_of("Float")),
                ("multiplicities", list_of("Int")),
                ("weights", list_of("Float")),
                ("periodic", named("Bool")),
            ],
            return_ty: named("Curve"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::TrimCurve,
            name: "trim_curve",
            params: vec![
                ("curve", named("Curve")),
                ("u0", named("Float")),
                ("u1", named("Float")),
            ],
            return_ty: named("Curve"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::OffsetCurve,
            name: "offset_curve",
            params: vec![
                ("curve", named("Curve")),
                ("distance", named("Length")),
                ("normal", direction3()),
            ],
            return_ty: named("Curve"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::ClosestPointOnCurve,
            name: "closest_point_on_curve",
            params: vec![("curve", named("Curve")), ("point", named("Point3"))],
            return_ty: list_of("ClosestPointResult"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::InterpolateCurve,
            name: "interpolate_curve",
            params: vec![
                ("points", list_of("Point3")),
                ("tolerance", named("Length")),
            ],
            return_ty: named("Curve"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::PlaneSurface,
            name: "plane_surface",
            params: vec![("origin", named("Point3")), ("normal", direction3())],
            return_ty: named("Surface"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::CylinderSurface,
            name: "cylinder_surface",
            params: vec![("axis", named("Axis3")), ("radius", named("Length"))],
            return_ty: named("Surface"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::ConeSurface,
            name: "cone_surface",
            params: vec![("axis", named("Axis3")), ("half_angle", named("Angle"))],
            return_ty: named("Surface"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::SphereSurface,
            name: "sphere_surface",
            params: vec![("center", named("Point3")), ("radius", named("Length"))],
            return_ty: named("Surface"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::TorusSurface,
            name: "torus_surface",
            params: vec![
                ("axis", named("Axis3")),
                ("major_radius", named("Length")),
                ("minor_radius", named("Length")),
            ],
            return_ty: named("Surface"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::EvaluateSurface,
            name: "evaluate_surface",
            params: vec![
                ("surface", named("Surface")),
                ("u", named("Float")),
                ("v", named("Float")),
            ],
            return_ty: named("SurfaceEvaluation"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::BezierSurface,
            name: "bezier_surface",
            params: vec![
                ("control_points", list_of_ref(list_of("Point3"))),
                ("weights", list_of_ref(list_of("Float"))),
            ],
            return_ty: named("Surface"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::BSplineSurface,
            name: "bspline_surface",
            params: vec![
                ("degree_u", named("Int")),
                ("degree_v", named("Int")),
                ("control_points", list_of_ref(list_of("Point3"))),
                ("knots_u", list_of("Float")),
                ("multiplicities_u", list_of("Int")),
                ("knots_v", list_of("Float")),
                ("multiplicities_v", list_of("Int")),
                ("weights", list_of_ref(list_of("Float"))),
                ("periodic_u", named("Bool")),
                ("periodic_v", named("Bool")),
            ],
            return_ty: named("Surface"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::TrimSurface,
            name: "trim_surface",
            params: vec![
                ("base", named("Surface")),
                ("outer", named("Curve")),
                ("holes", list_of("Curve")),
                ("tolerance", named("Length")),
            ],
            return_ty: named("Surface"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::OffsetSurface,
            name: "offset_surface",
            params: vec![("surface", named("Surface")), ("distance", named("Length"))],
            return_ty: named("Surface"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::IntersectCurves,
            name: "intersect_curves",
            params: vec![
                ("a", named("Curve")),
                ("b", named("Curve")),
                ("tolerance", named("Length")),
            ],
            return_ty: list_of("CurveIntersectionResult"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::IntersectCurveSurface,
            name: "intersect_curve_surface",
            params: vec![
                ("curve", named("Curve")),
                ("surface", named("Surface")),
                ("tolerance", named("Length")),
            ],
            return_ty: list_of("CurveSurfaceIntersectionResult"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::IntersectSurfaces,
            name: "intersect_surfaces",
            params: vec![
                ("a", named("Surface")),
                ("b", named("Surface")),
                ("tolerance", named("Length")),
            ],
            return_ty: list_of("Curve"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::ProjectPointToSurface,
            name: "project_point_to_surface",
            params: vec![("surface", named("Surface")), ("point", named("Point3"))],
            return_ty: list_of("SurfaceProjectionResult"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::DistanceCurveCurve,
            name: "distance_curve_curve",
            params: vec![("a", named("Curve")), ("b", named("Curve"))],
            return_ty: list_of("DistanceResult"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::DistanceCurveSurface,
            name: "distance_curve_surface",
            params: vec![("curve", named("Curve")), ("surface", named("Surface"))],
            return_ty: list_of("DistanceResult"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::DistanceSurfaceSurface,
            name: "distance_surface_surface",
            params: vec![("a", named("Surface")), ("b", named("Surface"))],
            return_ty: list_of("DistanceResult"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::MakeVertex,
            name: "make_vertex",
            params: vec![("point", named("Point3"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::MakeEdge,
            name: "make_edge",
            params: vec![("curve", named("Curve"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::MakeWire,
            name: "make_wire",
            params: vec![("edges", list_of("Geometry"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::MakeFace,
            name: "make_face",
            params: vec![("wire", named("Geometry"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::MakeFaceOnSurface,
            name: "make_face_on_surface",
            params: vec![
                ("surface", named("Surface")),
                ("outer", named("Geometry")),
                ("holes", list_of("Geometry")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::MakeShell,
            name: "make_shell",
            params: vec![("faces", list_of("Geometry"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::MakeSolid,
            name: "make_solid",
            params: vec![("shell", named("Geometry")), ("voids", list_of("Geometry"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Compound,
            name: "compound",
            params: vec![("shapes", list_of("Geometry"))],
            return_ty: named("Geometry"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_has_exactly_one_entry_per_builtin_fn_id() {
        let entries = catalogue();
        assert_eq!(entries.len(), BuiltinFnId::ALL.len());
        for id in BuiltinFnId::ALL {
            assert_eq!(
                entries.iter().filter(|e| e.id == id).count(),
                1,
                "expected exactly one catalogue entry for {id:?}"
            );
        }
    }

    /// `project/DECISION_LOG.md#DL-23`'s own "category/effect metadata"
    /// requirement: every catalogue entry has exactly one
    /// [`BuiltinCategory`], `BuiltinFnId::category` is a total match (a
    /// compile error, not a test failure, if a variant is ever missed —
    /// this test is the *evidence* that guarantee holds, not the guarantee
    /// itself), and a `Construction` builtin always returns `Geometry`
    /// while a `Query` builtin never does — the exact distinction
    /// `cad_feature_graph::graph::FeatureGraph::resolve_geometry_expr`'s
    /// own `is_geometry_type` check already relies on to keep a
    /// scalar-returning query out of the feature DAG.
    #[test]
    fn every_catalogue_entry_has_a_category_consistent_with_its_return_type() {
        for spec in catalogue() {
            let is_geometry_return = matches!(
                &spec.return_ty,
                HirTypeRef::Named { name, .. } if name == "Geometry"
            );
            match spec.id.category() {
                BuiltinCategory::Construction => assert!(
                    is_geometry_return,
                    "'{}' is Construction but does not return Geometry",
                    spec.name
                ),
                BuiltinCategory::Query => assert!(
                    !is_geometry_return,
                    "'{}' is Query but returns Geometry",
                    spec.name
                ),
                BuiltinCategory::Value => assert!(
                    !is_geometry_return,
                    "'{}' is Value but returns Geometry",
                    spec.name
                ),
            }
        }
    }

    #[test]
    fn every_catalogue_name_is_unique() {
        let entries = catalogue();
        let mut names: Vec<&str> = entries.iter().map(|e| e.name).collect();
        names.sort_unstable();
        let mut deduped = names.clone();
        deduped.dedup();
        assert_eq!(names, deduped, "duplicate builtin function name");
    }

    /// `project/DECISION_LOG.md#DL-21`'s own required invariant: the
    /// always-seeded standard type environment must make every
    /// `BuiltinFnId` signature resolvable with zero caller composition.
    /// Proven directly here — a targeted, catalogue-wide check — rather
    /// than left to be discovered by an unrelated test suite exploding,
    /// which is exactly what happened during `AICAD-076`'s own
    /// development (a draft `Axis3`/`Frame3` parameter broke 149
    /// Stage-3-unrelated `cad-hir` tests before this invariant existed as
    /// its own test).
    #[test]
    fn the_entire_builtin_catalogue_type_checks_against_an_otherwise_empty_program() {
        let (program, parse_diagnostics) = cad_parser::parse_program("", "test.aicad");
        assert!(parse_diagnostics.is_empty(), "{parse_diagnostics:?}");
        let lowered = crate::lower::lower_program(&program, "test.aicad", "");
        assert!(
            lowered.diagnostics.is_empty(),
            "empty program + standard builtin catalogue failed to lower cleanly: {:?}",
            lowered.diagnostics
        );
        let checked =
            crate::typeck::check_program(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert!(
            checked.diagnostics.is_empty(),
            "empty program + standard builtin catalogue failed to type-check cleanly \
             -- a BuiltinFnId signature references a nominal type the always-seeded \
             standard environment does not provide: {:?}",
            checked.diagnostics
        );
        // Every catalogue entry actually seeded a real `Fn` binding by its
        // own declared name -- ruling out a vacuous pass where a
        // signature silently resolved every parameter/return type to
        // `None` and therefore produced no diagnostic by accident.
        for spec in catalogue() {
            assert!(
                lowered
                    .bindings
                    .iter()
                    .any(|b| b.name == spec.name && b.kind == crate::ids::BindingKind::Fn),
                "builtin '{}' did not seed an Fn binding",
                spec.name
            );
        }
    }
}
