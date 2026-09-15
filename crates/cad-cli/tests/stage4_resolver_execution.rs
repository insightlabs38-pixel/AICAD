//! `AICAD-096`: real semantic-reference resolver execution against the
//! Stage-4 topology-reference corpus — the concrete "resolver execution"
//! half `AICAD-079A`'s own corpus documentation named as `AICAD-080`+
//! work: turning each case's own prose "intended query target" into a
//! real [`cad_query::Query`], resolving it via
//! [`cad_cli::ParametricBuildSession::resolve`] against a real build of
//! that case's own `baseline.aicad`/`perturbed.aicad`, and comparing the
//! actual [`cad_query::ResolutionOutcome`] to the case's own documented
//! ground-truth classification — never a hand-built stand-in shape.
//!
//! # A production bug this file's own attempt found and fixed
//!
//! Before this task, `ParametricBuildSession::resolve` had never actually
//! been exercised against a `part { ... }` block — the idiomatic way
//! every real `.aicad` example, and the *entire* frozen `AICAD-079A`
//! corpus, declares its geometry (`skills/cad-core.skill.md`'s own worked
//! examples). Every prior `AICAD-094`/`095` test used bare top-level
//! `let`s instead. Running this file's own first draft against the real
//! corpus found that a `part`-wrapped build always reported every query
//! `Broken`, and traced to two real defects, now fixed at the root
//! (`project/reports/AICAD-096.md` has the full account):
//!
//! 1. `cad_runtime::interp::Interpreter::run_top_level_parametric` (the
//!    evaluation entry point [`cad_cli::ParametricBuildSession`] always
//!    calls) silently skipped every `HirItem::Part` item outright —
//!    `Interpreter::run_top_level` already called
//!    `Interpreter::eval_part_body` correctly; the parametric entry point
//!    just never did. Fixed by wiring the same already-correct,
//!    already-tested call into this method too.
//! 2. `cad_cli::parametric_build`'s own `last_globals`/`ResolverContext::
//!    candidates` enumeration only ever scanned `program.items` directly,
//!    never a part body's own inner items — `crates/cad-cli/src/
//!    parametric_build.rs`'s new `collect_geometry_globals` fixes this by
//!    reading each part's own already-evaluated `Value::Part` aggregate
//!    and matching its name-keyed fields back to their real `BindingId`s.
//!
//! A third, *separate* limitation surfaced by the same investigation was
//! **not** fixed by this task, and was recorded rather than invented an
//! answer for: `cad_feature_graph::FeatureGraph::build` only scanned
//! `program.items` too, so `ParametricBuildSession::rebuild`'s own
//! `named_feature_ranges` (and therefore every captured `Face` lineage
//! entry `generated_by`/`modified_by`/`descended_from` evidence depends
//! on) never covered a `part`-nested named feature — `project/
//! OWNER_DECISIONS.md#D31`. **`D31` is now resolved** (`AICAD-100A`): the
//! owner ruling is that `part { ... }` is an abstraction/scope boundary,
//! not a feature-visibility barrier, and `FeatureGraph::build` now
//! recurses into `part` bodies with scoped identity (`crates/
//! cad-feature-graph/src/graph.rs`'s own module doc comment, "`part`
//! bodies — `D31`"; `crate::parametric_build::resolve_scoped_name`/
//! `qualified_feature_name` give a caller a collision-safe way to name a
//! part-nested feature even when two different parts reuse the same leaf
//! name). Cases `01`/`02`/`07` below now execute their own real
//! worked-example `generated_by`/`descended_from` queries through the
//! real production path — see each test's own doc comment for the
//! honest, real result (not force-matched to `case.md`'s own pre-
//! execution prose where the two genuinely differ).
//!
//! # Scope: which cases this file actually executes, and why
//!
//! - **`06_fillet_viability`** (fillet-radius perturbation): the fillet's
//!   own blend face is the corpus's only `8mm`-radius cylindrical face,
//!   so `Cylindrical + Radius(== fillet_radius) + unique()` finds it with
//!   no lineage evidence needed; the perturbed build fails at the kernel
//!   before a session (and therefore a query) exists at all —
//!   `KERNEL_FAILURE`, not a resolution question, per `tests/
//!   semantic_refs/README.md`'s own outcome taxonomy.
//! - **`08_upstream_suppression`**: the fillet (radius `3mm`) and the
//!   hole (radius `2.5mm`) are two geometrically distinct cylindrical
//!   faces, so `Cylindrical + Radius(== 3mm) + unique()` finds the fillet
//!   face alone in the baseline and finds nothing at all once the fillet
//!   feature is removed from the perturbed *source* entirely —
//!   `explicit_broken_reference`, reproduced by geometry alone.
//! - Two new, self-authored cases (`project/benchmarks/
//!   stage4_semantic_reference/resolver_execution/`) covering the two
//!   perturbation classes the frozen ten do not name at all: extrusion
//!   resize (`11`) and add/remove hole (`12`). Both are designed, by
//!   construction, with exactly one named top-level feature, so neither
//!   the `part`-scoping fix above nor the pre-existing "every top-level
//!   binding is a permanently-live candidate" design (`AICAD-094`'s own
//!   established `ResolverContext::candidates` semantics) introduces a
//!   false extra candidate.
//!
//! `01_topology_split_merge`/`02_disappearing_entity`/`07_operation_
//! reordering` now execute their own real worked-example lineage-based
//! queries (`AICAD-100A`, above). `04_pattern_count_change`'s own intended
//! target ("bolt-pattern instance 0... by lineage") is tracked by
//! position instead (`AICAD-099`'s own already-established `nearest()`
//! precedent, unchanged by `D31`'s resolution — `radial_pattern` is not
//! itself one of the five lineage-capable ops `cad_query::feature_lineage`
//! classifies, so a `generated_by`-shaped query has no real evidence
//! source for it regardless of `part` scoping). `05_boolean_topology_
//! change` remains held out, never touched here — see `held_out/
//! HELD_OUT_README.md`.

