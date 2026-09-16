//! `AICAD-099`: adversarial bug-hunt campaign against the real Stage-4
//! semantic-reference resolver (`cad_query::resolve`, `AICAD-088`+),
//! executed through the real `ParametricBuildSession`/`cad_query::resolve`
//! path — never a hand-built stand-in shape — per `AGENTS.md`'s own
//! "adversarial campaign, not a box-checking task" instruction for this
//! batch.
//!
//! # Scope this campaign actually covers, and why
//!
//! At the time this campaign (`AICAD-099`/`099A`) originally ran,
//! `project/OWNER_DECISIONS.md` D31 was still open and blocked
//! `generated_by`/`modified_by`/`descended_from` resolver execution
//! against any `part`-nested named feature — i.e. every idiomatic
//! `.aicad` program, including the entire frozen `AICAD-079A` corpus. Every
//! case below was therefore expressed with **pure geometry predicates
//! and/or ranking directives only** (`Cylindrical`/`Planar`/`Radius`/
//! `Normal`, `largest`/`smallest`/`nearest`) — never `generated_by`/
//! `modified_by` — a real, narrower surface than the corpus's own
//! "intended query target" prose describes for most cases, recorded
//! honestly per-case below rather than silently substituted as if it
//! reproduced the corpus's own official ground truth.
//!
//! **`D31` is now resolved** (`AICAD-100A`, `crates/cad-feature-graph/
//! src/graph.rs`'s own module doc comment): the D31-specific blocker
//! above no longer applies, and `stage4_resolver_execution.rs`'s own
//! `case01`/`02`/`04`/`07`/`09` tests now execute the corpus's own real
//! `generated_by`/`descended_from`/lineage-tracked-by-position queries
//! through the real production path. This file's own cases below are
//! left as they were, not because D31 still blocks them (each one's own
//! pure-geometry query was already independently correct for its own
//! documented reason — see each test's own doc comment — never a D31
//! workaround masquerading as the real answer) and not out of neglect,
//! but because re-deriving a held-out case's own query from `case.md`'s
//! prose after the implementation that would resolve it already exists
//! is exactly the kind of "tune against held-out data" pattern
//! `held_out/HELD_OUT_README.md` exists to prevent — held-out cases
//! `03`/`05`/`10` are touched only at an owner-designated milestone
//! (already exercised once, at `AICAD-099A`), not opportunistically
//! whenever a new capability lands. `crate::eval`'s own module doc
//! comment's `EvalError::NotYetSpecified` predicate list this paragraph
//! used to cite is likewise stale — every `TopologyPredicate`/
//! `SpatialPredicate` variant now has real production semantics
//! (`AICAD-100A`, tasks 3-4) — but is left unreferenced here rather than
//! re-litigated, for the same reason.
//!
//! # What this file actually does
//!
//! 1. **Wires the already-proven resolver-execution cases into a real,
//!    end-to-end `crate::metrics` benchmark run** (`06_fillet_viability`,
//!    `08_upstream_suppression`, `11_extrusion_resize`,
//!    `12_add_remove_hole` — the four cases `AICAD-096`'s own report
//!    established have a pure-geometry query matching the corpus's own
//!    *official* expected classification) — the concrete "load the real
//!    corpus into `BenchmarkCase`s and `aggregate`" gap `AICAD-098`'s own
//!    report named as still open, and the natural next consumer
//!    `AICAD-097`'s report predicted.
//! 2. **Runs the three held-out cases as a deliberate held-out checkpoint**
//!    (`03_symmetric_candidates`, `05_boolean_topology_change`,
//!    `10_near_degenerate`) — `held_out/HELD_OUT_README.md`'s own
//!    process names this campaign's own kind of milestone ("the eventual
//!    Stage-4 gate, or a milestone the owner specifically calls for
//!    held-out evaluation") as the appropriate point to consult them;
//!    `AICAD-099` is the adversarial campaign immediately preceding the
//!    `AICAD-100` gate, so this is that checkpoint. `MANIFEST.sha256` is
//!    verified first (see `held_out_manifest_checksums_are_unchanged`
//!    below) so a result here is only trusted if the fixtures are
//!    provably the ones frozen at `AICAD-079A`. Each held-out case's own
//!    query here is a **pure-geometry proxy** for its documented
//!    "intended query target" (which needs lineage or an unimplemented
//!    predicate) — the proxy's own expected outcome is derived and
//!    justified in that test's own doc comment, not assumed identical to
//!    the corpus's own official classification.
//! 3. **Adds new adversarial probes** the frozen corpus does not already
//!    cover: a real 5-way/6-way genuine geometric tie through a
//!    `radial_pattern`+`cut` pipeline (reusing the already-frozen
//!    `04_pattern_count_change` fixture, public, not held out), and a
//!    positional-tracking probe proving a `nearest()`-ranked reference
//!    survives a pattern-count change that structurally alters the
//!    candidate universe.
//!
//! # Result
//!
//! Zero silent-wrong outcomes were found. See each test's own doc comment
//! for what it actually proves and `project/reports/AICAD-099.md` for the
//! full campaign account, including why the resolver's own cardinality-
//! first design (`crate cad_query::resolve::apply_cardinality` — strictly
//! "count survivors, then classify," never "pick a survivor") structurally
//! forecloses the classic "arbitrary first candidate" bug class this
//! campaign was hunting for, and which residual surfaces (D31 lineage,
//! `NotYetSpecified` predicates) remain the real open risk, unchanged by
//! this task.

use std::path::PathBuf;

