//! The Stage-3 Feature DAG's node identity/dependency model (`AICAD-066`)
//! and its construction from supported modeling operations (`AICAD-067`),
//! per `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §9-10 (WP-06,
//! `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §7).
//!
//! ## What a "feature node" is here
//!
//! Per `project/OWNER_DECISIONS.md#D18`/`DECISION_LOG.md#DL-15`, every
//! Stage-2 geometry construction is an ordinary call to one of the eight
//! closed `cad_hir::builtins::BuiltinFnId` "supported modeling operations"
//! (`box`/`cylinder`/`transform`/`union`/`cut`/`intersect`/`fillet`/
//! `chamfer`, `cad_hir::builtins::catalogue`) — there is no other way an
//! `.aicad` program can produce a `Geometry` value today. [`FeatureGraph::
//! build`] walks an already-lowered [`cad_hir::hir::HirProgram`] and gives
//! every such call its own stable [`FeatureId`], plus the dependency edges
//! (`docs/plan/06...` §9's "input feature refs") that call's own
//! `Geometry`-typed arguments name — a direct, deliberately-reusable
//! analogue of `cad_runtime::params::ParamModel`'s own node-identity/
//! dependency-edge design for top-level `param` declarations (see that
//! module's doc comment), applied to feature construction instead.
//!
//! ## Node identity: minted, not reused from `BindingId`
//!
//! Unlike [`cad_runtime::params::ParamId`] (which wraps an existing
//! `BindingId` because every `param` is itself a declaration), a
//! `Geometry`-producing expression is not always bound to a name — `union
//! (base, cylinder(4mm, 10mm))`'s `cylinder(...)` argument is itself a
//! feature (an operation that consumes/produces a `Geometry` value,
//! `docs/plan/06...` §9), but nothing declares it. [`FeatureId`] is
//! therefore minted fresh by [`FeatureGraph`] in strictly increasing order
//! as each node is built, exactly like `cad_geometry_api::ir::GeomId` —
//! the direct precedent for "SSA-style identity for an unnamed
//! construction step" already established one layer below this one. A
//! top-level `let`/`const` whose value directly resolves to a feature node
//! additionally gets that node registered under its own `BindingId`
//! ([`FeatureGraph::find_by_binding`]), so a *named* feature keeps its
//! ordinary lexical identity too, and a later reference to that name
//! (`union(base, base)`) resolves to the *same* node rather than building
//! a duplicate — required for the dependency graph to actually be a DAG
//! over shared substructure, not a tree that silently re-evaluates shared
//! inputs.
//!
//! ## What this module deliberately does not do (later tasks' scope)
//!
//! - **Cache keys / dirty propagation** (`AICAD-068`) and **source-to-
//!   feature provenance** (`AICAD-069`) are explicitly separate tasks in
//!   `project/TASKS.yaml`'s fixed Batch S3-01/S3-02 ordering; this module
//!   builds the identity/dependency structure they will each build on top
//!   of, nothing more — mirrors `cad_runtime::params`' own precedent
//!   ("this task does not itself build a cache/dirty-propagation graph...
//!   only the parameter layer it will sit on top of").
//! - **Evaluating** a feature's own scalar parameters into actual
//!   `cad_units` quantities is `cad_runtime`'s job. [`FeatureNode::
//!   parameters`] stores each non-`Geometry` parameter's *expression*
//!   (borrowed from the source [`HirProgram`]), never an evaluated value —
//!   the same "borrowed expression, not a computed value" design
//!   `cad_runtime::params::ParamDecl::default` already uses for a `param`'s
//!   own default expression.
//! - **Interprocedural construction.** Only a direct call to a
//!   `RuntimeBuiltin` (or a bare reference to an already-built named
//!   feature) is recognized as a feature. A top-level `let`/`const` whose
//!   value is a call to an ordinary AICAD-source `fn` that itself
//!   constructs geometry (e.g. `let plate = rounded_plate(...);`, as
//!   `examples/brackets/stage2_mounting_plate.aicad` does throughout) is
//!   *not* inlined/flattened into this graph — "supported modeling
//!   operations" (this task's own title) names exactly the closed
//!   `BuiltinFnId` catalogue, not arbitrary user-defined functions that
//!   happen to call into it. Extending the feature DAG to see through
//!   ordinary function calls is a distinct, larger design question
//!   (how much of an arbitrary call graph is "one feature"?) this task
//!   does not need to answer and does not silently guess at.
//! - **Conditional/branching feature selection.** A `Geometry`-typed
//!   `if`/`match` expression (selecting between two differently-built
//!   features depending on a runtime value) is not modeled — recognized
//!   shapes are limited to a direct builtin call or a plain name
//!   reference to an already-built node. Which single feature identity (if
//!   any) a conditional expression should be assigned is exactly the kind
//!   of architecture question `AGENTS.md`'s "ambiguity is an error, never
//!   an arbitrary selection" non-negotiable protects against silently
//!   deciding; a top-level binding shaped this way is simply not modeled
//!   as a feature (not an error — see [`FeatureGraph::build`]'s own doc
//!   comment on when an unmodeled binding is/is not an error).
//! - **`part` bodies.** Only `program.items`-level (module top-level)
//!   `let`/`const` are scanned, matching `cad_runtime::params::ParamModel`'s
//!   own identical, explicitly documented scope boundary — `part`
//!   instantiation semantics are `AICAD-072`'s job, not yet decided.

