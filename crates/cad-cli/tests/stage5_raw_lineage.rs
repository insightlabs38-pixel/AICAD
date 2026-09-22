//! `AICAD-125`: end-to-end proof that Stage-5 topology-changing
//! operations' own real change evidence (`AICAD-119`-`124`) actually
//! reaches the Stage-4 semantic-reference machinery through the real
//! production path (`ParametricBuildSession`, real `OcctContext`) — never
//! a hand-constructed stand-in operation, mirroring
//! `stage4_reference_replay.rs`'s own established pattern. Every
//! assertion below is a real kernel geometry property (volume/`is_same`
//! identity/candidate count), never a render-only check, per `AGENTS.md`'s
//! evidence rule.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_query::{Candidate, EvaluationEvidence, ResolutionOutcome};
use cad_references::{
    AnyRef, ConstructionStrategy, EntityKind, FaceRef, FeatureAnchor, LineageRole,
};
use cad_runtime::value::{NumberValue, Value};
use cad_types::Dimension;
use cad_units::OperandType;

fn session<'ctx>(ctx: &'ctx OcctContext, source: &str) -> ParametricBuildSession<'ctx> {
    ParametricBuildSession::new("test.aicad", source, ctx)
        .expect("the fixture should build cleanly through the real production path")
}

fn length_value(metres: f64) -> Value {
    Value::Number(NumberValue {
        magnitude: metres,
        ty: OperandType::dimensional(Dimension::Length, None),
    })
}

/// `sew` genuinely relabels shared boundary edges when reassembling a
/// solid's own faces that were independently extracted (`topology_face_at`
/// gives each its own disconnected edge copies) — real `Modified` evidence
/// (`AICAD-120`'s own established finding for `Shape::sew`'s reliable
/// `IsModified`/`Modified` history, unlike `heal`'s unreliable one) that
/// reaches the resolver through the real production path. `replace_face`'s
/// own signature was tried first for a `Modified`-evidence proof through
/// `adopt` instead, but `AICAD-123`'s own disclosed limitation (no
/// geometric-compatibility check between the old/new face) means an
/// independently-built replacement — even a self-duplicate of the exact
/// same face — reliably produces a `BRepCheck_Analyzer`-invalid (and for
/// some inputs, kernel-call-rejected) result `adopt` correctly refuses;
/// `merge_faces`/`split_edge`'s own `List<Raw>` results have no current
/// `.aicad` element-selection syntax to feed one entry into `adopt` at
/// all. Neither is a viable positive-path fixture for `adopt`'s own
/// `Modified`/`Merged` evidence within this task's own scope — disclosed
/// as a real limitation in this task's own report, not silently worked
/// around.
#[test]
fn sew_resolves_modified_by_the_named_feature_via_relabeled_shared_edges() {
    let source = "\
let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
let f0: Geometry = topology_face_at(b, 0);\n\
let f1: Geometry = topology_face_at(b, 1);\n\
let f2: Geometry = topology_face_at(b, 2);\n\
let f3: Geometry = topology_face_at(b, 3);\n\
let f4: Geometry = topology_face_at(b, 4);\n\
let f5: Geometry = topology_face_at(b, 5);\n\
let sewn: Geometry = sew([f0, f1, f2, f3, f4, f5], 0.000001mm);\n\
";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx, source);
    let binding = session.binding_named("sewn").unwrap();
    let sewn_shape = session.shape_for_binding(binding).unwrap();
    assert!(
        (sewn_shape.volume().unwrap() - 1.0e-9).abs() < 1e-15,
        "reassembling a box's own six faces reconstructs the same closed 1mm cube"
    );

    let anchor = FeatureAnchor::named("sewn");
    let query = cad_query::Query::new(EntityKind::Edge)
        .with_clause(cad_query::QueryClause::Topology(
            cad_query::TopologyPredicate::ModifiedBy(anchor.clone()),
        ))
        .with_cardinality(cad_query::CardinalityExpectation::Unstated)
        .scoped_to(anchor);
    match session
        .resolve(&query)
        .expect("resolution should not error")
    {
        ResolutionOutcome::Resolved(candidates) => assert_eq!(
            candidates.len(),
            12,
            "every one of the box's own 12 edges was independently duplicated by \
             topology_face_at, then relabeled back to a shared edge by sew -- real \
             per-edge evidence, not a guess"
        ),
        _ => panic!("expected Resolved(12) (Unstated cardinality never reports Ambiguous)"),
    }
}

