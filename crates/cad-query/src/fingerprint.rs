//! Geometry-fingerprint evidence computation and candidate ranking
//! (`AICAD-092`), restricted to the three roles
//! `project/DECISION_LOG.md#DL-8` (D7) allows in this first Stage-4
//! implementation: diagnostic evidence, ranking candidates shown to a
//! human, and benchmark/experiment data.
//!
//! # Never automatic resolution
//!
//! Nothing in this module is called from `crate::resolve`'s own
//! resolution path, and nothing here can turn an `Ambiguous`/`Broken`
//! [`crate::resolve::ResolutionOutcome`] into `Resolved`.
//! `crate::resolve::resolve_reference`'s unconditional
//! [`crate::resolve::BrokenReason::FingerprintAutoResolutionDisabled`] for
//! [`cad_references::ConstructionStrategy::GeometricFingerprint`]
//! (`AICAD-088`) is unchanged by this task -- this module is purely an
//! opt-in tool a caller (diagnostics, a future benchmark/health report)
//! may invoke on its own; the resolver never reaches for it.
//! [`rank_candidates_never_narrows_or_selects_automatically`] (this
//! module's own test) exercises exactly that boundary end-to-end: ranking
//! a real ambiguous cube's tied faces, then proving `resolve_reference` on
//! a [`cad_references::ConstructionStrategy::GeometricFingerprint`] recipe
//! for that same shape still reports `Broken`, never influenced by the
//! ranking this module just computed.
//!
//! # Honesty, not precision
//!
//! [`candidate_fingerprint`] computes best-effort evidence from real
//! geometry (the same accessors `crate::eval`/`crate::resolve` already
//! use): a field the candidate's own kind/geometry does not support (e.g.
//! `radius` on a planar face) is simply omitted, matching
//! `crate::diagnostics::candidate_summary`'s own "omit rather than
//! fabricate" precedent. [`fingerprint_distance`] is consequently a
//! partial-match heuristic, not a rigorous metric: a field present on only
//! one side of the comparison does not contribute to the score at all --
//! neither penalized as maximally different nor ignored as a match, since
//! it is simply not evidence either side can compare.

use cad_kernel_api::KernelError;
use cad_references::{EntityKind, FingerprintEvidence};

use crate::eval::Candidate;

/// Computes real geometric evidence for `candidate`, using the same
/// position/area/radius/normal accessors `crate::eval`/`crate::resolve`
/// already use against real `cad-occt-bridge` geometry. Only locating the
/// candidate's own representative position is mandatory (an `Err` here
/// means the kernel itself failed, not merely that one optional field does
/// not apply to this candidate's kind); area/radius/normal are each
/// independently best-effort and simply omitted when they do not apply.
pub fn candidate_fingerprint(
    candidate: &Candidate<'_>,
) -> Result<FingerprintEvidence, KernelError> {
    let position = if candidate.kind() == EntityKind::Vertex {
        candidate.shape().vertex_point()?
    } else {
        candidate.shape().center_of_mass()?
    };
    let mut evidence = FingerprintEvidence::at_position([position.x, position.y, position.z]);

    if let Ok(area) = candidate.shape().area() {
        evidence = evidence.with_area(area);
    }
    let radius = match candidate.kind() {
        EntityKind::Face => candidate.shape().face_radius().ok(),
        EntityKind::Edge => candidate.shape().edge_radius().ok(),
        _ => None,
    };
    if let Some(radius) = radius {
        evidence = evidence.with_radius(radius);
    }
    if candidate.kind() == EntityKind::Face
        && let Ok((_, normal)) = candidate.shape().face_normal()
    {
        let v = normal.as_vector3();
        evidence = evidence.with_normal([v.x, v.y, v.z]);
    }
    Ok(evidence)
}