use cad_ast::Span;
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SourceSpan};
use cad_hir::builtins::{BuiltinFnId, BuiltinFnSpec};
use cad_hir::hir::{HirArg, HirCallee, HirExpr, HirItem, HirProgram};
use cad_hir::ids::BindingId;
use cad_hir::types::HirTypeRef;
use std::collections::HashMap;
use std::fmt;

/// SSA-style identity of one node within a single [`FeatureGraph`] — see
/// module doc comment "Node identity: minted, not reused from
/// `BindingId`". Meaningless (and never produced) against any other
/// `FeatureGraph`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FeatureId(u32);

impl FeatureId {
    pub fn index(self) -> u32 {
        self.0
    }
}

impl fmt::Display for FeatureId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "feature#{}", self.0)
    }
}

/// One feature node: a single supported-modeling-operation call, plus the
/// dependency edges and parameter expressions [`FeatureGraph::build`]
/// found for it. Carries exactly the subset of `docs/plan/06_REFERENCES_
/// QUERIES_FEATURE_DAG.md` §9's node-field list this task builds (`id`,
/// `source_span`, `kind`, `parameters`, `input feature refs`) — see this
/// module's doc comment for which fields are later tasks' own scope.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureNode<'a> {
    pub id: FeatureId,
    pub span: Span,
    /// The supported modeling operation this node calls — `docs/plan/06
    /// ...`'s "kind" field, restated as the same closed identity
    /// `cad_hir::builtins::catalogue` already gives it (never a second,
    /// parallel operation-kind enum).
    pub op: BuiltinFnId,
    /// The top-level `let`/`const` name this node was built directly from,
    /// if any. `None` for a node built from an unnamed nested argument
    /// expression (e.g. `cylinder(...)` inlined directly as another
    /// operation's argument, never itself bound to a name).
    pub name: Option<&'a str>,
    /// Other feature nodes this node's own `Geometry`-typed arguments
    /// reference — `docs/plan/06...`'s "input feature refs" — in the
    /// operation's declared parameter order (`cad_hir::builtins::
    /// BuiltinFnSpec::params`), regardless of the order the caller's own
    /// argument list (positional or named) actually wrote them in. May
    /// repeat the same [`FeatureId`] (`union(base, base)`).
    pub geometry_inputs: Vec<FeatureId>,
    /// This node's own non-`Geometry` (scalar) parameters — `docs/plan/06
    /// ...`'s "parameters" field — as `(declared parameter name, bound
    /// argument expression)` pairs, in declared parameter order. The
    /// expression is borrowed from the source program, never evaluated;
    /// see this module's doc comment "What this module deliberately does
    /// not do".
    pub parameters: Vec<(&'static str, &'a HirExpr)>,
}