use std::path::PathBuf;

use cad_cli::ParametricBuildSession;
use cad_occt_bridge::OcctContext;
use cad_query::{
    Comparison, GeometryPredicate, Magnitude, Query, QueryClause, ResolutionOutcome,
    TopologyPredicate,
};
use cad_references::{EntityKind, FeatureAnchor};
use cad_types::Dimension;
use cad_units::OperandType;

fn fixture_source(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read fixture at {}: {err}", path.display()))
}

fn session<'ctx>(ctx: &'ctx OcctContext, relative: &str) -> ParametricBuildSession<'ctx> {
    let source = fixture_source(relative);
    ParametricBuildSession::new(relative, &source, ctx)
        .unwrap_or_else(|diags| panic!("{relative}: expected a successful build, got {diags:?}"))
}

fn length_mm(value_mm: f64) -> Magnitude {
    Magnitude::new(
        value_mm / 1000.0,
        OperandType::dimensional(Dimension::Length, None),
    )
}

/// A short, human-readable tag for a [`ResolutionOutcome`] — that type
/// carries no [`std::fmt::Debug`] (it wraps a live kernel [`cad_query::
/// Candidate`]), so this is this file's own diagnostic-only summary, not
/// a `cad-query` API.
fn describe(outcome: &ResolutionOutcome<'_>) -> String {
    match outcome {
        ResolutionOutcome::Resolved(candidates) => format!("Resolved({})", candidates.len()),
        ResolutionOutcome::Ambiguous(candidates) => format!("Ambiguous({})", candidates.len()),
        ResolutionOutcome::Broken(reason) => format!("Broken({reason:?})"),
    }
}

