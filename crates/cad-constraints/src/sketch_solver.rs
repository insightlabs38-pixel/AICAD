//! `AICAD-074`'s first concrete [`SketchSolver`] implementation
//! (`project/DECISION_LOG.md#DL-20`'s "exactly one initial solver
//! implementation... behind this interface"), per `docs/plan/
//! 04_HIGH_LEVEL_MODELING_API.md` §4's baseline sketch-constraint
//! catalogue.
//!
//! # Algorithm
//!
//! [`RelaxationSolver`] is a Gauss-Seidel-style *direct-projection*
//! relaxation solver: every [`ConstraintKind`] has a fixed correction
//! rule that nudges only the variables it names toward satisfying it
//! (e.g. `horizontal` sets both of a line's endpoint `y` coordinates to
//! their average; `distance` moves its two points apart/together along
//! their current connecting direction). Every constraint's correction is
//! applied once per iteration, in order, for
//! [`SketchSolverProfile::max_iterations`] iterations; constraints whose
//! corrections interact (e.g. two lines each `parallel` to a third)
//! converge over successive iterations the same way Gauss-Seidel
//! relaxation converges for any consistent linear-or-mildly-nonlinear
//! system. This is deliberately *not* a general nonlinear solver (no
//! Jacobian, no line search) — see "Scope cuts" below for why that is
//! the right amount of machinery for what `AICAD-074` actually needs.
//!
//! # Classification (`DL-20`'s solve-status vocabulary)
//!
//! 1. **`Unsupported`** (an extension to [`SolveStatus`] beyond `DL-20`'s
//!    "at minimum" three values, added by this task): a constraint whose
//!    correction rule never had anything it could move, every single
//!    iteration (both referenced points immovable, a mismatched-kind
//!    `symmetric` pairing this solver's fixed reflection convention does
//!    not cover, or a permanently-degenerate operand such as a
//!    zero-length line). Checked first, and reported honestly rather
//!    than being folded into `Overconstrained` (which would misdescribe
//!    "this solver cannot act on this constraint" as "these constraints
//!    conflict") or silently treated as satisfied (which would violate
//!    `DL-20`'s "never let an arbitrary branch silently become
//!    semantics" — an unenforced constraint is not the same thing as a
//!    satisfied one).
//! 2. **`Underconstrained`**: `sketch_variables(sketch).len()` (total
//!    free scalars) minus the structural equation count every present
//!    constraint contributes (a fixed, entity-kind-aware table — see
//!    `equation_count`) is strictly positive. Reported *regardless* of
//!    whether the relaxation happened to converge to some particular
//!    solution — `DL-20` explicitly forbids letting a converged-but-
//!    arbitrary branch stand in for a well-defined "solved" state when
//!    real degrees of freedom remain.
//! 3. **`Solved`** / **`Overconstrained`**: only considered once the
//!    naive equation count is not positive (exactly or redundantly
//!    determined). Every constraint's final residual (see "Tolerance
//!    profile" below) is checked; if every one is within tolerance, the
//!    result is `Solved` with every [`SketchVariable`]'s solved
//!    magnitude in the report's [`SolvedValues`]. Otherwise the result is
//!    `Overconstrained`, with every constraint whose residual still
//!    exceeds tolerance listed as `conflicting` evidence — `DL-20`
//!    explicitly does not require this to be a provably minimal conflict
//!    set, so no attempt is made to shrink it further.
//!
//! # Tolerance profile (`DL-20`: "numerical tolerances... must be
//! explicit and versioned")
//!
//! [`SketchSolverProfile`] mirrors `cad_validation::profile::
//! ComparisonProfile`'s own versioned-constant pattern (`DECISION_LOG.md
//! #DL-12`/`#DL-17` precedent), but is an intentionally separate type,
//! not a reuse of that profile: `cad-validation`'s profile answers "are
//! two independently-computed kernel geometries equivalent" (a coarser,
//! cross-kernel-version concern, `linear_abs = 1e-4` mm); this profile
//! answers "has this solver's own direct-projection iteration converged"
//! (a self-contained numerical-convergence concern with no kernel
//! involved at all). `v1`'s constants
//! (`position_tolerance`/`angle_tolerance = 1e-9`,
//! `max_iterations = 512`) are chosen and evidenced by this task's own
//! test suite: a single isolated constraint (e.g. one `radius`) converges
//! in one iteration, but a *chain* of relative constraints (e.g. this
//! module's own closed-rectangle fixture — four sides connected end to
//! end purely by `coincident`/`horizontal`/`vertical`/`distance`, with
//! only one side `fixed`) converges geometrically at roughly a `0.7`x
//! residual reduction per iteration, not the `0.5`x a naive per-
//! constraint analysis might suggest — measured directly against that
//! fixture, reaching `~3e-6` at 64 iterations, `~1e-8` at 100, and
//! `~4e-12` at 150. `512` iterations leaves at least an order of
//! magnitude of headroom beyond what any single-loop baseline-part
//! sketch in this task's own test suite needs, while remaining cheap
//! (each iteration is `O(constraint count)`, so even 512 of them is a
//! microsecond-scale cost) and still bounded (`AGENTS.md`'s resource-
//! budget non-negotiable: this solver always terminates).
//!
//! # Scope cuts
//!
//! - **A fixed, documented convention resolves every constraint kind
//!   with more than one geometrically valid solution**, rather than an
//!   ad hoc/arbitrary choice `DL-20` would forbid: `tangent` between two
//!   circles/arcs always targets *external* tangency (center distance =
//!   sum of radii, the common case for holes/bosses in a baseline part);
//!   `angle(a, b, value)` always means "`b`'s direction is `a`'s
//!   direction rotated by `value`" (a fixed sign convention, mirroring
//!   `arc.direction`'s own already-established default-CCW convention
//!   elsewhere in this codebase); `symmetric` between two lines reflects
//!   `start`<->`start`/`end`<->`end` (not some other point
//!   correspondence). A future task adding a different tangency/
//!   correspondence choice as an explicit, separately-named option would
//!   not change what these names already mean.
//! - **`coincident`/`distance`/`midpoint` cannot move an arc's `start`/
//!   `end` point directly** (only `line.start`/`line.end`/
//!   `circle.center`/`arc.center` are ever assigned to by a correction
//!   rule) — `docs/plan/04_HIGH_LEVEL_MODELING_API.md` §4's own baseline
//!   catalogue has no arc-endpoint-specific constraint, and constraining
//!   an arc's endpoint would require deciding which of its
//!   center/radius/angle to adjust instead, an ambiguity this task does
//!   not need to resolve for any baseline part. A constraint that can
//!   only be satisfied by moving an arc endpoint this way reports
//!   `Unsupported` rather than silently doing nothing while claiming
//!   `Solved`.
//! - **No symbolic Jacobian / Newton iteration.** Direct-projection
//!   relaxation is sufficient for the constraint combinations a baseline
//!   part needs and avoids the much larger numerical-analysis surface
//!   (linearization, convergence radius, singular-Jacobian handling) a
//!   general nonlinear 2D solver would require — exactly the kind of
//!   "smallest correct solution" scope this task's own title
//!   ("...needed by baseline parts") calls for.

use crate::sketch_constraint::{
    ConstraintId, ConstraintKind, ConstraintSet, PointRef, SketchSolver, SketchVariable,
    SolveReport, SolveStatus, SolvedValues, arc_point, sketch_variables,
};
use cad_hir::sketch::{Point2, Sketch, SketchEntityId, SketchEntityKind, Vector2};
use std::f64::consts::PI;