/// Every way [`FeatureGraph::build`] can fail — both are purely
/// structural/static, no kernel or interpreter involvement, mirroring
/// `cad_geometry_api::ir::GeometryIrError`'s own "static, structural"
/// design one layer below this one.
#[derive(Debug, Clone, PartialEq)]
pub enum FeatureGraphError {
    /// A `Geometry`-typed parameter slot's bound argument expression is
    /// not a shape [`FeatureGraph::build`] recognizes as a feature (a
    /// direct supported-modeling-operation call, or a name already built
    /// into one) — see module doc comment "What this module deliberately
    /// does not do" for the (non-exhaustive) list of unrecognized shapes.
    /// `context` names the operation and parameter, e.g. `"cut.rhs"`,
    /// mirroring `GeometryIrError::DimensionMismatch::context`.
    UnresolvedGeometryInput { context: String, span: Span },
    /// A supported-modeling-operation call's own argument list did not
    /// resolve cleanly against `cad_hir::builtins::catalogue`'s signature
    /// (an unknown named argument, more positional arguments than declared
    /// parameters, or a parameter left unfilled). Defensive only —
    /// unreachable for a program `cad_hir::typeck::check_program` already
    /// accepted cleanly (every `RuntimeBuiltin` call is checked through
    /// the exact same call-checking machinery as an ordinary `fn`,
    /// `project/DECISION_LOG.md#DL-15`); mirrors `cad_runtime::error::
    /// RuntimeError::BuiltinArgumentShape`'s own identical "trusts, but
    /// verifies" doc comment.
    MalformedBuiltinCall { name: &'static str, span: Span },
}

impl FeatureGraphError {
    /// `GEOM` family (`cad_diagnostics::DIAGNOSTIC_FAMILIES`) — reused
    /// rather than a new family minted for this crate, since every
    /// variant here is the Feature-DAG-layer analogue of exactly the kind
    /// of static/structural well-formedness problem `cad_geometry_api::
    /// ir::GeometryIrError` (also `GEOM`) already reports one layer below
    /// (a dangling/non-geometry operand reference); `project/
    /// OWNER_DECISIONS.md#D10`'s "adding a new code in an existing
    /// family... is ordinary task work" applies directly, and `cad_
    /// runtime::error::RuntimeError`'s own `AICAD-065` precedent
    /// (`CyclicParamDependency`, `RUNTIME` family) deliberately did not
    /// mint a new family for a new-but-structurally-similar error either.
    pub fn code(&self) -> &'static str {
        match self {
            FeatureGraphError::UnresolvedGeometryInput { .. } => "GEOM-E005",
            FeatureGraphError::MalformedBuiltinCall { .. } => "GEOM-E006",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            FeatureGraphError::UnresolvedGeometryInput { .. } => "UNRESOLVED_GEOMETRY_INPUT",
            FeatureGraphError::MalformedBuiltinCall { .. } => "MALFORMED_BUILTIN_CALL",
        }
    }

    pub fn message(&self) -> String {
        match self {
            FeatureGraphError::UnresolvedGeometryInput { context, .. } => format!(
                "{context} requires a Geometry-producing feature expression (a supported \
                 modeling-operation call, or a reference to one already built), found something \
                 else"
            ),
            FeatureGraphError::MalformedBuiltinCall { name, .. } => {
                format!("call to `{name}` does not match its own declared parameter list")
            }
        }
    }

    pub fn span(&self) -> Span {
        match self {
            FeatureGraphError::UnresolvedGeometryInput { span, .. }
            | FeatureGraphError::MalformedBuiltinCall { span, .. } => *span,
        }
    }

