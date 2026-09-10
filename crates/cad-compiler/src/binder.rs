//! Name binding: scopes and symbol table (`AICAD-050`,
//! `docs/plan/02_LANGUAGE_AND_COMPILER.md` §17 phase 3, "Name binding" —
//! immediately after phase 2, `AICAD-044`'s module/import loader).
//!
//! Scope: given one already-parsed [`cad_ast::Program`] (plus the `file`
//! name and `source` text `crate::loader`'s own diagnostics need for
//! spans), resolve every value-level name reference to a declaration,
//! detect duplicate declarations within one scope, and enforce DL-2's
//! explicit-rebinding rule (`Stmt::Assign` may only target a `var`
//! binding — `cad_ast::item`'s own doc comment names this exact check as
//! "a binding-phase (`AICAD-050`) concern, not the parser's").
//!
//! ## What this task does *not* do
//!
//! - **Cross-module resolution.** `crate::loader::load_entry` already
//!   resolves the file graph, but its [`crate::loader::Module`] does not
//!   retain each file's raw source text (only `path`/`program`), and
//!   wiring this binder across the whole [`crate::loader::LoadResult`]
//!   would need it (for source-span diagnostics) — `AICAD-044`'s own
//!   report already deferred "symbol-level resolution" to "`AICAD-050`+"
//!   (not just `=050`), so this task binds one already-parsed program at
//!   a time, exactly like `cad_parser::parse_program` itself does, rather
//!   than reworking a previous task's completed struct to fetch a second
//!   one's dependency. A selective `import ... ::{Name}` therefore binds
//!   `Name` into the importing module's own scope (so ordinary uses of it
//!   are not flagged as undefined — the RFC's own evidenced import
//!   semantics), without verifying it actually exists in the target file;
//!   a whole-module `import path;` (no `{...}` list) binds no new name at
//!   all, since no frozen grammar/RFC material shows how such an import's
//!   members would later be referenced (inventing one would be exactly
//!   the speculative syntax `AGENTS.md` warns against).
//! - **Type-name resolution.** `cad_ast::item::Type::Named`/`Generic`
//!   references (`param width: Length`, `-> Vector2<Length>`) are a
//!   distinct namespace from the value-level names this module binds, and
//!   resolving them needs `cad-types`/`cad-units` (`AICAD-046`-`048`),
//!   not just `cad-ast`. Left for the type checker (`AICAD-052`).
//! - **Struct field duplicate-name checking.** `project/TASKS.yaml`
//!   explicitly assigns "structs/enums field and variant typing" to
//!   `AICAD-053`; field names are a separate namespace from this module's
//!   scope-based value/variant bindings.
//! - **Method-call/field-access member resolution.** `Expr::MethodCall`'s
//!   `method` and `Expr::Field`'s `field` are resolved against the
//!   receiver's *type*, not a lexical scope — a type-checking concern
//!   (`AICAD-052`+), not a name-binding one; this module walks into a
//!   method call/field access's `receiver` (and any call arguments) but
//!   never treats `method`/`field` themselves as scope-bound identifiers.
//!
//! ## Enum variants share the ordinary scope/symbol mechanism
//!
//! `cad_ast::expr::Pattern`'s own doc comment states the exact
//! disambiguation this module must perform: a bare identifier pattern
//! "binds a name, or matches an enum-variant-shaped name; disambiguating
//! those two readings is a binding-phase concern, not the parser's."
//! Rather than a separate global variant table, each `enum` declaration's
//! variants are declared as ordinary symbols (`SymbolKind::EnumVariant`)
//! in the *same* scope as the `enum` item itself, so variant visibility
//! follows the same lexical-scoping rule as every other declaration (a
//! variant declared inside a `part` is only visible inside that `part`).
//! A `Pattern::Ident` that resolves to an `EnumVariant` symbol is a
//! variant match (no new binding); anything else is an ordinary fresh
//! binding for that match arm, following Rust-like match-binding
//! shadowing (DL-1, "broadly Rust/TypeScript-like").