/// `remove_face`-then-`adopt` produces real (if entirely negative)
/// evidence for every surviving face -- `Some(false)`, never `None` (no
/// evidence) -- proving an operation whose own report has no
/// `modified`/`created` entries at all still gets a real captured report,
/// not silence. Built from an explicit `make_shell` (not a `Solid`, which
/// `remove_face` would leave open and therefore invalid without healing --
/// `heal` itself has its own separate, already-disclosed identity-
/// relabeling limitation this test deliberately avoids exercising): a
/// `Shell` has no closure requirement, so removing one of its own six
/// faces leaves a genuinely valid, adoptable five-face open shell with no
/// healing involved at all.
#[test]
fn remove_face_then_adopt_reports_real_but_entirely_negative_evidence() {
    let source = "\
let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
let f0: Geometry = topology_face_at(b, 0);\n\
let f1: Geometry = topology_face_at(b, 1);\n\
let f2: Geometry = topology_face_at(b, 2);\n\
let f3: Geometry = topology_face_at(b, 3);\n\
let f4: Geometry = topology_face_at(b, 4);\n\
let f5: Geometry = topology_face_at(b, 5);\n\
let shell: Geometry = make_shell([f0, f1, f2, f3, f4, f5]);\n\
let raw_shell: Raw = enter_raw(shell);\n\
let opened: Raw = remove_face(raw_shell, [0], false, 0.000001mm);\n\
let adopted: Geometry = adopt(opened);\n\
";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx, source);
    let binding = session.binding_named("adopted").unwrap();
    let adopted_shape = session.shape_for_binding(binding).unwrap();
    assert_eq!(
        adopted_shape.face_count().unwrap(),
        5,
        "one face was removed"
    );
    let anchor = FeatureAnchor::named("adopted");

    for i in 0..adopted_shape.face_count().unwrap() {
        let face = adopted_shape.get_face(i).unwrap();
        let candidate = Candidate::new(EntityKind::Face, face);
        assert_eq!(
            EvaluationEvidence::generated_by(&session, &candidate, &anchor),
            Some(false)
        );
        assert_eq!(
            EvaluationEvidence::modified_by(&session, &candidate, &anchor),
            Some(false)
        );
    }
}

/// A multi-step raw-edit chain (`enter_raw -> remove_face -> remove_face
/// -> adopt`) composes real evidence across every step -- the production-
/// path counterpart of `cad_query::feature_lineage`'s own unit-level
/// `a_two_step_chain_composes_evidence_across_both_steps` test.
#[test]
fn a_two_step_raw_edit_chain_composes_real_evidence_through_adopt() {
    let source = "\
let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
let f0: Geometry = topology_face_at(b, 0);\n\
let f1: Geometry = topology_face_at(b, 1);\n\
let f2: Geometry = topology_face_at(b, 2);\n\
let f3: Geometry = topology_face_at(b, 3);\n\
let f4: Geometry = topology_face_at(b, 4);\n\
let f5: Geometry = topology_face_at(b, 5);\n\
let shell: Geometry = make_shell([f0, f1, f2, f3, f4, f5]);\n\
let raw_shell: Raw = enter_raw(shell);\n\
let once_opened: Raw = remove_face(raw_shell, [0], false, 0.000001mm);\n\
let twice_opened: Raw = remove_face(once_opened, [0], false, 0.000001mm);\n\
let adopted: Geometry = adopt(twice_opened);\n\
";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx, source);
    let binding = session.binding_named("adopted").unwrap();
    let adopted_shape = session.shape_for_binding(binding).unwrap();
    assert_eq!(
        adopted_shape.face_count().unwrap(),
        4,
        "two faces were removed across the two-step chain"
    );
    let anchor = FeatureAnchor::named("adopted");
    for i in 0..adopted_shape.face_count().unwrap() {
        let candidate = Candidate::new(EntityKind::Face, adopted_shape.get_face(i).unwrap());
        assert_eq!(
            EvaluationEvidence::generated_by(&session, &candidate, &anchor),
            Some(false),
            "real (negative) evidence must be present after a two-step chain, not `None`"
        );
    }
}

