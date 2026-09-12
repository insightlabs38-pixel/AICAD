//! Cache keys and dirty propagation for the Stage-3 Feature DAG
//! (`AICAD-068`), per `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §9
//! ("cache key" as one of the node's own fields) and §10 ("Incremental
//! invalidation").
//!
//! `AICAD-066`/`AICAD-067` (`crate::graph`) built every [`crate::graph::
//! FeatureNode`]'s identity and dependency edges but deliberately left
//! cache keys/dirty propagation to this task (see `crate::graph`'s own
//! module doc comment, "What this module deliberately does not do"). This
//! module supplies both:
//!
//! - **[`CacheKey`]** — a purely *structural* content hash of one feature
//!   node: its operation, its `geometry_inputs`' own cache keys (so any
//!   upstream structural change propagates into every downstream key), and
//!   its own scalar `parameters`' *expressions* (not their evaluated
//!   values — this crate has no interpreter, mirroring `crate::graph`'s own
//!   "Evaluating a feature's own scalar parameters... is `cad_runtime`'s
//!   job" scope boundary). Two nodes with identical cache keys were built
//!   from source-structurally-identical calls; this is a purely static
//!   property, not a claim about the geometry two structurally-identical
//!   calls would actually produce (a `RuntimeBuiltin`'s own implementation
//!   is not this crate's concern).
//! - **[`FeatureGraph::dirty_set`]** (`crate::graph`) — given the set of
//!   top-level bindings whose *value* changed (e.g. a
//!   `cad_runtime::params::ParamOverrides` edit, though this crate does not
//!   depend on `cad-runtime` — see "Why no `cad-runtime` dependency" below),
//!   determines every feature node that must be rebuilt: directly, because
//!   one of its own parameter expressions references a changed binding
//!   (`docs/plan/06...` §10 step 1: "find parameter dependents"), or
//!   transitively, because one of its `geometry_inputs` is itself dirty
//!   (§10 step 2: "mark affected feature nodes dirty") — every other node
//!   is left out of the returned set entirely (§10 step 3: "preserve
//!   unaffected cached nodes").
//!
//! ## Why a hand-rolled hasher, not `DefaultHasher`
//!
//! `std::collections::hash_map::DefaultHasher`'s own documentation states
//! its algorithm "is not specified, and so it and its hashes should not be
//! relied upon over releases" — exactly the kind of unspecified-across-
//! versions behavior `project/DECISION_LOG.md#DL-12` Level 1's
//! byte-identical canonical-serialization requirement (for identical
//! compiler inputs/version) forbids relying on for an AICAD-owned
//! deterministic value. [`StableHasher`] is a small, explicitly-specified
//! FNV-1a 64-bit accumulator instead — simple enough to own outright rather
//! than take a new external dependency for it.
//!
//! ## Why no `cad-runtime` dependency
//!
//! `cad_runtime::params::collect_binding_refs` already walks an `HirExpr`
//! tree collecting referenced top-level bindings — structurally very close
//! to what [`collect`] does here. This module does not reuse it: `cad-hir`'s
//! own `ids.rs` already establishes the precedent this follows exactly
//! ("this module is a small, independent re-implementation of the same...
//! concept... since `cad-hir` cannot depend on `cad-compiler`"; see that
//! module's doc comment) — `cad-feature-graph` sits below `cad-runtime` in
//! the intended layering (the interpreter is expected to eventually consume
//! the feature graph, per `crate::graph`'s own doc comment referencing
//! `cad_runtime::params::ParamModel` as "this task's own reusable input"),
//! so a `cad-feature-graph -> cad-runtime` dependency would point the wrong
//! direction and risk a future cycle once `cad-runtime` itself depends on
//! this crate. Duplicating this one small traversal is the smaller, safer
//! cost.
//!
//! ## What this deliberately does not do
//!
//! - **Does not evaluate expressions or know which bindings are `param`s.**
//!   [`collect`] returns every [`BindingId`] a parameter expression
//!   references (matching `cad_runtime::params::collect_binding_refs`'s own
//!   identical "collect everything, let the caller filter" division of
//!   labor) — whether a given `BindingId` is a `param`, a `const`, a `let`,
//!   or something else is for the caller (the future param/feature-graph
//!   integration task) to decide, not this module.
//! - **Does not hash source spans.** Two calls that differ only in
//!   formatting/whitespace (identical span *content*, different span
//!   *position*) must not be treated as structurally different — spans are
//!   deliberately excluded from every hash computed here.
//! - **Does not detect a changed binding's own structural definition
//!   changing** (e.g. a `param`'s `default` expression itself being edited
//!   in source). That is a different program (`HirProgram` rebuilt from
//!   different source), and this crate already produces a wholly fresh
//!   [`crate::graph::FeatureGraph`] — with fresh, independently-correct
//!   cache keys — for that case; [`FeatureGraph::dirty_set`] is specifically
//!   the same-source, overridden-parameter-*value* rebuild path `docs/plan/
//!   06...` §10 describes.