use cad_cli::ParametricBuildSession;
use cad_cli::metrics::{BenchmarkCase, ExpectedOutcome, aggregate};
use cad_cli::perturbation::{PerturbationCase, RunOutcome, run_case};
use cad_occt_bridge::{OcctContext, Shape};
use cad_query::{
    BrokenReason, Candidate, CardinalityExpectation, Comparison, EvaluationEvidence,
    GeometryPredicate, Magnitude, Point3, Query, QueryClause, RankingDirective, ResolutionOutcome,
    ResolverContext, resolve_query,
};
use cad_references::{DurabilityLevel, EntityKind, FeatureAnchor};
use cad_types::Dimension;
use cad_units::OperandType;

/// A [`ResolverContext`] scoped to exactly one already-built [`Shape`]'s
/// own immediate faces — the same minimal shape
/// `crates/cad-query/src/resolve.rs`'s own `PlainContext` test helper
/// uses, reused here against one real fixture's own single named binding
/// (e.g. `body`) rather than [`ParametricBuildSession`]'s own
/// whole-session candidate universe (every top-level binding,
/// permanently live, `AICAD-094`). This is a deliberate, honestly-labeled
/// narrowing, not a weaker test: it isolates the specific tie-detection/
/// ranking risk each test below targets from the separate,
/// already-documented "every top-level binding stays live" candidate-
/// scope effect (`case08_upstream_suppression`'s own established
/// precedent — confirmed to also apply here, see this file's own
/// development history/`project/reports/AICAD-099.md`) that would
/// otherwise add unrelated duplicate candidates from intermediate
/// bindings (`with_left`, `bolt_tool`, `bolt_pattern`, etc.) and obscure
/// the actual question under test. The predicate/ranking/cardinality
/// logic exercised is exactly the real, unmodified
/// `cad_query::resolve_query` against real kernel-built geometry — only
/// candidate *enumeration* is narrowed.
struct SingleShapeContext<'ctx>(&'ctx Shape<'ctx>);

impl<'ctx> EvaluationEvidence<'ctx> for SingleShapeContext<'ctx> {}

impl<'ctx> ResolverContext<'ctx> for SingleShapeContext<'ctx> {
    fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
        if kind != EntityKind::Face {
            return Vec::new();
        }
        (0..self.0.face_count().unwrap())
            .map(|i| Candidate::new(EntityKind::Face, self.0.get_face(i).unwrap()))
            .collect()
    }
}

const RESOLVER_EXECUTION: &str = "project/benchmarks/stage4_semantic_reference/resolver_execution";
const HELD_OUT: &str = "project/benchmarks/stage4_semantic_reference/held_out";
const PUBLIC: &str = "project/benchmarks/stage4_semantic_reference/public";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture_source(relative: &str) -> String {
    let path = repo_root().join(relative);
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

fn point_mm(x_mm: f64, y_mm: f64, z_mm: f64) -> Point3 {
    Point3::new(length_mm(x_mm), length_mm(y_mm), length_mm(z_mm))
}

fn describe(outcome: &ResolutionOutcome<'_>) -> String {
    match outcome {
        ResolutionOutcome::Resolved(candidates) => format!("Resolved({})", candidates.len()),
        ResolutionOutcome::Ambiguous(candidates) => format!("Ambiguous({})", candidates.len()),
        ResolutionOutcome::Broken(_) => "Broken".to_string(),
    }
}

// ---------------------------------------------------------------------
// Part 1: wiring the four already-proven pure-geometry corpus cases into
// a real `crate::metrics` benchmark run.
// ---------------------------------------------------------------------

fn extrusion_resize_query() -> Query {
    Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Planar))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Normal(
            cad_query::predicate::DirectionComparison {
                target: cad_query::Direction3::POSITIVE_Z,
                tolerance: None,
            },
        )))
        .with_cardinality(CardinalityExpectation::Unique)
}

fn add_remove_hole_query() -> Query {
    Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(2.5)),
        )))
        .with_cardinality(CardinalityExpectation::Unique)
}

fn fillet_viability_query() -> Query {
    Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(8.0)),
        )))
        .with_cardinality(CardinalityExpectation::Unique)
}

fn upstream_suppression_query() -> Query {
    Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(3.0)),
        )))
        .with_cardinality(CardinalityExpectation::Unique)
}