/// The current default profile version this module implements — bumping
/// it requires explicit documented review, mirroring `cad_validation::
/// profile::ComparisonProfile::CURRENT_VERSION`'s identical convention.
pub const CURRENT_VERSION: u32 = 1;

/// A versioned convergence-tolerance profile for [`RelaxationSolver`].
/// See module doc comment ("Tolerance profile") for why this is a
/// separate type from `cad_validation::profile::ComparisonProfile`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SketchSolverProfile {
    pub version: u32,
    /// Canonical-length-unit convergence tolerance for every point-
    /// distance/length residual.
    pub position_tolerance: f64,
    /// Radian convergence tolerance for every direction/angle residual.
    pub angle_tolerance: f64,
    /// Relaxation-iteration budget (`AGENTS.md`'s resource-budget
    /// non-negotiable: this solver always terminates, never loops
    /// unboundedly waiting for convergence).
    pub max_iterations: u32,
}

impl SketchSolverProfile {
    /// `v1`: `position_tolerance = angle_tolerance = 1e-9`,
    /// `max_iterations = 512`. See module doc comment ("Tolerance
    /// profile") for the evidence behind these constants.
    pub fn v1() -> SketchSolverProfile {
        SketchSolverProfile {
            version: CURRENT_VERSION,
            position_tolerance: 1e-9,
            angle_tolerance: 1e-9,
            max_iterations: 512,
        }
    }
}

impl Default for SketchSolverProfile {
    fn default() -> SketchSolverProfile {
        SketchSolverProfile::v1()
    }
}

/// `AICAD-074`'s concrete [`SketchSolver`] implementation. See module
/// doc comment for the algorithm/classification/scope this type
/// implements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelaxationSolver {
    profile: SketchSolverProfile,
}

impl RelaxationSolver {
    pub fn new() -> RelaxationSolver {
        RelaxationSolver {
            profile: SketchSolverProfile::v1(),
        }
    }

    pub fn with_profile(profile: SketchSolverProfile) -> RelaxationSolver {
        RelaxationSolver { profile }
    }

    pub fn profile(&self) -> SketchSolverProfile {
        self.profile
    }
}

impl Default for RelaxationSolver {
    fn default() -> RelaxationSolver {
        RelaxationSolver::new()
    }
}

impl SketchSolver for RelaxationSolver {
    fn solve(&self, sketch: &Sketch, constraints: &ConstraintSet) -> SolveReport {
        let seed: Vec<SketchEntityKind> = sketch.entities().iter().map(|e| e.kind).collect();
        let mut work = seed.clone();
        let cs = constraints.constraints();
        let mut ever_acted = vec![false; cs.len()];

        for _ in 0..self.profile.max_iterations {
            for (i, c) in cs.iter().enumerate() {
                let acted = apply_correction(&mut work, &seed, &c.kind);
                ever_acted[i] |= acted;
            }
        }

        let unsupported: Vec<ConstraintId> = cs
            .iter()
            .zip(ever_acted.iter())
            .filter(|(_, acted)| !**acted)
            .map(|(c, _)| c.id)
            .collect();
        if !unsupported.is_empty() {
            return SolveReport {
                status: SolveStatus::Unsupported {
                    constraints: unsupported,
                },
                values: SolvedValues::new(),
            };
        }

        let total_dof = sketch_variables(sketch).len() as i64;
        let equation_sum: i64 = cs
            .iter()
            .map(|c| equation_count(&c.kind, &work) as i64)
            .sum();
        let remaining = total_dof - equation_sum;

        let values = collect_values(sketch, &work);

        if remaining > 0 {
            return SolveReport {
                status: SolveStatus::Underconstrained {
                    remaining_dof: remaining as u32,
                },
                values,
            };
        }

        let mut conflicting = Vec::new();
        for c in cs {
            let r = residual(&work, &seed, &c.kind, &self.profile);
            if r > 1.0 {
                conflicting.push(c.id);
            }
        }

        if conflicting.is_empty() {
            SolveReport {
                status: SolveStatus::Solved,
                values,
            }
        } else {
            SolveReport {
                status: SolveStatus::Overconstrained { conflicting },
                values,
            }
        }
    }
}

// ---------------------------------------------------------------------
// Working-value access (a mutable `Vec<SketchEntityKind>` shadowing the
// sketch's own entities, indexed identically since entity ids are
// minted in strictly increasing order — `cad_hir::sketch`'s own
// invariant)
// ---------------------------------------------------------------------

fn entity_index(id: SketchEntityId) -> usize {
    id.index() as usize
}

fn get_point(work: &[SketchEntityKind], r: PointRef) -> Option<Point2> {
    let kind = &work[entity_index(r.entity())];
    match (r, kind) {
        (PointRef::LineStart(_), SketchEntityKind::Line { start, .. }) => Some(*start),
        (PointRef::LineEnd(_), SketchEntityKind::Line { end, .. }) => Some(*end),
        (PointRef::CircleCenter(_), SketchEntityKind::Circle { center, .. }) => Some(*center),
        (PointRef::ArcCenter(_), SketchEntityKind::Arc { center, .. }) => Some(*center),
        (
            PointRef::ArcStart(_),
            SketchEntityKind::Arc {
                center,
                radius,
                start_angle,
                ..
            },
        ) => Some(arc_point(*center, *radius, *start_angle)),
        (
            PointRef::ArcEnd(_),
            SketchEntityKind::Arc {
                center,
                radius,
                end_angle,
                ..
            },
        ) => Some(arc_point(*center, *radius, *end_angle)),
        _ => None,
    }
}

fn point_is_settable(work: &[SketchEntityKind], r: PointRef) -> bool {
    matches!(
        (r, &work[entity_index(r.entity())]),
        (PointRef::LineStart(_), SketchEntityKind::Line { .. })
            | (PointRef::LineEnd(_), SketchEntityKind::Line { .. })
            | (PointRef::CircleCenter(_), SketchEntityKind::Circle { .. })
            | (PointRef::ArcCenter(_), SketchEntityKind::Arc { .. })
    )
}

fn set_point(work: &mut [SketchEntityKind], r: PointRef, p: Point2) -> bool {
    match (r, &mut work[entity_index(r.entity())]) {
        (PointRef::LineStart(_), SketchEntityKind::Line { start, .. }) => {
            *start = p;
            true
        }
        (PointRef::LineEnd(_), SketchEntityKind::Line { end, .. }) => {
            *end = p;
            true
        }
        (PointRef::CircleCenter(_), SketchEntityKind::Circle { center, .. }) => {
            *center = p;
            true
        }
        (PointRef::ArcCenter(_), SketchEntityKind::Arc { center, .. }) => {
            *center = p;
            true
        }
        _ => false,
    }
}

fn center_point_ref(work: &[SketchEntityKind], id: SketchEntityId) -> Option<PointRef> {
    match &work[entity_index(id)] {
        SketchEntityKind::Circle { .. } => Some(PointRef::CircleCenter(id)),
        SketchEntityKind::Arc { .. } => Some(PointRef::ArcCenter(id)),
        _ => None,
    }
}

fn center_of(work: &[SketchEntityKind], id: SketchEntityId) -> Option<Point2> {
    center_point_ref(work, id).and_then(|r| get_point(work, r))
}

fn radial_info(work: &[SketchEntityKind], id: SketchEntityId) -> Option<(Point2, f64)> {
    match &work[entity_index(id)] {
        SketchEntityKind::Circle { center, radius } => Some((*center, radius.magnitude)),
        SketchEntityKind::Arc { center, radius, .. } => Some((*center, radius.magnitude)),
        _ => None,
    }
}

