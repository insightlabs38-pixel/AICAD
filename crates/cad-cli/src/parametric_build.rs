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
use cad_query::{
    AdjacencyTarget, Candidate, EvaluationEvidence, ResolutionOutcome, ResolveError,
    ResolverContext,
};
use cad_references::{
    AnyRef, ConstructionStrategy, EntityKind, EpochCounter, FeatureAnchor, LineageRole,
};
use cad_runtime::interp::Interpreter;
use cad_runtime::params::{ParamModel, ParamOverrides};
use cad_runtime::value::Value;

use crate::reference_replay::{self, FeatureLineageIndex};

fn has_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == Severity::Error)
}

/// Every `let`/`const`/`param` declaration in `items`, paired with the
/// scope path (`D31`, `cad_feature_graph::graph::FeatureNode::scope`'s own
/// identical convention — empty for a top-level declaration, `["Wall"]`
/// for one declared directly inside `part Wall { ... }`) it was declared
/// under. Recurses into `HirItem::Part` bodies to any depth (`AICAD-101`),
/// matching every other Stage-5 `part`-aware walk in this crate
/// (`collect_geometry_globals`, `cad_feature_graph::graph::
/// FeatureGraph::build_items`).
fn collect_scoped_bindings<'a>(
    items: &'a [cad_hir::hir::HirItem],
    scope: &[String],
    out: &mut Vec<(Vec<String>, &'a str, BindingId)>,
) {
    for item in items {
        match item {
            cad_hir::hir::HirItem::Let { binding, name, .. }
            | cad_hir::hir::HirItem::Const { binding, name, .. }
            | cad_hir::hir::HirItem::Param { binding, name, .. } => {
                out.push((scope.to_vec(), name.as_str(), *binding));
            }
            cad_hir::hir::HirItem::Part {
                name: part_name,
                items: part_items,
                ..
            } => {
                let mut child_scope = scope.to_vec();
                child_scope.push(part_name.clone());
                collect_scoped_bindings(part_items, &child_scope, out);
            }
            _ => {}
        }
    }
}

/// Joins a `D31` scope path and a leaf name into the canonical dotted
/// qualified-name string (`AICAD-100A`) — e.g. `(["Wall"], "bored_a")` ->
/// `"Wall.bored_a"`. An empty scope yields the bare name unchanged, so a
/// top-level feature's own qualified name is byte-identical to its
/// pre-`D31` plain name — full backward compatibility for every existing
/// (non-`part`-nested) fixture/test.
pub(crate) fn qualified_feature_name(scope: &[String], name: &str) -> String {
    if scope.is_empty() {
        name.to_string()
    } else {
        format!("{}.{}", scope.join("."), name)
    }
}

/// Resolves `name` against `lowered`'s own `let`/`const`/`param`
/// declarations at any part-nesting depth — `D31`'s collision-safe name
/// resolution (`AICAD-100A`), replacing the pre-`D31` flat, scope-blind
/// `lowered.bindings` search this function used to be (which, now that a
/// program may declare the same plain name inside two different `part`
/// bodies, could otherwise silently return whichever declaration happened
/// to be minted first — exactly the kind of arbitrary-selection-by-
/// construction-order `AGENTS.md`'s "ambiguity is an error, never an
/// arbitrary selection" non-negotiable forbids).
///
/// `name` may be:
/// - a fully qualified dotted path (`"Wall.bored_a"`), matched exactly
///   against a declaration's own scope path + leaf name — always
///   unambiguous, regardless of how many other declarations share the
///   same leaf name elsewhere;
/// - a bare leaf name (`"bored_a"`, or a top-level feature's own plain
///   name, unchanged), resolved only if *exactly one* declaration
///   anywhere in the program carries that leaf name. A genuine collision
///   (the same bare name declared in two different parts, or in a part
///   and at the top level) reports `None` — never an arbitrary pick —
///   exactly mirroring `cad_query::resolve`'s own fail-closed
///   `BrokenReason::ScopeNotFound` contract one layer up (`crate::
///   parametric_build::ParametricBuildSession`'s `ResolverContext::
///   candidates_in_scope` implementation calls this function directly, so
///   a `None` here becomes that same fail-closed outcome, never a silent
///   fallback to an arbitrarily-chosen candidate).
fn resolve_scoped_name(lowered: &LowerResult, name: &str) -> Option<BindingId> {
    let mut all = Vec::new();
    collect_scoped_bindings(&lowered.program.items, &[], &mut all);

    if let Some((scope_part, leaf)) = name.rsplit_once('.') {
        let scope_path: Vec<&str> = scope_part.split('.').collect();
        return all
            .into_iter()
            .find(|(scope, n, _)| {
                scope
                    .iter()
                    .map(String::as_str)
                    .eq(scope_path.iter().copied())
                    && *n == leaf
            })
            .map(|(_, _, b)| b);
    }

    let mut matches = all.into_iter().filter(|(_, n, _)| *n == name);
    let first = matches.next()?;
    if matches.next().is_some() {
        // A genuine bare-name collision -- fail closed, never guess which
        // of the colliding declarations the caller meant.
        return None;
    }
    Some(first.2)
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
/// `let`s (`HirItem::Fn`) — a function body is a separate execution scope,
/// never a named/query-visible feature. A nested `part`-in-`part`
/// (`AICAD-101`) recurses to any depth via [`collect_part_fields`], the
/// `cad-cli`-side counterpart of `Interpreter::eval_part_body`'s own
/// recursive nested-part evaluation.
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
                collect_part_fields(part_items, fields, out);
            }
            _ => {}
        }
    }
}

