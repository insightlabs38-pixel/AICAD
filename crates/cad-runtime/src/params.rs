//! First-class `param` declarations and derived-expression dependencies
//! for the Stage-3 parametric modeling system (`AICAD-065`).
//!
//! Stage 2 already gave `param name: Type [= default];` real parse/HIR/
//! type-check/execution support (`Item::Param`/`HirItem::Param`,
//! `Interpreter::run_top_level`) — but every top-level value, `param`
//! included, was evaluated in plain *source order*, with no separate
//! identity, no recorded dependency between one param's default
//! expression and another param it references, and no way for a caller to
//! *override* a param's value and have the rest of the model recompute
//! correctly and deterministically. That is this module's job:
//!
//! - **Explicit parameter identity** — [`ParamId`], a thin newtype over
//!   the param's own [`cad_hir::ids::BindingId`]. `BindingId` is already
//!   process-stable for identical source (`crate::lower::lower_program`'s
//!   deterministic minting counter, `project/DECISION_LOG.md#DL-12` Level
//!   1), so this does not allocate a second, parallel identity — it names
//!   the existing one for parametric-model purposes.
//! - **Typed values** — unchanged from Stage 2: a param's value is always
//!   a [`crate::value::Value`] carrying its own [`cad_units::OperandType`]
//!   (`crate::value::Value::operand_type`), never an untyped float
//!   (`AGENTS.md`).
//! - **Derived-expression dependencies** — [`ParamModel::build`] walks
//!   every param's `default` expression and records exactly which *other*
//!   top-level params it references, as explicit [`ParamId`] edges — not
//!   implicit source-position order.
//! - **Deterministic evaluation** — [`ParamModel::evaluation_order`] is a
//!   topological sort over those edges, breaking every tie by original
//!   declaration order (never `HashMap`/`HashSet` iteration order), so
//!   identical source always produces the identical schedule (`DL-12`
//!   Level 1). A cycle is a structured [`ParamModelError`], never an
//!   arbitrary evaluation order (`AGENTS.md`: "ambiguity is an error,
//!   never an arbitrary selection" — a cyclic dependency graph is exactly
//!   that kind of ambiguity).
//! - **Edit/rebuild semantics** — [`ParamOverrides`] lets a caller replace
//!   any param's value outright (skipping its `default` expression
//!   entirely); [`crate::interp::Interpreter::run_top_level_parametric`]
//!   then recomputes every *other* param whose default expression
//!   (transitively) depends on an overridden one, in the same
//!   deterministic order, using the override in place of the default.
//!   This is the parametric model's "rebuild" primitive the upcoming
//!   feature DAG (`AICAD-066`/`AICAD-067`, Batch S3-01) builds on — this
//!   task does not itself build a cache/dirty-propagation graph (that is
//!   `AICAD-068`'s job), only the parameter layer it will sit on top of.
//!
//! ## Scope boundary
//!
//! Only *top-level* `param` items (`program.items`, matching
//! `Interpreter::run_top_level`'s own existing scope) are modeled —
//! `part`-body params are a future extension, exactly like `run_top_level`
//! already only covers top-level `let`/`const`/`param` today. Dependency
//! edges are only ever recorded between two top-level `param`s; a param's
//! default expression may also reference a top-level `let`/`const` (or
//! call an ordinary function), which still evaluates correctly through the
//! ordinary interpreter — it just is not part of *this* dependency graph,
//! matching this task's explicit scope ("derived expressions" of the
//! *parametric* model, not a general top-level value dependency graph for
//! every `let`/`const`, which remains `crate::interp`'s own pre-existing,
//! separately-documented gap: "no detection of circular top-level
//! const/let value dependencies").

use crate::error::RuntimeError;
use crate::value::Value;
use cad_ast::Span;
use cad_hir::hir::{
    HirArg, HirBlock, HirCallee, HirElseStmt, HirExpr, HirItem, HirMatchArm, HirProgram, HirStmt,
};
use cad_hir::ids::BindingId;
use cad_hir::typeck::CheckedType;
use cad_hir::types::HirTypeRef;
use std::collections::{HashMap, VecDeque};