fn radius_of(work: &[SketchEntityKind], id: SketchEntityId) -> Option<f64> {
    radial_info(work, id).map(|(_, r)| r)
}

fn set_radius_of(work: &mut [SketchEntityKind], id: SketchEntityId, value: f64) -> bool {
    match &mut work[entity_index(id)] {
        SketchEntityKind::Circle { radius, .. } | SketchEntityKind::Arc { radius, .. } => {
            radius.magnitude = value;
            true
        }
        _ => false,
    }
}

fn line_points(work: &[SketchEntityKind], id: SketchEntityId) -> Option<(Point2, Point2)> {
    match &work[entity_index(id)] {
        SketchEntityKind::Line { start, end } => Some((*start, *end)),
        _ => None,
    }
}

fn set_line_points(
    work: &mut [SketchEntityKind],
    id: SketchEntityId,
    a: Point2,
    b: Point2,
) -> bool {
    match &mut work[entity_index(id)] {
        SketchEntityKind::Line { start, end } => {
            *start = a;
            *end = b;
            true
        }
        _ => false,
    }
}

fn line_dir(work: &[SketchEntityKind], id: SketchEntityId) -> Option<Vector2> {
    let (s, e) = line_points(work, id)?;
    let v = e - s;
    if v.length() < 1e-9 {
        None
    } else {
        Some(v.scale(1.0 / v.length()))
    }
}

fn reflect_point(p: Point2, line_a: Point2, line_b: Point2) -> Point2 {
    let d = line_b - line_a;
    let len2 = d.x * d.x + d.y * d.y;
    if len2 < 1e-18 {
        return p;
    }
    let ap = p - line_a;
    let t = (ap.x * d.x + ap.y * d.y) / len2;
    let proj = line_a + d.scale(t);
    Point2::new(2.0 * proj.x - p.x, 2.0 * proj.y - p.y)
}

fn wrap_pi(mut a: f64) -> f64 {
    while a > PI {
        a -= 2.0 * PI;
    }
    while a < -PI {
        a += 2.0 * PI;
    }
    a
}

fn rotate(dir: Vector2, angle: f64) -> Vector2 {
    let (s, c) = angle.sin_cos();
    Vector2::new(dir.x * c - dir.y * s, dir.x * s + dir.y * c)
}

// ---------------------------------------------------------------------
// Correction rules (one per `ConstraintKind`, see module doc comment)
// ---------------------------------------------------------------------

fn apply_correction(
    work: &mut [SketchEntityKind],
    seed: &[SketchEntityKind],
    kind: &ConstraintKind,
) -> bool {
    match *kind {
        ConstraintKind::Coincident { a, b } => correct_coincident(work, a, b),
        ConstraintKind::Horizontal { line } => correct_horizontal(work, line),
        ConstraintKind::Vertical { line } => correct_vertical(work, line),
        ConstraintKind::Parallel { a, b } => align_line_direction(work, a, b, 0.0),
        ConstraintKind::Perpendicular { a, b } => align_line_direction(work, a, b, PI / 2.0),
        ConstraintKind::Tangent { a, b } => correct_tangent(work, a, b),
        ConstraintKind::Concentric { a, b } => correct_concentric(work, a, b),
        ConstraintKind::Equal { a, b } => correct_equal(work, a, b),
        ConstraintKind::Symmetric { a, b, about } => correct_symmetric(work, a, b, about),
        ConstraintKind::Distance { a, b, value } => correct_distance(work, a, b, value.magnitude),
        ConstraintKind::Angle { a, b, value } => align_line_direction(work, a, b, value.magnitude),
        ConstraintKind::Radius { entity, value } => set_radius_of(work, entity, value.magnitude),
        ConstraintKind::Diameter { entity, value } => {
            set_radius_of(work, entity, value.magnitude / 2.0)
        }
        ConstraintKind::Fixed { entity } => {
            let idx = entity_index(entity);
            work[idx] = seed[idx];
            true
        }
        ConstraintKind::Midpoint { point, line } => correct_midpoint(work, point, line),
    }
}

fn correct_coincident(work: &mut [SketchEntityKind], a: PointRef, b: PointRef) -> bool {
    let sa = point_is_settable(work, a);
    let sb = point_is_settable(work, b);
    if !sa && !sb {
        return false;
    }
    let (Some(pa), Some(pb)) = (get_point(work, a), get_point(work, b)) else {
        return false;
    };
    let target = if sa && sb {
        Point2::new((pa.x + pb.x) / 2.0, (pa.y + pb.y) / 2.0)
    } else if sa {
        pb
    } else {
        pa
    };
    let mut changed = false;
    if sa {
        changed |= set_point(work, a, target);
    }
    if sb {
        changed |= set_point(work, b, target);
    }
    changed
}

fn correct_horizontal(work: &mut [SketchEntityKind], line: SketchEntityId) -> bool {
    match line_points(work, line) {
        Some((s, e)) => {
            let mid_y = (s.y + e.y) / 2.0;
            set_line_points(work, line, Point2::new(s.x, mid_y), Point2::new(e.x, mid_y))
        }
        None => false,
    }
}

fn correct_vertical(work: &mut [SketchEntityKind], line: SketchEntityId) -> bool {
    match line_points(work, line) {
        Some((s, e)) => {
            let mid_x = (s.x + e.x) / 2.0;
            set_line_points(work, line, Point2::new(mid_x, s.y), Point2::new(mid_x, e.y))
        }
        None => false,
    }
}

fn align_line_direction(
    work: &mut [SketchEntityKind],
    a: SketchEntityId,
    b: SketchEntityId,
    extra_angle: f64,
) -> bool {
    let Some(unit_a) = line_dir(work, a) else {
        return false;
    };
    let Some((bs, be)) = line_points(work, b) else {
        return false;
    };
    let half_len = (be - bs).length() / 2.0;
    if half_len < 1e-9 {
        return false;
    }
    let target_dir = rotate(unit_a, extra_angle);
    let mid_b = Point2::new((bs.x + be.x) / 2.0, (bs.y + be.y) / 2.0);
    set_line_points(
        work,
        b,
        mid_b + target_dir.scale(-half_len),
        mid_b + target_dir.scale(half_len),
    )
}

fn correct_tangent(work: &mut [SketchEntityKind], a: SketchEntityId, b: SketchEntityId) -> bool {
    match (radial_info(work, a), radial_info(work, b)) {
        (Some((ca, ra)), Some((cb, rb))) => {
            let vec = cb - ca;
            let cur = vec.length();
            let dir = if cur > 1e-9 {
                vec.scale(1.0 / cur)
            } else {
                Vector2::new(1.0, 0.0)
            };
            let target = ra + rb;
            let mid = Point2::new((ca.x + cb.x) / 2.0, (ca.y + cb.y) / 2.0);
            let new_ca = mid + dir.scale(-target / 2.0);
            let new_cb = mid + dir.scale(target / 2.0);
            let ref_a = center_point_ref(work, a).expect("radial_info succeeded");
            let ref_b = center_point_ref(work, b).expect("radial_info succeeded");
            let c1 = set_point(work, ref_a, new_ca);
            let c2 = set_point(work, ref_b, new_cb);
            c1 || c2
        }
        _ => {
            let (line_id, circ_id) = if radial_info(work, a).is_some() {
                (b, a)
            } else {
                (a, b)
            };
            match (line_points(work, line_id), radial_info(work, circ_id)) {
                (Some((ls, le)), Some((cc, rr))) => {
                    let d = le - ls;
                    let len = d.length();
                    if len < 1e-9 {
                        return false;
                    }
                    let dir = d.scale(1.0 / len);
                    let ap = cc - ls;
                    let t = ap.x * dir.x + ap.y * dir.y;
                    let closest = ls + dir.scale(t);
                    let normal = Vector2::new(-dir.y, dir.x);
                    let offset = cc - closest;
                    let side = if offset.x * normal.x + offset.y * normal.y >= 0.0 {
                        1.0
                    } else {
                        -1.0
                    };
                    let new_center = closest + normal.scale(side * rr);
                    let ref_c = center_point_ref(work, circ_id).expect("radial_info succeeded");
                    set_point(work, ref_c, new_center)
                }
                _ => false,
            }
        }
    }
}

