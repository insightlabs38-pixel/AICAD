//! `AICAD-126` (Checkpoint C): proves the Stage-5 topology/raw/adoption/
//! reference-integrity surface (`AICAD-119`-`125`) as one connected
//! production-path vertical slice — construct, sew, heal, inspect,
//! raw-edit, explicitly adopt, and persistently reference — through a
//! single real `ParametricBuildSession`, mirroring `stage5_queries_
//! checkpoint.rs`'s own `AICAD-118` precedent (an integrated proof, not a
//! restatement of each already-proven-in-isolation capability).
//!
//! Every step below is independently already proven by its own task's own
//! dedicated test file (`stage5_sewing_healing.rs`, `stage5_topology_
//! inspection.rs`, `stage5_raw_editing.rs`, `stage5_raw_adoption.rs`,
//! `stage5_raw_lineage.rs`); this file's own job is proving they compose
//! into one real chain without any architecture boundary silently
//! breaking along the way.

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_query::{Candidate, EvaluationEvidence, ResolutionOutcome};
use cad_references::{EntityKind, FeatureAnchor};
use cad_runtime::value::Value;

/// construct (`box`) -> inspect (`topology_face_at`) -> construct
/// (`make_shell`) -> sew -> heal -> inspect (`is_valid`/`raw_topology_
/// kind_of`) -> raw-edit (`remove_face`) -> explicit adopt -> persistent
/// reference (`generated_by`/`modified_by`, resolved through the real
/// resolver, fail-closed).
const SOURCE: &str = "\
let b: Geometry = box(dx = 1mm, dy = 1mm, dz = 1mm);\n\
let f0: Geometry = topology_face_at(b, 0);\n\
let f1: Geometry = topology_face_at(b, 1);\n\
let f2: Geometry = topology_face_at(b, 2);\n\
let f3: Geometry = topology_face_at(b, 3);\n\
let f4: Geometry = topology_face_at(b, 4);\n\
let f5: Geometry = topology_face_at(b, 5);\n\
let shell: Geometry = make_shell([f0, f1, f2, f3, f4, f5]);\n\
let sewn: Geometry = sew([f0, f1, f2, f3, f4, f5], 0.000001mm);\n\
let healed: Geometry = heal(sewn, 0.000001mm);\n\
let raw_shell: Raw = enter_raw(shell);\n\
let raw_kind: String = raw_topology_kind_of(raw_shell);\n\
let opened: Raw = remove_face(raw_shell, [0], false, 0.000001mm);\n\
let adopted: Geometry = adopt(opened);\n\
";

fn session(ctx: &OcctContext) -> ParametricBuildSession<'_> {
    ParametricBuildSession::new("test.aicad", SOURCE, ctx)
        .expect("the whole vertical slice must build cleanly through the real production path")
}

fn str_value(value: &Value) -> &str {
    match value {
        Value::Str(s) => s,
        other => panic!("expected Value::Str, got {other:?}"),
    }
}

#[test]
fn the_whole_construct_sew_heal_raw_edit_adopt_chain_dispatches_through_one_real_session() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let session = session(&ctx);

    // construct
    let b = session
        .shape_for_binding(session.binding_named("b").unwrap())
        .unwrap();
    assert!((b.volume().unwrap() - 1.0e-9).abs() < 1e-15);

    // sew: reassembling the box's own six independently-extracted faces
    // reconstructs the same closed cube.
    let sewn = session
        .shape_for_binding(session.binding_named("sewn").unwrap())
        .unwrap();
    assert!(sewn.is_valid().unwrap());
    assert!((sewn.volume().unwrap() - 1.0e-9).abs() < 1e-15);

    // heal: a no-op on an already-valid solid.
    let healed = session
        .shape_for_binding(session.binding_named("healed").unwrap())
        .unwrap();
    assert!(healed.is_valid().unwrap());

    // inspect (raw): the raw tier's own snapshot of the shell classifies
    // its real topological kind.
    let raw_kind_binding = session.binding_named("raw_kind").unwrap();
    let raw_kind = session.last_global(raw_kind_binding).unwrap();
    assert_eq!(str_value(raw_kind), "Shell");

    // raw-edit + explicit adopt: a real, valid, adopted safe value.
    let adopted_binding = session.binding_named("adopted").unwrap();
    let adopted = session.shape_for_binding(adopted_binding).unwrap();
    assert!(adopted.is_valid().unwrap());
    assert_eq!(adopted.face_count().unwrap(), 5, "one face was removed");

    // persistent reference: real captured lineage reaches the resolver,
    // fail-closed (never a guess, never a panic) -- `remove_face`'s own
    // report has only `deleted` evidence, so every surviving face is
    // real, present (`Some(false)`), non-`None` evidence, exactly
    // `stage5_raw_lineage.rs`'s own established finding.
    let anchor = FeatureAnchor::named("adopted");
    for i in 0..adopted.face_count().unwrap() {
        let candidate = Candidate::new(EntityKind::Face, adopted.get_face(i).unwrap());
        assert_eq!(
            EvaluationEvidence::generated_by(&session, &candidate, &anchor),
            Some(false)
        );
        assert_eq!(
            EvaluationEvidence::modified_by(&session, &candidate, &anchor),
            Some(false)
        );
    }

    // The safe/raw/reference identity separation (D22) holds throughout:
    // `raw_shell`'s own raw handle never became a persistent reference
    // itself (only the *adopted* `GeomId` participates in
    // `FeatureAnchor`-addressed lineage), and no native/OCCT type crossed
    // into this test's own vocabulary -- every assertion above uses only
    // `cad_occt_bridge::Shape`'s kernel-neutral accessors.
    let query = cad_query::Query::new(EntityKind::Face)
        .with_clause(cad_query::QueryClause::Topology(
            cad_query::TopologyPredicate::GeneratedBy(anchor.clone()),
        ))
        .with_cardinality(cad_query::CardinalityExpectation::Unique)
        .scoped_to(anchor);
    match session
        .resolve(&query)
        .expect("resolution should not error")
    {
        ResolutionOutcome::Broken(reason) => {
            let _ = reason; // real fail-closed Broken(NoMatch) -- remove_face creates no new face
        }
        other => {
            let description = match &other {
                ResolutionOutcome::Resolved(c) => format!("Resolved({})", c.len()),
                ResolutionOutcome::Ambiguous(c) => format!("Ambiguous({})", c.len()),
                ResolutionOutcome::Broken(_) => unreachable!(),
            };
            panic!(
                "expected Broken(NoMatch) (remove_face never generates a face), got {description}"
            )
        }
    }
}