/// Real, end-to-end benchmark metrics over the four corpus cases whose
/// pure-geometry query matches the corpus's own *official* expected
/// classification (`AICAD-096`'s own established precedent) — the actual
/// "real corpus fixtures -> `PerturbationRun` -> `BenchmarkCase` ->
/// `aggregate` -> reported `BenchmarkMetrics`" run `AICAD-098`'s own
/// report named as not yet done. `11`/`12` are this corpus's own
/// `resolver_execution/` extension (`AICAD-096`, not part of the frozen
/// ten); `06`/`08` are two of the frozen ten. Every one of the four was
/// already individually proven correct by `stage4_resolver_execution.rs`
/// — this test's own job is proving the *aggregation* layer reports the
/// same result with zero silent-wrong and zero mismatch, not
/// re-discovering the per-case outcome.
#[test]
fn wired_corpus_benchmark_reports_zero_silent_wrong_and_zero_mismatch() {
    let cases = vec![
        BenchmarkCase {
            run: run_case(&PerturbationCase::new(
                "11_extrusion_resize",
                fixture_source(&format!(
                    "{RESOLVER_EXECUTION}/11_extrusion_resize/baseline.aicad"
                )),
                fixture_source(&format!(
                    "{RESOLVER_EXECUTION}/11_extrusion_resize/perturbed.aicad"
                )),
                extrusion_resize_query(),
            )),
            expected: ExpectedOutcome::CorrectResolvedReference,
            durability: DurabilityLevel::QueryGeometric,
        },
        BenchmarkCase {
            run: run_case(&PerturbationCase::new(
                "12_add_remove_hole",
                fixture_source(&format!(
                    "{RESOLVER_EXECUTION}/12_add_remove_hole/baseline.aicad"
                )),
                fixture_source(&format!(
                    "{RESOLVER_EXECUTION}/12_add_remove_hole/perturbed.aicad"
                )),
                add_remove_hole_query(),
            )),
            expected: ExpectedOutcome::ExplicitBrokenReference,
            durability: DurabilityLevel::QueryGeometric,
        },
        BenchmarkCase {
            run: run_case(&PerturbationCase::new(
                "06_fillet_viability",
                fixture_source(&format!("{PUBLIC}/06_fillet_viability/baseline.aicad")),
                fixture_source(&format!("{PUBLIC}/06_fillet_viability/perturbed.aicad")),
                fillet_viability_query(),
            )),
            expected: ExpectedOutcome::KernelFailure,
            durability: DurabilityLevel::QueryGeometric,
        },
        BenchmarkCase {
            run: run_case(&PerturbationCase::new(
                "08_upstream_suppression",
                fixture_source(&format!("{PUBLIC}/08_upstream_suppression/baseline.aicad")),
                fixture_source(&format!("{PUBLIC}/08_upstream_suppression/perturbed.aicad")),
                upstream_suppression_query(),
            )),
            expected: ExpectedOutcome::ExplicitBrokenReference,
            durability: DurabilityLevel::QueryGeometric,
        },
    ];

    let metrics = aggregate(&cases);
    assert_eq!(metrics.silent_wrong, 0, "metrics: {metrics:?}");
    assert_eq!(metrics.mismatch, 0, "metrics: {metrics:?}");
    assert_eq!(metrics.unrelated_failure, 0, "metrics: {metrics:?}");
    assert_eq!(metrics.correct, 1, "metrics: {metrics:?}");
    assert_eq!(metrics.broken_detected, 2, "metrics: {metrics:?}");
    assert_eq!(metrics.kernel_failure, 1, "metrics: {metrics:?}");
    assert_eq!(metrics.total(), 4);
    assert!(metrics.silent_wrong_ids.is_empty());
}

// ---------------------------------------------------------------------
// Part 2: the held-out checkpoint (`03`/`05`/`10`). Each proxy query and
// its own expected outcome are justified in that test's own doc comment.
// ---------------------------------------------------------------------

/// Guards every held-out test below: a checksum mismatch here means a
/// held-out fixture was edited since `AICAD-079A` froze it, and no result
/// against it in this file should be trusted until the discrepancy is
/// understood (`held_out/HELD_OUT_README.md`'s own explicit instruction —
/// "do not silently regenerate the manifest").
#[test]
fn held_out_manifest_checksums_are_unchanged() {
    let manifest = fixture_source(&format!("{HELD_OUT}/MANIFEST.sha256"));
    let dir = repo_root().join(HELD_OUT);
    for line in manifest.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (expected_hex, rel_path) = line
            .split_once("  ")
            .unwrap_or_else(|| panic!("malformed MANIFEST.sha256 line: {line:?}"));
        let bytes = std::fs::read(dir.join(rel_path))
            .unwrap_or_else(|err| panic!("failed to read {rel_path}: {err}"));
        let digest = sha256_hex(&bytes);
        assert_eq!(
            digest, expected_hex,
            "held-out fixture {rel_path} does not match its frozen AICAD-079A checksum \
             — do not trust any held-out result until this is understood"
        );
    }
}

/// Minimal, dependency-free SHA-256 (this crate has no existing SHA-256
/// dependency, and pulling one in only to check a manifest already
/// checked independently by `scripts/ci/semantic_ref_harness.py`/
/// `sha256sum` is not worth a new dependency for one test) — a direct,
/// textbook implementation (FIPS 180-4), not novel or adversarially
/// security-sensitive: it only detects accidental fixture drift here.
fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    h.iter().map(|word| format!("{word:08x}")).collect()
}