/// Stable identity of a top-level `param` declaration — see module doc
/// comment. Wraps the declaration's own [`BindingId`] rather than minting
/// a parallel one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ParamId(pub BindingId);

/// One modeled top-level `param`, plus its computed dependency edges.
#[derive(Debug, Clone)]
pub struct ParamDecl<'a> {
    pub id: ParamId,
    pub name: &'a str,
    pub span: Span,
    pub ty: &'a HirTypeRef,
    pub default: Option<&'a HirExpr>,
    /// Other top-level params this one's `default` expression directly
    /// references, in first-occurrence source order within that
    /// expression (deduplicated). Empty for a param with no `default`, or
    /// whose `default` references no other param.
    pub depends_on: Vec<ParamId>,
}

/// A caller-supplied edit: replaces a param's own `default` expression
/// entirely for one evaluation/rebuild. Keyed by [`ParamId`] (not name) so
/// a caller that already resolved a param via [`ParamModel::find_by_name`]
/// or [`ParamModel::declarations`] never has to re-resolve a name.
pub type ParamOverrides = HashMap<ParamId, Value>;

/// The first-class parametric model over one [`HirProgram`]'s top-level
/// `param` declarations — see module doc comment.
#[derive(Debug, Clone)]
pub struct ParamModel<'a> {
    decls: Vec<ParamDecl<'a>>,
    index_by_id: HashMap<ParamId, usize>,
    order: Vec<ParamId>,
}

/// A cyclic dependency among top-level `param` derived expressions,
/// detected by [`ParamModel::build`]. `cycle` is every param name/span
/// left unreachable by the topological sort (the cycle itself, plus
/// anything only reachable through it), in declaration order.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamModelError {
    pub cycle: Vec<(String, Span)>,
}

impl ParamModelError {
    /// Converts this into a [`RuntimeError::CyclicParamDependency`] a
    /// caller can render as an ordinary diagnostic — see that variant's
    /// own doc comment.
    pub fn into_runtime_error(self) -> RuntimeError {
        let span = self
            .cycle
            .first()
            .map(|(_, span)| *span)
            .expect("ParamModelError is only ever constructed with a non-empty cycle");
        let names = self.cycle.into_iter().map(|(name, _)| name).collect();
        RuntimeError::CyclicParamDependency { names, span }
    }
}

impl<'a> ParamModel<'a> {
    /// Builds the parametric model for `program`'s top-level `param`
    /// declarations — see module doc comment for exactly what this
    /// computes and its scope boundary.
    pub fn build(program: &'a HirProgram) -> Result<ParamModel<'a>, ParamModelError> {
        let mut decls: Vec<ParamDecl<'a>> = Vec::new();
        let mut index_by_id: HashMap<ParamId, usize> = HashMap::new();

        for item in &program.items {
            if let HirItem::Param {
                binding,
                name,
                ty,
                default,
                span,
            } = item
            {
                let id = ParamId(*binding);
                index_by_id.insert(id, decls.len());
                decls.push(ParamDecl {
                    id,
                    name: name.as_str(),
                    span: *span,
                    ty,
                    default: default.as_ref(),
                    depends_on: Vec::new(),
                });
            }
        }

        for decl in &mut decls {
            let Some(default) = decl.default else {
                continue;
            };
            let mut raw = Vec::new();
            collect_binding_refs(default, &mut raw);
            let mut deps: Vec<ParamId> = Vec::new();
            for b in raw {
                let candidate = ParamId(b);
                if index_by_id.contains_key(&candidate) && !deps.contains(&candidate) {
                    deps.push(candidate);
                }
            }
            decl.depends_on = deps;
        }

        let order = topological_order(&decls, &index_by_id)?;

        Ok(ParamModel {
            decls,
            index_by_id,
            order,
        })
    }

    /// Every modeled param, in source declaration order (not evaluation
    /// order — see [`ParamModel::evaluation_order`]).
    pub fn declarations(&self) -> &[ParamDecl<'a>] {
        &self.decls
    }

    /// The deterministic evaluation/rebuild order — see module doc
    /// comment "Deterministic evaluation".
    pub fn evaluation_order(&self) -> &[ParamId] {
        &self.order
    }

    /// The declaration for `id`, or `None` if `id` does not name a param
    /// this model declared.
    pub fn decl(&self, id: ParamId) -> Option<&ParamDecl<'a>> {
        self.index_by_id.get(&id).map(|&i| &self.decls[i])
    }