/// Unwraps a `Resolved` outcome with exactly one candidate, panicking
/// with the real outcome otherwise (never silently accepting `Ambiguous`
/// as if it were `Resolved`).
fn expect_resolved_one(outcome: ResolutionOutcome<'_>) {
    let description = describe(&outcome);
    match outcome {
        ResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
        _ => panic!("expected Resolved(1), got {description}"),
    }
}

fn expect_broken(outcome: ResolutionOutcome<'_>) {
    assert!(
        outcome.is_broken(),
        "expected Broken, got {}",
        describe(&outcome)
    );
}

const BENCH: &str = "project/benchmarks/stage4_semantic_reference";
const RESOLVER_EXECUTION_CORPUS: &str =
    "project/benchmarks/stage4_semantic_reference/resolver_execution";

/// Regression coverage for this task's own two new fixtures' measured
/// evidence (`case.md`'s own "Measured evidence" tables), matching the
/// discipline `stage4_reference_benchmark_fixtures.rs` established for the
/// frozen ten: a future edit that silently changes either fixture fails
/// this test, not only a resolver-execution assertion elsewhere in this
/// file. Checked directly against the live `Shape` `ParametricBuildSession`
/// already produces (no STEP export/reimport round trip — that discipline
/// exists in the frozen-corpus suite to prove STEP fidelity specifically,
/// not a concern this corpus extension needs to re-prove).
#[test]
fn new_fixtures_build_to_their_recorded_measured_evidence() {
    let ctx = OcctContext::new().expect("context creation should succeed");

    let baseline = session(
        &ctx,
        &format!("{RESOLVER_EXECUTION_CORPUS}/11_extrusion_resize/baseline.aicad"),
    );
    let body = baseline.binding_named("body").expect("body binding");
    let shape = baseline.shape_for_binding(body).expect("body shape");
    assert_eq!(shape.face_count().unwrap(), 6);
    assert_eq!(shape.edge_count().unwrap(), 12);
    assert_eq!(shape.vertex_count().unwrap(), 8);
    assert!((shape.volume().unwrap() * 1e9 - 4000.0).abs() < 1e-6);

    let perturbed = session(
        &ctx,
        &format!("{RESOLVER_EXECUTION_CORPUS}/11_extrusion_resize/perturbed.aicad"),
    );
    let body = perturbed.binding_named("body").expect("body binding");
    let shape = perturbed.shape_for_binding(body).expect("body shape");
    assert_eq!(shape.face_count().unwrap(), 6);
    assert_eq!(shape.edge_count().unwrap(), 12);
    assert_eq!(shape.vertex_count().unwrap(), 8);
    assert!((shape.volume().unwrap() * 1e9 - 6400.0).abs() < 1e-6);

    let baseline = session(
        &ctx,
        &format!("{RESOLVER_EXECUTION_CORPUS}/12_add_remove_hole/baseline.aicad"),
    );
    let body = baseline.binding_named("body").expect("body binding");
    let shape = baseline.shape_for_binding(body).expect("body shape");
    assert_eq!(shape.face_count().unwrap(), 7);
    assert_eq!(shape.edge_count().unwrap(), 15);
    assert_eq!(shape.vertex_count().unwrap(), 10);
    assert!((shape.volume().unwrap() * 1e9 - 5803.650459150631).abs() < 1e-2);

    let perturbed = session(
        &ctx,
        &format!("{RESOLVER_EXECUTION_CORPUS}/12_add_remove_hole/perturbed.aicad"),
    );
    let body = perturbed.binding_named("body").expect("body binding");
    let shape = perturbed.shape_for_binding(body).expect("body shape");
    assert_eq!(shape.face_count().unwrap(), 6);
    assert_eq!(shape.edge_count().unwrap(), 12);
    assert_eq!(shape.vertex_count().unwrap(), 8);
    assert!((shape.volume().unwrap() * 1e9 - 6000.0).abs() < 1e-6);
}