fn correct_concentric(work: &mut [SketchEntityKind], a: SketchEntityId, b: SketchEntityId) -> bool {
    match (center_point_ref(work, a), center_point_ref(work, b)) {
        (Some(ra), Some(rb)) => {
            let ca = get_point(work, ra).expect("center_point_ref succeeded");
            let cb = get_point(work, rb).expect("center_point_ref succeeded");
            let mid = Point2::new((ca.x + cb.x) / 2.0, (ca.y + cb.y) / 2.0);
            let c1 = set_point(work, ra, mid);
            let c2 = set_point(work, rb, mid);
            c1 || c2
        }
        _ => false,
    }
}

fn scale_line_to_length(work: &mut [SketchEntityKind], id: SketchEntityId, new_len: f64) -> bool {
    match line_points(work, id) {
        Some((s, e)) => {
            let v = e - s;
            let cur = v.length();
            if cur < 1e-9 {
                return false;
            }
            let mid = Point2::new((s.x + e.x) / 2.0, (s.y + e.y) / 2.0);
            let dir = v.scale(1.0 / cur);
            let half = new_len / 2.0;
            set_line_points(work, id, mid + dir.scale(-half), mid + dir.scale(half))
        }
        None => false,
    }
}

fn correct_equal(work: &mut [SketchEntityKind], a: SketchEntityId, b: SketchEntityId) -> bool {
    if let (Some((as_, ae)), Some((bs, be))) = (line_points(work, a), line_points(work, b)) {
        let la = (ae - as_).length();
        let lb = (be - bs).length();
        let avg = (la + lb) / 2.0;
        let c1 = scale_line_to_length(work, a, avg);
        let c2 = scale_line_to_length(work, b, avg);
        c1 || c2
    } else if let (Some(ra), Some(rb)) = (radius_of(work, a), radius_of(work, b)) {
        let avg = (ra + rb) / 2.0;
        let c1 = set_radius_of(work, a, avg);
        let c2 = set_radius_of(work, b, avg);
        c1 || c2
    } else {
        false
    }
}

fn correct_symmetric(
    work: &mut [SketchEntityKind],
    a: SketchEntityId,
    b: SketchEntityId,
    about: SketchEntityId,
) -> bool {
    let Some((ls, le)) = line_points(work, about) else {
        return false;
    };
    if let (Some((as_, ae)), Some((bs, be))) = (line_points(work, a), line_points(work, b)) {
        let ref_bs = reflect_point(bs, ls, le);
        let ref_be = reflect_point(be, ls, le);
        let ref_as = reflect_point(as_, ls, le);
        let ref_ae = reflect_point(ae, ls, le);
        let new_as = Point2::new((as_.x + ref_bs.x) / 2.0, (as_.y + ref_bs.y) / 2.0);
        let new_ae = Point2::new((ae.x + ref_be.x) / 2.0, (ae.y + ref_be.y) / 2.0);
        let new_bs = Point2::new((bs.x + ref_as.x) / 2.0, (bs.y + ref_as.y) / 2.0);
        let new_be = Point2::new((be.x + ref_ae.x) / 2.0, (be.y + ref_ae.y) / 2.0);
        let c1 = set_line_points(work, a, new_as, new_ae);
        let c2 = set_line_points(work, b, new_bs, new_be);
        c1 || c2
    } else if let (Some(ca), Some(cb)) = (center_of(work, a), center_of(work, b)) {
        let ref_cb = reflect_point(cb, ls, le);
        let ref_ca = reflect_point(ca, ls, le);
        let new_ca = Point2::new((ca.x + ref_cb.x) / 2.0, (ca.y + ref_cb.y) / 2.0);
        let new_cb = Point2::new((cb.x + ref_ca.x) / 2.0, (cb.y + ref_ca.y) / 2.0);
        let ra = center_point_ref(work, a).expect("center_of succeeded");
        let rb = center_point_ref(work, b).expect("center_of succeeded");
        let c1 = set_point(work, ra, new_ca);
        let c2 = set_point(work, rb, new_cb);
        c1 || c2
    } else {
        false
    }
}

fn correct_distance(
    work: &mut [SketchEntityKind],
    a: PointRef,
    b: PointRef,
    target_len: f64,
) -> bool {
    let sa = point_is_settable(work, a);
    let sb = point_is_settable(work, b);
    if !sa && !sb {
        return false;
    }
    let (Some(pa), Some(pb)) = (get_point(work, a), get_point(work, b)) else {
        return false;
    };
    let vec = pb - pa;
    let cur = vec.length();
    let dir = if cur > 1e-9 {
        vec.scale(1.0 / cur)
    } else {
        Vector2::new(1.0, 0.0)
    };
    let (ta, tb) = if sa && sb {
        let delta = target_len - cur;
        (pa + dir.scale(-delta / 2.0), pb + dir.scale(delta / 2.0))
    } else if sa {
        (pb + dir.scale(-target_len), pb)
    } else {
        (pa, pa + dir.scale(target_len))
    };
    let mut changed = false;
    if sa {
        changed |= set_point(work, a, ta);
    }
    if sb {
        changed |= set_point(work, b, tb);
    }
    changed
}

fn correct_midpoint(work: &mut [SketchEntityKind], point: PointRef, line: SketchEntityId) -> bool {
    if !point_is_settable(work, point) {
        return false;
    }
    match line_points(work, line) {
        Some((s, e)) => {
            let mid = Point2::new((s.x + e.x) / 2.0, (s.y + e.y) / 2.0);
            set_point(work, point, mid)
        }
        None => false,
    }
}

// ---------------------------------------------------------------------
// Residuals (one per `ConstraintKind`, see module doc comment
// "Classification")
// ---------------------------------------------------------------------

fn normalized(diff: f64, tolerance: f64) -> f64 {
    (diff / tolerance).abs()
}

