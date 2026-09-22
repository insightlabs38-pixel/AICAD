//! Dynamic execution-trace identity for geometry-producing `RuntimeBuiltin`
//! calls (`AICAD-107`, `project/DECISION_LOG.md#DL-27`).
//!
//! `cad_feature_graph::graph::FeatureGraph` (Stage 3) builds feature
//! identity/dependency edges by a purely *static* walk over top-level (and
//! `part`-nested, `D31`) `let`/`const` declarations — deliberately never
//! looking through a user-defined `fn` call, a loop, or a branch (see that
//! module's own doc comment, "What this module deliberately does not do").
//! `D25`/`DL-27` requires Stage 5 to keep geometry visible to the feature/
//! dependency/provenance system even when it is built through exactly those
//! abstractions. A static AST walk cannot do this: which branch of an `if`
//! actually runs, and how many times a loop body actually executes, are
//! only known by *running* the program.
//!
//! This module is therefore the execution-trace counterpart: as
//! [`crate::interp::Interpreter`] runs, every successfully-dispatched
//! Geometry-returning `RuntimeBuiltin` call — wherever it occurs, including
//! deep inside a user function, a loop iteration, or a taken branch —
//! records one [`TraceEntry`], keyed by a [`CallPath`] that is this call's
//! own deterministic, AICAD-owned dynamic identity (never an OCCT/native
//! identity, never a bare source span, which — unlike this module's own
//! [`CallPath`] — collides across repeated dynamic visits to the same call
//! expression; see [`CallPath`]'s own doc comment). `cad-feature-graph`'s
//! `crate::trace_graph` module turns a completed trace into a real
//! dependency graph (`FeatureId`-shaped nodes, `geometry_inputs` edges,
//! `dirty_set`), reusing exactly the same dirty-propagation contract
//! `crate::graph::FeatureGraph::dirty_set` already established — see that
//! module's own doc comment for why the two graphs stay deliberately
//! separate types rather than one sharing a single constructor.
//!
//! ## What a [`TraceEntry`] deliberately does not carry
//!
//! No `&HirExpr` reference. Everything a caller needs (which top-level
//! bindings this call's own scalar arguments depend on, for dirty
//! propagation; which source span each argument came from, for
//! diagnostics/navigation) is computed once, eagerly, while the
//! interpreter still holds the argument expression, and stored as owned
//! data. This avoids tying every trace type to the interpreter's own `'a`
//! HIR borrow, which would otherwise require re-annotating essentially
//! every expression-evaluating method in `crate::interp` with an explicit
//! `'a` (today those methods take an ordinarily-elided, method-local
//! lifetime) purely to let this module retain a reference past the call
//! that produced it — a large, invasive, and unnecessary blast radius for
//! data this module only ever needs in already-resolved form.

use cad_ast::Span;
use cad_hir::builtins::BuiltinFnId;
use cad_hir::ids::BindingId;
use std::ops::Range;

/// One frame of a [`CallPath`]'s own dynamic nesting — see that type's doc
/// comment.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathFrame {
    /// Entering one ordinary AICAD-source `fn` call (never a
    /// `RuntimeBuiltin` call, which is always the *leaf* of a [`CallPath`],
    /// not a frame of its own — see [`CallPath`]'s own doc comment), keyed
    /// by that call expression's own source span
    /// ([`crate::interp::Interpreter::call`]).
    Call(Span),
    /// One dynamic iteration of a `for`/`while`/`loop` construct, keyed by
    /// that loop's own source span plus this construct's own 0-based
    /// iteration counter. A nested loop contributes one frame per active
    /// nesting level, each counted independently.
    Iteration(Span, u64),
}

/// Deterministic, AICAD-owned identity of one dynamic construction of a
/// feature: the full stack of [`PathFrame`]s active (outermost first) at
/// the moment a `RuntimeBuiltin` geometry call executed, plus that call's
/// own leaf source span.
///
/// # Why a bare [`Span`] is not enough
///
/// A source span identifies *where in the source text* a call expression
/// is written — but the same call expression can execute more than once at
/// run time (inside a loop, or inside a function called more than once),
/// and each such dynamic execution can legitimately build different
/// geometry (different loop-variable value, different caller-supplied
/// argument). `crate::interp::Interpreter::call_geom_ranges` (kept,
/// unchanged, for the top-level-only static-analysis callers that predate
/// this module) is keyed by bare `Span` for exactly this reason: a second
/// dynamic visit to the same span overwrites the first entry. A
/// `CallPath` disambiguates every such repeated visit deterministically:
/// - Two different textual call sites always have different leaf spans
///   (source text cannot contain two identical spans), so an ordinary
///   "two different `let`s each calling the same helper" case is already
///   disambiguated with no extra machinery.
/// - The *same* call expression executed twice by a loop is disambiguated
///   by the enclosing [`PathFrame::Iteration`] frame (a different
///   iteration index).
/// - The *same* call expression executed at increasing recursion depth is
///   disambiguated by the growing frame stack itself (each recursive
///   re-entry pushes one more [`PathFrame::Call`] frame before reaching
///   the same inner call site again), with no separate depth counter
///   needed.
///
/// A `CallPath` is meaningless outside the one interpreter run that
/// produced it — exactly like [`GeomId`]/`cad_feature_graph::graph::
/// FeatureId`'s own identical "meaningless outside its own build" identity
/// contract; a source edit changes which spans exist at all, so no
/// `CallPath` is ever compared across two different runs. Cross-run
/// stability is a different, already-solved problem this module does not
/// need to re-solve: a *named* top-level/`part`-nested binding still
/// resolves by its own stable `BindingId` (unaffected by anything in this
/// module), and Stage-4 persistent semantic references already have their
/// own durable identity independent of `CallPath`/`FeatureId` alike.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallPath {
    frames: Vec<PathFrame>,
    leaf: Span,
}