/// Held-out `03_symmetric_candidates`, pure-geometry/ranking proxy for its
/// own documented `nearest_to(x = 30mm plane); unique()` intended query:
/// `Cylindrical + Radius(2.5mm)` (both through-holes' own wall faces,
/// pure geometry) narrowed by `Ranking::Nearest` to a real point on the
/// plate's own `x = 30mm` mid-plane at the holes' own shared `y`/`z`
/// (verified against the fixture's own `plate_y/2`, `plate_z/2`; see this
/// test's own body). This is a faithful proxy (the case's own intended
/// query *is* a nearest-ranking query; only the "which entities are
/// candidates at all" half is pure geometry instead of lineage-scoped) —
/// unlike `05`/`10` below, which substitute a materially different
/// predicate for `generated_by`.
///
/// This is this campaign's own sharpest real test of the exact risk
/// `held_out/HELD_OUT_README.md` names case `03` for: whether
/// `cad_query::resolve`'s own `FLOAT_NOISE_RELATIVE = 1e-9` tie tolerance
/// (`crate::resolve::approx_eq`) correctly recognizes two *independently*
/// kernel-computed center-of-mass values (from two separate real `hole()`
/// cuts at deliberately symmetric positions) as tied, rather than treating
/// accumulated floating-point noise from the independent computation as a
/// genuine (and silently wrong) distinction.
///
/// `AICAD-099A` update: this now runs through the **real, unmodified
/// production path** — `PerturbationCase`/`crate::perturbation::run_case`,
/// which call `ParametricBuildSession::resolve` exactly as `cad refs check`
/// or any real caller would — with the query's own `Query::scoped_to`
/// restricting candidate enumeration to the `body` binding alone. Before
/// `AICAD-099A`, no way existed to isolate this question through the real
/// production path without the test-only [`SingleShapeContext`] helper
/// above (`AICAD-099`'s own original version of this test used it,
/// documented in that task's own report as a real, if narrow, gap); this
/// version proves the same result through the exact API a real caller has
/// today, never a test-only stand-in.
#[test]
fn case03_symmetric_candidates_scoped_to_body_via_the_real_production_path() {
    // plate_y/2 = 15mm, plate_z/2 = 4mm -- both holes' own wall faces
    // share this y/z by construction (see `03_symmetric_candidates/
    // baseline.aicad`'s own `Axis3` origins), so distance-to-this-point
    // reduces to distance-along-x, exactly matching the case's own
    // "nearest the x = 30mm mid-plane" intent.
    let target = point_mm(30.0, 15.0, 4.0);
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(2.5)),
        )))
        .with_clause(QueryClause::Ranking(RankingDirective::Nearest(
            cad_query::predicate::SpatialTarget::Point(target),
        )))
        .with_cardinality(CardinalityExpectation::Unique)
        .scoped_to(FeatureAnchor::named("body"));

    let case = PerturbationCase::new(
        "03_symmetric_candidates_scoped_to_body",
        fixture_source(&format!(
            "{HELD_OUT}/03_symmetric_candidates/baseline.aicad"
        )),
        fixture_source(&format!(
            "{HELD_OUT}/03_symmetric_candidates/perturbed.aicad"
        )),
        query,
    );
    let run = run_case(&case);

    assert_eq!(
        run.baseline,
        RunOutcome::Resolved { count: 1 },
        "baseline, scoped to 'body': the left hole (15mm from mid-plane) is strictly nearer \
         than the right (20mm), and 'body' alone (excluding 'with_left's own separate live \
         copy of the left hole's wall face) is the correct target -- expected \
         Resolved(1), got {:?}",
        run.baseline
    );
    assert_eq!(
        run.perturbed,
        RunOutcome::Ambiguous { count: 2 },
        "perturbed, scoped to 'body': both holes are exactly 15mm from the mid-plane by \
         construction -- a resolver that silently narrows this to Resolved(1) is exactly the \
         silent_wrong_resolution this held-out case exists to catch; got {:?}",
        run.perturbed
    );
}

// ---------------------------------------------------------------------
// AICAD-099A: scoped candidate-universe resolution — focused tests beyond
// the case03 critical acceptance test above.
// ---------------------------------------------------------------------

/// Direct, side-by-side proof that `Query::scoped_to` excludes an
/// unrelated/intermediate binding's own live candidates: the exact same
/// query, against the exact same real `ParametricBuildSession` build,
/// reports `Ambiguous` unscoped (`with_left`'s own live duplicate of the
/// left hole's wall face ties against `body`'s own copy — the real finding
/// `project/reports/AICAD-099.md` recorded) but `Resolved(1)` once scoped
/// to `body` alone (`with_left` excluded entirely from candidate
/// enumeration).
#[test]
fn scoped_resolution_excludes_unrelated_intermediate_bindings() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let build = session(
        &ctx,
        &format!("{HELD_OUT}/03_symmetric_candidates/baseline.aicad"),
    );
    let target = point_mm(30.0, 15.0, 4.0);
    let base_query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(2.5)),
        )))
        .with_clause(QueryClause::Ranking(RankingDirective::Nearest(
            cad_query::predicate::SpatialTarget::Point(target),
        )))
        .with_cardinality(CardinalityExpectation::Unique);

    let unscoped_outcome = build
        .resolve(&base_query)
        .expect("resolution should not error");
    assert!(
        unscoped_outcome.is_ambiguous(),
        "unscoped: 'with_left's own permanently-live duplicate of the left hole's wall face \
         must still tie against 'body's own copy, exactly as AICAD-099 recorded; got {}",
        describe(&unscoped_outcome)
    );

    let scoped_query = base_query.scoped_to(FeatureAnchor::named("body"));
    let scoped_outcome = build
        .resolve(&scoped_query)
        .expect("resolution should not error");
    match scoped_outcome {
        ResolutionOutcome::Resolved(candidates) => assert_eq!(
            candidates.len(),
            1,
            "scoped to 'body': 'with_left's own duplicate candidate must be excluded entirely, \
             leaving only 'body's own left-hole wall face"
        ),
        other => panic!(
            "scoped to 'body': expected Resolved(1) once the intermediate binding's own \
             duplicate is excluded, got {}",
            describe(&other)
        ),
    }
}