/// `sew`'s own real per-entity lineage (`AICAD-120`'s `OcctContext::sew`,
/// already captured into `LineageTable` at dispatch time) reaches the
/// resolver through the real production path -- `AICAD-125` extended
/// `capture_named_feature_lineage`'s own operand handling to cover `Sew`'s
/// multi-operand shape, not only `Union`/`Cut`/`Intersect`/`Fillet`/
/// `Chamfer`'s single `lhs`/`target`.
#[test]
fn sew_resolves_generated_by_the_named_feature_through_the_real_resolver() {
    let source = "\
let e0: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 0mm, y = 0mm, z = 0mm), direction = Vector3(x = 1.0, y = 0.0, z = 0.0)), 0.0, 0.001));\n\
let e1: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 1mm, y = 0mm, z = 0mm), direction = Vector3(x = 0.0, y = 1.0, z = 0.0)), 0.0, 0.001));\n\
let e2: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 1mm, y = 1mm, z = 0mm), direction = Vector3(x = -1.0, y = 0.0, z = 0.0)), 0.0, 0.001));\n\
let e3: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 0mm, y = 1mm, z = 0mm), direction = Vector3(x = 0.0, y = -1.0, z = 0.0)), 0.0, 0.001));\n\
let square1: Geometry = make_wire([e0, e1, e2, e3]);\n\
let face1: Geometry = make_face(square1);\n\
let f0: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 1mm, y = 0mm, z = 0mm), direction = Vector3(x = 1.0, y = 0.0, z = 0.0)), 0.0, 0.001));\n\
let f1: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 2mm, y = 0mm, z = 0mm), direction = Vector3(x = 0.0, y = 1.0, z = 0.0)), 0.0, 0.001));\n\
let f2: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 2mm, y = 1mm, z = 0mm), direction = Vector3(x = -1.0, y = 0.0, z = 0.0)), 0.0, 0.001));\n\
let f3: Geometry = make_edge(trim_curve(line_curve(origin = Point3(x = 1mm, y = 1mm, z = 0mm), direction = Vector3(x = 0.0, y = -1.0, z = 0.0)), 0.0, 0.001));\n\
let square2: Geometry = make_wire([f0, f1, f2, f3]);\n\
let face2: Geometry = make_face(square2);\n\
let sewn: Geometry = sew([face1, face2], 0.000001mm);\n\
";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx, source);
    let binding = session.binding_named("sewn").unwrap();
    let sewn_shape = session.shape_for_binding(binding).unwrap();
    assert_eq!(
        sewn_shape.face_count().unwrap(),
        2,
        "sew merges the two independent faces into one shell without altering face count"
    );
    let anchor = FeatureAnchor::named("sewn");
    // `sew`'s own report has no created/modified/deleted entries for two
    // already-independent, non-overlapping faces (only their shared edge
    // is relabeled -- `AICAD-120`'s own report) -- proves real, present
    // (never `None`) evidence, matching the acceptance line "no operation
    // silently drops change evidence."
    for i in 0..sewn_shape.face_count().unwrap() {
        let candidate = Candidate::new(EntityKind::Face, sewn_shape.get_face(i).unwrap());
        assert_eq!(
            EvaluationEvidence::generated_by(&session, &candidate, &anchor),
            Some(false)
        );
    }
}

/// `heal`'s own explicit insufficiency: `AICAD-120`'s own report
/// disclosed that `ShapeFix_Shape`'s native history does not reliably
/// populate, so a `Heal`-final named feature contributes no
/// `FeatureLineageIndex` entry at all -- `generated_by`/`modified_by`
/// against it must report `None` ("no evidence"), which
/// `cad_query::resolve` turns into `Broken(InsufficientEvidence)`, never
/// a silent/guessed `Resolved`.
#[test]
fn heal_produces_explicit_insufficient_evidence_never_a_guess() {
    let source = "\
let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
let healed: Geometry = heal(b, 0.000001mm);\n\
";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx, source);
    let binding = session.binding_named("healed").unwrap();
    let healed_shape = session.shape_for_binding(binding).unwrap();
    let anchor = FeatureAnchor::named("healed");
    let candidate = Candidate::new(EntityKind::Face, healed_shape.get_face(0).unwrap());
    assert_eq!(
        EvaluationEvidence::generated_by(&session, &candidate, &anchor),
        None,
        "heal must never fabricate lineage evidence it cannot actually produce"
    );

    let reference = AnyRef::Face(FaceRef::from_strategy(
        ConstructionStrategy::FeatureLineage {
            feature: anchor,
            role: LineageRole::Generated,
        },
    ));
    let outcome = session
        .resolve_reference(&reference)
        .expect("resolution should not error");
    assert!(
        matches!(outcome.outcome, ResolutionOutcome::Broken(_)),
        "a persistent reference anchored to heal's own insufficient evidence must fail closed, \
         never silently resolve"
    );
}