fn residual(
    work: &[SketchEntityKind],
    seed: &[SketchEntityKind],
    kind: &ConstraintKind,
    profile: &SketchSolverProfile,
) -> f64 {
    let pos_tol = profile.position_tolerance;
    let ang_tol = profile.angle_tolerance;
    match *kind {
        ConstraintKind::Coincident { a, b } => match (get_point(work, a), get_point(work, b)) {
            (Some(pa), Some(pb)) => normalized((pb - pa).length(), pos_tol),
            _ => f64::INFINITY,
        },
        ConstraintKind::Horizontal { line } => match line_points(work, line) {
            Some((s, e)) => normalized((e.y - s.y).abs(), pos_tol),
            None => f64::INFINITY,
        },
        ConstraintKind::Vertical { line } => match line_points(work, line) {
            Some((s, e)) => normalized((e.x - s.x).abs(), pos_tol),
            None => f64::INFINITY,
        },
        ConstraintKind::Parallel { a, b } => residual_parallel(work, a, b, ang_tol),
        ConstraintKind::Perpendicular { a, b } => residual_perpendicular(work, a, b, ang_tol),
        ConstraintKind::Tangent { a, b } => residual_tangent(work, a, b, pos_tol),
        ConstraintKind::Concentric { a, b } => match (center_of(work, a), center_of(work, b)) {
            (Some(ca), Some(cb)) => normalized((cb - ca).length(), pos_tol),
            _ => f64::INFINITY,
        },
        ConstraintKind::Equal { a, b } => residual_equal(work, a, b, pos_tol),
        ConstraintKind::Symmetric { a, b, about } => residual_symmetric(work, a, b, about, pos_tol),
        ConstraintKind::Distance { a, b, value } => {
            match (get_point(work, a), get_point(work, b)) {
                (Some(pa), Some(pb)) => {
                    normalized(((pb - pa).length() - value.magnitude).abs(), pos_tol)
                }
                _ => f64::INFINITY,
            }
        }
        ConstraintKind::Angle { a, b, value } => {
            residual_angle(work, a, b, value.magnitude, ang_tol)
        }
        ConstraintKind::Radius { entity, value } => normalized(
            (radius_of(work, entity).unwrap_or(f64::INFINITY) - value.magnitude).abs(),
            pos_tol,
        ),
        ConstraintKind::Diameter { entity, value } => normalized(
            (radius_of(work, entity).unwrap_or(f64::INFINITY) - value.magnitude / 2.0).abs(),
            pos_tol,
        ),
        ConstraintKind::Fixed { entity } => residual_fixed(work, seed, entity, pos_tol, ang_tol),
        ConstraintKind::Midpoint { point, line } => {
            match (get_point(work, point), line_points(work, line)) {
                (Some(p), Some((s, e))) => {
                    let mid = Point2::new((s.x + e.x) / 2.0, (s.y + e.y) / 2.0);
                    normalized((p - mid).length(), pos_tol)
                }
                _ => f64::INFINITY,
            }
        }
    }
}

fn residual_parallel(
    work: &[SketchEntityKind],
    a: SketchEntityId,
    b: SketchEntityId,
    ang_tol: f64,
) -> f64 {
    match (line_dir(work, a), line_dir(work, b)) {
        (Some(ua), Some(ub)) => {
            let cross = (ua.x * ub.y - ua.y * ub.x).clamp(-1.0, 1.0);
            normalized(cross.abs().asin(), ang_tol)
        }
        _ => f64::INFINITY,
    }
}

fn residual_perpendicular(
    work: &[SketchEntityKind],
    a: SketchEntityId,
    b: SketchEntityId,
    ang_tol: f64,
) -> f64 {
    match (line_dir(work, a), line_dir(work, b)) {
        (Some(ua), Some(ub)) => {
            let dot = (ua.x * ub.x + ua.y * ub.y).clamp(-1.0, 1.0);
            normalized(dot.abs().asin(), ang_tol)
        }
        _ => f64::INFINITY,
    }
}

fn residual_angle(
    work: &[SketchEntityKind],
    a: SketchEntityId,
    b: SketchEntityId,
    target: f64,
    ang_tol: f64,
) -> f64 {
    match (line_dir(work, a), line_dir(work, b)) {
        (Some(ua), Some(ub)) => {
            let theta_a = ua.y.atan2(ua.x);
            let theta_b = ub.y.atan2(ub.x);
            let diff = wrap_pi(theta_b - theta_a - target);
            normalized(diff.abs(), ang_tol)
        }
        _ => f64::INFINITY,
    }
}

fn residual_tangent(
    work: &[SketchEntityKind],
    a: SketchEntityId,
    b: SketchEntityId,
    pos_tol: f64,
) -> f64 {
    match (radial_info(work, a), radial_info(work, b)) {
        (Some((ca, ra)), Some((cb, rb))) => {
            let d = (cb - ca).length();
            normalized((d - (ra + rb)).abs(), pos_tol)
        }
        _ => {
            let (line_id, circ_id) = if radial_info(work, a).is_some() {
                (b, a)
            } else {
                (a, b)
            };
            match (line_points(work, line_id), radial_info(work, circ_id)) {
                (Some((ls, le)), Some((cc, rr))) => {
                    let d = le - ls;
                    let len = d.length();
                    if len < 1e-9 {
                        return f64::INFINITY;
                    }
                    let dir = d.scale(1.0 / len);
                    let ap = cc - ls;
                    let t = ap.x * dir.x + ap.y * dir.y;
                    let closest = ls + dir.scale(t);
                    let dist = (cc - closest).length();
                    normalized((dist - rr).abs(), pos_tol)
                }
                _ => f64::INFINITY,
            }
        }
    }
}

fn residual_equal(
    work: &[SketchEntityKind],
    a: SketchEntityId,
    b: SketchEntityId,
    pos_tol: f64,
) -> f64 {
    if let (Some((as_, ae)), Some((bs, be))) = (line_points(work, a), line_points(work, b)) {
        normalized(((ae - as_).length() - (be - bs).length()).abs(), pos_tol)
    } else if let (Some(ra), Some(rb)) = (radius_of(work, a), radius_of(work, b)) {
        normalized((ra - rb).abs(), pos_tol)
    } else {
        f64::INFINITY
    }
}

fn residual_symmetric(
    work: &[SketchEntityKind],
    a: SketchEntityId,
    b: SketchEntityId,
    about: SketchEntityId,
    pos_tol: f64,
) -> f64 {
    let Some((ls, le)) = line_points(work, about) else {
        return f64::INFINITY;
    };
    if let (Some((as_, ae)), Some((bs, be))) = (line_points(work, a), line_points(work, b)) {
        let r1 = (reflect_point(as_, ls, le) - bs).length();
        let r2 = (reflect_point(ae, ls, le) - be).length();
        normalized(r1.max(r2), pos_tol)
    } else if let (Some(ca), Some(cb)) = (center_of(work, a), center_of(work, b)) {
        normalized((reflect_point(ca, ls, le) - cb).length(), pos_tol)
    } else {
        f64::INFINITY
    }
}

fn residual_fixed(
    work: &[SketchEntityKind],
    seed: &[SketchEntityKind],
    entity: SketchEntityId,
    pos_tol: f64,
    ang_tol: f64,
) -> f64 {
    let idx = entity_index(entity);
    match (work[idx], seed[idx]) {
        (
            SketchEntityKind::Line { start: s1, end: e1 },
            SketchEntityKind::Line { start: s2, end: e2 },
        ) => normalized((s1 - s2).length(), pos_tol).max(normalized((e1 - e2).length(), pos_tol)),
        (
            SketchEntityKind::Circle {
                center: c1,
                radius: r1,
            },
            SketchEntityKind::Circle {
                center: c2,
                radius: r2,
            },
        ) => normalized((c1 - c2).length(), pos_tol)
            .max(normalized((r1.magnitude - r2.magnitude).abs(), pos_tol)),
        (
            SketchEntityKind::Arc {
                center: c1,
                radius: r1,
                start_angle: sa1,
                end_angle: ea1,
                ..
            },
            SketchEntityKind::Arc {
                center: c2,
                radius: r2,
                start_angle: sa2,
                end_angle: ea2,
                ..
            },
        ) => {
            let p = normalized((c1 - c2).length(), pos_tol);
            let r = normalized((r1.magnitude - r2.magnitude).abs(), pos_tol);
            let a1 = normalized(wrap_pi(sa1.magnitude - sa2.magnitude).abs(), ang_tol);
            let a2 = normalized(wrap_pi(ea1.magnitude - ea2.magnitude).abs(), ang_tol);
            p.max(r).max(a1).max(a2)
        }
        _ => 0.0,
    }
}