impl CallPath {
    /// Constructs a `CallPath` directly from its own frames/leaf span.
    /// Ordinary production code never needs this (a `CallPath` is always
    /// built by `crate::interp::Interpreter::call` from its own live
    /// dynamic nesting) — it is `pub` so downstream crates (`cad_feature_
    /// graph::trace_graph`'s own unit tests, in particular) can construct
    /// one directly without running a full interpreter, exactly like
    /// `cad_ast::Span::new` is itself an ordinary public constructor.
    pub fn new(frames: Vec<PathFrame>, leaf: Span) -> CallPath {
        CallPath { frames, leaf }
    }

    /// Every enclosing dynamic frame (function-call/loop-iteration nesting)
    /// this call executed under, outermost first — empty for a call made
    /// directly at top level (or directly inside a `part` body) with no
    /// enclosing function call or loop iteration.
    pub fn frames(&self) -> &[PathFrame] {
        &self.frames
    }

    /// This call's own leaf source span — the `RuntimeBuiltin` call
    /// expression itself, exactly the same span
    /// [`crate::interp::Interpreter::geom_range_for_call`] already keys on
    /// for a top-level call with no repeated dynamic visits.
    pub fn leaf(&self) -> Span {
        self.leaf
    }
}

/// One successfully-dispatched Geometry-returning `RuntimeBuiltin` call
/// this interpreter run performed, at its own full dynamic [`CallPath`]
/// identity — the execution-trace analogue of `cad_feature_graph::graph::
/// FeatureNode`, extended (`AICAD-107`) to see through user functions,
/// branches, and loops. Never built for a non-Geometry-returning builtin
/// (`is_valid`/`volume`/`area`, `AICAD-105`) — mirrors
/// `cad_feature_graph::graph::Builder::resolve_geometry_expr`'s own
/// identical `is_geometry_type` gate.
#[derive(Debug, Clone)]
pub struct TraceEntry {
    pub path: CallPath,
    /// The supported modeling operation this call invokes — the same
    /// closed identity `cad_hir::builtins::catalogue` already gives it.
    pub op: BuiltinFnId,
    /// The exact, contiguous raw `GeomId` range this call pushed onto the
    /// interpreter's own accumulated `cad_geometry_api::GeometryGraph` —
    /// identical in meaning to `crate::interp::Interpreter::
    /// call_geom_ranges`' own per-span range, just keyed by this call's own
    /// full dynamic identity instead of a bare span.
    pub geom_range: Range<u32>,
    /// The [`CallPath`]s of every other traced call this call's own
    /// `Geometry`-typed arguments consumed, in the operation's declared
    /// parameter order — the dynamic-execution counterpart of
    /// `cad_feature_graph::graph::FeatureNode::geometry_inputs`. May repeat
    /// the same `CallPath` (`union(base, base)`). Omits an argument whose
    /// own producing call this interpreter run could not resolve (not
    /// expected to occur for a call reached through this module's own
    /// tracking, since every `Geometry` value is produced by exactly one
    /// traced call — kept as a defensive "best effort," never a panic).
    pub geometry_inputs: Vec<CallPath>,
    /// This call's own non-`Geometry` (scalar) arguments, as `(declared
    /// parameter name, this call's own argument expression span)` pairs —
    /// the argument's own *value* is not stored (this module has no
    /// interpreter-independent notion of "value", and every non-`Geometry`
    /// argument that matters for dirty propagation is already captured by
    /// `binding_refs` below); the span exists for diagnostics/source
    /// navigation only.
    pub parameters: Vec<(String, Span)>,
    /// Every top-level (or `part`-nested) declaration `BindingId` this
    /// call's own scalar arguments transitively depend on, resolved
    /// *through* any enclosing function-call argument-passing chain (a
    /// local function parameter's own provenance is the provenance of
    /// whichever expression the caller supplied for it, recursively) —
    /// see `crate::interp::Interpreter::provenance_of`'s own doc comment
    /// for the exact algorithm and its one deliberate conservative
    /// approximation (a nested ordinary-function call's own result is
    /// treated as depending on the union of its own arguments' provenance,
    /// never on what that function's body actually does with them).
    /// Deduplicated, first-occurrence order — mirrors `cad_feature_graph::
    /// cache::node_cache_key`'s own identical convention.
    pub binding_refs: Vec<BindingId>,
    /// The enclosing `part` name path (`D31`) of whichever top-level (or
    /// `part`-nested) `let`/`const` this call's own construction ultimately
    /// executed under — inherited unchanged through any function call/loop
    /// iteration in between (a helper function's own *declaration* site
    /// never determines scope; only which top-level binding's evaluation
    /// dynamically reached this call does), matching `cad_feature_graph::
    /// graph::FeatureNode::scope`'s own identical convention and values.
    pub scope: Vec<String>,
}