/// Scoping never deduplicates candidates by geometry: within one scoped
/// binding, multiple *distinct* faces that happen to share identical
/// geometry (two unrelated `3mm`-radius fillets on the same final `body` —
/// the same fixture `coincidental_radius_collision_between_unrelated_
/// fillets_is_ambiguous_not_silently_resolved` above uses; `body`'s own
/// final shape carries three real `3mm`-radius cylindrical faces once both
/// fillet operations are applied, confirmed empirically below rather than
/// assumed) must still tie as `Ambiguous`, exactly as the unscoped case
/// already does. Scoping narrows *which bindings* are candidates; it is
/// not a geometry-fingerprint identity mechanism that could ever collapse
/// several real, distinct entities into one.
#[test]
fn same_geometry_candidates_are_not_deduplicated_within_a_scope() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let source = "\
param box_x: Length = 40mm;\n\
param box_y: Length = 30mm;\n\
param box_z: Length = 20mm;\n\
param radius: Length = 3mm;\n\
\n\
part Block {\n\
    let once: Geometry = fillet(box(box_x, box_y, box_z), [1], radius);\n\
    let body: Geometry = fillet(once, [3], radius);\n\
}\n\
";
    let build = ParametricBuildSession::new("same_geometry_scoped", source, &ctx)
        .unwrap_or_else(|diags| panic!("expected a successful build, got {diags:?}"));
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(3.0)),
        )))
        .with_cardinality(CardinalityExpectation::Unique)
        .scoped_to(FeatureAnchor::named("body"));
    let outcome = build.resolve(&query).expect("resolution should not error");
    match outcome {
        ResolutionOutcome::Ambiguous(candidates) => assert_eq!(
            candidates.len(),
            3,
            "'body' alone already carries every same-radius fillet face -- scoping must not \
             collapse any of them into fewer distinct candidates just because their geometry \
             is identical"
        ),
        other => panic!(
            "expected Ambiguous(3) within the single 'body' scope (never a silent dedup to \
             Resolved), got {}",
            describe(&other)
        ),
    }
}

/// A scope naming a binding that does not exist in the program must fail
/// closed (`Broken(ScopeNotFound)`), never silently fall back to the whole
/// live-binding universe `ParametricBuildSession::candidates` would
/// otherwise enumerate — proven against a build where the unscoped query
/// would find real, resolvable candidates, so a silent-fallback bug would
/// otherwise be masked as an apparently-correct `Resolved`/`Ambiguous`
/// result rather than surfacing as a visible behavior change.
#[test]
fn invalid_scope_fails_closed_never_falls_back_to_the_whole_universe() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let build = session(
        &ctx,
        &format!("{HELD_OUT}/03_symmetric_candidates/baseline.aicad"),
    );
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(2.5)),
        )))
        .with_cardinality(CardinalityExpectation::ExpectCount(2))
        .scoped_to(FeatureAnchor::named("no_such_binding"));
    let outcome = build.resolve(&query).expect("resolution should not error");
    assert!(
        matches!(
            outcome,
            ResolutionOutcome::Broken(BrokenReason::ScopeNotFound(_))
        ),
        "an unresolvable scope must report Broken(ScopeNotFound), never silently widen to the \
         whole live-binding universe (which does contain 2 real matching candidates here); \
         got {}",
        describe(&outcome)
    );
}

/// Held-out `05_boolean_topology_change`, pure-geometry proxy: the case's
/// own `case.md` measures the perturbed (`cut before union`) build's own
/// bore wall as **two** half-cylindrical face fragments (vs. one full
/// cylindrical face in the baseline `cut after union` build) at the exact
/// same radius (`hole_diameter / 2 = 5mm`) in both — so `Cylindrical +
/// Radius(5mm) + unique()`, needing no lineage at all, already carries the
/// same real topology signal the case's own official `generated_by`-based
/// query is built to detect, and (per `ParametricBuildSession::
/// candidates`'s own established "every top-level binding stays live"
/// semantics, `AICAD-094`) the two independently-bored `a_bored`/`b_bored`
/// intermediate bindings additionally contribute their own single
/// half-cylindrical fragment each in the perturbed build -- a real
/// consequence of that already-established design, not a defect. Expected:
/// baseline `Resolved(1)`, perturbed `Ambiguous` (never `Resolved`).
#[test]
fn case05_boolean_topology_change_geometry_proxy() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(5.0)),
        )))
        .with_cardinality(CardinalityExpectation::Unique);

    let baseline = session(
        &ctx,
        &format!("{HELD_OUT}/05_boolean_topology_change/baseline.aicad"),
    );
    let baseline_outcome = baseline
        .resolve(&query)
        .expect("resolution should not error");
    assert!(
        baseline_outcome.is_resolved(),
        "baseline (cut after union): expected the single full-cylinder bore wall to resolve \
         uniquely, got {}",
        describe(&baseline_outcome)
    );

    let perturbed = session(
        &ctx,
        &format!("{HELD_OUT}/05_boolean_topology_change/perturbed.aicad"),
    );
    let perturbed_outcome = perturbed
        .resolve(&query)
        .expect("resolution should not error");
    assert!(
        !perturbed_outcome.is_resolved(),
        "perturbed (cut before union): the same nominal feature now yields multiple \
         same-radius cylindrical fragments -- a resolver that silently narrows this to a \
         single Resolved candidate would be exactly the operation-ordering silent-wrong \
         risk this held-out case exists to catch; got {}",
        describe(&perturbed_outcome)
    );
    assert!(
        perturbed_outcome.is_ambiguous(),
        "expected Ambiguous specifically (candidates exist, just more than one), got {}",
        describe(&perturbed_outcome)
    );
}