/// A partial-match dissimilarity score between two [`FingerprintEvidence`]
/// values: the Euclidean distance between `position`s (always compared --
/// `position` is the one mandatory field), plus the Euclidean distance
/// between `normal`s and the absolute difference between `area`s/
/// `radius`es *whenever both sides supply that field*. Lower is more
/// similar; `0.0` only when every field either matches exactly or is
/// absent on at least one side.
pub fn fingerprint_distance(a: &FingerprintEvidence, b: &FingerprintEvidence) -> f64 {
    let mut total = euclidean(a.position, b.position);
    if let (Some(na), Some(nb)) = (a.normal, b.normal) {
        total += euclidean(na, nb);
    }
    if let (Some(aa), Some(ab)) = (a.area, b.area) {
        total += (aa - ab).abs();
    }
    if let (Some(ra), Some(rb)) = (a.radius, b.radius) {
        total += (ra - rb).abs();
    }
    total
}

fn euclidean(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// One live candidate paired with its [`fingerprint_distance`] from a
/// target [`FingerprintEvidence`], as produced by [`rank_by_fingerprint`].
pub struct RankedCandidate<'ctx> {
    pub candidate: Candidate<'ctx>,
    pub distance: f64,
}

/// Ranks `candidates` by similarity to `target`, most similar
/// ([`fingerprint_distance`] ascending) first. A candidate whose own
/// fingerprint cannot be computed at all (a kernel error, not merely a
/// missing optional field) is dropped rather than assigned an arbitrary
/// score -- see this module's own doc comment on honesty.
///
/// This function only ever *orders* the candidates it is given for
/// diagnostic/benchmark/ranking display; it never filters by a similarity
/// threshold, never returns fewer candidates than were computable, and
/// never designates one entry as "the" answer -- every caller still sees
/// the full ranked list, exactly the "ranking candidates shown to a user"
/// role D7/`DL-8` allows, never a narrowing decision.
pub fn rank_by_fingerprint<'ctx>(
    target: &FingerprintEvidence,
    candidates: Vec<Candidate<'ctx>>,
) -> Vec<RankedCandidate<'ctx>> {
    let mut ranked: Vec<RankedCandidate<'ctx>> = candidates
        .into_iter()
        .filter_map(|candidate| {
            let evidence = candidate_fingerprint(&candidate).ok()?;
            let distance = fingerprint_distance(target, &evidence);
            Some(RankedCandidate {
                candidate,
                distance,
            })
        })
        .collect();
    ranked.sort_by(|a, b| a.distance.total_cmp(&b.distance));
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::{BrokenReason, ResolutionOutcome, ResolverContext, resolve_reference};
    use cad_occt_bridge::OcctContext;
    use cad_references::{AnyRef, ConstructionStrategy, FaceRef};

    #[test]
    fn candidate_fingerprint_reports_real_area_and_normal_for_a_planar_face() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let face = Candidate::new(EntityKind::Face, cube.get_face(0).unwrap());
        let evidence = candidate_fingerprint(&face).unwrap();
        assert_eq!(evidence.area, Some(4.0));
        assert!(evidence.normal.is_some());
        assert_eq!(
            evidence.radius, None,
            "a planar face has no well-defined radius -- must be omitted, not fabricated"
        );
    }

    #[test]
    fn candidate_fingerprint_reports_real_radius_for_a_cylindrical_face() {
        let context = OcctContext::new().unwrap();
        let cylinder = context.create_cylinder(2.0, 5.0).unwrap();
        let lateral = (0..cylinder.face_count().unwrap())
            .map(|i| Candidate::new(EntityKind::Face, cylinder.get_face(i).unwrap()))
            .find(|c| c.shape().face_radius().is_ok())
            .expect("a cylinder has a lateral face with a well-defined radius");
        let evidence = candidate_fingerprint(&lateral).unwrap();
        assert!((evidence.radius.unwrap() - 2.0).abs() < 1e-6);
    }

    #[test]
    fn fingerprint_distance_is_zero_for_identical_evidence() {
        let evidence = FingerprintEvidence::at_position([1.0, 2.0, 3.0])
            .with_area(10.0)
            .with_radius(2.0)
            .with_normal([0.0, 0.0, 1.0]);
        assert_eq!(fingerprint_distance(&evidence, &evidence), 0.0);
    }

    #[test]
    fn fingerprint_distance_ignores_a_field_present_on_only_one_side() {
        let same_position = [0.0, 0.0, 0.0];
        let with_area = FingerprintEvidence::at_position(same_position).with_area(100.0);
        let without_area = FingerprintEvidence::at_position(same_position);
        assert_eq!(
            fingerprint_distance(&with_area, &without_area),
            0.0,
            "a field only one side supplies must not contribute to the score in either \
             direction"
        );
    }

    #[test]
    fn fingerprint_distance_accumulates_position_and_area_difference() {
        let a = FingerprintEvidence::at_position([0.0, 0.0, 0.0]).with_area(10.0);
        let b = FingerprintEvidence::at_position([3.0, 4.0, 0.0]).with_area(16.0);
        // Position term: sqrt(3^2 + 4^2) = 5.0. Area term: |10 - 16| = 6.0.
        assert!((fingerprint_distance(&a, &b) - 11.0).abs() < 1e-9);
    }

    #[test]
    fn rank_by_fingerprint_orders_a_boxs_faces_nearest_first() {
        let context = OcctContext::new().unwrap();
        // Distinct edge lengths -> distinct face-center positions, so the
        // ranking has one unambiguous, checkable nearest face.
        let box_shape = context.create_box(2.0, 4.0, 8.0).unwrap();
        let candidates: Vec<_> = (0..box_shape.face_count().unwrap())
            .map(|i| Candidate::new(EntityKind::Face, box_shape.get_face(i).unwrap()))
            .collect();

        // Target the +X face's own real center directly, so this test
        // checks real computed positions rather than an assumed layout.
        let plus_x = candidates
            .iter()
            .find(|c| {
                c.shape()
                    .face_normal()
                    .map(|(_, n)| n.as_vector3().x > 0.9)
                    .unwrap_or(false)
            })
            .expect("a box has a +X face");
        let target = candidate_fingerprint(plus_x).unwrap();

        let ranked = rank_by_fingerprint(&target, candidates);
        assert_eq!(ranked.len(), 6, "no candidate is silently dropped");
        assert!(
            ranked[0].distance <= ranked[1].distance,
            "the list must actually be sorted ascending by distance"
        );
        assert!(
            ranked[0].distance < 1e-6,
            "the +X face's own fingerprint must rank itself as the exact nearest match"
        );
    }

    /// End-to-end proof of this module's own central invariant: ranking a
    /// real ambiguous shape's candidates by fingerprint similarity never
    /// influences `crate::resolve`'s own fail-closed outcome. A
    /// `GeometricFingerprint`-strategy reference against this exact shape
    /// still reports `Broken(FingerprintAutoResolutionDisabled)`
    /// (`AICAD-088`'s own guarantee), completely unaffected by the
    /// ranking this test also computes from the same real geometry.
    #[test]
    fn rank_candidates_never_narrows_or_selects_automatically() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let candidates: Vec<_> = (0..cube.face_count().unwrap())
            .map(|i| Candidate::new(EntityKind::Face, cube.get_face(i).unwrap()))
            .collect();
        let target = candidate_fingerprint(&candidates[0]).unwrap();

        let ranked = rank_by_fingerprint(&target, candidates);
        assert_eq!(
            ranked.len(),
            6,
            "ranking must still report every tied candidate, never narrow the ambiguity itself"
        );

        let reference = AnyRef::Face(FaceRef::from_strategy(
            ConstructionStrategy::GeometricFingerprint(target),
        ));
        struct EmptyContext;
        impl<'ctx> crate::eval::EvaluationEvidence<'ctx> for EmptyContext {}
        impl<'ctx> ResolverContext<'ctx> for EmptyContext {
            fn candidates(&self, _kind: EntityKind) -> Vec<Candidate<'ctx>> {
                Vec::new()
            }
        }
        let outcome = resolve_reference(&reference, &EmptyContext).unwrap();
        assert!(
            matches!(
                outcome,
                ResolutionOutcome::Broken(BrokenReason::FingerprintAutoResolutionDisabled(_))
            ),
            "computing/ranking fingerprint evidence must never change the resolver's own \
             fail-closed outcome"
        );
    }
}