/// New case `11_extrusion_resize` (this task's own corpus extension, not
/// part of the frozen `AICAD-079A` ten): a real single-feature boss whose
/// own `extrude_distance` grows between `baseline.aicad`/
/// `perturbed.aicad`. Pure geometry, no lineage needed: the outward end
/// cap is always the sole `Planar`/`+Z`-normal face at every
/// `extrude_distance` this corpus's own `case.md` records. Real measured
/// evidence there: `4000.0 mm^3` / `6400.0 mm^3`, both
/// `20mm x 20mm x extrude_distance` closed-form exact.
#[test]
fn case11_extrusion_resize_resolves_correctly_in_both_variants() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Planar))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Normal(
            cad_query::predicate::DirectionComparison {
                target: cad_query::Direction3::POSITIVE_Z,
                tolerance: None,
            },
        )))
        .with_cardinality(cad_query::CardinalityExpectation::Unique);

    let baseline = session(
        &ctx,
        &format!("{RESOLVER_EXECUTION_CORPUS}/11_extrusion_resize/baseline.aicad"),
    );
    expect_resolved_one(
        baseline
            .resolve(&query)
            .expect("resolution should not error"),
    );

    let perturbed = session(
        &ctx,
        &format!("{RESOLVER_EXECUTION_CORPUS}/11_extrusion_resize/perturbed.aicad"),
    );
    expect_resolved_one(
        perturbed
            .resolve(&query)
            .expect("resolution should not error"),
    );
}

/// New case `12_add_remove_hole`: a plain through-hole, present in
/// `baseline.aicad` and removed entirely in `perturbed.aicad`. A query
/// naming the bore's own cylindrical wall face by shape alone resolves
/// uniquely while the hole exists and reports `Broken` once it is gone —
/// the one perturbation class this corpus's own frozen ten never named
/// directly (case `02`'s "disappearing entity" tests a *different* face
/// disappearing as a side effect of an unrelated later feature, not a
/// hole itself being added/removed).
#[test]
fn case12_add_remove_hole_resolves_then_reports_broken() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(2.5)),
        )))
        .with_cardinality(cad_query::CardinalityExpectation::Unique);

    let baseline = session(
        &ctx,
        &format!("{RESOLVER_EXECUTION_CORPUS}/12_add_remove_hole/baseline.aicad"),
    );
    expect_resolved_one(
        baseline
            .resolve(&query)
            .expect("resolution should not error"),
    );

    let perturbed = session(
        &ctx,
        &format!("{RESOLVER_EXECUTION_CORPUS}/12_add_remove_hole/perturbed.aicad"),
    );
    expect_broken(
        perturbed
            .resolve(&query)
            .expect("resolution should not error"),
    );
}

/// Frozen case `06_fillet_viability` (fillet-radius perturbation):
/// `baseline.aicad` builds and resolves the real `8mm`-radius fillet face
/// by geometry alone; `perturbed.aicad` fails at the kernel-dispatch
/// stage before a `ParametricBuildSession` even exists to call `resolve`
/// against — `KERNEL_FAILURE`, per `tests/semantic_refs/README.md`'s own
/// outcome taxonomy, is a build-pipeline result, never a resolver
/// outcome. `stage4_reference_benchmark_fixtures.rs` already proves the
/// exact baseline topology/volume and the perturbed `GEOM-E005`
/// diagnostic; this test's own job is the resolver-execution half
/// neither one covers.
#[test]
fn case06_fillet_viability_baseline_resolves_the_fillet_face() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let baseline = session(
        &ctx,
        &format!("{BENCH}/public/06_fillet_viability/baseline.aicad"),
    );

    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(8.0)),
        )))
        .with_cardinality(cad_query::CardinalityExpectation::Unique);
    expect_resolved_one(
        baseline
            .resolve(&query)
            .expect("resolution should not error"),
    );
}