    /// Builds this error's `cad_diagnostics::Diagnostic`, mirroring
    /// `GeometryIrError::to_diagnostic`'s own pattern exactly.
    pub fn to_diagnostic(&self, file: &str, source: &str) -> Diagnostic {
        let span = self.span();
        let line_index = cad_ast::LineIndex::new(source);
        let start = line_index.line_column(source, span.start);
        let end = line_index.line_column(source, span.end);
        let code = DiagnosticCode::parse(self.code())
            .expect("FeatureGraphError::code always returns a well-formed FAMILY-Exxx code");
        Diagnostic::new(
            code,
            Severity::Error,
            "feature-graph",
            self.title(),
            self.message(),
        )
        .expect("every FeatureGraphError code carries the 'E' severity letter")
        .with_source(SourceSpan {
            file: file.to_string(),
            start: Position::new(start.line, start.column),
            end: Position::new(end.line, end.column),
        })
    }
}

impl fmt::Display for FeatureGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for FeatureGraphError {}

/// The Stage-3 feature DAG built from one already-lowered [`HirProgram`]'s
/// top-level supported-modeling-operation calls — see module doc comment.
#[derive(Debug, Clone)]
pub struct FeatureGraph<'a> {
    nodes: Vec<FeatureNode<'a>>,
    named: HashMap<BindingId, FeatureId>,
}

impl<'a> FeatureGraph<'a> {
    /// Builds the feature DAG for `program`'s top-level `let`/`const`
    /// declarations — see module doc comment for exactly what is/is not
    /// modeled.
    ///
    /// A top-level `let`/`const` whose value is *not* a recognized
    /// feature shape (an ordinary scalar computation, a call to a
    /// user-defined `fn`, ...) is simply not added to the graph — this is
    /// not an error; only a `Geometry`-typed *parameter slot* of an
    /// already-recognized feature call that fails to resolve is
    /// ([`FeatureGraphError::UnresolvedGeometryInput`]), since at that
    /// point the enclosing call unambiguously requires one.
    pub fn build(program: &'a HirProgram) -> Result<FeatureGraph<'a>, FeatureGraphError> {
        let mut builder = Builder {
            builtins: index_builtins(&program.items),
            catalogue: cad_hir::builtins::catalogue()
                .into_iter()
                .map(|spec| (spec.id, spec))
                .collect(),
            nodes: Vec::new(),
            named: HashMap::new(),
        };

        for item in &program.items {
            let (binding, name, value) = match item {
                HirItem::Let {
                    binding,
                    name,
                    value,
                    ..
                }
                | HirItem::Const {
                    binding,
                    name,
                    value,
                    ..
                } => (binding, name, value),
                _ => continue,
            };
            if let Some(id) = builder.resolve_geometry_expr(value)? {
                let node = &mut builder.nodes[id.0 as usize];
                if node.name.is_none() {
                    node.name = Some(name.as_str());
                }
                builder.named.insert(*binding, id);
            }
        }

        Ok(FeatureGraph {
            nodes: builder.nodes,
            named: builder.named,
        })
    }

    /// Every feature node, in build (dependency-respecting, since a node
    /// always exists before anything that references it) order.
    pub fn nodes(&self) -> &[FeatureNode<'a>] {
        &self.nodes
    }

    pub fn get(&self, id: FeatureId) -> Option<&FeatureNode<'a>> {
        self.nodes.get(id.0 as usize)
    }

    /// The feature node a top-level `let`/`const`'s own `BindingId`
    /// resolved to, if its value was a recognized feature shape.
    pub fn find_by_binding(&self, binding: BindingId) -> Option<FeatureId> {
        self.named.get(&binding).copied()
    }
}