use cad_hir::hir::{
    HirArg, HirBlock, HirCallee, HirElseStmt, HirExpr, HirLiteral, HirMatchArm, HirPattern,
    HirRecordPatternField, HirStmt,
};
use cad_hir::ids::BindingId;
use std::fmt;

/// A purely structural content hash of one [`crate::graph::FeatureNode`] —
/// see module doc comment. Meaningless outside the [`crate::graph::
/// FeatureGraph`] build that produced it (two different graphs happening to
/// share a key value is expected and fine — it means those two nodes are
/// structurally identical, not that they are "the same node").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CacheKey(u64);

impl fmt::Display for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// FNV-1a 64-bit, chosen explicitly over `DefaultHasher` — see module doc
/// comment "Why a hand-rolled hasher".
struct StableHasher(u64);

impl StableHasher {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        StableHasher(Self::OFFSET_BASIS)
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn write_u8(&mut self, tag: u8) {
        self.write_bytes(&[tag]);
    }

    fn write_u64(&mut self, v: u64) {
        self.write_bytes(&v.to_le_bytes());
    }

    fn write_bool(&mut self, v: bool) {
        self.write_u8(v as u8);
    }

    /// Length-prefixed so adjacent fields can never be ambiguous (e.g.
    /// hashing `"ab", "c"` must never collide with `"a", "bc"`).
    fn write_str(&mut self, s: &str) {
        self.write_u64(s.len() as u64);
        self.write_bytes(s.as_bytes());
    }

    fn write_binding(&mut self, b: BindingId) {
        self.write_u64(b.index() as u64);
    }

    fn finish(self) -> u64 {
        self.0
    }
}