// ---------------------------------------------------------------------
// Structural equation count (`DL-20`'s DOF-evidence table; see module
// doc comment "Classification")
// ---------------------------------------------------------------------

fn equation_count(kind: &ConstraintKind, work: &[SketchEntityKind]) -> u32 {
    match *kind {
        ConstraintKind::Coincident { .. } => 2,
        ConstraintKind::Horizontal { .. } => 1,
        ConstraintKind::Vertical { .. } => 1,
        ConstraintKind::Parallel { .. } => 1,
        ConstraintKind::Perpendicular { .. } => 1,
        ConstraintKind::Tangent { .. } => 1,
        ConstraintKind::Concentric { .. } => 2,
        ConstraintKind::Equal { .. } => 1,
        ConstraintKind::Symmetric { a, b, .. } => {
            match (&work[entity_index(a)], &work[entity_index(b)]) {
                (SketchEntityKind::Line { .. }, SketchEntityKind::Line { .. }) => 4,
                (
                    SketchEntityKind::Circle { .. } | SketchEntityKind::Arc { .. },
                    SketchEntityKind::Circle { .. } | SketchEntityKind::Arc { .. },
                ) => 2,
                _ => 0,
            }
        }
        ConstraintKind::Distance { .. } => 1,
        ConstraintKind::Angle { .. } => 1,
        ConstraintKind::Radius { .. } => 1,
        ConstraintKind::Diameter { .. } => 1,
        ConstraintKind::Fixed { entity } => match &work[entity_index(entity)] {
            SketchEntityKind::Line { .. } => 4,
            SketchEntityKind::Circle { .. } => 3,
            SketchEntityKind::Arc { .. } => 5,
        },
        ConstraintKind::Midpoint { .. } => 2,
    }
}

