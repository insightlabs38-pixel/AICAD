//! Turns a completed interpreter execution trace
//! (`cad_runtime::feature_trace`) into a real feature/dependency graph
//! (`AICAD-107`, `project/DECISION_LOG.md#DL-27`).
//!
//! `crate::graph::FeatureGraph` (Stage 3) builds feature identity purely
//! statically, from an already-lowered `HirProgram`'s own top-level (and
//! `part`-nested, `D31`) `let`/`const` declarations — deliberately never
//! looking through a user-defined `fn` call, a taken `if`/`match` branch, or
//! a loop (see that module's own doc comment, "What this module
//! deliberately does not do"). `D25`/`DL-27` requires Stage 5 to keep
//! geometry built through exactly those abstractions visible to the
//! feature/dependency/provenance system, which a static AST walk cannot do
//! (which branch actually ran, and how many times a loop body actually
//! executed, are only knowable by running the program).
//!
//! [`TraceFeatureGraph`] is therefore the execution-trace counterpart:
//! built from [`cad_runtime::interp::Interpreter::trace`] (populated as the
//! interpreter runs — see that method's own doc comment), covering every
//! Geometry-returning `RuntimeBuiltin` call the run actually performed,
//! wherever it occurred. It deliberately stays a **separate type** from
//! [`crate::graph::FeatureGraph`] rather than one constructor branch of it:
//! the two have genuinely different node identity ([`crate::graph::
//! FeatureId`] is minted from a purely static AST position;
//! [`TraceFeatureId`] is minted from a dynamic [`cad_runtime::
//! feature_trace::CallPath`]) and different failure modes (a static walk
//! can find a call shape it does not recognize and report
//! [`crate::graph::FeatureGraphError`]; a trace, built from an
//! already-successfully-executed run, cannot — every entry it contains is
//! already known-valid). Collapsing them into one type would either erase
//! that distinction or force one of the two callers (a purely static
//! tool with no interpreter, vs. `cad-cli`'s own real build/rebuild
//! pipeline) to depend on machinery it does not need.
//!
//! [`TraceFeatureGraph::dirty_set`] reuses exactly the same dirty-
//! propagation contract [`crate::graph::FeatureGraph::dirty_set`] already
//! established (direct dependents via `binding_refs`, transitive
//! dependents via `geometry_inputs`) — the *policy* is identical; only the
//! node identity/source differs.

use cad_hir::builtins::BuiltinFnId;
use cad_hir::ids::BindingId;
use cad_runtime::feature_trace::{CallPath, TraceEntry};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// SSA-style identity of one node within a single [`TraceFeatureGraph`] —
/// deliberately a distinct type from [`crate::graph::FeatureId`] (see this
/// module's own doc comment for why); reuses that type's own underlying
/// representation and `Display` spelling for a consistent developer
/// experience across both graphs (a diagnostic/log line naming
/// `trace-feature#3` reads the same way `feature#3` already does).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TraceFeatureId(u32);

impl fmt::Display for TraceFeatureId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "trace-feature#{}", self.0)
    }
}

/// One feature node built from one [`TraceEntry`] — the execution-trace
/// counterpart of [`crate::graph::FeatureNode`]; see this module's own doc
/// comment for exactly how the two differ. Owns (moves) its source
/// [`TraceEntry`]'s own fields rather than borrowing them: unlike
/// [`crate::graph::FeatureNode`] (which borrows `&'a HirExpr` straight from
/// the still-alive `HirProgram` a purely static walk never needs to leave),
/// a [`TraceEntry`] already carries only owned data (see that type's own
/// doc comment for why), so there is nothing to borrow from and no
/// lifetime parameter to thread here either.
#[derive(Debug, Clone)]
pub struct TraceFeatureNode {
    pub id: TraceFeatureId,
    pub path: CallPath,
    pub op: BuiltinFnId,
    /// Other trace-feature nodes this node's own `Geometry`-typed arguments
    /// reference, resolved from [`TraceEntry::geometry_inputs`]' own
    /// [`CallPath`]s against this graph's own path-to-id table — the
    /// dynamic-execution counterpart of [`crate::graph::FeatureNode::
    /// geometry_inputs`]. Omits an input `CallPath` this graph's own
    /// [`TraceFeatureGraph::build`] did not itself assign an id to
    /// (defensive only: a well-formed trace never has this, since every
    /// `Geometry` value's producing call is itself always traced — see
    /// [`cad_runtime::feature_trace::TraceEntry::geometry_inputs`]'s own
    /// doc comment).
    pub geometry_inputs: Vec<TraceFeatureId>,
    pub parameters: Vec<(String, cad_ast::Span)>,
    pub binding_refs: Vec<BindingId>,
    pub scope: Vec<String>,
    /// The exact, contiguous raw `GeomId` range this node's own call pushed
    /// — copied straight from [`TraceEntry::geom_range`], the sole source a
    /// consumer (`cad-cli`'s own incremental dispatch) needs to translate a
    /// dirty node into the kernel-dispatch work it implies, with no
    /// separate span-keyed lookup required.
    pub geom_range: std::ops::Range<u32>,
}

