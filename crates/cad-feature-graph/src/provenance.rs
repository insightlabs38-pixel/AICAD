//! Source-to-feature provenance for the Stage-3 Feature DAG (`AICAD-069`),
//! per `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §9 (the node's own
//! "provenance" field, alongside the `source_span`/`cache key` fields
//! `AICAD-066`/`AICAD-067`/`AICAD-068` already populate).
//!
//! This module supplies the two things `AICAD-069`'s own title names:
//!
//! - **Source-to-feature mapping** — [`crate::graph::FeatureGraph::
//!   feature_at`] answers "which feature node does this byte offset belong
//!   to", the reverse direction of the forward `FeatureId -> FeatureNode`
//!   lookups [`crate::graph::FeatureGraph::get`]/[`crate::graph::
//!   FeatureGraph::find_by_binding`] already provide. Useful for tooling
//!   that starts from a cursor/selection position (an editor, a
//!   diagnostic's own source span) and needs the feature it falls inside.
//! - **Provenance** — [`Provenance`], attached to every [`crate::graph::
//!   FeatureNode`], records (a) [`Declaration`]: whether the node came
//!   from a top-level `let`, a top-level `const`, or was never itself
//!   declared (an anonymous nested-argument construction, e.g. `cylinder
//!   (...)` inlined directly inside `cut(base, cylinder(...))`); and (b)
//!   [`Provenance::transitive_bindings`]: every top-level binding this
//!   feature's own construction *ultimately* traces back to — its own
//!   direct [`crate::graph::FeatureNode::binding_refs`]
//!   (`AICAD-068`) plus, transitively, every `geometry_inputs` ancestor's
//!   own already-computed closure. This is the forward-looking complement
//!   to `AICAD-068`'s [`crate::graph::FeatureGraph::dirty_set`] (which
//!   answers "given changed bindings, which features are dirty";
//!   `transitive_bindings` instead answers, for one specific feature,
//!   "which bindings could ever make this one dirty" — useful for
//!   diagnostics/tooling that wants to explain a feature without first
//!   picking a hypothetical changed-bindings set).
//!
//! ## What this deliberately does not do
//!
//! - **Not `docs/plan/14_COLLABORATION_PROVENANCE_SECURITY.md`'s "feature-
//!   level provenance".** That document's §5/§6 define a much larger
//!   provenance concept — `created_by: human | AI | package | import |
//!   optimizer | reconstruction`, actor/tool identifiers, commit/build
//!   IDs, review/approval state, AI-governance policy — that belongs to
//!   the Collaboration/Git-integration work package (WP-14), which has no
//!   Stage-3 task and depends on infrastructure (commit metadata, an
//!   actor-identity model, an AI-governance policy engine) nothing in
//!   Stage 0-3 builds. Implementing any of it here would be exactly
//!   `AGENTS.md`'s "begin later-stage work because it will be useful
//!   soon" — out of scope for this task, not silently folded in.
//! - **Not multi-file source identification.** `cad_ast::Span` is
//!   explicitly single-file only (that type's own doc comment: "multi-file
//!   spans are not yet needed by any Stage-2 task"); Stage 3 has no module
//!   system for the feature graph to resolve across files, so
//!   [`crate::graph::FeatureGraph::feature_at`] takes a plain byte offset,
//!   not a `(file, offset)` pair.
//! - **Not a query/explain-feature CLI surface.** This module supplies the
//!   data; a `cad`-CLI-facing "explain this feature"/"what does this
//!   depend on" command is tooling built on top of it, not this task's own
//!   scope (mirrors `docs/plan/06...` §12's own "reference health report"
//!   CLI feature, which is explicitly a *later* addition on top of the
//!   data model, not part of building the data model itself).

use cad_hir::ids::BindingId;

/// How one [`crate::graph::FeatureNode`] came to exist in the program's own
/// top-level declarations — see module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Declaration {
    /// The direct value of a top-level `let <name> = ...;`.
    Let,
    /// The direct value of a top-level `const <name> = ...;`.
    Const,
    /// Never itself the direct value of a top-level `let`/`const` — built
    /// only because another feature's own call referenced it as a nested
    /// argument expression (e.g. `cylinder(...)` inlined directly inside
    /// `cut(base, cylinder(...))`) or because [`crate::graph::
    /// FeatureGraph::build`] simply never visited a `let`/`const` binding
    /// this exact node ended up under (defensive; every node reachable
    /// from `FeatureGraph::build`'s own top-level scan is fed through the
    /// same naming step, so this case does not arise for a node built by
    /// that function today, but nothing here assumes it cannot in a future
    /// caller/extension).
    Anonymous,
}

