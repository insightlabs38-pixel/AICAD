//! Parametric build orchestration (`AICAD-079B` Stage-3 gate remediation).
//!
//! # The gap this closes
//!
//! The Stage-3 owner gate (`project/gates/stage-3-gate.md`, prepared by
//! `AICAD-079B`) recommended **PASS** but flagged one real gap: `cad_runtime::
//! params::ParamModel` and `cad_feature_graph::FeatureGraph` were each
//! independently proven correct (`STAGE3-A_PARAMETRIC_GRAPH.md`), but no
//! production `cad-cli` execution path ever connected them — `crate::build::
//! build_source`'s own pipeline calls `Interpreter::run_top_level` (plain
//! Stage-2 source-order evaluation, no parameter identity/dependency
//! semantics at all) and `cad_geometry_runtime::dispatch_graph` (an
//! unconditional full-graph kernel dispatch, no reuse of any prior build's
//! own results). `CURRENT_STAGE.md`'s own exit-gate text ("parameter edit ->
//! dirty propagation/incremental rebuild -> correct affected geometry")
//! describes a pipeline this codebase did not yet execute as one connected
//! path. This module is that connection.
//!
//! # Root cause found while connecting them
//!
//! Wiring `Interpreter::run_top_level_parametric` into a real build
//! immediately surfaced a genuine, previously-undiscovered defect: it
//! evaluated every top-level `let`/`const` *before* any `param`, so a `let`
//! referencing an earlier `param` — the ordinary, universal Stage-3 pattern
//! every fixture under `examples/`/`project/benchmarks/` uses — failed with
//! `RuntimeError::UnboundValue`. No existing test exercised that shape
//! (every `run_top_level_parametric` test used params only, no geometry
//! `let`s), which is why the defect went undetected until this integration
//! was actually attempted. Fixed at the root (`crates/cad-runtime/src/
//! interp.rs`'s own `run_top_level_parametric`, params now evaluated first,
//! in `ParamModel`'s own dependency-ordered schedule, then `let`/`const` in
//! source order) rather than worked around here.
//!
//! # What this module actually connects
//!
//! [`ParametricBuildSession`] owns one real `cad_occt_bridge::OcctContext`
//! (kept alive across an initial build and every subsequent rebuild — *not*
//! a disk cache, daemon, or watch server; an ordinary owned value for the
//! lifetime of one build session, exactly the "single process/session can
//! build, edit parameter, rebuild" scope the campaign brief's own
//! remediation instructions call for) and re-derives `ParamModel`/
//! `FeatureGraph` fresh from the same already-lowered `HirProgram` on every
//! call (both are pure, cheap functions of already-owned HIR — re-deriving
//! them avoids a self-referential-struct problem with no correctness cost,
//! since neither type is itself mutated between calls).
//!
//! [`ParametricBuildSession::rebuild`] is the one orchestration boundary:
//!
//! 1. Compute which top-level `param` bindings actually changed value this
//!    round (every directly-overridden param, plus every param `ParamModel`
//!    itself records as transitively depending on one — `changed_param_
//!    bindings`, using only `ParamModel`'s own already-public dependency
//!    edges, never a second dependency graph).
//! 2. Run `Interpreter::run_top_level_parametric` (the fixed version) to get
//!    a freshly-evaluated `GeometryGraph` — structurally identical to the
//!    prior round's own graph for unchanged source (same call sequence, same
//!    `GeomId` positions), differing only in the evaluated scalar parameters
//!    of nodes downstream of a changed param.
//! 3. Ask `FeatureGraph::dirty_set` (the sole semantic authority for *what*
//!    is dirty — this module never second-guesses it) which named/anonymous
//!    feature nodes are dirty given step 1's changed bindings.
//! 4. Translate each dirty `FeatureId` into the exact raw `GeomId`s that
//!    feature's own call pushed (`Interpreter::geom_range_for_call`, keyed by
//!    the same call-expression span `FeatureGraph` itself uses — no
//!    positional guessing, and correct for both single-node and compound/
//!    decomposed builtins).
//! 5. Dispatch only that recompute set through `cad_geometry_runtime::
//!    dispatch_graph_incremental`, reusing every other node's already-built
//!    `Shape` directly from the prior round's own results (a real move of an
//!    owned kernel resource, not a fresh kernel call — `Shape` is not
//!    `Clone`, by design, so genuine reuse is the only way this could work
//!    at all).
//!
//! `FeatureGraph`/`ParamModel` remain the sole authorities for dependency
//! structure/dirty propagation; this module performs no parameter or
//! dependency reasoning of its own beyond the one-hop `changed_param_
//! bindings` translation described above, and introduces no second
//! parameter system, no second dependency graph, and no persistent
//! cross-process cache.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use cad_diagnostics::{Diagnostic, Severity};
use cad_feature_graph::FeatureGraph;
use cad_geometry_runtime::{
    GraphResults, IncrementalStats, dispatch_graph_incremental_with_lineage,
};
use cad_hir::ids::BindingId;
use cad_hir::lower::LowerResult;
use cad_hir::typeck::TypeCheckResult;
use cad_occt_bridge::{OcctContext, Shape};
use cad_query::{Candidate, EvaluationEvidence, ResolveError, ResolverContext};
use cad_references::{AnyRef, EntityKind, EpochCounter, FeatureAnchor};
use cad_runtime::interp::Interpreter;
use cad_runtime::params::{ParamModel, ParamOverrides};
use cad_runtime::value::Value;