    /// Looks up a modeled param by its source name — a convenience for a
    /// caller building a [`ParamOverrides`] map from user-facing input
    /// (e.g. a future CLI `--param name=value` flag, not this task's own
    /// scope). `None` if no top-level param named `name` exists.
    pub fn find_by_name(&self, name: &str) -> Option<ParamId> {
        self.decls
            .iter()
            .find(|decl| decl.name == name)
            .map(|decl| decl.id)
    }
}

/// Deterministic Kahn's-algorithm topological sort, breaking every tie by
/// original declaration order — never a `HashMap`/`HashSet` iteration
/// order — so identical source always produces the identical schedule
/// (`project/DECISION_LOG.md#DL-12` Level-1 determinism).
fn topological_order(
    decls: &[ParamDecl<'_>],
    index_by_id: &HashMap<ParamId, usize>,
) -> Result<Vec<ParamId>, ParamModelError> {
    let n = decls.len();
    let mut indegree = vec![0usize; n];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, decl) in decls.iter().enumerate() {
        for dep in &decl.depends_on {
            let dep_idx = index_by_id[dep];
            dependents[dep_idx].push(i);
            indegree[i] += 1;
        }
    }

    let mut queue: VecDeque<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
    let mut visited = vec![false; n];
    let mut order_idx = Vec::with_capacity(n);
    while let Some(i) = queue.pop_front() {
        if visited[i] {
            continue;
        }
        visited[i] = true;
        order_idx.push(i);
        for &dependent in &dependents[i] {
            indegree[dependent] -= 1;
            if indegree[dependent] == 0 {
                queue.push_back(dependent);
            }
        }
    }

    if order_idx.len() != n {
        let cycle = (0..n)
            .filter(|&i| !visited[i])
            .map(|i| (decls[i].name.to_string(), decls[i].span))
            .collect();
        return Err(ParamModelError { cycle });
    }

    Ok(order_idx.into_iter().map(|i| decls[i].id).collect())
}

/// Compares an override [`Value`] against a param's own already-checked
/// [`CheckedType`] (`cad_hir::typeck::TypeCheckResult::binding_types`) —
/// reuses the type checker's own resolution rather than re-deriving
/// `HirTypeRef` resolution independently in this crate (`crate::value`'s
/// module doc comment already documents why this crate never re-derives
/// type-checker context). Only [`CheckedType::Value`] (an ordinary
/// scalar/dimensional value — the only shape a `param` has ever been able
/// to declare in practice) is currently supported; every other
/// `CheckedType` shape (struct/enum/list/range/...) is not yet a
/// supported *override* target and always compares unequal — a documented
/// scope boundary, not a silent accept.
pub(crate) fn value_matches_checked_type(value: &Value, checked: &CheckedType) -> bool {
    match checked {
        CheckedType::Value(hir_type) => value.operand_type().as_ref() == Some(hir_type),
        _ => false,
    }
}