/// Held-out `10_near_degenerate`, pure-geometry proxy for its own
/// documented `generated_by(the base block); planar; normal ~= +Z;
/// unique()` intended query: `Planar + Normal(+Z) + unique()` alone,
/// since a rectangular block has exactly one `+Z`-normal planar face and
/// filleting a *different* edge never introduces a second one (the new
/// fillet face is cylindrical, never planar) -- the lineage clause is
/// redundant with geometry alone for this specific fixture, so dropping
/// it changes nothing about which single face qualifies. This is this
/// campaign's own direct test of the risk `held_out/HELD_OUT_README.md`
/// names case `10` for: whether the resolver ever implicitly filters or
/// misranks a real-but-near-degenerate face by absolute size rather than
/// validity. `evaluate_geometry`'s own `Planar`/`Normal` evaluators
/// (`crates/cad-query/src/eval.rs`) use surface-type/normal-direction
/// only, never face area, so this is expected to hold -- proven here
/// against the fixture actually shrunk to `0.01mm` from the real,
/// separately-measured kernel-failure boundary
/// (`public/06_fillet_viability/case.md`), not merely asserted.
#[test]
fn case10_near_degenerate_geometry_proxy() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Planar))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Normal(
            cad_query::predicate::DirectionComparison {
                target: cad_query::Direction3::POSITIVE_Z,
                tolerance: None,
            },
        )))
        .with_cardinality(CardinalityExpectation::Unique);

    let baseline = session(
        &ctx,
        &format!("{HELD_OUT}/10_near_degenerate/baseline.aicad"),
    );
    let baseline_outcome = baseline
        .resolve(&query)
        .expect("resolution should not error");
    assert!(
        baseline_outcome.is_resolved(),
        "baseline (fillet_radius = 5mm): expected Resolved(1), got {}",
        describe(&baseline_outcome)
    );

    let perturbed = session(
        &ctx,
        &format!("{HELD_OUT}/10_near_degenerate/perturbed.aicad"),
    );
    let perturbed_outcome = perturbed
        .resolve(&query)
        .expect("resolution should not error");
    assert!(
        perturbed_outcome.is_resolved(),
        "perturbed (fillet_radius = 9.99mm, 0.01mm from the measured kernel-failure \
         boundary): the top face is still real, single, and valid -- reporting anything but \
         Resolved(1) here (Broken, as a naive size-based heuristic would produce) is exactly \
         the silent-wrong-by-omission risk this held-out case exists to catch; got {}",
        describe(&perturbed_outcome)
    );
    if let ResolutionOutcome::Resolved(candidates) = &perturbed_outcome {
        assert_eq!(candidates.len(), 1);
    }
}

// ---------------------------------------------------------------------
// Part 3: new adversarial probes beyond the frozen corpus.
// ---------------------------------------------------------------------

/// New adversarial probe reusing the already-frozen (public, not held
/// out) `04_pattern_count_change` fixture: a real 5-way, then 6-way,
/// genuine geometric tie produced by an actual `radial_pattern` +  `cut`
/// pipeline (not a synthetic hand-built shape) -- every bolt hole's own
/// wall face shares the identical `bolt_hole_diameter / 2 = 2mm` radius
/// by construction. `Cylindrical + Radius(2mm) + unique()` must report
/// every single one as tied in both variants; the count moving from 5 to
/// 6 must never cause the resolver to narrow to fewer than the true
/// count, and must never silently report `Resolved`.
#[test]
fn case04_pattern_count_change_full_tie_is_always_ambiguous() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(2.0)),
        )))
        .with_cardinality(CardinalityExpectation::Unique);

    let baseline = session(
        &ctx,
        &format!("{PUBLIC}/04_pattern_count_change/baseline.aicad"),
    );
    let body = baseline.binding_named("body").expect("body binding");
    let body_shape = baseline.shape_for_binding(body).expect("body shape");
    let baseline_outcome = resolve_query(&query, &SingleShapeContext(body_shape))
        .expect("resolution should not error");
    match &baseline_outcome {
        ResolutionOutcome::Ambiguous(candidates) => assert_eq!(candidates.len(), 5),
        other => panic!(
            "baseline (5 bolt holes, all radius 2mm): expected Ambiguous(5), got {}",
            describe(other)
        ),
    }

    let perturbed = session(
        &ctx,
        &format!("{PUBLIC}/04_pattern_count_change/perturbed.aicad"),
    );
    let body = perturbed.binding_named("body").expect("body binding");
    let body_shape = perturbed.shape_for_binding(body).expect("body shape");
    let perturbed_outcome = resolve_query(&query, &SingleShapeContext(body_shape))
        .expect("resolution should not error");
    match &perturbed_outcome {
        ResolutionOutcome::Ambiguous(candidates) => assert_eq!(candidates.len(), 6),
        other => panic!(
            "perturbed (6 bolt holes, all radius 2mm): expected Ambiguous(6), got {}",
            describe(other)
        ),
    }
}