/// The execution-trace feature/dependency graph — see module doc comment.
#[derive(Debug, Clone)]
pub struct TraceFeatureGraph {
    nodes: Vec<TraceFeatureNode>,
    by_path: HashMap<CallPath, TraceFeatureId>,
}

impl TraceFeatureGraph {
    /// Builds the graph from `trace` — every [`TraceEntry`]
    /// [`cad_runtime::interp::Interpreter::trace`] returned after a
    /// successful run, in that same execution order (already
    /// dependency-respecting: a call's own `geometry_inputs` were always
    /// dispatched, and therefore already traced, before the call itself
    /// — `cad_runtime::interp::Interpreter::call`'s own evaluate-arguments-
    /// before-dispatching order). Infallible: unlike [`crate::graph::
    /// FeatureGraph::build`], there is no "unrecognized shape" case here —
    /// every entry already represents a call this interpreter run actually,
    /// successfully performed.
    pub fn build(trace: &[TraceEntry]) -> TraceFeatureGraph {
        let mut nodes = Vec::with_capacity(trace.len());
        let mut by_path = HashMap::with_capacity(trace.len());
        for entry in trace {
            let id = TraceFeatureId(nodes.len() as u32);
            let geometry_inputs = entry
                .geometry_inputs
                .iter()
                .filter_map(|input_path| by_path.get(input_path).copied())
                .collect();
            nodes.push(TraceFeatureNode {
                id,
                path: entry.path.clone(),
                op: entry.op,
                geometry_inputs,
                parameters: entry.parameters.clone(),
                binding_refs: entry.binding_refs.clone(),
                scope: entry.scope.clone(),
                geom_range: entry.geom_range.clone(),
            });
            by_path.insert(entry.path.clone(), id);
        }
        TraceFeatureGraph { nodes, by_path }
    }

    /// Every feature node, in build (dependency-respecting) order.
    pub fn nodes(&self) -> &[TraceFeatureNode] {
        &self.nodes
    }

    pub fn get(&self, id: TraceFeatureId) -> Option<&TraceFeatureNode> {
        self.nodes.get(id.0 as usize)
    }

    /// The node built from the traced call at `path`, if any — how a
    /// caller resolves a [`cad_runtime::interp::Interpreter::
    /// geom_id_path`] result (a top-level/`part`-nested binding's own
    /// current `Geometry` value, resolved back to its producing
    /// [`CallPath`]) into this graph's own node identity, for named lookup.
    pub fn find_by_path(&self, path: &CallPath) -> Option<TraceFeatureId> {
        self.by_path.get(path).copied()
    }