fn collect_binding_refs(expr: &HirExpr, sink: &mut Vec<BindingId>) {
    match expr {
        HirExpr::Literal { .. } => {}
        HirExpr::Ident { binding, .. } => {
            if let Some(b) = binding {
                sink.push(*b);
            }
        }
        HirExpr::Unary { operand, .. } => collect_binding_refs(operand, sink),
        HirExpr::Binary { lhs, rhs, .. } => {
            collect_binding_refs(lhs, sink);
            collect_binding_refs(rhs, sink);
        }
        HirExpr::Call { callee, args, .. } => {
            if let HirCallee::Fn {
                binding: Some(b), ..
            } = callee
            {
                sink.push(*b);
            }
            for arg in args {
                collect_arg_refs(arg, sink);
            }
        }
        HirExpr::Field { receiver, .. } => collect_binding_refs(receiver, sink),
        HirExpr::Block(block) => collect_block_refs(block, sink),
        HirExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_binding_refs(cond, sink);
            collect_block_refs(then_branch, sink);
            collect_binding_refs(else_branch, sink);
        }
        HirExpr::Match {
            scrutinee, arms, ..
        } => {
            collect_binding_refs(scrutinee, sink);
            for arm in arms {
                collect_match_arm_refs(arm, sink);
            }
        }
        HirExpr::ListLiteral { elements, .. } => {
            for e in elements {
                collect_binding_refs(e, sink);
            }
        }
        HirExpr::Range { start, end, .. } => {
            collect_binding_refs(start, sink);
            collect_binding_refs(end, sink);
        }
        HirExpr::RecordLiteral {
            binding, fields, ..
        } => {
            if let Some(b) = binding {
                sink.push(*b);
            }
            for f in fields {
                collect_binding_refs(&f.value, sink);
            }
        }
    }
}

fn collect_arg_refs(arg: &HirArg, sink: &mut Vec<BindingId>) {
    match arg {
        HirArg::Positional(e) => collect_binding_refs(e, sink),
        HirArg::Named { value, .. } => collect_binding_refs(value, sink),
    }
}

fn collect_match_arm_refs(arm: &HirMatchArm, sink: &mut Vec<BindingId>) {
    collect_binding_refs(&arm.body, sink);
}

fn collect_block_refs(block: &HirBlock, sink: &mut Vec<BindingId>) {
    for stmt in &block.stmts {
        collect_stmt_refs(stmt, sink);
    }
    if let Some(trailing) = &block.trailing {
        collect_binding_refs(trailing, sink);
    }
}