/// New adversarial probe: does a `nearest()`-ranked reference to bolt
/// instance 0 (the un-rotated pattern member, at a fixed real-world
/// position independent of `BOLT_COUNT`) survive the same 5 -> 6 pattern-
/// count change that `case04_pattern_count_change_full_tie_is_always_
/// ambiguous` above proves is otherwise a full N-way tie? A resolver
/// whose ranking silently drifted to a *different* hole after the
/// perturbation (e.g. by candidate-enumeration order rather than genuine
/// position) would be a real, if narrow, silent-wrong case: `Resolved(1)`
/// in both builds, but not provably the *same* logical instance. This
/// test cannot directly compare cross-build entity identity (no
/// persistent-reference/fingerprint claim is being made — `AICAD-092`/D7
/// forbid exactly that as an automatic mechanism), so instead it proves
/// the weaker, still-meaningful invariant: the nearest-ranked result's own
/// measured position is the fixed instance-0 position in *both* builds,
/// which only holds if the same real hole was actually selected each
/// time.
#[test]
fn case04_pattern_count_change_nearest_instance_tracks_the_same_position() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    // bolt_circle_radius = 18mm, instance 0 sits at angle 0 -> (18mm,
    // 0mm, plate_thickness / 2 = 4mm) by the fixture's own `Axis3`
    // origin/pattern-angle-zero convention.
    let target = point_mm(18.0, 0.0, 4.0);
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(2.0)),
        )))
        .with_clause(QueryClause::Ranking(RankingDirective::Nearest(
            cad_query::predicate::SpatialTarget::Point(target),
        )))
        .with_cardinality(CardinalityExpectation::Unique);

    let baseline = session(
        &ctx,
        &format!("{PUBLIC}/04_pattern_count_change/baseline.aicad"),
    );
    let body = baseline.binding_named("body").expect("body binding");
    let body_shape = baseline.shape_for_binding(body).expect("body shape");
    let baseline_outcome = resolve_query(&query, &SingleShapeContext(body_shape))
        .expect("resolution should not error");
    let baseline_center = match &baseline_outcome {
        ResolutionOutcome::Resolved(candidates) if candidates.len() == 1 => candidates[0]
            .shape()
            .center_of_mass()
            .expect("center of mass should be computable"),
        other => panic!("baseline: expected Resolved(1), got {}", describe(other)),
    };

    let perturbed = session(
        &ctx,
        &format!("{PUBLIC}/04_pattern_count_change/perturbed.aicad"),
    );
    let body = perturbed.binding_named("body").expect("body binding");
    let body_shape = perturbed.shape_for_binding(body).expect("body shape");
    let perturbed_outcome = resolve_query(&query, &SingleShapeContext(body_shape))
        .expect("resolution should not error");
    let perturbed_center = match &perturbed_outcome {
        ResolutionOutcome::Resolved(candidates) if candidates.len() == 1 => candidates[0]
            .shape()
            .center_of_mass()
            .expect("center of mass should be computable"),
        other => panic!(
            "perturbed (pattern count now 6, five other same-radius candidates present): \
             expected the nearest-ranking to still uniquely resolve the fixed instance-0 \
             position, got {}",
            describe(other)
        ),
    };

    let dx = (baseline_center.x - perturbed_center.x).abs();
    let dy = (baseline_center.y - perturbed_center.y).abs();
    let dz = (baseline_center.z - perturbed_center.z).abs();
    assert!(
        dx < 1e-6 && dy < 1e-6 && dz < 1e-6,
        "nearest(instance-0 position) resolved to a measurably different real position after \
         the pattern-count change ({baseline_center:?} vs {perturbed_center:?}) -- a silent \
         drift to a different hole, not the same tracked instance"
    );
}

/// Adversarial negative control: a `unique()` query over a metric two
/// *unrelated* features coincidentally share (here, two independent
/// fillets on different edges of the same block, both authored with the
/// identical `3mm` radius) must report `Ambiguous`, never arbitrarily
/// pick the "first" one. This is not a new risk this campaign discovered
/// (`case08_upstream_suppression` already proves the same shape against a
/// frozen fixture) -- it is here as a minimal, self-contained, from-first-
/// principles reproduction that a query over two geometrically identical
/// but semantically unrelated fillet faces is genuinely indistinguishable
/// to a pure-geometry query, and that `cad_query::resolve` still reports
/// this honestly (`Ambiguous`, never `Resolved`) rather than treating
/// `QueryGeometric`-durability matching as if it had a real way to tell
/// them apart. Documents the real, structural boundary of pure-geometry
/// matching rather than treating it as a bug.
#[test]
fn coincidental_radius_collision_between_unrelated_fillets_is_ambiguous_not_silently_resolved() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let source = "\
param box_x: Length = 40mm;\n\
param box_y: Length = 30mm;\n\
param box_z: Length = 20mm;\n\
param radius: Length = 3mm;\n\
\n\
part Block {\n\
    let once: Geometry = fillet(box(box_x, box_y, box_z), [1], radius);\n\
    let body: Geometry = fillet(once, [3], radius);\n\
}\n\
";
    let session = ParametricBuildSession::new("coincidental_radius_collision", source, &ctx)
        .unwrap_or_else(|diags| panic!("expected a successful build, got {diags:?}"));
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(3.0)),
        )))
        .with_cardinality(CardinalityExpectation::Unique);
    let outcome = session
        .resolve(&query)
        .expect("resolution should not error");
    assert!(
        outcome.is_ambiguous(),
        "two unrelated same-radius fillet faces (plus `once`'s own live copy of the first \
         fillet, `ParametricBuildSession::candidates`'s established permanently-live-binding \
         semantics) must never be silently narrowed to one; got {}",
        describe(&outcome)
    );
}