use crate::reference_replay::{self, FeatureLineageIndex};

fn has_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == Severity::Error)
}

/// Every top-level `let`/`const`/`param` binding name in `program`, paired
/// with its own [`BindingId`] — the only name resolution this module needs
/// (mirrors `crate::build::resolve_named_output`'s own plain-name lookup,
/// restricted here to what [`ParametricBuildSession`] itself resolves by
/// name: a parameter to override, or a binding whose resulting geometry the
/// caller wants back).
fn top_level_binding_named(lowered: &LowerResult, name: &str) -> Option<BindingId> {
    lowered
        .bindings
        .iter()
        .find(|b| b.name == name)
        .map(|b| b.id)
}

/// Collects every `let`/`const`/`param` binding's own current value into
/// `out`, recursing into `part { ... }` bodies — `AICAD-096`'s own
/// regression fix: every `AICAD-079A` corpus fixture, and idiomatic
/// `.aicad` source in general (`skills/cad-core.skill.md`'s own worked
/// examples), wraps its geometry in a `part { ... }` block, and this
/// function's predecessor (a shallow `for item in
/// &self.lowered.program.items` loop reading `interp.global(binding)`
/// directly) silently found none of them: every named feature a real
/// `part`-wrapped program declares was invisible to
/// [`ParametricBuildSession::rebuild`]'s own `last_globals` snapshot and
/// [`ResolverContext::candidates`] alike, so every query against a
/// `part`-wrapped build reported `Broken` regardless of whether the named
/// reference actually existed — never a *wrong* answer, D7's fail-closed
/// direction, but one that made resolver execution against the frozen
/// corpus impossible until fixed at the root.
///
/// A `part` body's own inner bindings are deliberately **not** written
/// into [`Interpreter::globals`] by [`Interpreter::eval_part_body`]
/// (`cad_runtime::value::Value::Part`'s own doc comment: collected into
/// one `Value::Part` aggregate, name-keyed, "rather than written into
/// `self.globals` directly" — an `AICAD-071` design choice this task has
/// no reason to revisit). This function is the `cad-cli`-side counterpart
/// that design already implies: given `interp.global(part_binding)`'s own
/// `Value::Part { fields, .. }`, match each of the part's own HIR items
/// back to its evaluated value **by name** (the only key `fields` carries)
/// and record it under that item's own real [`BindingId`] — never guessed,
/// never positional, an exact name match against the same HIR the part
/// was built from. Does not recurse into a function body's own local
/// `let`s (`HirItem::Fn`) or into a nested `part`-in-`part` (matching
/// `eval_part_body`'s own identical, already-documented boundary) — only
/// one level of `part` nesting exists in the grammar today.
fn collect_geometry_globals(
    items: &[cad_hir::hir::HirItem],
    interp: &Interpreter<'_>,
    out: &mut HashMap<BindingId, Value>,
) {
    for item in items {
        match item {
            cad_hir::hir::HirItem::Let { binding, .. }
            | cad_hir::hir::HirItem::Const { binding, .. }
            | cad_hir::hir::HirItem::Param { binding, .. } => {
                if let Some(value) = interp.global(*binding) {
                    out.insert(*binding, value.clone());
                }
            }
            cad_hir::hir::HirItem::Part {
                binding: part_binding,
                items: part_items,
                ..
            } => {
                let Some(Value::Part { fields, .. }) = interp.global(*part_binding) else {
                    continue;
                };
                for inner in part_items {
                    let (inner_binding, inner_name) = match inner {
                        cad_hir::hir::HirItem::Let { binding, name, .. }
                        | cad_hir::hir::HirItem::Const { binding, name, .. }
                        | cad_hir::hir::HirItem::Param { binding, name, .. } => {
                            (*binding, name.as_str())
                        }
                        _ => continue,
                    };
                    if let Some((_, value)) = fields.iter().find(|(name, _)| name == inner_name) {
                        out.insert(inner_binding, value.clone());
                    }
                }
            }
            _ => {}
        }
    }
}