    /// Every feature node that must be rebuilt after the top-level bindings
    /// in `changed` had their own *value* overridden — the execution-trace
    /// counterpart of [`crate::graph::FeatureGraph::dirty_set`], reusing
    /// its identical two-step contract (directly dirty via `binding_refs`;
    /// transitively dirty via any dirty `geometry_inputs`) unchanged, over
    /// this graph's own node/id shape. Correct in one linear pass because
    /// [`TraceFeatureGraph::nodes`] is already dependency-respecting order
    /// (see [`TraceFeatureGraph::build`]'s own doc comment).
    pub fn dirty_set(&self, changed: &HashSet<BindingId>) -> HashSet<TraceFeatureId> {
        let mut dirty = HashSet::new();
        for node in &self.nodes {
            let directly_dirty = node.binding_refs.iter().any(|b| changed.contains(b));
            let transitively_dirty = node.geometry_inputs.iter().any(|dep| dirty.contains(dep));
            if directly_dirty || transitively_dirty {
                dirty.insert(node.id);
            }
        }
        dirty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_ast::Span;
    use std::ops::Range;

    /// A real, lowering-minted [`BindingId`] this crate cannot fabricate on
    /// its own (`BindingId::new` is `pub(crate)` to `cad-hir`) — mirrors
    /// `crate::graph`'s own test module's identical `binding_of` helper.
    fn real_binding(source: &str, name: &str) -> BindingId {
        let (program, parse_diagnostics) = cad_parser::parse_program(source, "test.aicad");
        assert!(parse_diagnostics.is_empty(), "{parse_diagnostics:?}");
        let lowered = cad_hir::lower::lower_program(&program, "test.aicad", source);
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        lowered
            .bindings
            .iter()
            .find(|b| b.name == name)
            .unwrap_or_else(|| panic!("no binding named {name:?}"))
            .id
    }

    fn entry(
        leaf: Span,
        op: BuiltinFnId,
        geometry_inputs: Vec<CallPath>,
        binding_refs: Vec<BindingId>,
    ) -> TraceEntry {
        TraceEntry {
            path: CallPath::new(Vec::new(), leaf),
            op,
            geom_range: Range { start: 0, end: 1 },
            geometry_inputs,
            parameters: Vec::new(),
            binding_refs,
            scope: Vec::new(),
        }
    }

    #[test]
    fn each_trace_entry_becomes_its_own_node_in_order() {
        let box_entry = entry(Span::new(0, 1), BuiltinFnId::Box, vec![], vec![]);
        let cyl_entry = entry(Span::new(2, 3), BuiltinFnId::Cylinder, vec![], vec![]);
        let graph = TraceFeatureGraph::build(&[box_entry, cyl_entry]);
        assert_eq!(graph.nodes().len(), 2);
        assert_eq!(graph.nodes()[0].op, BuiltinFnId::Box);
        assert_eq!(graph.nodes()[1].op, BuiltinFnId::Cylinder);
    }

    #[test]
    fn geometry_inputs_resolve_to_the_producing_nodes_own_id() {
        let box_path = CallPath::new(Vec::new(), Span::new(0, 1));
        let box_entry = entry(Span::new(0, 1), BuiltinFnId::Box, vec![], vec![]);
        let cyl_entry = entry(Span::new(2, 3), BuiltinFnId::Cylinder, vec![], vec![]);
        let cyl_path = CallPath::new(Vec::new(), Span::new(2, 3));
        let cut_entry = entry(
            Span::new(4, 5),
            BuiltinFnId::Cut,
            vec![box_path, cyl_path],
            vec![],
        );
        let graph = TraceFeatureGraph::build(&[box_entry, cyl_entry, cut_entry]);
        let cut_node = &graph.nodes()[2];
        assert_eq!(
            cut_node.geometry_inputs,
            vec![graph.nodes()[0].id, graph.nodes()[1].id]
        );
    }

    #[test]
    fn find_by_path_resolves_a_traced_calls_own_node() {
        let path = CallPath::new(Vec::new(), Span::new(0, 1));
        let box_entry = entry(Span::new(0, 1), BuiltinFnId::Box, vec![], vec![]);
        let graph = TraceFeatureGraph::build(&[box_entry]);
        assert_eq!(graph.find_by_path(&path), Some(graph.nodes()[0].id));
    }

    #[test]
    fn dirty_set_marks_direct_and_transitive_dependents_only() {
        let radius = real_binding("param radius: Length = 4mm;\n", "radius");
        let base_entry = entry(Span::new(0, 1), BuiltinFnId::Box, vec![], vec![]);
        let base_path = CallPath::new(Vec::new(), Span::new(0, 1));
        let boss_entry = entry(Span::new(2, 3), BuiltinFnId::Cylinder, vec![], vec![radius]);
        let boss_path = CallPath::new(Vec::new(), Span::new(2, 3));
        let combined_entry = entry(
            Span::new(4, 5),
            BuiltinFnId::Union,
            vec![base_path, boss_path],
            vec![],
        );
        let untouched_entry = entry(Span::new(6, 7), BuiltinFnId::Box, vec![], vec![]);
        let graph =
            TraceFeatureGraph::build(&[base_entry, boss_entry, combined_entry, untouched_entry]);

        let base_id = graph.nodes()[0].id;
        let boss_id = graph.nodes()[1].id;
        let combined_id = graph.nodes()[2].id;
        let untouched_id = graph.nodes()[3].id;

        let mut changed = HashSet::new();
        changed.insert(radius);
        let dirty = graph.dirty_set(&changed);
        assert!(dirty.contains(&boss_id));
        assert!(dirty.contains(&combined_id));
        assert!(!dirty.contains(&base_id));
        assert!(!dirty.contains(&untouched_id));
    }
}