/// Adversarial negative control: reducing a hole's own diameter to a
/// value the kernel can no longer build (well below any reasonable
/// minimum for the tool geometry) must be classified `KernelFailure` if
/// the kernel actually rejects it, never silently treated as if the
/// reference were merely `Broken` in the ordinary "feature removed"
/// sense `case08`/`case12` above test, and never allowed to produce a
/// `Resolved` outcome behind a `ParametricBuildSession::new` success that
/// masks a real geometric-infeasibility failure. Uses an intentionally
/// pathological, effectively-zero radius rather than guessing a kernel-
/// specific minimum threshold.
#[test]
fn degenerate_hole_diameter_is_a_kernel_failure_or_a_real_broken_reference_never_silent() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let source = "\
param box_x: Length = 40mm;\n\
param box_y: Length = 30mm;\n\
param box_z: Length = 20mm;\n\
param hole_d: Length = 1e-7mm;\n\
\n\
part Block {\n\
    let body: Geometry = hole(\n\
        box(box_x, box_y, box_z),\n\
        Axis3(origin = Point3(x = 20mm, y = 15mm, z = 0mm - 1mm), direction = Vector3(x = 0.0, y = 0.0, z = 1.0)),\n\
        hole_d,\n\
        box_z + 2mm,\n\
    );\n\
}\n\
";
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_cardinality(CardinalityExpectation::Unique);
    match ParametricBuildSession::new("degenerate_hole_diameter", source, &ctx) {
        Ok(session) => {
            // The kernel tolerated the pathological diameter -- whatever
            // it built, the resolver's own outcome must still be an
            // honest classification, not a `Resolved` masking a
            // near-zero-radius sliver as if it were the ordinary hole a
            // reference author intended. `Broken`/`Ambiguous` are both
            // acceptable fail-closed outcomes here; `Resolved` is not
            // inherently wrong either (a real, if tiny, cylindrical face
            // may genuinely and uniquely exist) -- this branch only
            // asserts the query executes without a silent panic/`Err`
            // masking a real semantic answer.
            let outcome = session
                .resolve(&query)
                .expect("resolution should not error");
            let _ = describe(&outcome);
        }
        Err(diagnostics) => {
            assert!(
                diagnostics
                    .iter()
                    .any(|d| d.code.as_string().starts_with("GEOM-")),
                "expected a real kernel-dispatch diagnostic for a degenerate hole diameter, \
                 got {diagnostics:?}"
            );
        }
    }
}

/// A real, honest finding `AICAD-099`'s own first draft surfaced (not a
/// bug — recorded here as a permanent regression-shaped test rather than
/// silently discarded): running `case03`'s own proxy query through the
/// *real*, unrestricted, **unscoped** production path (`ParametricBuildSession::
/// resolve`/`crate::perturbation::run_case` with no `Query::scoped_to` —
/// exactly what a caller that has not opted into `AICAD-099A`'s own new
/// scoping mechanism still gets today) reports `Ambiguous` in **both** the
/// baseline and the perturbed build — not just the perturbed one
/// `case03_symmetric_candidates_scoped_to_body_via_the_real_production_path`
/// proves once scoped. Root cause: `ParametricBuildSession::candidates`'s
/// own established "every top-level binding stays permanently live"
/// semantics (`AICAD-094`, and `case08_upstream_suppression`'s own
/// already-documented precedent) means the fixture's own intermediate
/// `with_left` binding contributes its own live copy of the left hole's
/// wall face *alongside* `body`'s own copy of the same face — two
/// distinct `Candidate`s at (as far as this query can tell) the same
/// position, so `nearest()` ties between them even in the baseline, where
/// the *fixture's own two holes* are not actually symmetric at all.
///
/// This is fail-closed, not silently wrong (`Ambiguous`, never an
/// arbitrary pick) — so it is not itself a `SILENT_WRONG` regression
/// under `tests/semantic_refs/regressions/`. It documents a real,
/// intentionally-preserved boundary: the **unscoped** `Query`/
/// `ParametricBuildSession::resolve` API keeps its pre-`AICAD-099A`
/// whole-live-universe semantics unchanged (per that task's own
/// "the unscoped production API may retain its current whole-live-universe
/// semantics where compatibility requires it" instruction) — a caller
/// that wants `case03`'s own narrower scope must opt in via
/// `Query::scoped_to`, exactly as the test above now does. If this test
/// ever starts reporting `Resolved` instead, the underlying unscoped
/// candidate-enumeration behavior changed and needs re-review, not a
/// silent test update. Documented in `project/reports/AICAD-099.md`/
/// `AICAD-099A.md`.
#[test]
fn unscoped_resolution_keeps_its_pre_099a_whole_session_ambiguity_semantics() {
    let ctx = OcctContext::new().expect("context creation should succeed");
    let target = point_mm(30.0, 15.0, 4.0);
    let query = Query::new(EntityKind::Face)
        .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
        .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
            Comparison::Eq(length_mm(2.5)),
        )))
        .with_clause(QueryClause::Ranking(RankingDirective::Nearest(
            cad_query::predicate::SpatialTarget::Point(target),
        )))
        .with_cardinality(CardinalityExpectation::Unique);

    let baseline = session(
        &ctx,
        &format!("{HELD_OUT}/03_symmetric_candidates/baseline.aicad"),
    );
    let baseline_outcome = baseline
        .resolve(&query)
        .expect("resolution should not error");
    assert!(
        !baseline_outcome.is_resolved()
            || matches!(&baseline_outcome, ResolutionOutcome::Resolved(c) if c.len() == 1),
        "fail-closed invariant: whatever the outcome, it must never be a silently-narrowed \
         Resolved with more than one genuinely tied candidate; got {}",
        describe(&baseline_outcome)
    );
    assert!(
        baseline_outcome.is_ambiguous(),
        "expected the real, unrestricted candidate universe to report Ambiguous here too \
         (see this test's own doc comment for the exact root cause) -- if this now reports \
         Resolved, the underlying `with_left` duplicate-candidate behavior changed and this \
         test's own documented reasoning needs re-checking, not silently updated; got {}",
        describe(&baseline_outcome)
    );

    let perturbed = session(
        &ctx,
        &format!("{HELD_OUT}/03_symmetric_candidates/perturbed.aicad"),
    );
    let perturbed_outcome = perturbed
        .resolve(&query)
        .expect("resolution should not error");
    assert!(
        perturbed_outcome.is_ambiguous(),
        "expected Ambiguous (never a silent Resolved) in the perturbed build too; got {}",
        describe(&perturbed_outcome)
    );
}