/// Every `param` whose own override *actually differs* between two rounds:
/// present with a different value in `current` than in `previous` (or
/// present in only one of the two — a newly-added override, or one reverted
/// back to its own `default`). Deliberately **not** "every currently
/// overridden param" — an override that is simply still in effect from an
/// earlier round (a repeated rebuild with no new edit) must not be treated
/// as changed again, or every rebuild after the first edit would keep
/// marking the identical dependents dirty forever, defeating incrementality
/// entirely (the exact regression `editing_a_param_rebuilds_only_the_
/// affected_branch_and_reuses_the_rest`'s own "repeated rebuild" assertion
/// guards against).
fn directly_changed_params(
    current: &ParamOverrides,
    previous: &ParamOverrides,
) -> HashSet<cad_runtime::params::ParamId> {
    let mut changed = HashSet::new();
    for (id, value) in current {
        if previous.get(id) != Some(value) {
            changed.insert(*id);
        }
    }
    for id in previous.keys() {
        if !current.contains_key(id) {
            changed.insert(*id);
        }
    }
    changed
}

/// Every [`BindingId`] whose top-level `param` value actually changes this
/// round: every param in `directly_changed`, plus every param `model` itself
/// records as (transitively) depending on one — computed purely from
/// `ParamModel::declarations`'s own already-public `depends_on` edges, never
/// a second, independently-maintained dependency graph. A plain top-level
/// `let`/`const` is never itself a "changed binding" by this function (only
/// `param`s can be overridden at all); if a `let` consumes a changed param,
/// `FeatureGraph::dirty_set` is what marks *that* dependency dirty, not this
/// function.
fn changed_param_bindings(
    model: &ParamModel<'_>,
    directly_changed: HashSet<cad_runtime::params::ParamId>,
) -> HashSet<BindingId> {
    let mut changed = directly_changed;
    let mut worklist: Vec<_> = changed.iter().copied().collect();
    while let Some(id) = worklist.pop() {
        for decl in model.declarations() {
            if decl.depends_on.contains(&id) && changed.insert(decl.id) {
                worklist.push(decl.id);
            }
        }
    }
    changed.into_iter().map(|id| id.0).collect()
}

/// What one [`ParametricBuildSession::rebuild`] round actually did — the
/// deterministic evidence `AGENTS.md`'s evidence rule and this remediation's
/// own regression tests both need, not a public diagnostic.
#[derive(Debug, Clone)]
pub struct RebuildOutcome {
    /// Every *named* feature (a top-level `let`/`const` resolving to a
    /// supported-modeling-operation call) `FeatureGraph::dirty_set` marked
    /// dirty this round, in feature build order. An anonymous (unnamed
    /// nested-argument) dirty feature contributes to `stats` but has no name
    /// to report here.
    pub dirty_feature_names: Vec<String>,
    /// Exactly which raw `GeomId`s were recomputed against the kernel this
    /// round vs. reused directly from the prior round's own results, in
    /// node order — see [`IncrementalStats`]'s own doc comment.
    pub stats: IncrementalStats,
}