/// Owns the in-progress node list plus the lookup tables [`FeatureGraph::
/// build`] needs while walking `program` — kept as its own type so
/// [`FeatureGraph`] itself never carries the `builtins`/`catalogue` tables
/// past construction (they are meaningless once building is done).
struct Builder<'a> {
    /// Every top-level `RuntimeBuiltin` `fn` item's own `BindingId` ->
    /// which `BuiltinFnId` it implements (`cad_hir::lower::Lowerer::
    /// seed_builtins` always appends these as ordinary top-level
    /// `HirItem::Fn` entries, never inside a `part` body — see module doc
    /// comment "`part` bodies").
    builtins: HashMap<BindingId, BuiltinFnId>,
    catalogue: HashMap<BuiltinFnId, BuiltinFnSpec>,
    nodes: Vec<FeatureNode<'a>>,
    named: HashMap<BindingId, FeatureId>,
}

impl<'a> Builder<'a> {
    /// Resolves `expr` as a `Geometry`-producing feature expression,
    /// returning the node it names (building a fresh one if `expr` is a
    /// direct supported-modeling-operation call not seen before) — or
    /// `None` if `expr` is not a recognized shape at all (see module doc
    /// comment "What this module deliberately does not do"). Never builds
    /// a duplicate node for a name already resolved into one (`union(base,
    /// base)`).
    fn resolve_geometry_expr(
        &mut self,
        expr: &'a HirExpr,
    ) -> Result<Option<FeatureId>, FeatureGraphError> {
        match expr {
            HirExpr::Ident {
                binding: Some(b), ..
            } => Ok(self.named.get(b).copied()),
            HirExpr::Call {
                callee: HirCallee::Fn {
                    binding: Some(b), ..
                },
                args,
                span,
            } => {
                let Some(&op) = self.builtins.get(b) else {
                    // A call to a user-defined `fn` (or an unresolved
                    // name) is never itself a feature node — see module
                    // doc comment "Interprocedural construction".
                    return Ok(None);
                };
                let spec = self.catalogue.get(&op).expect(
                    "catalogue() covers every BuiltinFnId, including every op index_builtins found",
                );
                if !is_geometry_type(&spec.return_ty) {
                    // Forward-compatible only: every current supported
                    // modeling operation returns Geometry, so this branch
                    // is unreachable today, but a hypothetical future
                    // scalar-returning `RuntimeBuiltin` must not be
                    // silently treated as a feature.
                    return Ok(None);
                }
                let fn_name = spec.name;
                let param_flags: Vec<(&'static str, bool)> = spec
                    .params
                    .iter()
                    .map(|(pname, pty)| (*pname, is_geometry_type(pty)))
                    .collect();
                let slots = resolve_slots(&param_flags, fn_name, args, *span)?;

                let mut geometry_inputs = Vec::with_capacity(param_flags.len());
                let mut parameters = Vec::with_capacity(param_flags.len());
                for ((pname, is_geometry), arg_expr) in param_flags.iter().zip(slots.iter()) {
                    if *is_geometry {
                        let child = self.resolve_geometry_expr(arg_expr)?.ok_or_else(|| {
                            FeatureGraphError::UnresolvedGeometryInput {
                                context: format!("{fn_name}.{pname}"),
                                span: arg_expr.span(),
                            }
                        })?;
                        geometry_inputs.push(child);
                    } else {
                        parameters.push((*pname, *arg_expr));
                    }
                }

                let id = FeatureId(self.nodes.len() as u32);
                self.nodes.push(FeatureNode {
                    id,
                    span: *span,
                    op,
                    name: None,
                    geometry_inputs,
                    parameters,
                });
                Ok(Some(id))
            }
            _ => Ok(None),
        }
    }
}

/// Indexes every top-level `RuntimeBuiltin` `fn` item by its own
/// `BindingId` -> `BuiltinFnId`. Deliberately does *not* recurse into
/// `HirItem::Part` bodies (see module doc comment "`part` bodies") —
/// unlike `cad_runtime::interp::index_fns`'s own recursive analogue (which
/// indexes *every* callable `fn`, since the interpreter must be able to
/// call one from inside a `part` body too), this index only ever needs to
/// recognize the eight builtins `cad_hir::lower::Lowerer::seed_builtins`
/// always seeds at the top level.
fn index_builtins(items: &[HirItem]) -> HashMap<BindingId, BuiltinFnId> {
    let mut out = HashMap::new();
    for item in items {
        if let HirItem::Fn {
            binding,
            body: cad_hir::hir::FunctionImplementation::RuntimeBuiltin(id),
            ..
        } = item
        {
            out.insert(*binding, *id);
        }
    }
    out
}

fn is_geometry_type(ty: &HirTypeRef) -> bool {
    matches!(ty, HirTypeRef::Named { name, .. } if name == "Geometry")
}

/// Resolves `args` (an already-type-checked call's own argument list)
/// against `params` (`(declared parameter name, is-Geometry)` pairs, in
/// declared order), mirroring `cad_runtime::interp::Interpreter::call`'s
/// own identical positional-then-named slot-filling algorithm exactly (see
/// that function's own body) — but over borrowed `HirExpr`s, never
/// evaluated `Value`s, since this crate has no interpreter.
fn resolve_slots<'e>(
    params: &[(&'static str, bool)],
    fn_name: &'static str,
    args: &'e [HirArg],
    span: Span,
) -> Result<Vec<&'e HirExpr>, FeatureGraphError> {
    let mut slots: Vec<Option<&'e HirExpr>> = vec![None; params.len()];
    let mut next_positional = 0usize;
    for arg in args {
        match arg {
            HirArg::Positional(expr) => {
                if next_positional >= slots.len() {
                    return Err(FeatureGraphError::MalformedBuiltinCall {
                        name: fn_name,
                        span,
                    });
                }
                slots[next_positional] = Some(expr);
                next_positional += 1;
            }
            HirArg::Named {
                name: arg_name,
                value,
                ..
            } => {
                let idx = params
                    .iter()
                    .position(|(pname, _)| pname == arg_name)
                    .ok_or(FeatureGraphError::MalformedBuiltinCall {
                        name: fn_name,
                        span,
                    })?;
                slots[idx] = Some(value);
            }
        }
    }
    slots
        .into_iter()
        .map(|slot| {
            slot.ok_or(FeatureGraphError::MalformedBuiltinCall {
                name: fn_name,
                span,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_hir::lower::LowerResult;

    fn lowered(source: &str) -> LowerResult {
        let (program, parse_diagnostics) = cad_parser::parse_program(source, "test.aicad");
        assert!(
            parse_diagnostics.is_empty(),
            "test source failed to parse: {parse_diagnostics:?}"
        );
        let lowered = cad_hir::lower::lower_program(&program, "test.aicad", source);
        assert!(
            lowered.diagnostics.is_empty(),
            "test source failed to lower cleanly: {:?}",
            lowered.diagnostics
        );
        let checked = cad_hir::typeck::check_program(
            &lowered.program,
            &lowered.bindings,
            "test.aicad",
            source,
        );
        assert!(
            checked.diagnostics.is_empty(),
            "test source failed to type-check: {:?}",
            checked.diagnostics
        );
        lowered
    }

    fn binding_of(lowered: &LowerResult, name: &str) -> BindingId {
        for item in &lowered.program.items {
            match item {
                HirItem::Let {
                    binding, name: n, ..
                }
                | HirItem::Const {
                    binding, name: n, ..
                } if n == name => {
                    return *binding;
                }
                _ => {}
            }
        }
        panic!("no top-level let/const named {name:?}");
    }

    #[test]
    fn single_box_becomes_one_named_feature_node() {
        let lowered = lowered("let base = box(10mm, 10mm, 10mm);\n");
        let graph = FeatureGraph::build(&lowered.program).expect("builds cleanly");
        assert_eq!(graph.nodes().len(), 1);
        let node = &graph.nodes()[0];
        assert_eq!(node.op, BuiltinFnId::Box);
        assert_eq!(node.name, Some("base"));
        assert!(node.geometry_inputs.is_empty());
        assert_eq!(node.parameters.len(), 3);
        assert_eq!(node.parameters[0].0, "dx");

        let base = binding_of(&lowered, "base");
        assert_eq!(graph.find_by_binding(base), Some(node.id));
    }

    #[test]
    fn dependent_feature_gets_a_geometry_input_edge() {
        let lowered = lowered(
            "let base = box(10mm, 10mm, 10mm);\n\
             let hole = cylinder(1mm, 10mm);\n\
             let drilled = cut(base, hole);\n",
        );
        let graph = FeatureGraph::build(&lowered.program).expect("builds cleanly");
        assert_eq!(graph.nodes().len(), 3);

        let base = graph.find_by_binding(binding_of(&lowered, "base")).unwrap();
        let hole = graph.find_by_binding(binding_of(&lowered, "hole")).unwrap();
        let drilled = graph
            .find_by_binding(binding_of(&lowered, "drilled"))
            .unwrap();

        let drilled_node = graph.get(drilled).unwrap();
        assert_eq!(drilled_node.op, BuiltinFnId::Cut);
        assert_eq!(drilled_node.geometry_inputs, vec![base, hole]);
        assert!(drilled_node.parameters.is_empty());
    }

    #[test]
    fn unnamed_nested_call_gets_its_own_anonymous_node() {
        let lowered = lowered(
            "let base = box(10mm, 10mm, 10mm);\nlet drilled = cut(base, cylinder(1mm, 10mm));\n",
        );
        let graph = FeatureGraph::build(&lowered.program).expect("builds cleanly");
        // base, the inline cylinder(...), and drilled: three nodes total.
        assert_eq!(graph.nodes().len(), 3);
        let cylinder_node = graph
            .nodes()
            .iter()
            .find(|n| n.op == BuiltinFnId::Cylinder)
            .expect("an anonymous Cylinder node was built");
        assert_eq!(cylinder_node.name, None);
    }

    #[test]
    fn shared_named_input_is_reused_not_duplicated() {
        let lowered = lowered(
            "let base = box(10mm, 10mm, 10mm);\n\
             let boss = cylinder(4mm, 12mm);\n\
             let a = union(base, boss);\n\
             let b = cut(base, boss);\n",
        );
        let graph = FeatureGraph::build(&lowered.program).expect("builds cleanly");
        // base, boss, a, b: exactly four nodes -- base/boss are each built
        // once and referenced twice, never duplicated.
        assert_eq!(graph.nodes().len(), 4);

        let base = graph.find_by_binding(binding_of(&lowered, "base")).unwrap();
        let boss = graph.find_by_binding(binding_of(&lowered, "boss")).unwrap();
        let a = graph
            .get(graph.find_by_binding(binding_of(&lowered, "a")).unwrap())
            .unwrap();
        let b = graph
            .get(graph.find_by_binding(binding_of(&lowered, "b")).unwrap())
            .unwrap();
        assert_eq!(a.geometry_inputs, vec![base, boss]);
        assert_eq!(b.geometry_inputs, vec![base, boss]);
    }

    #[test]
    fn named_arguments_resolve_to_the_correct_parameter_slot() {
        let lowered = lowered("let base = box(dz = 3mm, dx = 10mm, dy = 20mm);\n");
        let graph = FeatureGraph::build(&lowered.program).expect("builds cleanly");
        let node = &graph.nodes()[0];
        let by_name = |n: &str| -> &HirExpr {
            node.parameters
                .iter()
                .find(|(pname, _)| *pname == n)
                .map(|(_, expr)| expr)
                .unwrap()
        };
        // Sanity: every declared parameter name was actually filled,
        // regardless of the caller's own (fully-named, reordered) list.
        assert!(matches!(by_name("dx"), HirExpr::Literal { .. }));
        assert!(matches!(by_name("dy"), HirExpr::Literal { .. }));
        assert!(matches!(by_name("dz"), HirExpr::Literal { .. }));
    }

    #[test]
    fn alias_reference_shares_the_same_feature_id() {
        let lowered = lowered("let base = box(10mm, 10mm, 10mm);\nlet alias = base;\n");
        let graph = FeatureGraph::build(&lowered.program).expect("builds cleanly");
        assert_eq!(graph.nodes().len(), 1, "alias must not build a second node");
        let base = graph.find_by_binding(binding_of(&lowered, "base")).unwrap();
        let alias = graph
            .find_by_binding(binding_of(&lowered, "alias"))
            .unwrap();
        assert_eq!(base, alias);
    }

    #[test]
    fn non_geometry_let_is_simply_not_modeled() {
        let lowered = lowered("let width: Length = 10mm;\n");
        let graph = FeatureGraph::build(&lowered.program).expect("builds cleanly");
        assert!(graph.nodes().is_empty());
        assert_eq!(graph.find_by_binding(binding_of(&lowered, "width")), None);
    }

    #[test]
    fn call_to_a_user_defined_fn_is_not_a_feature_node() {
        let lowered = lowered(
            "fn make_it() -> Geometry {\n\
             \treturn box(1mm, 1mm, 1mm);\n\
             }\n\
             let x = make_it();\n",
        );
        let graph = FeatureGraph::build(&lowered.program).expect("builds cleanly");
        assert!(graph.nodes().is_empty());
    }

    #[test]
    fn dangling_geometry_argument_is_a_structured_error() {
        let lowered = lowered(
            "fn make_it() -> Geometry {\n\
             \treturn box(1mm, 1mm, 1mm);\n\
             }\n\
             let x = make_it();\n\
             let y = union(x, box(1mm, 1mm, 1mm));\n",
        );
        let err = FeatureGraph::build(&lowered.program).expect_err("x is not a built feature");
        match &err {
            FeatureGraphError::UnresolvedGeometryInput { context, .. } => {
                assert_eq!(context, "union.a");
            }
            other => panic!("expected UnresolvedGeometryInput, got {other:?}"),
        }
        assert_eq!(err.code(), "GEOM-E005");
        let diagnostic = err.to_diagnostic("test.aicad", "x");
        assert_eq!(diagnostic.category, "feature-graph");
    }

    #[test]
    fn forward_reference_is_unresolved_not_silently_reordered() {
        // `cad_hir::lower`'s own two-pass declare-then-lower-body structure
        // (see that module's `lower_items`) resolves `later`'s name even
        // though it is declared textually *after* `earlier` references it
        // -- but this graph builder walks `program.items` in declaration
        // order and only ever registers a name once its own node is
        // built, so a genuine forward reference is unresolved here
        // regardless (matching `cad_runtime`'s own plain-source-order
        // top-level evaluation, which could not evaluate this either).
        let lowered = lowered(
            "let earlier = union(later, box(2mm, 2mm, 2mm));\n\
             let later = box(1mm, 1mm, 1mm);\n",
        );
        let err = FeatureGraph::build(&lowered.program).expect_err("later is not yet built");
        assert_eq!(
            err,
            FeatureGraphError::UnresolvedGeometryInput {
                context: "union.a".to_string(),
                span: {
                    let HirItem::Let { value, .. } = &lowered.program.items[0] else {
                        unreachable!()
                    };
                    let HirExpr::Call { args, .. } = value else {
                        unreachable!()
                    };
                    args[0].span()
                },
            }
        );
    }

    #[test]
    fn every_error_variant_converts_to_a_well_formed_diagnostic() {
        let errors = vec![
            FeatureGraphError::UnresolvedGeometryInput {
                context: "cut.rhs".to_string(),
                span: Span::new(0, 1),
            },
            FeatureGraphError::MalformedBuiltinCall {
                name: "box",
                span: Span::new(0, 1),
            },
        ];
        for err in errors {
            let code = err.code();
            assert!(code.starts_with("GEOM-E"));
            DiagnosticCode::parse(code).expect("every FeatureGraphError code must be well-formed");
            let diagnostic = err.to_diagnostic("test.aicad", "x");
            assert_eq!(diagnostic.category, "feature-graph");
            assert!(!diagnostic.message.is_empty());
            assert!(!err.to_string().is_empty());
        }
    }
}