use cad_ast::{
    Arg, Block, BlockExpr, ElseBranch, ElseClause, Expr, FnParam, Item, MatchArm, MatchArmBody,
    Pattern, Program, Span, Spanned, Stmt,
};
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SeverityLetter, SourceSpan};
use std::collections::HashMap;

/// What kind of declaration a [`Symbol`] came from — governs both
/// whether `Stmt::Assign` may target it (only [`SymbolKind::Var`]) and
/// how [`Pattern::Ident`] resolution treats it (only
/// [`SymbolKind::EnumVariant`] is a match, never a fresh binding).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Let,
    Var,
    Const,
    /// A function parameter, or an item-level `param` declaration
    /// (`cad_ast::Item::Param`) — both are immutable configurable inputs,
    /// not rebindable via `Stmt::Assign` (DL-2).
    Param,
    Fn,
    Struct,
    Enum,
    /// One variant of the enum named `enum_name`, declared in the same
    /// scope as that `Item::Enum` — see module doc comment.
    EnumVariant {
        enum_name: String,
    },
    Part,
    /// A name brought into scope by a selective `import ...::{Name}` —
    /// see module doc comment for why this task does not verify it
    /// against the target module's actual exports.
    Import,
    ForLoopVar,
    /// A fresh name introduced by a non-variant `Pattern::Ident` in a
    /// `match` arm.
    MatchBinding,
}

impl SymbolKind {
    fn is_mutable(&self) -> bool {
        matches!(self, SymbolKind::Var)
    }

    fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Let => "let binding",
            SymbolKind::Var => "var binding",
            SymbolKind::Const => "const",
            SymbolKind::Param => "parameter",
            SymbolKind::Fn => "function",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::EnumVariant { .. } => "enum variant",
            SymbolKind::Part => "part",
            SymbolKind::Import => "imported name",
            SymbolKind::ForLoopVar => "for-loop variable",
            SymbolKind::MatchBinding => "match binding",
        }
    }
}

/// One declared name: what it is, and the span of its declaring
/// identifier (used to point a "previously declared here" note at the
/// original declaration when a duplicate is found).
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
}

/// The result of binding one program: every `TYPE-E4##`/`TYPE-I4##`
/// diagnostic raised (undefined names, duplicate declarations, illegal
/// assignment targets). Deterministic, source-order traversal — see
/// `bind_program`'s own doc comment.
#[derive(Debug, Clone)]
pub struct BindResult {
    pub diagnostics: Vec<Diagnostic>,
}

struct Binder<'a> {
    file: &'a str,
    source: &'a str,
    /// Lexical scope stack, outermost (module scope) first. Always has at
    /// least one entry — `bind_program` pushes the module scope and never
    /// pops it.
    scopes: Vec<HashMap<String, Symbol>>,
    diagnostics: Vec<Diagnostic>,
}

/// Binds one already-parsed program, per this module's documented scope.
/// `file`/`source` are used only to attach source spans to diagnostics
/// (the same two pieces of context `cad_parser::parse_program` and
/// `crate::loader::load_entry`'s own diagnostics need) — traversal order
/// is depth-first, source order throughout, so results are deterministic
/// for the same input (`project/DECISION_LOG.md#DL-12`, D5 Level 1).
pub fn bind_program(program: &Program, file: &str, source: &str) -> BindResult {
    let mut binder = Binder {
        file,
        source,
        scopes: vec![HashMap::new()],
        diagnostics: Vec::new(),
    };
    binder.bind_items(&program.items);
    BindResult {
        diagnostics: binder.diagnostics,
    }
}