/// One parametric build session: an initial build, plus zero or more
/// parameter-edit/rebuild rounds against the *same* real kernel context —
/// see this module's own doc comment for the full design.
pub struct ParametricBuildSession<'ctx> {
    file: String,
    source: String,
    lowered: LowerResult,
    checked: TypeCheckResult,
    ctx: &'ctx OcctContext,
    overrides: ParamOverrides,
    /// `overrides` exactly as it stood after the *previous* successful
    /// [`ParametricBuildSession::rebuild`] call — the baseline
    /// [`directly_changed_params`] diffs the current `overrides` against, so
    /// a param that is merely still overridden from an earlier round (no new
    /// edit since) is correctly treated as unchanged, not dirty again.
    last_applied_overrides: ParamOverrides,
    prior_results: Option<GraphResults<'ctx>>,
    /// A snapshot of every top-level binding's own evaluated `Value` after
    /// the most recent successful build/rebuild — captured because the
    /// `Interpreter` that produced it is local to that one call and does
    /// not outlive it (see [`ParametricBuildSession::rebuild`]).
    last_globals: HashMap<BindingId, Value>,
    /// One raw-topology-handle epoch (`AICAD-093`) per session, advanced
    /// at the start of every [`ParametricBuildSession::rebuild`] round
    /// (including the initial build) — see
    /// [`ParametricBuildSession::epoch_counter`]'s own doc comment for why
    /// this is the real integration `AICAD-093`'s own report named as
    /// this task's job.
    epoch: EpochCounter,
    /// Real per-round `Face` lineage evidence (`AICAD-094`) — see
    /// `crate::reference_replay`'s own module doc comment for exactly
    /// which features this covers and why.
    feature_lineage: FeatureLineageIndex<'ctx>,
}

impl<'ctx> ParametricBuildSession<'ctx> {
    /// Parses/lowers/type-checks `source` (as if it were the contents of
    /// `file`) and performs the initial build against `ctx` — every node
    /// recomputed, since there is nothing yet to reuse. `ctx` is owned by
    /// the caller and borrowed for this session's entire lifetime, so every
    /// `Shape` this session ever produces (initial or rebuilt) lives in the
    /// same kernel context, letting a later round genuinely reuse an
    /// earlier round's own `Shape` value.
    pub fn new(file: &str, source: &str, ctx: &'ctx OcctContext) -> Result<Self, Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();

        let (program, parse_diagnostics) = cad_parser::parse_program(source, file);
        diagnostics.extend(parse_diagnostics);
        if has_error(&diagnostics) {
            return Err(diagnostics);
        }

        let lowered = cad_hir::lower_program(&program, file, source);
        diagnostics.extend(lowered.diagnostics.clone());
        if has_error(&diagnostics) {
            return Err(diagnostics);
        }

        let checked = cad_hir::check_program(&lowered.program, &lowered.bindings, file, source);
        diagnostics.extend(checked.diagnostics.clone());
        if has_error(&diagnostics) {
            return Err(diagnostics);
        }

        let mut session = ParametricBuildSession {
            file: file.to_string(),
            source: source.to_string(),
            lowered,
            checked,
            ctx,
            overrides: ParamOverrides::new(),
            last_applied_overrides: ParamOverrides::new(),
            prior_results: None,
            last_globals: HashMap::new(),
            epoch: EpochCounter::new(),
            feature_lineage: FeatureLineageIndex::new(),
        };
        session.rebuild().map_err(|(_, diagnostics)| diagnostics)?;
        Ok(session)
    }

    /// Resolves `name` against this program's own top-level bindings — the
    /// only name lookup this session needs (a `param` to override, or a
    /// binding whose resulting geometry a caller wants via
    /// [`ParametricBuildSession::shape_for_binding`]).
    pub fn binding_named(&self, name: &str) -> Option<BindingId> {
        top_level_binding_named(&self.lowered, name)
    }

    /// Overrides top-level `param` named `name` to `value` for every
    /// subsequent [`ParametricBuildSession::rebuild`] call (cumulative,
    /// exactly like `cad_runtime::params::ParamOverrides`'s own semantics —
    /// call again with a different value to edit it again, or with a
    /// different param name to add another override alongside this one).
    /// Does not itself rebuild — a caller wanting the resulting geometry
    /// must call [`ParametricBuildSession::rebuild`] afterward, exactly
    /// mirroring the campaign brief's own "build, edit parameter, rebuild"
    /// three-step shape.
    pub fn set_param(&mut self, name: &str, value: Value) -> Result<(), Box<Diagnostic>> {
        let model = ParamModel::build(&self.lowered.program).map_err(|err| {
            Box::new(
                err.into_runtime_error()
                    .to_diagnostic(&self.file, &self.source),
            )
        })?;
        let id = model.find_by_name(name).ok_or_else(|| {
            Box::new(environment_diagnostic(
                &self.file,
                &self.source,
                &format!("no top-level param named '{name}' in this program"),
            ))
        })?;
        self.overrides.insert(id, value);
        Ok(())
    }