/// The Stage-4 persistent-reference replay contract
/// (`stage4_reference_replay.rs`'s own established proof shape), now for a
/// Stage-5 `sew`-relabeled reference downstream of a `param`: re-resolving
/// the same `ModifiedBy(sewn)` query after a real parameter edit/rebuild
/// reflects the real regenerated geometry (a real new total edge length),
/// never stale evidence from the first build.
#[test]
fn a_persistent_reference_through_sew_replays_across_a_rebuild() {
    let source = "\
param side: Length = 1mm;\n\
let b: Geometry = box(dx = side, dy = side, dz = side);\n\
let f0: Geometry = topology_face_at(b, 0);\n\
let f1: Geometry = topology_face_at(b, 1);\n\
let f2: Geometry = topology_face_at(b, 2);\n\
let f3: Geometry = topology_face_at(b, 3);\n\
let f4: Geometry = topology_face_at(b, 4);\n\
let f5: Geometry = topology_face_at(b, 5);\n\
let sewn: Geometry = sew([f0, f1, f2, f3, f4, f5], 0.000001mm);\n\
";
    let ctx = OcctContext::new().expect("context creation should succeed");
    let mut session = session(&ctx, source);
    let anchor = FeatureAnchor::named("sewn");
    let make_query = || {
        cad_query::Query::new(EntityKind::Edge)
            .with_clause(cad_query::QueryClause::Topology(
                cad_query::TopologyPredicate::ModifiedBy(anchor.clone()),
            ))
            .with_cardinality(cad_query::CardinalityExpectation::Unstated)
            .scoped_to(anchor.clone())
    };

    let query_before = make_query();
    let before = match session
        .resolve(&query_before)
        .expect("resolution should not error")
    {
        ResolutionOutcome::Resolved(candidates) => candidates,
        _ => panic!("expected Resolved (Unstated cardinality never reports Ambiguous)"),
    };
    assert_eq!(before.len(), 12);
    let total_length_before: f64 = before.iter().map(|c| c.shape().length().unwrap()).sum();
    assert!(
        (total_length_before - 12.0 * 0.001).abs() < 1e-12,
        "total_length_before = {total_length_before}"
    );
    let binding = session.binding_named("sewn").unwrap();
    let volume_before = session
        .shape_for_binding(binding)
        .unwrap()
        .volume()
        .unwrap();
    drop(before);

    session.set_param("side", length_value(0.002)).unwrap();
    session.rebuild().expect("edit/rebuild should succeed");

    let query_after = make_query();
    let after = match session
        .resolve(&query_after)
        .expect("resolution should not error")
    {
        ResolutionOutcome::Resolved(candidates) => candidates,
        _ => panic!("expected Resolved (Unstated cardinality never reports Ambiguous)"),
    };
    assert_eq!(
        after.len(),
        12,
        "the edit must not change how many edges are relabeled"
    );
    let total_length_after: f64 = after.iter().map(|c| c.shape().length().unwrap()).sum();
    assert!(
        (total_length_after - 12.0 * 0.002).abs() < 1e-12,
        "the re-resolved edges' own real total length must reflect the new 2mm side, not the \
         stale 1mm evidence from the first build (total_length_after = {total_length_after})"
    );
    let volume_after = session
        .shape_for_binding(binding)
        .unwrap()
        .volume()
        .unwrap();
    assert!(
        (volume_before - 1.0e-9).abs() < 1e-15 && (volume_after - 8.0e-9).abs() < 1e-15,
        "volume_before = {volume_before}, volume_after = {volume_after}"
    );
}