fn collect_stmt_refs(stmt: &HirStmt, sink: &mut Vec<BindingId>) {
    match stmt {
        HirStmt::Let { value, .. } | HirStmt::Var { value, .. } => {
            collect_binding_refs(value, sink)
        }
        HirStmt::Assign { target, value, .. } => {
            if let Some(b) = target {
                sink.push(*b);
            }
            collect_binding_refs(value, sink);
        }
        HirStmt::Expr { expr, .. } => collect_binding_refs(expr, sink),
        HirStmt::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_binding_refs(cond, sink);
            collect_block_refs(then_branch, sink);
            if let Some(else_stmt) = else_branch {
                match else_stmt {
                    HirElseStmt::Block(block) => collect_block_refs(block, sink),
                    HirElseStmt::If(nested) => collect_stmt_refs(nested, sink),
                }
            }
        }
        HirStmt::For { iterable, body, .. } => {
            collect_binding_refs(iterable, sink);
            collect_block_refs(body, sink);
        }
        HirStmt::While { cond, body, .. } => {
            collect_binding_refs(cond, sink);
            collect_block_refs(body, sink);
        }
        HirStmt::Loop { body, .. } => collect_block_refs(body, sink),
        HirStmt::Match {
            scrutinee, arms, ..
        } => {
            collect_binding_refs(scrutinee, sink);
            for arm in arms {
                collect_match_arm_refs(arm, sink);
            }
        }
        HirStmt::Return { value, .. } => {
            if let Some(v) = value {
                collect_binding_refs(v, sink);
            }
        }
        HirStmt::Break { .. } | HirStmt::Continue { .. } => {}
    }
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

    #[test]
    fn independent_params_have_no_dependencies() {
        let lowered = lowered("param width: Length = 80mm;\nparam height: Length = 40mm;\n");
        let model = ParamModel::build(&lowered.program).expect("no cycle");
        assert_eq!(model.declarations().len(), 2);
        for decl in model.declarations() {
            assert!(decl.depends_on.is_empty());
        }
        assert_eq!(model.evaluation_order().len(), 2);
    }

    #[test]
    fn derived_param_depends_on_its_referenced_param() {
        let lowered =
            lowered("param width: Length = 80mm;\nparam double_width: Length = width * 2.0;\n");
        let model = ParamModel::build(&lowered.program).expect("no cycle");
        let width = model.find_by_name("width").expect("width declared");
        let double_width = model
            .find_by_name("double_width")
            .expect("double_width declared");
        let derived = model.decl(double_width).expect("declared");
        assert_eq!(derived.depends_on, vec![width]);

        let order = model.evaluation_order();
        let width_pos = order.iter().position(|&id| id == width).unwrap();
        let double_pos = order.iter().position(|&id| id == double_width).unwrap();
        assert!(
            width_pos < double_pos,
            "width must evaluate before double_width depending on it"
        );
    }

    #[test]
    fn evaluation_order_is_deterministic_across_rebuilds() {
        let source =
            "param a: Length = 1mm;\nparam b: Length = a + 1mm;\nparam c: Length = b + a;\n";
        let lowered1 = lowered(source);
        let model1 = ParamModel::build(&lowered1.program).expect("no cycle");
        let lowered2 = lowered(source);
        let model2 = ParamModel::build(&lowered2.program).expect("no cycle");

        let names1: Vec<&str> = model1
            .evaluation_order()
            .iter()
            .map(|&id| model1.decl(id).unwrap().name)
            .collect();
        let names2: Vec<&str> = model2
            .evaluation_order()
            .iter()
            .map(|&id| model2.decl(id).unwrap().name)
            .collect();
        assert_eq!(names1, names2);
        assert_eq!(names1, vec!["a", "b", "c"]);
    }

    #[test]
    fn direct_cycle_is_detected_not_silently_ordered() {
        // Two params whose defaults reference each other type-check fine
        // (params are ordinary typed bindings to the type checker), but
        // the parametric model must reject the cycle rather than pick an
        // arbitrary order.
        let lowered = lowered("param a: Length = b;\nparam b: Length = a;\n");
        let err = ParamModel::build(&lowered.program).expect_err("cycle must be rejected");
        let names: Vec<&str> = err.cycle.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn self_referential_param_is_a_cycle() {
        let lowered = lowered("param a: Length = a + 1mm;\n");
        let err = ParamModel::build(&lowered.program).expect_err("self-cycle must be rejected");
        assert_eq!(err.cycle.len(), 1);
        assert_eq!(err.cycle[0].0, "a");
    }

    #[test]
    fn param_referencing_a_let_is_not_a_param_dependency() {
        // `let`/`const` references inside a param default are legal
        // (evaluated by the ordinary interpreter, see module doc comment
        // "Scope boundary") but are not part of this module's own
        // param-to-param dependency graph.
        let lowered = lowered("const factor: Float = 2.0;\nparam width: Length = 40mm * factor;\n");
        let model = ParamModel::build(&lowered.program).expect("no cycle");
        let width = model.find_by_name("width").expect("declared");
        assert!(model.decl(width).unwrap().depends_on.is_empty());
    }

    #[test]
    fn transitive_chain_orders_correctly() {
        let lowered = lowered(
            "param a: Length = 1mm;\n\
             param b: Length = a * 2.0;\n\
             param c: Length = b * 2.0;\n\
             param d: Length = c + a;\n",
        );
        let model = ParamModel::build(&lowered.program).expect("no cycle");
        let order: Vec<&str> = model
            .evaluation_order()
            .iter()
            .map(|&id| model.decl(id).unwrap().name)
            .collect();
        let pos = |name: &str| order.iter().position(|&n| n == name).unwrap();
        assert!(pos("a") < pos("b"));
        assert!(pos("b") < pos("c"));
        assert!(pos("c") < pos("d"));
        assert!(pos("a") < pos("d"));
    }
}