    /// Performs one build/rebuild round — see this module's own doc comment
    /// for the exact five-step orchestration. Returns the incremental
    /// evidence for this round, or every diagnostic collected (paired with
    /// this round's own dirty-feature-name/stats snapshot at the point of
    /// failure, empty on the very first call) if execution or dispatch
    /// failed.
    pub fn rebuild(&mut self) -> Result<RebuildOutcome, (RebuildOutcome, Vec<Diagnostic>)> {
        // Advance the raw-handle epoch (`AICAD-093`) at the start of every
        // round, including the initial build: any `RawHandle` a caller
        // minted around a candidate from *before* this call must be
        // rejected once this round's own regenerated geometry exists,
        // whether or not the wrapped `Shape` value happens to still be
        // alive via reuse -- see `crate::reference_replay`'s own module
        // doc comment and `EpochCounter::advance`'s doc comment.
        self.epoch.advance();

        let empty_outcome = || RebuildOutcome {
            dirty_feature_names: Vec::new(),
            stats: IncrementalStats::default(),
        };

        let model = match ParamModel::build(&self.lowered.program) {
            Ok(model) => model,
            Err(err) => {
                return Err((
                    empty_outcome(),
                    vec![
                        err.into_runtime_error()
                            .to_diagnostic(&self.file, &self.source),
                    ],
                ));
            }
        };
        let feature_graph = match FeatureGraph::build(&self.lowered.program) {
            Ok(graph) => graph,
            Err(err) => {
                return Err((
                    empty_outcome(),
                    vec![err.to_diagnostic(&self.file, &self.source)],
                ));
            }
        };

        let directly_changed =
            directly_changed_params(&self.overrides, &self.last_applied_overrides);
        let changed = changed_param_bindings(&model, directly_changed);

        let mut interp = Interpreter::new(
            &self.lowered.program,
            &self.lowered.bindings,
            &self.file,
            &self.source,
        );
        if let Err(diagnostic) = interp.run_top_level_parametric(
            &self.lowered.program,
            &model,
            &self.overrides,
            Some(&self.checked),
        ) {
            return Err((empty_outcome(), vec![*diagnostic]));
        }

        let dirty_features = feature_graph.dirty_set(&changed);
        let graph = interp.geometry_graph();
        let mut dirty_geom_ids = HashSet::new();
        let mut dirty_feature_names = Vec::new();
        // Every named top-level feature's own raw geom range, dirty or
        // not (`AICAD-094`): `reference_replay::capture_named_feature_
        // lineage` only actually captures the ones the dispatch below
        // really recomputed this round (present in its own returned
        // `LineageTable`), so passing every named feature here rather
        // than filtering by `dirty_features` first is what lets the very
        // first build -- which recomputes every node unconditionally but
        // reports an empty `dirty_features` set, since nothing has
        // "changed" relative to a not-yet-existing prior round -- still
        // capture real lineage, not just later incremental rounds.
        let mut named_feature_ranges: Vec<(FeatureAnchor, Range<u32>)> = Vec::new();
        for node in feature_graph.nodes() {
            let range = interp.geom_range_for_call(node.span);
            if dirty_features.contains(&node.id)
                && let Some(range) = &range
            {
                for index in range.clone() {
                    dirty_geom_ids.insert(graph.nodes()[index as usize].id);
                }
            }
            if let Some(name) = node.name {
                if dirty_features.contains(&node.id) {
                    dirty_feature_names.push(name.to_string());
                }
                if let Some(range) = range {
                    named_feature_ranges.push((FeatureAnchor::named(name), range));
                }
            }
        }

        let prior = self.prior_results.take();
        let (results, stats, lineage_table) = match dispatch_graph_incremental_with_lineage(
            graph,
            self.ctx,
            prior,
            &dirty_geom_ids,
        ) {
            Ok(triple) => triple,
            Err(err) => {
                let outcome = RebuildOutcome {
                    dirty_feature_names,
                    stats: IncrementalStats::default(),
                };
                return Err((outcome, vec![err.to_diagnostic(&self.file, &self.source)]));
            }
        };

        match reference_replay::capture_named_feature_lineage(
            graph,
            &results,
            &lineage_table,
            &named_feature_ranges,
        ) {
            Ok(captured) => self.feature_lineage.extend(captured),
            Err(err) => {
                let outcome = RebuildOutcome {
                    dirty_feature_names,
                    stats,
                };
                return Err((
                    outcome,
                    vec![environment_diagnostic(
                        &self.file,
                        &self.source,
                        &format!("failed to classify feature lineage evidence: {err}"),
                    )],
                ));
            }
        }

        let mut last_globals = HashMap::new();
        collect_geometry_globals(&self.lowered.program.items, &interp, &mut last_globals);
        self.last_globals = last_globals;
        self.prior_results = Some(results);
        self.last_applied_overrides = self.overrides.clone();

        Ok(RebuildOutcome {
            dirty_feature_names,
            stats,
        })
    }