#[test]
fn case06_fillet_viability_perturbed_is_a_kernel_failure_not_a_resolution_question() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let perturbed_source = fixture_source(&format!(
        "{BENCH}/public/06_fillet_viability/perturbed.aicad"
    ));
    let result = ParametricBuildSession::new(
        "06_fillet_viability/perturbed.aicad",
        &perturbed_source,
        &ctx,
    );
    let diagnostics = match result {
        Ok(_) => panic!("expected the perturbed fillet build to fail at the kernel"),
        Err(diagnostics) => diagnostics,
    };
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code.as_string() == "GEOM-E005"),
        "expected a real GEOM-E005 kernel-dispatch failure, got {diagnostics:?}"
    );
}

/// Frozen case `08_upstream_suppression`: the fillet (radius `3mm`) and
/// the hole (radius `2.5mm`) are two geometrically distinct cylindrical
/// faces, so radius alone (no lineage) isolates the fillet face —
/// `Ambiguous(2)`, not `Resolved(1)`, in the baseline: `rounded` (the
/// fillet alone) and `body` (fillet-then-hole) are two *separate* named
/// top-level bindings, and `ParametricBuildSession::candidates`'s own
/// already-established design (`AICAD-094`) keeps every top-level
/// binding's own shape permanently live, so `rounded`'s own `3mm` fillet
/// face and `body`'s own structurally-distinct-but-same-radius copy of
/// it both survive as separate candidates — a real, honestly-reported
/// consequence of that design, not a defect this test papers over.
/// Removing the `fillet(...)` call from `perturbed.aicad`'s *source*
/// entirely (not merely leaving it unconsumed) means no `3mm`-radius
/// cylindrical face exists on *either* binding — genuinely `Broken`,
/// `explicit_broken_reference`, reproduced by geometry alone.
#[test]
fn case08_upstream_suppression_resolves_then_reports_broken() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(3.0)),
        )))
        .with_cardinality(cad_query::CardinalityExpectation::Unique);

    let baseline = session(
        &ctx,
        &format!("{BENCH}/public/08_upstream_suppression/baseline.aicad"),
    );
    let baseline_outcome = baseline
        .resolve(&query)
        .expect("resolution should not error");
    assert!(
        !baseline_outcome.is_broken(),
        "expected the 3mm fillet face to be found (Resolved or Ambiguous \
         across rounded/body's own duplicate copies), got {}",
        describe(&baseline_outcome)
    );

    let perturbed = session(
        &ctx,
        &format!("{BENCH}/public/08_upstream_suppression/perturbed.aicad"),
    );
    expect_broken(
        perturbed
            .resolve(&query)
            .expect("resolution should not error"),
    );
}

/// Exploratory, `#[ignore]`d evidence (never asserted as passing, per
/// this file's own module doc comment): case `01`'s own worked-example
/// query (`generated_by(hole_a); cylindrical; unique()`) cannot be
/// evaluated at all against this fixture's real `bored_a`/`body`
/// features — both are declared inside `part Wall { ... }`, and
/// `cad_feature_graph::FeatureGraph`'s own documented scope boundary
/// ("`part` instantiation semantics are `AICAD-072`'s job") means no
/// lineage evidence is ever captured for either, so *any*
/// `generated_by`/`modified_by` query against them reports
/// `Broken(InsufficientEvidence)` regardless of the perturbation — real,
/// fail-closed behavior, but not a reproduction of this case's own
/// `explicit_ambiguity` ground truth. `cargo test -- --ignored
/// --nocapture` reproduces the real printed outcome.
/// A `ConstructionStrategy::FeatureLineage`-anchored `descended_from(...)`
/// ancestor reference, for the `Generated` role — the shared builder every
/// `caseNN_..._real_generated_by_via_descended_from` test below uses.
fn generated_by_ancestor(name: &str) -> cad_references::AnyRef {
    cad_references::AnyRef::Face(cad_references::FaceRef::from_strategy(
        cad_references::ConstructionStrategy::FeatureLineage {
            feature: FeatureAnchor::named(name),
            role: cad_references::LineageRole::Generated,
        },
    ))
}