/// Recurses into one `part` body's own item list (`items`), matching each
/// leaf `let`/`const`/`param` and nested `part` against `fields` (that
/// same part's own evaluated `Value::Part::fields`, name-keyed) by name,
/// at any nesting depth (`AICAD-101`) — see [`collect_geometry_globals`]'s
/// own doc comment for why a name match against the source HIR, rather
/// than a positional one, is the only correct key.
fn collect_part_fields(
    items: &[cad_hir::hir::HirItem],
    fields: &[(String, Value)],
    out: &mut HashMap<BindingId, Value>,
) {
    for item in items {
        match item {
            cad_hir::hir::HirItem::Let { binding, name, .. }
            | cad_hir::hir::HirItem::Const { binding, name, .. }
            | cad_hir::hir::HirItem::Param { binding, name, .. } => {
                if let Some((_, value)) = fields.iter().find(|(n, _)| n == name) {
                    out.insert(*binding, value.clone());
                }
            }
            cad_hir::hir::HirItem::Part {
                binding: nested_binding,
                name,
                items: nested_items,
                ..
            } => {
                if let Some((
                    _,
                    value @ Value::Part {
                        fields: nested_fields,
                        ..
                    },
                )) = fields.iter().find(|(n, _)| n == name)
                {
                    out.insert(*nested_binding, value.clone());
                    collect_part_fields(nested_items, nested_fields, out);
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
    /// This session's own real registered-query registry (`AICAD-100A`),
    /// backing [`ResolverContext::lookup_query`] for a
    /// [`ConstructionStrategy::SemanticQuery`] reference — see
    /// [`ParametricBuildSession::register_query`]'s own doc comment for
    /// how a caller populates it. Empty until a caller (today, `cad-cli`
    /// itself or a test; once `query { ... }` `.aicad` source syntax
    /// lands, the real lowering path too) registers something — never
    /// pre-seeded with a guess.
    queries: HashMap<cad_references::QueryHandle, cad_query::Query>,
    /// Every real persistent reference this program's own source
    /// declared (`AICAD-100A`, `crate::query_lowering::lower_hir_queries`)
    /// — `(D31`-qualified name, the `AnyRef` that name now denotes`)`,
    /// populated once at construction time (a program's own `query {
    /// ... }` declarations do not change across a session's `rebuild`
    /// rounds, only their resolution results do). Empty for a program
    /// with no `query { ... }` declarations at all — never a guess.
    source_references: Vec<(String, AnyRef)>,
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

        let (source_queries, query_diagnostics) =
            crate::query_lowering::lower_hir_queries(file, source, &lowered.program.items);
        diagnostics.extend(query_diagnostics);
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
            queries: HashMap::new(),
            source_references: Vec::new(),
        };
        let mut source_references = Vec::with_capacity(source_queries.len());
        for source_query in source_queries {
            session.register_query(source_query.handle, source_query.query);
            source_references.push((source_query.name, source_query.reference));
        }
        session.source_references = source_references;

        session.rebuild().map_err(|(_, diagnostics)| diagnostics)?;
        Ok(session)
    }

    /// Every real persistent reference this program's own `.aicad` source
    /// declared via `query { ... }` (`AICAD-100A`) — `(D31`-qualified
    /// name, the `AnyRef` it denotes`)`, in declaration order. This is the
    /// production evidence source `crate::refs_check::refs_check_source`
    /// consumes instead of the previously-always-empty placeholder set
    /// (`crate::refs_check`'s own module doc comment, now stale — see this
    /// method for the real one).
    pub fn source_references(&self) -> &[(String, AnyRef)] {
        &self.source_references
    }

    /// Resolves `name` against this program's own `let`/`const`/`param`
    /// declarations (top-level or `part`-nested, `D31`) — the only name
    /// lookup this session needs (a `param` to override, or a binding
    /// whose resulting geometry a caller wants via
    /// [`ParametricBuildSession::shape_for_binding`]). See
    /// `crate::parametric_build::resolve_scoped_name`'s own doc comment
    /// for the exact bare-name-vs-qualified-path, collision-safe contract.
    pub fn binding_named(&self, name: &str) -> Option<BindingId> {
        resolve_scoped_name(&self.lowered, name)
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
        // Every named feature's own bare leaf name, counted across the
        // *whole* graph regardless of scope (`D31`) — used below to decide
        // whether a bare-name `FeatureAnchor` alias may be registered
        // alongside a part-nested feature's own qualified name. A count of
        // exactly 1 means the bare name is unambiguous program-wide; a
        // count > 1 (the same leaf name declared in two different parts,
        // or a part and the top level) means the bare alias must be
        // withheld for *both* colliding features, so a `FeatureAnchor::
        // named(bare_name)` lookup fails closed (`InsufficientEvidence`)
        // rather than silently landing on whichever one this loop happens
        // to visit last.
        let mut bare_name_counts: HashMap<&str, usize> = HashMap::new();
        for node in feature_graph.nodes() {
            if let Some(name) = node.name {
                *bare_name_counts.entry(name).or_insert(0) += 1;
            }
        }
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
                let qualified = qualified_feature_name(&node.scope, name);
                if dirty_features.contains(&node.id) {
                    dirty_feature_names.push(qualified.clone());
                }
                if let Some(range) = range {
                    named_feature_ranges
                        .push((FeatureAnchor::named(qualified.clone()), range.clone()));
                    // Also register the bare leaf name as a convenience
                    // alias resolving to the exact same evidence, but only
                    // when it is unambiguous program-wide (`D31`) -- see
                    // `bare_name_counts`'s own doc comment above. For an
                    // ordinary top-level feature (empty scope),
                    // `qualified == name` already, so this is a no-op,
                    // never a duplicate registration.
                    if qualified != name && bare_name_counts.get(name) == Some(&1) {
                        named_feature_ranges.push((FeatureAnchor::named(name), range));
                    }
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

    /// Registers `query` under `handle` in this session's own real query
    /// registry (`AICAD-100A`), making a later `ConstructionStrategy::
    /// SemanticQuery { query: handle, .. }` reference resolvable via
    /// [`ResolverContext::lookup_query`] — the production evidence source
    /// that strategy previously had none of in this session (`cad_query::
    /// resolve`'s own module doc comment, "Evidence this module does not
    /// itself produce"). This is real production API, not a test-only
    /// bypass: a `.aicad` `query { ... }` declaration (once source syntax
    /// for it exists) lowers into exactly this same call, and `cad-cli`
    /// itself (or any other real caller) may call it directly today.
    /// Returns the previously-registered `Query` for `handle`, if any
    /// (re-registering the same handle is a deliberate replace, not an
    /// error — unlike [`cad_references::FeatureExports::export`]'s own
    /// one-time-only contract, a query registration is expected to be
    /// refreshed across rebuilds of the same session).
    pub fn register_query(
        &mut self,
        handle: cad_references::QueryHandle,
        query: cad_query::Query,
    ) -> Option<cad_query::Query> {
        self.queries.insert(handle, query)
    }
}

/// `generated_by`/`modified_by` evidence sourced from this session's own
/// real per-round `feature_lineage` index (`AICAD-094`) — the same
/// classification logic `crate::query::resolve`'s own `AICAD-088` tests
/// (`LineageBackedContext`) first proved against one hand-constructed
/// operation, reused here unchanged against this session's real
/// incremental-rebuild evidence instead. `descended_from` is real too
/// (`AICAD-100A`, composed via `crate::reference_replay::
/// descended_from_closure` — see that function's own doc comment) for a
/// `FeatureLineage`-anchored ancestor. `resolve_ref`/`resolve_target`
/// remain at their default `None` ("no evidence") — `adjacent_to`/
/// `inside`/`within` need an already-*resolved* arbitrary-query/reference
/// lookup (not merely a named-feature lookup) this session does not build
/// a production source for; see `ResolverContext::lookup_query`/
/// `resolve_export`/`resolve_structural_role`/`resolve_user_confirmed`'s
/// own identical scope boundary below.
impl<'ctx> EvaluationEvidence<'ctx> for ParametricBuildSession<'ctx> {
    fn generated_by(&self, candidate: &Candidate<'ctx>, anchor: &FeatureAnchor) -> Option<bool> {
        let report = self
            .feature_lineage
            .get(&(anchor.clone(), candidate.kind()))?;
        let shapes =
            reference_replay::feature_result_shapes_for_role(report, LineageRole::Generated);
        Some(
            shapes
                .iter()
                .any(|shape| shape.is_same(candidate.shape()).unwrap_or(false)),
        )
    }

    fn modified_by(&self, candidate: &Candidate<'ctx>, anchor: &FeatureAnchor) -> Option<bool> {
        let report = self
            .feature_lineage
            .get(&(anchor.clone(), candidate.kind()))?;
        let shapes =
            reference_replay::feature_result_shapes_for_role(report, LineageRole::Modified);
        Some(
            shapes
                .iter()
                .any(|shape| shape.is_same(candidate.shape()).unwrap_or(false)),
        )
    }

    /// `TopologyPredicate::DescendedFrom`/`ConstructionStrategy::Ancestry`
    /// evidence (`AICAD-100A`), composed from this session's own real
    /// captured per-feature lineage reports — see
    /// `crate::reference_replay::descended_from_closure`'s own doc comment
    /// for the exact composition algorithm and why it is real evidence,
    /// never synthesized geometric coincidence.
    ///
    /// Only supports an `ancestor` whose own recipe strategy is
    /// `ConstructionStrategy::FeatureLineage` (the realistic, common case:
    /// "descended from *this feature's* own output" — matching how every
    /// corpus fixture and `generated_by`/`modified_by` already anchor
    /// lineage) — composing ancestry from an arbitrary *other* reference
    /// strategy would first require resolving that reference to a live
    /// historical entity, which no Stage-4 evidence source in this session
    /// builds (see this crate's own established "evidence this module does
    /// not itself produce" precedent); `None` ("no evidence"), never a
    /// guess, for any other strategy.
    fn descended_from(&self, candidate: &Candidate<'ctx>, ancestor: &AnyRef) -> Option<bool> {
        let ConstructionStrategy::FeatureLineage { feature, role } = &ancestor.recipe().strategy
        else {
            return None;
        };
        let known = reference_replay::descended_from_closure(
            &self.feature_lineage,
            feature,
            *role,
            candidate.kind(),
        )?;
        Some(
            known
                .iter()
                .any(|k| k.is_same(candidate.shape()).unwrap_or(false)),
        )
    }

    /// `adjacent_to`/`inside`/`within` all need an already-*resolved*
    /// reference target (`crate::eval`'s own module doc comment) —
    /// `AICAD-100A` supplies real production evidence by recursively
    /// calling this session's own `cad_query::resolve_reference` against
    /// itself (`self` already implements `ResolverContext`), the exact
    /// same fail-closed resolution path every top-level query/reference in
    /// this session already goes through, never a second/parallel
    /// resolution mechanism. Only a target that resolves `Resolved`
    /// (exactly the expected count — `Unique` for a bare reference, or the
    /// nested query's own stated cardinality) is real, determinate
    /// evidence; `Ambiguous`/`Broken`/a hard `ResolveError` all report
    /// `None` ("no evidence") here — a caller cannot reason about "is X
    /// adjacent to an ambiguous, not-yet-narrowed target," so this never
    /// guesses which candidate among an ambiguous target the caller meant.
    fn resolve_ref(&self, reference: &AnyRef) -> Option<Vec<Candidate<'ctx>>> {
        match cad_query::resolve_reference(reference, self) {
            Ok(ResolutionOutcome::Resolved(candidates)) => Some(candidates),
            _ => None,
        }
    }

    /// Resolves an [`AdjacencyTarget`] the same way as
    /// [`ParametricBuildSession::resolve_ref`] — a bare [`AnyRef`] target
    /// through that same method (always `Unique`); a nested [`cad_query::
    /// Query`] target through `cad_query::resolve_query` directly, honoring
    /// *that* query's own stated cardinality (so an author-written
    /// `adjacent_to(query { ... })` with no cardinality clause may
    /// legitimately designate more than one candidate, per
    /// `CardinalityExpectation::Unstated`'s own "always `Resolved`,
    /// whatever the count" semantics — never forced through the stricter
    /// single-reference `Unique` rule a bare `AnyRef` target implies).
    fn resolve_target(&self, target: &AdjacencyTarget) -> Option<Vec<Candidate<'ctx>>> {
        match target {
            AdjacencyTarget::Ref(reference) => self.resolve_ref(reference),
            AdjacencyTarget::Query(query) => match cad_query::resolve_query(query, self) {
                Ok(ResolutionOutcome::Resolved(candidates)) => Some(candidates),
                _ => None,
            },
        }
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
/// `candidates_in_scope` (`AICAD-099A`, below) narrows this same
/// enumeration to one named binding on request, rather than aggregating
/// every one of them. `lookup_query`/`resolve_export`/
/// `resolve_structural_role`/`resolve_user_confirmed` (`AICAD-100A`, all
/// below) now have real production evidence sources too — see each
/// method's own doc comment for exactly what real evidence backs it and
/// why.
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

    /// `AICAD-099A`: restricts candidate enumeration to exactly one named
    /// top-level binding's own current `Shape`, instead of
    /// [`ParametricBuildSession::candidates`]'s own "every live top-level
    /// binding" universe — the concrete fix for `AICAD-099`'s own recorded
    /// finding (`project/reports/AICAD-099.md`'s "A real, honest finding"):
    /// an intermediate binding (e.g. `with_left`) permanently carrying its
    /// own live copy of a face a later binding (`body`) also carries can
    /// make a whole-session query spuriously ambiguous even when the two
    /// candidates are, semantically, the same intended entity reached
    /// through two different bindings.
    ///
    /// Reuses exactly the binding/feature identity machinery this session
    /// already owns — [`ParametricBuildSession::binding_named`] (the same
    /// name lookup [`ParametricBuildSession::set_param`] uses) and
    /// [`ParametricBuildSession::shape_for_binding`] (the same live-shape
    /// lookup [`ParametricBuildSession::candidates`] uses per-binding
    /// above) — never a topology index, an OCCT handle, or a geometry
    /// fingerprint. `None` (never a guessed fallback to the whole
    /// universe) when: `scope` is [`FeatureAnchor::CurrentFeature`] (no
    /// notion of "the enclosing feature" exists for an ad hoc resolver
    /// call outside an `expose { ... }` body); `scope` names a binding
    /// this program does not declare; or that binding did not evaluate to
    /// a `Geometry` value in the most recent build/rebuild round (matching
    /// [`ParametricBuildSession::shape_for_binding`]'s own identical
    /// `None` cases). Candidates within the resolved scope are never
    /// deduplicated by geometry — `reference_replay::candidates_of_kind`
    /// is the exact same enumeration [`ParametricBuildSession::candidates`]
    /// already uses per-binding, unchanged.
    fn candidates_in_scope(
        &self,
        kind: EntityKind,
        scope: &FeatureAnchor,
    ) -> Option<Vec<Candidate<'ctx>>> {
        let FeatureAnchor::Named(name) = scope else {
            return None;
        };
        let binding = self.binding_named(name)?;
        let shape = self.shape_for_binding(binding)?;
        Some(reference_replay::candidates_of_kind(shape, kind))
    }

    /// This session's own real registered-query registry (`AICAD-100A`,
    /// [`ParametricBuildSession::register_query`]) — the production
    /// evidence source for `ConstructionStrategy::SemanticQuery`.
    fn lookup_query(&self, handle: &cad_references::QueryHandle) -> Option<&cad_query::Query> {
        self.queries.get(handle)
    }

    /// Real production evidence for `ConstructionStrategy::ExplicitExport`
    /// (`AICAD-100A`): `.aicad` source has no `expose { ... }` syntax yet
    /// to name an arbitrary *sub-entity* of a feature (`rfcs/
    /// 0003-semantic-references.md` §7), but it already has a real,
    /// already-production "explicit name" mechanism for a feature's own
    /// *whole* geometry value — the same `<part>.<field>` addressing
    /// `cad build --name` (`crate::build::resolve_named_output`) and
    /// `crate::parametric_build::resolve_scoped_name` (`D31`) both already
    /// use. Reused here unchanged: `feature` (when [`FeatureAnchor::
    /// Named`]) plus `export_name` are joined into that same qualified
    /// path (`"Wall.body"`) and resolved via `resolve_scoped_name` — an
    /// explicitly-*named* top-level or `part`-nested binding's own current
    /// whole `Geometry` value is exactly what `docs/plan/06...` §11 means
    /// by "feature exported the entity by semantic name," so this is real
    /// evidence, not an invented shortcut. `feature = FeatureAnchor::
    /// CurrentFeature` has no meaning outside an (unimplemented)
    /// `expose { ... }` body -- `None`, matching `candidates_in_scope`'s
    /// own identical restriction.
    fn resolve_export(
        &self,
        feature: &FeatureAnchor,
        export_name: &str,
    ) -> Option<Vec<Candidate<'ctx>>> {
        let FeatureAnchor::Named(scope) = feature else {
            return None;
        };
        let qualified = qualified_feature_name(std::slice::from_ref(scope), export_name);
        let binding = resolve_scoped_name(&self.lowered, &qualified)?;
        let shape = self.shape_for_binding(binding)?;
        Some(vec![Candidate::new(
            EntityKind::Solid,
            shape.duplicate().ok()?,
        )])
    }

    /// Real production evidence for `ConstructionStrategy::StructuralRole`
    /// (`AICAD-100A`): like [`ParametricBuildSession::resolve_export`]
    /// above, `.aicad` source has no syntax yet to tag an entity with an
    /// open structural-role string (`crate::recipe::ConstructionStrategy::
    /// StructuralRole`'s own doc comment). The closest real, already-
    /// production naming mechanism a `.aicad` program has today is a
    /// binding's own declared name — `StructuralRole("top_mounting_
    /// surface")` resolves to the program's own binding literally named
    /// `top_mounting_surface`, via the exact same collision-safe bare-name
    /// lookup (`D31`'s `resolve_scoped_name`) every other named-binding
    /// evidence source in this session already uses. Genuine evidence, not
    /// a guess: a `StructuralRole`-strategy reference only ever resolves
    /// when a real author actually gave an entity that exact source name.
    fn resolve_structural_role(&self, role: &str) -> Option<Vec<Candidate<'ctx>>> {
        let binding = resolve_scoped_name(&self.lowered, role)?;
        let shape = self.shape_for_binding(binding)?;
        Some(vec![Candidate::new(
            EntityKind::Solid,
            shape.duplicate().ok()?,
        )])
    }

    /// Real production evidence for `ConstructionStrategy::UserConfirmed`
    /// (`AICAD-100A`): Stage 4 has no confirmation *workflow* (a UI or CLI
    /// surface where a human is shown an ambiguous candidate set and picks
    /// one) — `crate::recipe::ConstructionStrategy::UserConfirmed`'s own
    /// doc comment is explicit that this strategy only records that a
    /// confirmation occurred, never defines the workflow itself, and this
    /// task does not invent one. What *is* real: once a human has picked
    /// an entity, the only way this session can be told which one is by
    /// that entity's own real source name — so, like
    /// [`ParametricBuildSession::resolve_structural_role`], this resolves
    /// `confirmation_note` as a bare binding name via the same real,
    /// collision-safe lookup. A future confirmation workflow that captures
    /// richer audit evidence than "the binding a human picked" would
    /// extend this, not replace the underlying binding-resolution
    /// mechanism.
    fn resolve_user_confirmed(&self, confirmation_note: &str) -> Option<Vec<Candidate<'ctx>>> {
        let binding = resolve_scoped_name(&self.lowered, confirmation_note)?;
        let shape = self.shape_for_binding(binding)?;
        Some(vec![Candidate::new(
            EntityKind::Solid,
            shape.duplicate().ok()?,
        )])
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