    /// The most recent build/rebuild round's own dispatched `Shape` for
    /// `binding`, if `binding` names a top-level `let`/`const`/`param` that
    /// evaluated to a `Geometry` value. `None` if `binding` is unknown, does
    /// not hold a `Geometry` value, or no successful build has run yet.
    pub fn shape_for_binding(&self, binding: BindingId) -> Option<&Shape<'ctx>> {
        let Value::Geometry(id) = self.last_globals.get(&binding)? else {
            return None;
        };
        match self.prior_results.as_ref()?.get(id.index() as usize)? {
            cad_geometry_runtime::NodeResult::Shape(shape) => Some(shape),
            _ => None,
        }
    }

    /// This session's own raw-topology-handle epoch (`AICAD-093`),
    /// advanced once at the start of every
    /// [`ParametricBuildSession::rebuild`] round. A caller minting a
    /// `cad_references::RawHandle` around a `Shape`/`Candidate` this
    /// session produced should mint it against `self.epoch_counter().
    /// current()` and re-check it against this same accessor after any
    /// later `rebuild()` call — exactly the property
    /// `crate::reference_replay`'s own module doc comment and
    /// `project/reports/AICAD-093.md`'s own "Limitations" section name as
    /// this task's job: "connecting one `EpochCounter` per build session
    /// and calling `advance()` exactly on regeneration."
    pub fn epoch_counter(&self) -> &EpochCounter {
        &self.epoch
    }

    /// Resolves `query` against this session's own current regenerated
    /// state (`AICAD-094`) — the real "reference replay" entry point:
    /// calling this both before and after a [`ParametricBuildSession::
    /// rebuild`] round observes this session's *actual* candidate/lineage
    /// evidence at each point, never a parallel/hand-built stand-in.
    pub fn resolve(
        &self,
        query: &cad_query::Query,
    ) -> Result<cad_query::ResolutionOutcome<'ctx>, ResolveError> {
        cad_query::resolve_query(query, self)
    }

    /// Resolves a stable [`AnyRef`] (any of its seven construction
    /// strategies — see `cad_query::resolve`'s own module doc comment)
    /// against this session's own current regenerated state, paired with
    /// that reference's own static durability — see
    /// [`ParametricBuildSession::resolve`]'s own doc comment.
    pub fn resolve_reference(
        &self,
        reference: &AnyRef,
    ) -> Result<cad_query::ReferenceResolution<'ctx>, ResolveError> {
        cad_query::resolve_reference_with_durability(reference, self)
    }
}