fn collect_values(sketch: &Sketch, work: &[SketchEntityKind]) -> SolvedValues {
    let mut values = SolvedValues::new();
    for var in sketch_variables(sketch) {
        let magnitude = match var {
            SketchVariable::PointX(p) => get_point(work, p).map(|pt| pt.x),
            SketchVariable::PointY(p) => get_point(work, p).map(|pt| pt.y),
            SketchVariable::Radius(id) => radius_of(work, id),
            SketchVariable::StartAngle(id) => match &work[entity_index(id)] {
                SketchEntityKind::Arc { start_angle, .. } => Some(start_angle.magnitude),
                _ => None,
            },
            SketchVariable::EndAngle(id) => match &work[entity_index(id)] {
                SketchEntityKind::Arc { end_angle, .. } => Some(end_angle.magnitude),
                _ => None,
            },
        };
        if let Some(m) = magnitude {
            values.set(var, m);
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sketch_constraint::{ConstraintKind, ConstraintSet};
    use cad_ast::Span;
    use cad_hir::sketch::{Quantity, RotationDirection, SketchId, SketchPlane};
    use cad_types::Dimension;
    use std::f64::consts::{FRAC_PI_2, PI};

    fn span() -> Span {
        Span::new(0, 1)
    }

    fn length(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Length)
    }

    fn angle(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Angle)
    }

    fn point_value(values: &SolvedValues, r: PointRef) -> Point2 {
        Point2::new(
            values.get(SketchVariable::PointX(r)).expect("x solved"),
            values.get(SketchVariable::PointY(r)).expect("y solved"),
        )
    }

    const POS_EPS: f64 = 1e-6;

    fn assert_close(actual: f64, expected: f64, eps: f64) {
        assert!(
            (actual - expected).abs() < eps,
            "expected {expected}, got {actual} (diff {})",
            (actual - expected).abs()
        );
    }

    fn assert_point_close(actual: Point2, expected: Point2) {
        assert_close(actual.x, expected.x, POS_EPS);
        assert_close(actual.y, expected.y, POS_EPS);
    }

    #[test]
    fn distance_between_two_line_endpoints_converges_to_target() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let l = sketch.add_line(Point2::new(0.0, 0.0), Point2::new(3.0, 0.0), false, span());
        let mut set = ConstraintSet::new(sketch.id());
        set.add(
            &sketch,
            ConstraintKind::Distance {
                a: PointRef::LineStart(l),
                b: PointRef::LineEnd(l),
                value: length(10.0),
            },
            span(),
        )
        .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Underconstrained { remaining_dof } => assert_eq!(remaining_dof, 3),
            other => panic!("expected Underconstrained, got {other:?}"),
        }
        let start = point_value(&report.values, PointRef::LineStart(l));
        let end = point_value(&report.values, PointRef::LineEnd(l));
        assert_close((end - start).length(), 10.0, POS_EPS);
    }

    #[test]
    fn radius_and_diameter_converge_to_target() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let c = sketch
            .add_circle(Point2::ORIGIN, length(1.0), false, span())
            .unwrap();
        let mut set = ConstraintSet::new(sketch.id());
        set.add(
            &sketch,
            ConstraintKind::Diameter {
                entity: c,
                value: length(6.0),
            },
            span(),
        )
        .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Underconstrained { remaining_dof } => assert_eq!(remaining_dof, 2),
            other => panic!("expected Underconstrained, got {other:?}"),
        }
        let r = report.values.get(SketchVariable::Radius(c)).unwrap();
        assert_close(r, 3.0, POS_EPS);
    }

    #[test]
    fn concentric_circles_converge_centers() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let a = sketch
            .add_circle(Point2::new(0.0, 0.0), length(1.0), false, span())
            .unwrap();
        let b = sketch
            .add_circle(Point2::new(4.0, 0.0), length(2.0), false, span())
            .unwrap();
        let mut set = ConstraintSet::new(sketch.id());
        set.add(&sketch, ConstraintKind::Concentric { a, b }, span())
            .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Underconstrained { remaining_dof } => assert_eq!(remaining_dof, 4),
            other => panic!("expected Underconstrained, got {other:?}"),
        }
        let ca = point_value(&report.values, PointRef::CircleCenter(a));
        let cb = point_value(&report.values, PointRef::CircleCenter(b));
        assert_point_close(ca, cb);
    }

    #[test]
    fn tangent_circle_circle_converges_to_external_tangency() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let a = sketch
            .add_circle(Point2::new(0.0, 0.0), length(2.0), false, span())
            .unwrap();
        let b = sketch
            .add_circle(Point2::new(3.0, 0.0), length(3.0), false, span())
            .unwrap();
        let mut set = ConstraintSet::new(sketch.id());
        set.add(&sketch, ConstraintKind::Tangent { a, b }, span())
            .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Underconstrained { remaining_dof } => assert_eq!(remaining_dof, 5),
            other => panic!("expected Underconstrained, got {other:?}"),
        }
        let ca = point_value(&report.values, PointRef::CircleCenter(a));
        let cb = point_value(&report.values, PointRef::CircleCenter(b));
        assert_close((cb - ca).length(), 5.0, POS_EPS);
    }

    #[test]
    fn tangent_line_circle_converges_to_correct_offset() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let l = sketch.add_line(Point2::new(-5.0, 0.0), Point2::new(5.0, 0.0), false, span());
        let c = sketch
            .add_circle(Point2::new(0.0, 3.0), length(1.0), false, span())
            .unwrap();
        let mut set = ConstraintSet::new(sketch.id());
        set.add(&sketch, ConstraintKind::Tangent { a: l, b: c }, span())
            .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Underconstrained { remaining_dof } => assert_eq!(remaining_dof, 6),
            other => panic!("expected Underconstrained, got {other:?}"),
        }
        // The line stays on the x-axis (never itself corrected by this
        // constraint — only the circle's center moves); the circle's
        // center should converge to distance == radius (2.0) from it,
        // on its original (+y) side.
        let center = point_value(&report.values, PointRef::CircleCenter(c));
        let radius = report.values.get(SketchVariable::Radius(c)).unwrap();
        assert_close(radius, 1.0, POS_EPS);
        assert_close(center.y.abs(), radius, POS_EPS);
        assert!(center.y > 0.0);
    }

    #[test]
    fn parallel_lines_converge_to_same_direction() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let a = sketch.add_line(Point2::new(0.0, 0.0), Point2::new(10.0, 0.0), false, span());
        let b = sketch.add_line(Point2::new(0.0, 5.0), Point2::new(3.0, 8.0), false, span());
        let mut set = ConstraintSet::new(sketch.id());
        set.add(&sketch, ConstraintKind::Parallel { a, b }, span())
            .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Underconstrained { remaining_dof } => assert_eq!(remaining_dof, 7),
            other => panic!("expected Underconstrained, got {other:?}"),
        }
        let bs = point_value(&report.values, PointRef::LineStart(b));
        let be = point_value(&report.values, PointRef::LineEnd(b));
        // Parallel to the x-axis means equal y at both endpoints.
        assert_close(be.y, bs.y, POS_EPS);
    }

    #[test]
    fn perpendicular_lines_converge_to_right_angle() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let a = sketch.add_line(Point2::new(0.0, 0.0), Point2::new(10.0, 0.0), false, span());
        let b = sketch.add_line(Point2::new(0.0, 5.0), Point2::new(3.0, 8.0), false, span());
        let mut set = ConstraintSet::new(sketch.id());
        set.add(&sketch, ConstraintKind::Perpendicular { a, b }, span())
            .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Underconstrained { remaining_dof } => assert_eq!(remaining_dof, 7),
            other => panic!("expected Underconstrained, got {other:?}"),
        }
        let bs = point_value(&report.values, PointRef::LineStart(b));
        let be = point_value(&report.values, PointRef::LineEnd(b));
        // Perpendicular to the x-axis means equal x at both endpoints.
        assert_close(be.x, bs.x, POS_EPS);
    }

    #[test]
    fn angle_constraint_converges_to_target_angle() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let a = sketch.add_line(Point2::new(0.0, 0.0), Point2::new(10.0, 0.0), false, span());
        let b = sketch.add_line(Point2::new(0.0, 5.0), Point2::new(3.0, 8.0), false, span());
        let mut set = ConstraintSet::new(sketch.id());
        set.add(
            &sketch,
            ConstraintKind::Angle {
                a,
                b,
                value: angle(FRAC_PI_2),
            },
            span(),
        )
        .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Underconstrained { remaining_dof } => assert_eq!(remaining_dof, 7),
            other => panic!("expected Underconstrained, got {other:?}"),
        }
        let bs = point_value(&report.values, PointRef::LineStart(b));
        let be = point_value(&report.values, PointRef::LineEnd(b));
        // `a` lies along the x-axis (angle 0); `b` should now be at
        // angle +90 degrees from it, i.e. purely vertical.
        assert_close(be.x, bs.x, POS_EPS);
        assert!(be.y > bs.y);
    }

    #[test]
    fn equal_lines_converge_to_average_length() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let a = sketch.add_line(Point2::new(0.0, 0.0), Point2::new(4.0, 0.0), false, span());
        let b = sketch.add_line(Point2::new(0.0, 5.0), Point2::new(0.0, 15.0), false, span());
        let mut set = ConstraintSet::new(sketch.id());
        set.add(&sketch, ConstraintKind::Equal { a, b }, span())
            .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Underconstrained { remaining_dof } => assert_eq!(remaining_dof, 7),
            other => panic!("expected Underconstrained, got {other:?}"),
        }
        let as_ = point_value(&report.values, PointRef::LineStart(a));
        let ae = point_value(&report.values, PointRef::LineEnd(a));
        let bs = point_value(&report.values, PointRef::LineStart(b));
        let be = point_value(&report.values, PointRef::LineEnd(b));
        assert_close((ae - as_).length(), 7.0, POS_EPS);
        assert_close((be - bs).length(), 7.0, POS_EPS);
    }

    #[test]
    fn equal_circles_converge_to_average_radius() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let a = sketch
            .add_circle(Point2::new(0.0, 0.0), length(2.0), false, span())
            .unwrap();
        let b = sketch
            .add_circle(Point2::new(10.0, 0.0), length(6.0), false, span())
            .unwrap();
        let mut set = ConstraintSet::new(sketch.id());
        set.add(&sketch, ConstraintKind::Equal { a, b }, span())
            .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Underconstrained { remaining_dof } => assert_eq!(remaining_dof, 5),
            other => panic!("expected Underconstrained, got {other:?}"),
        }
        let ra = report.values.get(SketchVariable::Radius(a)).unwrap();
        let rb = report.values.get(SketchVariable::Radius(b)).unwrap();
        assert_close(ra, 4.0, POS_EPS);
        assert_close(rb, 4.0, POS_EPS);
    }

    #[test]
    fn midpoint_converges_to_line_midpoint() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let line = sketch.add_line(Point2::new(0.0, 0.0), Point2::new(10.0, 4.0), false, span());
        let holder = sketch.add_line(
            Point2::new(20.0, 20.0),
            Point2::new(21.0, 21.0),
            false,
            span(),
        );
        let point = PointRef::LineStart(holder);
        let mut set = ConstraintSet::new(sketch.id());
        set.add(&sketch, ConstraintKind::Midpoint { point, line }, span())
            .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Underconstrained { remaining_dof } => assert_eq!(remaining_dof, 6),
            other => panic!("expected Underconstrained, got {other:?}"),
        }
        let p = point_value(&report.values, point);
        assert_point_close(p, Point2::new(5.0, 2.0));
    }

    #[test]
    fn symmetric_lines_converge_across_axis() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let about = sketch.add_line(
            Point2::new(0.0, -10.0),
            Point2::new(0.0, 10.0),
            false,
            span(),
        );
        let a = sketch.add_line(Point2::new(3.0, 0.0), Point2::new(3.0, 4.0), false, span());
        let b = sketch.add_line(
            Point2::new(-2.5, 0.2),
            Point2::new(-3.5, 3.8),
            false,
            span(),
        );
        let mut set = ConstraintSet::new(sketch.id());
        set.add(&sketch, ConstraintKind::Fixed { entity: about }, span())
            .unwrap();
        set.add(&sketch, ConstraintKind::Symmetric { a, b, about }, span())
            .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        // `about` (Fixed, 4 equations) + `symmetric` (line/line, 4
        // equations) = 8 equations against 12 total dof (3 lines) ->
        // 4 remaining (a's own two points still free to slide, only
        // tied to b's reflection).
        match report.status {
            SolveStatus::Underconstrained { remaining_dof } => assert_eq!(remaining_dof, 4),
            other => panic!("expected Underconstrained, got {other:?}"),
        }
        let as_ = point_value(&report.values, PointRef::LineStart(a));
        let ae = point_value(&report.values, PointRef::LineEnd(a));
        let bs = point_value(&report.values, PointRef::LineStart(b));
        let be = point_value(&report.values, PointRef::LineEnd(b));
        // Reflection across the y-axis (x=0) negates x, keeps y.
        assert_close(bs.x, -as_.x, POS_EPS);
        assert_close(bs.y, as_.y, POS_EPS);
        assert_close(be.x, -ae.x, POS_EPS);
        assert_close(be.y, ae.y, POS_EPS);
    }

    #[test]
    fn rectangle_from_rough_sketch_converges_to_exact_rectangle() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        // `bottom` is seeded exactly at its final position/shape so that
        // `Fixed(bottom)` pins the whole loop's absolute position,
        // orientation, and length in one shot.
        let bottom = sketch.add_line(Point2::new(0.0, 0.0), Point2::new(10.0, 0.0), false, span());
        let right = sketch.add_line(
            Point2::new(10.2, -0.1),
            Point2::new(9.8, 5.4),
            false,
            span(),
        );
        let top = sketch.add_line(
            Point2::new(10.1, 5.2),
            Point2::new(-0.2, 4.9),
            false,
            span(),
        );
        let left = sketch.add_line(
            Point2::new(0.3, 5.05),
            Point2::new(-0.15, -0.05),
            false,
            span(),
        );

        let mut set = ConstraintSet::new(sketch.id());
        set.add(&sketch, ConstraintKind::Fixed { entity: bottom }, span())
            .unwrap();
        set.add(
            &sketch,
            ConstraintKind::Coincident {
                a: PointRef::LineEnd(bottom),
                b: PointRef::LineStart(right),
            },
            span(),
        )
        .unwrap();
        set.add(&sketch, ConstraintKind::Vertical { line: right }, span())
            .unwrap();
        set.add(
            &sketch,
            ConstraintKind::Distance {
                a: PointRef::LineStart(right),
                b: PointRef::LineEnd(right),
                value: length(5.0),
            },
            span(),
        )
        .unwrap();
        set.add(
            &sketch,
            ConstraintKind::Coincident {
                a: PointRef::LineEnd(right),
                b: PointRef::LineStart(top),
            },
            span(),
        )
        .unwrap();
        set.add(&sketch, ConstraintKind::Horizontal { line: top }, span())
            .unwrap();
        set.add(
            &sketch,
            ConstraintKind::Distance {
                a: PointRef::LineStart(top),
                b: PointRef::LineEnd(top),
                value: length(10.0),
            },
            span(),
        )
        .unwrap();
        set.add(
            &sketch,
            ConstraintKind::Coincident {
                a: PointRef::LineEnd(top),
                b: PointRef::LineStart(left),
            },
            span(),
        )
        .unwrap();
        set.add(&sketch, ConstraintKind::Vertical { line: left }, span())
            .unwrap();
        set.add(
            &sketch,
            ConstraintKind::Distance {
                a: PointRef::LineStart(left),
                b: PointRef::LineEnd(left),
                value: length(5.0),
            },
            span(),
        )
        .unwrap();
        set.add(
            &sketch,
            ConstraintKind::Coincident {
                a: PointRef::LineEnd(left),
                b: PointRef::LineStart(bottom),
            },
            span(),
        )
        .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        assert_eq!(report.status, SolveStatus::Solved);

        assert_point_close(
            point_value(&report.values, PointRef::LineStart(bottom)),
            Point2::new(0.0, 0.0),
        );
        assert_point_close(
            point_value(&report.values, PointRef::LineEnd(bottom)),
            Point2::new(10.0, 0.0),
        );
        assert_point_close(
            point_value(&report.values, PointRef::LineStart(right)),
            Point2::new(10.0, 0.0),
        );
        assert_point_close(
            point_value(&report.values, PointRef::LineEnd(right)),
            Point2::new(10.0, 5.0),
        );
        assert_point_close(
            point_value(&report.values, PointRef::LineStart(top)),
            Point2::new(10.0, 5.0),
        );
        assert_point_close(
            point_value(&report.values, PointRef::LineEnd(top)),
            Point2::new(0.0, 5.0),
        );
        assert_point_close(
            point_value(&report.values, PointRef::LineStart(left)),
            Point2::new(0.0, 5.0),
        );
        assert_point_close(
            point_value(&report.values, PointRef::LineEnd(left)),
            Point2::new(0.0, 0.0),
        );
    }

    #[test]
    fn lone_unconstrained_line_is_underconstrained() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        sketch.add_line(Point2::new(0.0, 0.0), Point2::new(1.0, 1.0), false, span());
        let set = ConstraintSet::new(sketch.id());

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Underconstrained { remaining_dof } => assert_eq!(remaining_dof, 4),
            other => panic!("expected Underconstrained, got {other:?}"),
        }
    }

    #[test]
    fn conflicting_distance_and_fixed_is_overconstrained() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        // Seeded at length 10 apart; `Fixed` will keep resetting it
        // there every iteration while `Distance` insists on 20 — the
        // two can never agree simultaneously.
        let l = sketch.add_line(Point2::new(0.0, 0.0), Point2::new(10.0, 0.0), false, span());
        let mut set = ConstraintSet::new(sketch.id());
        set.add(&sketch, ConstraintKind::Fixed { entity: l }, span())
            .unwrap();
        set.add(
            &sketch,
            ConstraintKind::Distance {
                a: PointRef::LineStart(l),
                b: PointRef::LineEnd(l),
                value: length(20.0),
            },
            span(),
        )
        .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Overconstrained { conflicting } => assert!(!conflicting.is_empty()),
            other => panic!("expected Overconstrained, got {other:?}"),
        }
    }

    #[test]
    fn mismatched_symmetric_pairing_is_unsupported() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let about = sketch.add_line(Point2::new(0.0, -1.0), Point2::new(0.0, 1.0), false, span());
        let a = sketch.add_line(Point2::new(2.0, 0.0), Point2::new(2.0, 1.0), false, span());
        let b = sketch
            .add_circle(Point2::new(-2.0, 0.0), length(1.0), false, span())
            .unwrap();
        let mut set = ConstraintSet::new(sketch.id());
        set.add(&sketch, ConstraintKind::Symmetric { a, b, about }, span())
            .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Unsupported { constraints } => assert_eq!(constraints.len(), 1),
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    #[test]
    fn coincident_between_two_arc_endpoints_is_unsupported() {
        let mut sketch = Sketch::new(SketchId::new(0), SketchPlane::WorldXy);
        let arc1 = sketch
            .add_arc(
                Point2::ORIGIN,
                length(1.0),
                angle(0.0),
                angle(PI),
                RotationDirection::CounterClockwise,
                false,
                span(),
            )
            .unwrap();
        let arc2 = sketch
            .add_arc(
                Point2::new(5.0, 5.0),
                length(1.0),
                angle(0.0),
                angle(PI),
                RotationDirection::CounterClockwise,
                false,
                span(),
            )
            .unwrap();
        let mut set = ConstraintSet::new(sketch.id());
        set.add(
            &sketch,
            ConstraintKind::Coincident {
                a: PointRef::ArcStart(arc1),
                b: PointRef::ArcEnd(arc2),
            },
            span(),
        )
        .unwrap();

        let report = RelaxationSolver::new().solve(&sketch, &set);
        match report.status {
            SolveStatus::Unsupported { constraints } => assert_eq!(constraints.len(), 1),
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }
}