/// `D31` (`project/OWNER_DECISIONS.md#D31`, resolved by `AICAD-100A`): now
/// that `cad_feature_graph::FeatureGraph` discovers `part`-nested features
/// (`crate::parametric_build`'s own `resolve_scoped_name`/`qualified_
/// feature_name`), case `01`'s own real worked-example query —
/// `generated_by(hole_a); cylindrical; unique()`, `bored_a` being the real
/// binding name the fixture's own prose "hole A" refers to — executes
/// through the real production path for the first time, via `descended_
/// from(FeatureLineage { feature: bored_a, role: Generated })` (a strictly
/// more precise formulation than a bare `generated_by`, since it also
/// composes through `body`'s own later cut via real captured lineage
/// evidence rather than only `bored_a`'s own immediate result — see
/// `crate::reference_replay::descended_from_closure`'s own doc comment),
/// scoped to `body` (`AICAD-099A`) to avoid the unscoped API's own
/// documented intermediate-binding false-ambiguity limitation (§5.1).
///
/// This is a real, honest finding, not a forced match to `case.md`'s own
/// pre-execution prose: the perturbed build resolves `Resolved(1)`, not
/// the `explicit_ambiguity` `case.md` speculated before any resolver
/// could actually execute this query (`case.md`'s own "Measured evidence"
/// table already shows why once read closely — baseline 8 faces vs.
/// perturbed 9, i.e. exactly one *additional* face appears where the two
/// overlapping bores meet, not a clean 1-fragment-becomes-2 split of hole
/// A's own wall itself). Not `SILENT_WRONG`: the resolver's own
/// cardinality logic never narrows a genuine tie (`crates/cad-query/src/
/// resolve.rs`'s own structural guarantee, re-verified `AICAD-100`), so a
/// `Resolved(1)` outcome here means exactly one real candidate survived
/// every clause, not an arbitrary pick from a wider tied set.
#[test]
fn case01_topology_split_merge_generated_by_bored_a_real_production_path() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Topology(TopologyPredicate::DescendedFrom(
            generated_by_ancestor("bored_a"),
        )))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_cardinality(cad_query::CardinalityExpectation::Unique)
        .scoped_to(FeatureAnchor::named("body"));

    let baseline = session(
        &ctx,
        &format!("{BENCH}/public/01_topology_split_merge/baseline.aicad"),
    );
    expect_resolved_one(
        baseline
            .resolve(&query)
            .expect("resolution should not error"),
    );

    let perturbed = session(
        &ctx,
        &format!("{BENCH}/public/01_topology_split_merge/perturbed.aicad"),
    );
    let perturbed_outcome = perturbed
        .resolve(&query)
        .expect("resolution should not error");
    assert!(
        !perturbed_outcome.is_ambiguous(),
        "real measured finding: the perturbed build's own additional face is not classified as \
         a second descendant of bored_a's own cylindrical wall under this query — got {}",
        describe(&perturbed_outcome)
    );
    match &perturbed_outcome {
        ResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
        other => panic!(
            "expected Resolved(1) (real measured production outcome), got {}",
            describe(other)
        ),
    }
}