/// `generated_by`/`modified_by` evidence sourced from this session's own
/// real per-round `feature_lineage` index (`AICAD-094`) — the same
/// classification logic `crate::query::resolve`'s own `AICAD-088` tests
/// (`LineageBackedContext`) first proved against one hand-constructed
/// operation, reused here unchanged against this session's real
/// incremental-rebuild evidence instead. `descended_from`/`resolve_ref`/
/// `resolve_target` are left at their default `None` ("no evidence") —
/// `Ancestry`/`adjacent_to`/`inside`/`within` resolution needs an
/// already-*resolved*-reference lookup this task does not build a
/// production source for, matching every predecessor Stage-4 task's own
/// identical "not yet produced" precedent (see `crate::eval`'s own module
/// doc comment).
impl<'ctx> EvaluationEvidence<'ctx> for ParametricBuildSession<'ctx> {
    fn generated_by(&self, candidate: &Candidate<'ctx>, anchor: &FeatureAnchor) -> Option<bool> {
        let report = self.feature_lineage.get(anchor)?;
        Some(report.results.iter().any(|result| {
            result.origin == Some(cad_query::feature_lineage::ResultEntityOrigin::New)
                && result.entity.is_same(candidate.shape()).unwrap_or(false)
        }))
    }

    fn modified_by(&self, candidate: &Candidate<'ctx>, anchor: &FeatureAnchor) -> Option<bool> {
        let report = self.feature_lineage.get(anchor)?;
        Some(report.results.iter().any(|result| {
            let is_ordinary_carry_forward = result.origin.is_none()
                && result
                    .predecessors
                    .first()
                    .map(|&i| {
                        report.prior[i].state
                            == cad_query::feature_lineage::PriorEntityState::Unchanged
                    })
                    .unwrap_or(false);
            !is_ordinary_carry_forward
                && result.origin != Some(cad_query::feature_lineage::ResultEntityOrigin::New)
                && result.entity.is_same(candidate.shape()).unwrap_or(false)
        }))
    }
}

/// `candidates(kind)` aggregates every immediate `kind`-shaped sub-entity
/// of every named top-level feature's own *current* `Shape` (`AICAD-094`)
/// — real, whole-session candidate enumeration sourced from this round's
/// actual dispatch results, via [`ParametricBuildSession::shape_for_
/// binding`]/[`reference_replay::candidates_of_kind`]. "Every named
/// top-level feature" includes one declared inside a `part { ... }` body
/// (`AICAD-096`, via `self.last_globals`'s own `collect_geometry_globals`
/// population in [`ParametricBuildSession::rebuild`]) — not only a bare
/// top-level `let`, matching how every real `.aicad` program (and the
/// entire frozen `AICAD-079A` corpus) actually declares its geometry.
/// Every other
/// `ResolverContext` method (`lookup_query`/`resolve_export`/
/// `resolve_structural_role`/`resolve_user_confirmed`) is left at its
/// default `None` — this session has no production `SemanticQuery`
/// registry, export registry, structural-role tag registry, or
/// user-confirmation registry, matching `cad_query::resolve`'s own module
/// doc comment ("Evidence this module does not itself produce") and every
/// predecessor Stage-4 task's identical scope boundary.
impl<'ctx> ResolverContext<'ctx> for ParametricBuildSession<'ctx> {
    fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
        let mut out = Vec::new();
        for &binding in self.last_globals.keys() {
            if let Some(shape) = self.shape_for_binding(binding) {
                out.extend(reference_replay::candidates_of_kind(shape, kind));
            }
        }
        out
    }
}

/// `EXPORT-E003` — the third `EXPORT`-family code (`crate::build`'s own
/// `NamedOutputError` already owns `EXPORT-E001`/`EXPORT-E002`); this is the
/// same *kind* of concern (resolving a plain top-level binding name against
/// the program's own declarations, `DL-18`'s "adding a new code in an
/// existing family... is ordinary task work"), just for a `param` override
/// rather than a `--name` export target.
fn environment_diagnostic(file: &str, source: &str, message: &str) -> Diagnostic {
    let code =
        cad_diagnostics::DiagnosticCode::new("EXPORT", cad_diagnostics::SeverityLetter::Error, 3)
            .expect("family/number given here are always a valid diagnostic code");
    let line_index = cad_ast::LineIndex::new(source);
    let start = line_index.line_column(source, 0);
    Diagnostic::new(
        code,
        Severity::Error,
        "export",
        "UNKNOWN_PARAM_NAME",
        message.to_string(),
    )
    .expect("this code always carries the 'E' severity letter")
    .with_source(cad_diagnostics::SourceSpan {
        file: file.to_string(),
        start: cad_diagnostics::Position::new(start.line, start.column),
        end: cad_diagnostics::Position::new(start.line, start.column),
    })
}