/// Computes one feature node's own [`CacheKey`] from its operation name,
/// its already-computed `geometry_inputs` cache keys (in `crate::graph::
/// FeatureNode::geometry_inputs` order), and its own scalar `parameters`
/// (in declared-parameter order) — plus every [`BindingId`] referenced
/// anywhere across those parameter expressions, deduplicated in
/// first-occurrence order (matching `cad_runtime::params::ParamDecl::
/// depends_on`'s own identical dedup convention).
pub(crate) fn node_cache_key(
    op_name: &str,
    geometry_input_keys: &[CacheKey],
    parameters: &[(&'static str, &HirExpr)],
) -> (CacheKey, Vec<BindingId>) {
    let mut hasher = StableHasher::new();
    hasher.write_str(op_name);

    hasher.write_u64(geometry_input_keys.len() as u64);
    for key in geometry_input_keys {
        hasher.write_u64(key.0);
    }

    let mut refs = Vec::new();
    hasher.write_u64(parameters.len() as u64);
    for (name, expr) in parameters {
        hasher.write_str(name);
        hash_expr(expr, &mut hasher, &mut refs);
    }

    let mut deduped: Vec<BindingId> = Vec::with_capacity(refs.len());
    for b in refs {
        if !deduped.contains(&b) {
            deduped.push(b);
        }
    }

    (CacheKey(hasher.finish()), deduped)
}

const TAG_LITERAL: u8 = 1;
const TAG_IDENT: u8 = 2;
const TAG_UNARY: u8 = 3;
const TAG_BINARY: u8 = 4;
const TAG_CALL: u8 = 5;
const TAG_FIELD: u8 = 6;
const TAG_BLOCK: u8 = 7;
const TAG_IF: u8 = 8;
const TAG_MATCH: u8 = 9;
const TAG_LIST: u8 = 10;
const TAG_RANGE: u8 = 11;
const TAG_RECORD: u8 = 12;

/// Structurally hashes `expr` into `hasher`, simultaneously collecting
/// every [`BindingId`] it references (not declares — see per-variant
/// comments below) into `sink`, in one traversal. Exhaustive over every
/// [`HirExpr`] shape (mirrors `cad_runtime::params::collect_binding_refs`'s
/// own exhaustive traversal — see module doc comment "Why no `cad-runtime`
/// dependency" for why this is a deliberate re-implementation, not a
/// shared helper).
fn hash_expr(expr: &HirExpr, hasher: &mut StableHasher, sink: &mut Vec<BindingId>) {
    match expr {
        HirExpr::Literal { value, .. } => {
            hasher.write_u8(TAG_LITERAL);
            hash_literal(value, hasher);
        }
        HirExpr::Ident { name, binding, .. } => {
            hasher.write_u8(TAG_IDENT);
            match binding {
                Some(b) => {
                    hasher.write_bool(true);
                    hasher.write_binding(*b);
                    sink.push(*b);
                }
                None => {
                    // Unresolved name: defensive only (an already
                    // type-checked program never reaches this crate with
                    // one — see `crate::graph::FeatureGraphError::
                    // MalformedBuiltinCall`'s own identical "trusts, but
                    // verifies" framing), but still hashed deterministically
                    // by spelling rather than silently ignored.
                    hasher.write_bool(false);
                    hasher.write_str(name);
                }
            }
        }
        HirExpr::Unary { op, operand, .. } => {
            hasher.write_u8(TAG_UNARY);
            hasher.write_str(op.as_str());
            hash_expr(operand, hasher, sink);
        }
        HirExpr::Binary { op, lhs, rhs, .. } => {
            hasher.write_u8(TAG_BINARY);
            hasher.write_str(op.as_str());
            hash_expr(lhs, hasher, sink);
            hash_expr(rhs, hasher, sink);
        }
        HirExpr::Call { callee, args, .. } => {
            hasher.write_u8(TAG_CALL);
            hash_callee(callee, hasher, sink);
            hasher.write_u64(args.len() as u64);
            for arg in args {
                hash_arg(arg, hasher, sink);
            }
        }
        HirExpr::Field {
            receiver, field, ..
        } => {
            hasher.write_u8(TAG_FIELD);
            hash_expr(receiver, hasher, sink);
            hasher.write_str(field);
        }
        HirExpr::Block(block) => {
            hasher.write_u8(TAG_BLOCK);
            hash_block(block, hasher, sink);
        }
        HirExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            hasher.write_u8(TAG_IF);
            hash_expr(cond, hasher, sink);
            hash_block(then_branch, hasher, sink);
            hash_expr(else_branch, hasher, sink);
        }
        HirExpr::Match {
            scrutinee, arms, ..
        } => {
            hasher.write_u8(TAG_MATCH);
            hash_expr(scrutinee, hasher, sink);
            hasher.write_u64(arms.len() as u64);
            for arm in arms {
                hash_match_arm(arm, hasher, sink);
            }
        }
        HirExpr::ListLiteral { elements, .. } => {
            hasher.write_u8(TAG_LIST);
            hasher.write_u64(elements.len() as u64);
            for e in elements {
                hash_expr(e, hasher, sink);
            }
        }
        HirExpr::Range {
            start,
            end,
            inclusive,
            ..
        } => {
            hasher.write_u8(TAG_RANGE);
            hash_expr(start, hasher, sink);
            hash_expr(end, hasher, sink);
            hasher.write_bool(*inclusive);
        }
        HirExpr::RecordLiteral {
            name,
            binding,
            fields,
            ..
        } => {
            hasher.write_u8(TAG_RECORD);
            hasher.write_str(name);
            match binding {
                Some(b) => {
                    hasher.write_bool(true);
                    hasher.write_binding(*b);
                    sink.push(*b);
                }
                None => hasher.write_bool(false),
            }
            hasher.write_u64(fields.len() as u64);
            for f in fields {
                hasher.write_str(&f.name);
                hash_expr(&f.value, hasher, sink);
            }
        }
    }
}

fn hash_literal(value: &HirLiteral, hasher: &mut StableHasher) {
    match value {
        HirLiteral::Number { text, unit } => {
            hasher.write_u8(1);
            hasher.write_str(text);
            match unit {
                Some(u) => {
                    hasher.write_bool(true);
                    hasher.write_str(u);
                }
                None => hasher.write_bool(false),
            }
        }
        HirLiteral::Str(s) => {
            hasher.write_u8(2);
            hasher.write_str(s);
        }
        HirLiteral::RawStr(s) => {
            hasher.write_u8(3);
            hasher.write_str(s);
        }
        HirLiteral::Bool(b) => {
            hasher.write_u8(4);
            hasher.write_bool(*b);
        }
    }
}

fn hash_callee(callee: &HirCallee, hasher: &mut StableHasher, sink: &mut Vec<BindingId>) {
    match callee {
        HirCallee::Fn { name, binding, .. } => {
            hasher.write_u8(1);
            match binding {
                Some(b) => {
                    hasher.write_bool(true);
                    hasher.write_binding(*b);
                    sink.push(*b);
                }
                None => {
                    hasher.write_bool(false);
                    hasher.write_str(name);
                }
            }
        }
        HirCallee::Method { name, .. } => {
            // Resolved against the receiver's type, not lexical scope
            // (`HirCallee::Method`'s own doc comment) — never a binding
            // reference, matching `cad_runtime::params::collect_binding_refs`'s
            // own identical omission.
            hasher.write_u8(2);
            hasher.write_str(name);
        }
    }
}

fn hash_arg(arg: &HirArg, hasher: &mut StableHasher, sink: &mut Vec<BindingId>) {
    match arg {
        HirArg::Positional(expr) => {
            hasher.write_u8(1);
            hash_expr(expr, hasher, sink);
        }
        HirArg::Named { name, value, .. } => {
            hasher.write_u8(2);
            hasher.write_str(name);
            hash_expr(value, hasher, sink);
        }
    }
}

fn hash_block(block: &HirBlock, hasher: &mut StableHasher, sink: &mut Vec<BindingId>) {
    hasher.write_u64(block.stmts.len() as u64);
    for stmt in &block.stmts {
        hash_stmt(stmt, hasher, sink);
    }
    match &block.trailing {
        Some(trailing) => {
            hasher.write_bool(true);
            hash_expr(trailing, hasher, sink);
        }
        None => hasher.write_bool(false),
    }
}

fn hash_stmt(stmt: &HirStmt, hasher: &mut StableHasher, sink: &mut Vec<BindingId>) {
    match stmt {
        HirStmt::Let { value, ty, .. } => {
            // `binding` here is a fresh declaration, not a reference — never
            // pushed to `sink`, matching `cad_runtime::params::
            // collect_stmt_refs`'s own identical treatment.
            hasher.write_u8(1);
            hasher.write_bool(ty.is_some());
            hash_expr(value, hasher, sink);
        }
        HirStmt::Var { value, ty, .. } => {
            hasher.write_u8(2);
            hasher.write_bool(ty.is_some());
            hash_expr(value, hasher, sink);
        }
        HirStmt::Assign { target, value, .. } => {
            hasher.write_u8(3);
            match target {
                Some(b) => {
                    hasher.write_bool(true);
                    hasher.write_binding(*b);
                    sink.push(*b);
                }
                None => hasher.write_bool(false),
            }
            hash_expr(value, hasher, sink);
        }
        HirStmt::Expr { expr, .. } => {
            hasher.write_u8(4);
            hash_expr(expr, hasher, sink);
        }
        HirStmt::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            hasher.write_u8(5);
            hash_expr(cond, hasher, sink);
            hash_block(then_branch, hasher, sink);
            match else_branch {
                Some(HirElseStmt::Block(block)) => {
                    hasher.write_u8(1);
                    hash_block(block, hasher, sink);
                }
                Some(HirElseStmt::If(nested)) => {
                    hasher.write_u8(2);
                    hash_stmt(nested, hasher, sink);
                }
                None => hasher.write_u8(0),
            }
        }
        HirStmt::For {
            binding,
            iterable,
            body,
            ..
        } => {
            hasher.write_u8(6);
            // `binding` is the loop's own fresh per-iteration declaration,
            // not a reference.
            let _ = binding;
            hash_expr(iterable, hasher, sink);
            hash_block(body, hasher, sink);
        }
        HirStmt::While { cond, body, .. } => {
            hasher.write_u8(7);
            hash_expr(cond, hasher, sink);
            hash_block(body, hasher, sink);
        }
        HirStmt::Loop { body, .. } => {
            hasher.write_u8(8);
            hash_block(body, hasher, sink);
        }
        HirStmt::Match {
            scrutinee, arms, ..
        } => {
            hasher.write_u8(9);
            hash_expr(scrutinee, hasher, sink);
            hasher.write_u64(arms.len() as u64);
            for arm in arms {
                hash_match_arm(arm, hasher, sink);
            }
        }
        HirStmt::Return { value, .. } => {
            hasher.write_u8(10);
            match value {
                Some(v) => {
                    hasher.write_bool(true);
                    hash_expr(v, hasher, sink);
                }
                None => hasher.write_bool(false),
            }
        }
        HirStmt::Break { .. } => hasher.write_u8(11),
        HirStmt::Continue { .. } => hasher.write_u8(12),
    }
}