impl<'a> Binder<'a> {
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes
            .pop()
            .expect("push_scope/pop_scope calls are always balanced");
    }

    /// Declares `name` with `kind` in the innermost active scope. A name
    /// already declared in that *same* scope is a duplicate-binding
    /// error (RFC-consistent with `AGENTS.md`'s general aversion to
    /// silently-overwritten identity); a name that merely shadows an
    /// *outer* scope's binding is allowed (DL-1, "broadly Rust-like" —
    /// ordinary Rust allows `let` shadowing, including a function body
    /// shadowing one of its own parameters).
    fn declare(&mut self, name: &Spanned<String>, kind: SymbolKind) {
        let scope = self
            .scopes
            .last_mut()
            .expect("bind_program always keeps at least one scope active");
        if let Some(existing) = scope.get(&name.node) {
            self.diagnostics.push(duplicate_binding_diagnostic(
                self.file,
                self.source,
                name,
                existing,
            ));
        } else {
            scope.insert(
                name.node.clone(),
                Symbol {
                    name: name.node.clone(),
                    kind,
                    span: name.span,
                },
            );
        }
    }

    fn resolve(&self, name: &str) -> Option<&Symbol> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    /// Records an `UNDEFINED_NAME` diagnostic if `name` resolves nowhere
    /// in the active scope chain.
    fn check_used(&mut self, name: &Spanned<String>) {
        if self.resolve(&name.node).is_none() {
            self.diagnostics
                .push(undefined_name_diagnostic(self.file, self.source, name));
        }
    }

    /// Declares every item in `items` in the current (innermost) scope
    /// first, *then* checks each item's own body — a single two-pass
    /// walk per nesting level, so sibling items (including a `part`'s own
    /// nested items) may forward-reference and mutually recurse with each
    /// other regardless of source order, matching ordinary top-level
    /// function/type declaration order-independence in Rust-like
    /// languages.
    fn bind_items(&mut self, items: &[Item]) {
        for item in items {
            self.declare_item_name(item);
        }
        for item in items {
            self.check_item_body(item);
        }
    }

    fn declare_item_name(&mut self, item: &Item) {
        match item {
            Item::Let { name, .. } => self.declare(name, SymbolKind::Let),
            Item::Const { name, .. } => self.declare(name, SymbolKind::Const),
            Item::Param { name, .. } => self.declare(name, SymbolKind::Param),
            Item::Fn { name, .. } => self.declare(name, SymbolKind::Fn),
            Item::Struct { name, .. } => self.declare(name, SymbolKind::Struct),
            Item::Enum { name, variants, .. } => {
                self.declare(name, SymbolKind::Enum);
                for variant in variants {
                    self.declare(
                        variant,
                        SymbolKind::EnumVariant {
                            enum_name: name.node.clone(),
                        },
                    );
                }
            }
            Item::Part { name, .. } => self.declare(name, SymbolKind::Part),
            Item::Import { names, .. } => {
                // Whole-module imports (`names: None`) bind no name — see
                // module doc comment.
                for name in names.iter().flatten() {
                    self.declare(name, SymbolKind::Import);
                }
            }
        }
    }

    fn check_item_body(&mut self, item: &Item) {
        match item {
            Item::Let { value, .. } | Item::Const { value, .. } => self.check_expr(value),
            Item::Param { default, .. } => {
                if let Some(default) = default {
                    self.check_expr(default);
                }
            }
            Item::Fn { params, body, .. } => {
                self.push_scope();
                self.bind_fn_params(params);
                self.check_block(body);
                self.pop_scope();
            }
            // Field types are a distinct namespace this task does not
            // resolve (module doc comment); no expressions to check.
            Item::Struct { .. } => {}
            // Variants were already declared by `declare_item_name`;
            // nothing further to check.
            Item::Enum { .. } => {}
            Item::Part { items, .. } => {
                self.push_scope();
                self.bind_items(items);
                self.pop_scope();
            }
            Item::Import { .. } => {}
        }
    }

    /// Declares every parameter in the current (already-pushed) scope,
    /// checking each `= default` expression *before* declaring that
    /// parameter's own name — a default expression referring to its own
    /// parameter name (`fn f(x: Int = x)`) is `UNDEFINED_NAME`, not a
    /// self-reference, matching every parameter list's evaluation order
    /// in ordinary left-to-right languages.
    fn bind_fn_params(&mut self, params: &[FnParam]) {
        for param in params {
            if let Some(default) = &param.default {
                self.check_expr(default);
            }
            self.declare(&param.name, SymbolKind::Param);
        }
    }

    fn check_block(&mut self, block: &Block) {
        self.push_scope();
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        self.pop_scope();
    }

    fn check_block_expr(&mut self, block: &BlockExpr) {
        self.push_scope();
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        if let Some(trailing) = &block.trailing {
            self.check_expr(trailing);
        }
        self.pop_scope();
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, value, .. } => {
                self.check_expr(value);
                self.declare(name, SymbolKind::Let);
            }
            Stmt::Var { name, value, .. } => {
                self.check_expr(value);
                self.declare(name, SymbolKind::Var);
            }
            Stmt::Assign { name, value, .. } => {
                self.check_expr(value);
                match self.resolve(&name.node) {
                    None => self.diagnostics.push(undefined_name_diagnostic(
                        self.file,
                        self.source,
                        name,
                    )),
                    Some(sym) if !sym.kind.is_mutable() => {
                        self.diagnostics.push(assign_to_immutable_diagnostic(
                            self.file,
                            self.source,
                            name,
                            sym,
                        ));
                    }
                    Some(_) => {}
                }
            }
            Stmt::Expr { expr, .. } => self.check_expr(expr),
            Stmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.check_expr(cond);
                self.check_block(then_branch);
                if let Some(else_clause) = else_branch {
                    self.check_else_clause(else_clause);
                }
            }
            Stmt::For {
                var,
                iterable,
                body,
                ..
            } => {
                self.check_expr(iterable);
                self.push_scope();
                self.declare(var, SymbolKind::ForLoopVar);
                self.check_block(body);
                self.pop_scope();
            }
            Stmt::While { cond, body, .. } => {
                self.check_expr(cond);
                self.check_block(body);
            }
            Stmt::Loop { body, .. } => self.check_block(body),
            Stmt::Match {
                scrutinee, arms, ..
            } => {
                self.check_expr(scrutinee);
                for arm in arms {
                    self.check_match_arm(arm);
                }
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.check_expr(value);
                }
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }

    fn check_else_clause(&mut self, clause: &ElseClause) {
        match clause {
            ElseClause::Block(block) => self.check_block(block),
            ElseClause::If(stmt) => self.check_stmt(stmt),
        }
    }

    fn check_else_branch(&mut self, branch: &ElseBranch) {
        match branch {
            ElseBranch::Block(block_expr) => self.check_block_expr(block_expr),
            ElseBranch::If(expr) => self.check_expr(expr),
        }
    }

    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(_) => {}
            Expr::Ident(name) => self.check_used(name),
            Expr::Unary { operand, .. } => self.check_expr(operand),
            Expr::Binary { lhs, rhs, .. } => {
                self.check_expr(lhs);
                self.check_expr(rhs);
            }
            Expr::Paren { inner, .. } => self.check_expr(inner),
            Expr::Call { callee, args, .. } => {
                self.check_used(callee);
                for arg in args {
                    self.check_arg(arg);
                }
            }
            Expr::MethodCall { receiver, args, .. } => {
                self.check_expr(receiver);
                for arg in args {
                    self.check_arg(arg);
                }
                // `method` is resolved against `receiver`'s type later
                // (module doc comment), never against a lexical scope.
            }
            Expr::Field { receiver, .. } => self.check_expr(receiver),
            Expr::Block(block_expr) => self.check_block_expr(block_expr),
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.check_expr(cond);
                self.check_block_expr(then_branch);
                self.check_else_branch(else_branch);
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                self.check_expr(scrutinee);
                for arm in arms {
                    self.check_match_arm(arm);
                }
            }
            Expr::ListLiteral { elements, .. } => {
                for element in elements {
                    self.check_expr(element);
                }
            }
            Expr::Range { start, end, .. } => {
                self.check_expr(start);
                self.check_expr(end);
            }
        }
    }

    fn check_arg(&mut self, arg: &Arg) {
        match arg {
            Arg::Positional(expr) => self.check_expr(expr),
            // `name` is a call-site keyword-argument label, not a
            // scope-bound identifier.
            Arg::Named { value, .. } => self.check_expr(value),
        }
    }

    fn check_match_arm(&mut self, arm: &MatchArm) {
        self.push_scope();
        self.bind_pattern(&arm.pattern);
        match &arm.body {
            MatchArmBody::Expr(expr) => self.check_expr(expr),
            MatchArmBody::Block(block_expr) => self.check_block_expr(block_expr),
        }
        self.pop_scope();
    }

    /// See module doc comment "Enum variants share the ordinary
    /// scope/symbol mechanism" for the variant-vs-fresh-binding
    /// disambiguation this implements.
    fn bind_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard(_) | Pattern::Literal(_) => {}
            Pattern::Ident(name) => {
                let is_variant = matches!(
                    self.resolve(&name.node).map(|sym| &sym.kind),
                    Some(SymbolKind::EnumVariant { .. })
                );
                if !is_variant {
                    // A fresh binding in this arm's own (just-pushed,
                    // otherwise-empty) scope; `declare`'s duplicate check
                    // can never fire here since nothing else has been
                    // declared into this scope yet.
                    self.declare(name, SymbolKind::MatchBinding);
                }
            }
        }
    }
}