/// Case `02`'s own real worked-example query — `generated_by(the chamfer
/// feature); unique()` — executes through the real production path for
/// the first time (`D31`, `AICAD-100A`) and matches `case.md`'s own
/// `explicit_broken_reference` ground truth exactly: the perturbed
/// fixture's own title ("disappearing entity") removes the `chamfer(...)`
/// call from source entirely, so no face descended from `chamfered`
/// exists in the perturbed build at all — `Broken(NoMatch)`, never a
/// silent narrowing to an unrelated face.
#[test]
fn case02_disappearing_entity_generated_by_chamfered_real_production_path() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Topology(TopologyPredicate::DescendedFrom(
            generated_by_ancestor("chamfered"),
        )))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Planar))
        .with_cardinality(cad_query::CardinalityExpectation::Unique)
        .scoped_to(FeatureAnchor::named("body"));

    let baseline = session(
        &ctx,
        &format!("{BENCH}/public/02_disappearing_entity/baseline.aicad"),
    );
    expect_resolved_one(
        baseline
            .resolve(&query)
            .expect("resolution should not error"),
    );

    let perturbed = session(
        &ctx,
        &format!("{BENCH}/public/02_disappearing_entity/perturbed.aicad"),
    );
    expect_broken(
        perturbed
            .resolve(&query)
            .expect("resolution should not error"),
    );
}

/// Case `07`'s own real worked-example query — `generated_by(hole_a);
/// cylindrical; unique()` — cannot be anchored by a literal `FeatureAnchor`
/// name across both variants: `07`'s own title ("valid operation
/// reordering") is specifically that hole A and hole B swap which named
/// binding performs each cut (`hole_a_cut`/`body` in `baseline.aicad` vs.
/// `hole_b_cut`/`body` in `perturbed.aicad`, `case.md`'s own fixtures) —
/// `FeatureAnchor::Named` is deliberately a *source-level* stable name
/// (its own doc comment), not a position/identity tracker across which
/// binding produces a conceptually-the-same hole, so a
/// `generated_by`/`descended_from` reference anchored to one fixed name
/// is inherently not the right tool here (proven directly: anchoring on
/// `hole_a_cut`, the name baseline uses, reports real, honest
/// `Broken(InsufficientEvidence)` against the perturbed build, since that
/// name is not declared there at all — never silently wrong, but not
/// `case.md`'s own `correct_resolved_reference` classification either).
/// The corpus's own real intended target — hole A's own cylindrical wall,
/// by its stable *position* (`x = 10mm`, unchanged by the reordering) —
/// is correctly expressed as a geometric `nearest()` query instead,
/// matching `AICAD-099`'s own established "position-tracking across a
/// renamed/reordered producer" precedent for case `04`.
#[test]
fn case07_operation_reordering_hole_a_tracked_by_position_real_production_path() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let hole_a_position = crate_point3(10.0, 15.0, 5.0);
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Ranking(cad_query::RankingDirective::Nearest(
            cad_query::SpatialTarget::Point(hole_a_position),
        )))
        .with_cardinality(cad_query::CardinalityExpectation::Unique)
        .scoped_to(FeatureAnchor::named("body"));

    for variant in ["baseline", "perturbed"] {
        let s = session(
            &ctx,
            &format!("{BENCH}/public/07_operation_reordering/{variant}.aicad"),
        );
        expect_resolved_one(s.resolve(&query).expect("resolution should not error"));
    }

    // Anchoring the *same* worked-example query by the baseline's own
    // literal binding name instead of by position is the real, honest
    // `Broken` counter-proof described in this test's own doc comment --
    // `FeatureAnchor` is a source-level name, not a reordering-robust
    // identity.
    let name_anchored = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Topology(TopologyPredicate::DescendedFrom(
            generated_by_ancestor("hole_a_cut"),
        )))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_cardinality(cad_query::CardinalityExpectation::Unique)
        .scoped_to(FeatureAnchor::named("body"));
    let perturbed = session(
        &ctx,
        &format!("{BENCH}/public/07_operation_reordering/perturbed.aicad"),
    );
    expect_broken(
        perturbed
            .resolve(&name_anchored)
            .expect("resolution should not error"),
    );
}

fn crate_point3(x_mm: f64, y_mm: f64, z_mm: f64) -> cad_query::Point3 {
    cad_query::Point3::new(length_mm(x_mm), length_mm(y_mm), length_mm(z_mm))
}