/// One feature node's own provenance — `docs/plan/06_REFERENCES_QUERIES_
/// FEATURE_DAG.md` §9's "provenance" field — see module doc comment for
/// exactly what this crate's own (structural, Stage-3, no Git/AI-
/// governance) notion of "provenance" covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub declared_as: Declaration,
    /// Every top-level [`BindingId`] this feature's own construction
    /// ultimately traces back to, deduplicated in first-occurrence order:
    /// this node's own [`crate::graph::FeatureNode::binding_refs`] first,
    /// then each `geometry_inputs` ancestor's own already-computed closure
    /// in `geometry_inputs` order. Empty for a feature built entirely from
    /// literals with no upstream feature and no referenced binding (e.g.
    /// `box(10mm, 10mm, 10mm)` alone).
    pub transitive_bindings: Vec<BindingId>,
}

impl Provenance {
    /// Constructs a fresh anonymous node's provenance (no bindings other
    /// than its own — [`crate::graph::Builder::resolve_geometry_expr`]
    /// upgrades `declared_as` afterward if the node turns out to be a named
    /// top-level `let`/`const`'s direct value).
    pub(crate) fn compute(
        own_binding_refs: &[BindingId],
        geometry_input_closures: &[&[BindingId]],
    ) -> Provenance {
        let mut transitive_bindings: Vec<BindingId> = Vec::with_capacity(own_binding_refs.len());
        for b in own_binding_refs {
            if !transitive_bindings.contains(b) {
                transitive_bindings.push(*b);
            }
        }
        for closure in geometry_input_closures {
            for b in *closure {
                if !transitive_bindings.contains(b) {
                    transitive_bindings.push(*b);
                }
            }
        }
        Provenance {
            declared_as: Declaration::Anonymous,
            transitive_bindings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_hir::hir::{HirExpr, HirItem};
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

    /// A real, lowering-minted [`BindingId`] this crate cannot fabricate on
    /// its own (`BindingId::new` is `pub(crate)` to `cad-hir`) — obtained
    /// from a top-level `let <name> = <other name>;`'s own resolved
    /// `HirExpr::Ident`, mirroring `crate::cache`'s own test helper of the
    /// same shape.
    fn binding_via_ident(lowered: &LowerResult, name: &str) -> BindingId {
        for item in &lowered.program.items {
            if let HirItem::Let { name: n, value, .. } = item
                && n == name
                && let HirExpr::Ident {
                    binding: Some(b), ..
                } = value
            {
                return *b;
            }
        }
        panic!("no top-level `let {name} = <ident>;`");
    }

    #[test]
    fn no_bindings_and_no_geometry_inputs_yields_empty_closure() {
        let prov = Provenance::compute(&[], &[]);
        assert_eq!(prov.declared_as, Declaration::Anonymous);
        assert!(prov.transitive_bindings.is_empty());
    }

    #[test]
    fn own_binding_refs_become_the_closure_when_there_are_no_geometry_inputs() {
        let lowered = lowered("param w: Length = 1mm;\nlet echo = w;\n");
        let w = binding_via_ident(&lowered, "echo");
        let prov = Provenance::compute(&[w], &[]);
        assert_eq!(prov.transitive_bindings, vec![w]);
    }

    #[test]
    fn geometry_input_closures_are_merged_and_deduplicated() {
        let lowered = lowered(
            "param w: Length = 1mm;\n\
             param h: Length = 2mm;\n\
             let echo_w = w;\n\
             let echo_h = h;\n",
        );
        let w = binding_via_ident(&lowered, "echo_w");
        let h = binding_via_ident(&lowered, "echo_h");

        // Two upstream "geometry inputs" whose own closures already
        // contain `w` (one of them twice, as a repeated reference would
        // produce) and `h` respectively.
        let input_a_closure = vec![w, w];
        let input_b_closure = vec![h];
        let prov = Provenance::compute(&[], &[&input_a_closure, &input_b_closure]);
        assert_eq!(prov.transitive_bindings, vec![w, h]);
    }

    #[test]
    fn own_refs_precede_inherited_refs_in_first_occurrence_order() {
        let lowered = lowered(
            "param w: Length = 1mm;\n\
             param h: Length = 2mm;\n\
             let echo_w = w;\n\
             let echo_h = h;\n",
        );
        let w = binding_via_ident(&lowered, "echo_w");
        let h = binding_via_ident(&lowered, "echo_h");

        let input_closure = vec![h];
        let prov = Provenance::compute(&[w], &[&input_closure]);
        assert_eq!(prov.transitive_bindings, vec![w, h]);
    }
}