fn severity_letter(severity: Severity) -> SeverityLetter {
    match severity {
        Severity::Error => SeverityLetter::Error,
        Severity::Warning => SeverityLetter::Warning,
        Severity::Info => SeverityLetter::Info,
    }
}

/// Builds a source-spanned diagnostic, mirroring `crate::loader`'s own
/// `diagnostic` helper exactly (family `TYPE` here, since name binding is
/// part of RFC-0005 §2's broader semantic-analysis family, not `IMPORT`'s
/// narrower module-resolution one; category `"binding"`).
fn diagnostic(
    number: u16,
    severity: Severity,
    title: &str,
    message: String,
    file: &str,
    source: &str,
    span: Span,
) -> Diagnostic {
    let line_index = cad_ast::LineIndex::new(source);
    let start = line_index.line_column(source, span.start);
    let end = line_index.line_column(source, span.end);
    let code = DiagnosticCode::new("TYPE", severity_letter(severity), number)
        .expect("TYPE family and 1..=999 number are always valid");
    Diagnostic::new(code, severity, "binding", title, message)
        .expect("severity_letter always agrees with the severity passed in")
        .with_source(SourceSpan {
            file: file.to_string(),
            start: Position::new(start.line, start.column),
            end: Position::new(end.line, end.column),
        })
}