/// Hashes one match arm's `pattern` (structurally only — matching
/// `cad_runtime::params::collect_match_arm_refs`'s own deliberate choice
/// not to collect binding references from patterns at all, since a
/// pattern's own bindings are either fresh per-arm declarations or
/// references to enum-variant *type* identity, never a `param`/`let`
/// value dependency) plus its `body` (structurally, and for binding
/// references, exactly like any other expression).
fn hash_match_arm(arm: &HirMatchArm, hasher: &mut StableHasher, sink: &mut Vec<BindingId>) {
    hash_pattern(&arm.pattern, hasher);
    hash_expr(&arm.body, hasher, sink);
}

fn hash_pattern(pattern: &HirPattern, hasher: &mut StableHasher) {
    match pattern {
        HirPattern::Wildcard { .. } => hasher.write_u8(1),
        HirPattern::Literal { value, .. } => {
            hasher.write_u8(2);
            hash_literal(value, hasher);
        }
        HirPattern::Variant { name, variant, .. } => {
            hasher.write_u8(3);
            hasher.write_str(name);
            hasher.write_binding(*variant);
        }
        HirPattern::Binding { name, .. } => {
            // A fresh per-arm declaration -- hashed by its own declared
            // name (its `BindingId` is minted fresh per lowering pass and
            // is not itself source-stable identity to hash structurally).
            hasher.write_u8(4);
            hasher.write_str(name);
        }
        HirPattern::Tuple {
            name,
            variant,
            elems,
            ..
        } => {
            hasher.write_u8(5);
            hasher.write_str(name);
            hash_optional_variant(*variant, hasher);
            hasher.write_u64(elems.len() as u64);
            for e in elems {
                hash_pattern(e, hasher);
            }
        }
        HirPattern::Record {
            name,
            variant,
            fields,
            ..
        } => {
            hasher.write_u8(6);
            hasher.write_str(name);
            hash_optional_variant(*variant, hasher);
            hasher.write_u64(fields.len() as u64);
            for f in fields {
                hash_record_pattern_field(f, hasher);
            }
        }
    }
}

fn hash_optional_variant(variant: Option<BindingId>, hasher: &mut StableHasher) {
    match variant {
        Some(b) => {
            hasher.write_bool(true);
            hasher.write_binding(b);
        }
        None => hasher.write_bool(false),
    }
}

fn hash_record_pattern_field(field: &HirRecordPatternField, hasher: &mut StableHasher) {
    hasher.write_str(&field.name);
    hash_pattern(&field.pattern, hasher);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_ast::Span;
    use cad_hir::hir::HirItem;
    use cad_hir::lower::LowerResult;

    fn number(text: &str, unit: Option<&str>) -> HirExpr {
        HirExpr::Literal {
            value: HirLiteral::Number {
                text: text.to_string(),
                unit: unit.map(|u| u.to_string()),
            },
            ty: None,
            span: Span::new(0, 1),
        }
    }

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

    /// Returns a clone of the top-level `let <name> = <other name>;`
    /// expression's own resolved `HirExpr::Ident` — a real, lowering-minted
    /// [`BindingId`] this crate cannot fabricate on its own (`BindingId::
    /// new` is `pub(crate)` to `cad-hir`; see that type's own doc comment).
    fn ident_expr(lowered: &LowerResult, name: &str) -> HirExpr {
        for item in &lowered.program.items {
            if let HirItem::Let { name: n, value, .. } = item
                && n == name
            {
                assert!(
                    matches!(value, HirExpr::Ident { .. }),
                    "expected `let {name} = <ident>;`"
                );
                return value.clone();
            }
        }
        panic!("no top-level let named {name:?}");
    }

    #[test]
    fn identical_literal_parameters_produce_the_same_key() {
        let a = number("10", Some("mm"));
        let b = number("10", Some("mm"));
        let (key_a, refs_a) = node_cache_key("box", &[], &[("dx", &a)]);
        let (key_b, refs_b) = node_cache_key("box", &[], &[("dx", &b)]);
        assert_eq!(key_a, key_b);
        assert!(refs_a.is_empty());
        assert!(refs_b.is_empty());
    }

    #[test]
    fn different_literal_text_changes_the_key() {
        let a = number("10", Some("mm"));
        let b = number("20", Some("mm"));
        let (key_a, _) = node_cache_key("box", &[], &[("dx", &a)]);
        let (key_b, _) = node_cache_key("box", &[], &[("dx", &b)]);
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn different_unit_changes_the_key_even_with_identical_text() {
        let a = number("10", Some("mm"));
        let b = number("10", Some("cm"));
        let (key_a, _) = node_cache_key("box", &[], &[("dx", &a)]);
        let (key_b, _) = node_cache_key("box", &[], &[("dx", &b)]);
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn different_operation_name_changes_the_key_for_identical_parameters() {
        let a = number("10", Some("mm"));
        let (key_box, _) = node_cache_key("box", &[], &[("dx", &a)]);
        let (key_cylinder, _) = node_cache_key("cylinder", &[], &[("dx", &a)]);
        assert_ne!(key_box, key_cylinder);
    }

    #[test]
    fn ident_parameter_is_collected_as_a_binding_reference() {
        let lowered = lowered("param w: Length = 1mm;\nlet echo = w;\n");
        let expr = ident_expr(&lowered, "echo");
        let HirExpr::Ident {
            binding: Some(w), ..
        } = &expr
        else {
            unreachable!()
        };
        let (_, refs) = node_cache_key("box", &[], &[("dx", &expr)]);
        assert_eq!(refs, vec![*w]);
    }

    #[test]
    fn repeated_binding_reference_is_deduplicated() {
        let lowered = lowered("param w: Length = 1mm;\nlet echo = w;\n");
        let lhs = ident_expr(&lowered, "echo");
        let rhs = ident_expr(&lowered, "echo");
        let HirExpr::Ident {
            binding: Some(w), ..
        } = &lhs
        else {
            unreachable!()
        };
        let w = *w;
        let sum = HirExpr::Binary {
            op: cad_ast::BinaryOp::Add,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            span: Span::new(0, 1),
        };
        let (_, refs) = node_cache_key("box", &[], &[("dx", &sum)]);
        assert_eq!(refs, vec![w]);
    }

    #[test]
    fn geometry_input_keys_participate_in_the_parent_key() {
        let a = number("10", Some("mm"));
        let (child_key_a, _) = node_cache_key("box", &[], &[("dx", &a)]);
        let b = number("20", Some("mm"));
        let (child_key_b, _) = node_cache_key("box", &[], &[("dx", &b)]);
        assert_ne!(child_key_a, child_key_b);

        let (parent_with_a, _) = node_cache_key("cut", &[child_key_a], &[]);
        let (parent_with_b, _) = node_cache_key("cut", &[child_key_b], &[]);
        assert_ne!(
            parent_with_a, parent_with_b,
            "a structural change to an upstream node's own key must change every \
             downstream node's key too"
        );
    }

    #[test]
    fn cache_key_ignores_span_position() {
        let a = HirExpr::Literal {
            value: HirLiteral::Number {
                text: "10".to_string(),
                unit: Some("mm".to_string()),
            },
            ty: None,
            span: Span::new(0, 5),
        };
        let b = HirExpr::Literal {
            value: HirLiteral::Number {
                text: "10".to_string(),
                unit: Some("mm".to_string()),
            },
            ty: None,
            span: Span::new(40, 45),
        };
        let (key_a, _) = node_cache_key("box", &[], &[("dx", &a)]);
        let (key_b, _) = node_cache_key("box", &[], &[("dx", &b)]);
        assert_eq!(key_a, key_b);
    }
}