fn undefined_name_diagnostic(file: &str, source: &str, name: &Spanned<String>) -> Diagnostic {
    diagnostic(
        401,
        Severity::Error,
        "UNDEFINED_NAME",
        format!("Cannot find '{}' in this scope.", name.node),
        file,
        source,
        name.span,
    )
}

fn duplicate_binding_diagnostic(
    file: &str,
    source: &str,
    name: &Spanned<String>,
    existing: &Symbol,
) -> Diagnostic {
    let line_index = cad_ast::LineIndex::new(source);
    let existing_pos = line_index.line_column(source, existing.span.start);
    diagnostic(
        402,
        Severity::Error,
        "DUPLICATE_BINDING",
        format!(
            "'{}' is already defined as a {} at line {}, column {}.",
            name.node,
            existing.kind.as_str(),
            existing_pos.line,
            existing_pos.column
        ),
        file,
        source,
        name.span,
    )
}

fn assign_to_immutable_diagnostic(
    file: &str,
    source: &str,
    name: &Spanned<String>,
    existing: &Symbol,
) -> Diagnostic {
    diagnostic(
        403,
        Severity::Error,
        "ASSIGN_TO_IMMUTABLE",
        format!(
            "Cannot assign to '{}': it is a {}, not a 'var' binding.",
            name.node,
            existing.kind.as_str()
        ),
        file,
        source,
        name.span,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bind(source: &str) -> Vec<Diagnostic> {
        let (program, parse_diagnostics) = cad_parser::parse_program(source, "test.aicad");
        assert!(
            parse_diagnostics.is_empty(),
            "test source failed to parse: {parse_diagnostics:?}"
        );
        bind_program(&program, "test.aicad", source).diagnostics
    }

    fn codes(diagnostics: &[Diagnostic]) -> Vec<String> {
        diagnostics.iter().map(|d| d.code.as_string()).collect()
    }

    // --- Basic resolution ---
    //
    // Note on source syntax: `cad_ast::item::Block` (a `fn`/`if`/`for`/
    // `while`/`loop` body) has no trailing-expression slot — only
    // `BlockExpr` (used in *expression* position, e.g. an `if` used as
    // the right-hand side of a `let`) does. Every statement-position
    // block below therefore ends its last expression with `;`
    // (`Stmt::Expr`, value discarded) rather than relying on a bare
    // trailing expression, matching `specs/language/grammar.ebnf`
    // exactly; these tests check name resolution, not return-value
    // correctness, so a function that never actually returns its
    // declared type is not itself a bug here.

    #[test]
    fn top_level_let_is_visible_to_a_later_fn() {
        let diags = bind("let width = 5mm; fn area() -> Length { width; }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn forward_reference_between_sibling_fns_resolves() {
        // `a` calls `b`, declared *after* it — both are declared into
        // module scope before either body is checked.
        let diags = bind("fn a() -> Int { b(); } fn b() -> Int { 1; }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn recursive_fn_call_resolves() {
        let diags = bind("fn fact(n: Int) -> Int { fact(n); }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn undefined_identifier_is_reported() {
        let diags = bind("fn f() -> Int { missing; }");
        assert_eq!(codes(&diags), vec!["TYPE-E401"]);
    }

    #[test]
    fn undefined_call_callee_is_reported() {
        let diags = bind("fn f() -> Int { missing_fn(); }");
        assert_eq!(codes(&diags), vec!["TYPE-E401"]);
    }

    #[test]
    fn method_name_is_never_checked_as_a_scope_name() {
        // `frobnicate` is not declared anywhere, but it is a method name
        // (resolved against `x`'s type later), not a scope-bound
        // identifier — only `x` itself must resolve.
        let diags = bind("fn f(x: Int) -> Int { x.frobnicate(); }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn field_name_is_never_checked_as_a_scope_name() {
        let diags = bind("fn f(x: Int) -> Int { x.frobnicate; }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    // --- Duplicate bindings ---

    #[test]
    fn duplicate_top_level_item_is_reported() {
        let diags = bind("let a = 1; let a = 2;");
        assert_eq!(codes(&diags), vec!["TYPE-E402"]);
    }

    #[test]
    fn duplicate_fn_param_is_reported() {
        let diags = bind("fn f(x: Int, x: Int) -> Int { x; }");
        assert_eq!(codes(&diags), vec!["TYPE-E402"]);
    }

    #[test]
    fn duplicate_let_in_same_block_is_reported() {
        let diags = bind("fn f() -> Int { let a = 1; let a = 2; a; }");
        assert_eq!(codes(&diags), vec!["TYPE-E402"]);
    }

    #[test]
    fn shadowing_a_param_with_a_body_let_is_allowed() {
        // Rust itself allows this (DL-1, "broadly Rust-like"): the body
        // is its own nested scope, distinct from the parameter scope.
        let diags = bind("fn f(x: Int) -> Int { let x = 5; x; }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn shadowing_in_a_nested_scope_is_allowed() {
        // The grammar has no bare `{ ... }` block *statement* (only
        // `BlockExpr`, used in expression position) — an `if` body is
        // used here as the nested scope instead.
        let diags = bind("fn f() -> Int { let a = 1; if true { let a = 2; a; } a; }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn duplicate_enum_variant_is_reported() {
        let diags = bind("enum E { A, A }");
        assert_eq!(codes(&diags), vec!["TYPE-E402"]);
    }

    // --- Assignment / mutability (DL-2) ---

    #[test]
    fn assigning_to_a_var_is_allowed() {
        let diags = bind("fn f() -> Int { var x = 1; x = 2; x; }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn assigning_to_a_let_is_rejected() {
        let diags = bind("fn f() -> Int { let x = 1; x = 2; x; }");
        assert_eq!(codes(&diags), vec!["TYPE-E403"]);
    }

    #[test]
    fn assigning_to_a_param_is_rejected() {
        let diags = bind("fn f(x: Int) -> Int { x = 2; x; }");
        assert_eq!(codes(&diags), vec!["TYPE-E403"]);
    }

    #[test]
    fn assigning_to_an_undefined_name_is_undefined_not_immutable() {
        let diags = bind("fn f() -> Int { missing = 2; }");
        assert_eq!(codes(&diags), vec!["TYPE-E401"]);
    }

    // --- Control flow scoping ---

    #[test]
    fn for_loop_variable_is_visible_only_inside_the_loop() {
        let diags = bind("fn f(xs: Int) -> Int { for x in xs { x; } x; }");
        // `x` used after the loop is undefined; `x` inside the loop is
        // fine.
        assert_eq!(codes(&diags), vec!["TYPE-E401"]);
    }

    #[test]
    fn while_and_loop_bodies_get_their_own_scope() {
        let diags = bind("fn f() -> Int { while true { let a = 1; a; } loop { let a = 2; a; } }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn if_branches_each_get_their_own_scope() {
        let diags = bind("fn f(c: Bool) -> Int { if c { let a = 1; a; } else { let a = 2; a; } }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn if_expr_else_if_chain_resolves_each_branch() {
        let diags =
            bind("fn f(c: Bool) -> Int { let r = if c { 1 } else if c { 2 } else { 3 }; r; }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn let_bound_in_then_branch_is_not_visible_after_if() {
        let diags = bind("fn f(c: Bool) -> Int { if c { let a = 1; a; } a; }");
        assert_eq!(codes(&diags), vec!["TYPE-E401"]);
    }

    // --- Match: variant vs. fresh binding ---

    #[test]
    fn match_arm_identifier_matching_a_variant_is_not_a_new_binding() {
        let diags = bind(
            "enum MotorSize { NEMA17, NEMA23 } fn f(m: MotorSize) -> Int { match m { NEMA17 => 1, NEMA23 => 2, } }",
        );
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn match_arm_identifier_not_matching_any_variant_is_a_fresh_binding() {
        // `other` is not a declared variant anywhere, so it is a
        // catch-all binding introducing `other` for the arm body.
        let diags = bind(
            "enum MotorSize { NEMA17 } fn f(m: MotorSize) -> MotorSize { match m { other => other, } }",
        );
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn match_binding_does_not_leak_outside_its_arm() {
        let diags = bind("fn f(m: Int) -> Int { match m { other => other, } other; }");
        assert_eq!(codes(&diags), vec!["TYPE-E401"]);
    }

    #[test]
    fn match_binding_can_shadow_an_outer_name() {
        let diags = bind("fn f(m: Int) -> Int { let other = 1; match m { other => other, } }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn part_scoped_enum_variant_is_not_visible_outside_the_part() {
        let diags = bind(
            "part P { enum E { A } fn f(x: E) -> Int { match x { A => 1, } } } fn g() -> Int { A; }",
        );
        // `A` used at module scope (outside the part) is undefined.
        assert_eq!(codes(&diags), vec!["TYPE-E401"]);
    }

    // --- Imports (module doc comment: selective only) ---

    #[test]
    fn selectively_imported_name_is_usable() {
        let diags = bind("import std.fasteners::{ISO4762}; fn f() -> Int { ISO4762; }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn whole_module_import_binds_no_name() {
        // `cycloidal` itself is never usable as a bare identifier — no
        // frozen syntax defines what that would even mean (module doc
        // comment).
        let diags = bind("import robotics.cycloidal; fn f() -> Int { cycloidal; }");
        assert_eq!(codes(&diags), vec!["TYPE-E401"]);
    }

    // --- Parts ---

    #[test]
    fn part_body_can_see_module_scope() {
        let diags = bind("let width = 5mm; part Box { fn area() -> Length { width; } }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn part_forward_references_its_own_sibling() {
        let diags = bind("part Box { fn a() -> Int { b(); } fn b() -> Int { 1; } }");
        assert!(diags.is_empty(), "{diags:?}");
    }

    // --- Diagnostic shape ---

    #[test]
    fn diagnostics_carry_a_source_span() {
        let diags = bind("fn f() -> Int { missing; }");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].source.is_some());
        assert_eq!(diags[0].source.as_ref().unwrap().file, "test.aicad");
    }

    #[test]
    fn duplicate_binding_message_names_the_original_location() {
        let diags = bind("let a = 1;\nlet a = 2;");
        assert_eq!(diags.len(), 1);
        assert!(
            diags[0].message.contains("line 1"),
            "message was: {}",
            diags[0].message
        );
    }
}
