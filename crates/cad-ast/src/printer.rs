//! `AICAD-045` — a minimal AST pretty-printer ("formatter hooks", per
//! `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §3, WP-02's own "Owns"
//! list, which groups the formatter with the parser/AST it prints
//! rather than a separate crate/work-package).
//!
//! **Contract**: [`print_program`] is a pure function of the AST —
//! output depends only on parsed structure, never on the original
//! source's whitespace/formatting (per `AGENTS.md`'s "The formatter
//! must preserve program semantics" and `DECISION_LOG.md#DL-1`,
//! "[i]ndentation is formatting only, never syntax"). This is what makes
//! the printer's own round-trip property provable: reformatting an
//! already-formatted program is a no-op fixed point
//! (`print(parse(print(parse(source)))) == print(parse(source))`), which
//! `crates/cad-ast/tests/printer_round_trip.rs` checks directly rather
//! than attempting a span-insensitive AST equality (spans necessarily
//! differ between the original and reprinted source's byte offsets, so
//! comparing printed *text* stability is the meaningful check, not
//! `PartialEq` on the re-parsed `Program`).
//!
//! **Scope**: exactly the AST surface implemented through `AICAD-044`
//! (every `Item`/`Stmt`/`Expr`/`Type`/`Pattern`/`ImportPath` variant that
//! exists today). No comment/doc-comment preservation (`cad-lexer`
//! discards `//`/`/* */` trivia entirely and only keeps `///` doc
//! comments as a distinct, not-yet-attached-to-any-AST-node token kind —
//! attaching them to items is a later task's job, not this one's).
//! Explicit `(parenthesized)` grouping is preserved exactly as written
//! (`Expr::Paren` is printed verbatim) rather than re-derived from
//! operator precedence — see this module's own precedence note below for
//! why no *other* parentheses ever need to be synthesized.
//!
//! **Precedence note** (why the printer never needs to add its own
//! parentheses beyond an explicit `Expr::Paren` node): `cad-parser`'s
//! precedence-climbing `parse_binary_level` always builds a `Binary`
//! node's `rhs` from the *next tighter* precedence level, and folds same-
//! precedence operators left-associatively into `lhs`. So a `Binary`
//! node's `rhs` can only ever be unparenthesized-and-still-correct if it
//! is strictly higher precedence than its parent (exactly what parsing
//! produced), and a `lhs` can only be same-or-higher precedence (again
//! exactly what left-associative folding produced) — printing the tree
//! exactly as parsed, with no added parentheses, always reparses back to
//! the identical structure. Any source that actually needed different
//! grouping already has an explicit `Expr::Paren` node from parsing.

use crate::{
    Arg, Block, BlockExpr, ElseBranch, ElseClause, Expr, Field, FnParam, ImportPath, Item, Literal,
    MatchArm, MatchArmBody, Pattern, Program, Stmt, Type,
};

const INDENT_UNIT: &str = "    ";

/// Formats `program` as canonical AICAD source text (always ending in
/// exactly one trailing newline).
pub fn print_program(program: &Program) -> String {
    let mut printer = Printer {
        out: String::new(),
        indent: 0,
    };
    printer.print_program(program);
    printer.finish()
}

struct Printer {
    out: String,
    indent: usize,
}

impl Printer {
    fn finish(mut self) -> String {
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.out
    }

    fn write(&mut self, s: &str) {
        self.out.push_str(s);
    }

    fn newline(&mut self) {
        self.out.push('\n');
    }

    fn push_indent(&mut self) {
        for _ in 0..self.indent {
            self.out.push_str(INDENT_UNIT);
        }
    }

    // --- program / items --------------------------------------------

    fn print_program(&mut self, program: &Program) {
        for (i, item) in program.items.iter().enumerate() {
            if i > 0 {
                self.newline();
            }
            self.push_indent();
            self.print_item(item);
            self.newline();
        }
    }

    fn print_item(&mut self, item: &Item) {
        match item {
            Item::Let {
                name, ty, value, ..
            } => {
                self.write("let ");
                self.write(&name.node);
                self.print_type_annotation(ty);
                self.write(" = ");
                self.print_expr(value);
                self.write(";");
            }
            Item::Const {
                name, ty, value, ..
            } => {
                self.write("const ");
                self.write(&name.node);
                self.print_type_annotation(ty);
                self.write(" = ");
                self.print_expr(value);
                self.write(";");
            }
            Item::Param {
                name, ty, default, ..
            } => {
                self.write("param ");
                self.write(&name.node);
                self.write(": ");
                self.print_type(ty);
                if let Some(default) = default {
                    self.write(" = ");
                    self.print_expr(default);
                }
                self.write(";");
            }
            Item::Fn {
                is_pure,
                name,
                params,
                return_ty,
                body,
                ..
            } => {
                if *is_pure {
                    self.write("pure ");
                }
                self.write("fn ");
                self.write(&name.node);
                self.write("(");
                self.print_fn_params(params);
                self.write(")");
                if let Some(return_ty) = return_ty {
                    self.write(" -> ");
                    self.print_type(return_ty);
                }
                self.write(" ");
                self.print_block(body);
            }
            Item::Struct { name, fields, .. } => {
                self.write("struct ");
                self.write(&name.node);
                self.write(" ");
                self.print_field_list(fields);
            }
            Item::Enum { name, variants, .. } => {
                self.write("enum ");
                self.write(&name.node);
                self.write(" ");
                self.print_brace_list(variants.len(), |p, i| p.write(&variants[i].node));
            }
            Item::Part { name, items, .. } => {
                self.write("part ");
                self.write(&name.node);
                self.write(" ");
                self.print_item_block(items);
            }
            Item::Import { path, names, .. } => {
                self.write("import ");
                self.print_import_path(path);
                if let Some(names) = names {
                    self.write("::{");
                    for (i, name) in names.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.write(&name.node);
                    }
                    self.write("}");
                }
                self.write(";");
            }
        }
    }

    fn print_type_annotation(&mut self, ty: &Option<Type>) {
        if let Some(ty) = ty {
            self.write(": ");
            self.print_type(ty);
        }
    }

    fn print_type(&mut self, ty: &Type) {
        match ty {
            Type::Named(name) => self.write(&name.node),
            Type::Generic { name, args, .. } => {
                self.write(&name.node);
                self.write("<");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.print_type(arg);
                }
                self.write(">");
            }
        }
    }

    fn print_fn_params(&mut self, params: &[FnParam]) {
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.write(&param.name.node);
            self.write(": ");
            self.print_type(&param.ty);
            if let Some(default) = &param.default {
                self.write(" = ");
                self.print_expr(default);
            }
        }
    }

    fn print_field_list(&mut self, fields: &[Field]) {
        self.print_brace_list(fields.len(), |p, i| {
            p.write(&fields[i].name.node);
            p.write(": ");
            p.print_type(&fields[i].ty);
        });
    }

    fn print_item_block(&mut self, items: &[Item]) {
        if items.is_empty() {
            self.write("{}");
            return;
        }
        self.write("{");
        self.newline();
        self.indent += 1;
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                self.newline();
            }
            self.push_indent();
            self.print_item(item);
            self.newline();
        }
        self.indent -= 1;
        self.push_indent();
        self.write("}");
    }

    /// Shared shape for a `{ entry, entry, ... }` list where each entry
    /// is one line followed by a trailing comma (struct fields, enum
    /// variants) — `write_entry(self, index)` writes just that entry's
    /// own text, this method owns the braces/indentation/commas/empty
    /// case around it.
    fn print_brace_list(&mut self, len: usize, mut write_entry: impl FnMut(&mut Self, usize)) {
        if len == 0 {
            self.write("{}");
            return;
        }
        self.write("{");
        self.newline();
        self.indent += 1;
        for i in 0..len {
            self.push_indent();
            write_entry(self, i);
            self.write(",");
            self.newline();
        }
        self.indent -= 1;
        self.push_indent();
        self.write("}");
    }

    // --- statements ----------------------------------------------------

    fn print_block(&mut self, block: &Block) {
        if block.stmts.is_empty() {
            self.write("{}");
            return;
        }
        self.write("{");
        self.newline();
        self.indent += 1;
        for stmt in &block.stmts {
            self.push_indent();
            self.print_stmt(stmt);
            self.newline();
        }
        self.indent -= 1;
        self.push_indent();
        self.write("}");
    }

    fn print_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                name, ty, value, ..
            } => {
                self.write("let ");
                self.write(&name.node);
                self.print_type_annotation(ty);
                self.write(" = ");
                self.print_expr(value);
                self.write(";");
            }
            Stmt::Var {
                name, ty, value, ..
            } => {
                self.write("var ");
                self.write(&name.node);
                self.print_type_annotation(ty);
                self.write(" = ");
                self.print_expr(value);
                self.write(";");
            }
            Stmt::Assign { name, value, .. } => {
                self.write(&name.node);
                self.write(" = ");
                self.print_expr(value);
                self.write(";");
            }
            Stmt::Expr { expr, .. } => {
                self.print_expr(expr);
                self.write(";");
            }
            Stmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.write("if ");
                self.print_expr(cond);
                self.write(" ");
                self.print_block(then_branch);
                if let Some(clause) = else_branch {
                    self.write(" else ");
                    match clause {
                        ElseClause::Block(block) => self.print_block(block),
                        ElseClause::If(inner) => self.print_stmt(inner),
                    }
                }
            }
            Stmt::For {
                var,
                iterable,
                body,
                ..
            } => {
                self.write("for ");
                self.write(&var.node);
                self.write(" in ");
                self.print_expr(iterable);
                self.write(" ");
                self.print_block(body);
            }
            Stmt::While { cond, body, .. } => {
                self.write("while ");
                self.print_expr(cond);
                self.write(" ");
                self.print_block(body);
            }
            Stmt::Loop { body, .. } => {
                self.write("loop ");
                self.print_block(body);
            }
            Stmt::Match {
                scrutinee, arms, ..
            } => {
                self.write("match ");
                self.print_expr(scrutinee);
                self.write(" ");
                self.print_match_arms(arms);
            }
            Stmt::Return { value, .. } => {
                self.write("return");
                if let Some(value) = value {
                    self.write(" ");
                    self.print_expr(value);
                }
                self.write(";");
            }
            Stmt::Break { .. } => self.write("break;"),
            Stmt::Continue { .. } => self.write("continue;"),
        }
    }

    fn print_match_arms(&mut self, arms: &[MatchArm]) {
        if arms.is_empty() {
            self.write("{}");
            return;
        }
        self.write("{");
        self.newline();
        self.indent += 1;
        for arm in arms {
            self.push_indent();
            self.print_pattern(&arm.pattern);
            self.write(" => ");
            match &arm.body {
                MatchArmBody::Expr(expr) => {
                    self.print_expr(expr);
                    self.write(",");
                }
                MatchArmBody::Block(block_expr) => self.print_block_expr(block_expr),
            }
            self.newline();
        }
        self.indent -= 1;
        self.push_indent();
        self.write("}");
    }

    fn print_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard(_) => self.write("_"),
            Pattern::Literal(lit) => self.print_literal(&lit.node),
            Pattern::Ident(name) => self.write(&name.node),
        }
    }

    // --- expressions -----------------------------------------------

    fn print_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(lit) => self.print_literal(&lit.node),
            Expr::Ident(name) => self.write(&name.node),
            Expr::Unary { op, operand, .. } => {
                self.write(op.as_str());
                self.print_expr(operand);
            }
            Expr::Binary { op, lhs, rhs, .. } => {
                self.print_expr(lhs);
                self.write(" ");
                self.write(op.as_str());
                self.write(" ");
                self.print_expr(rhs);
            }
            Expr::Paren { inner, .. } => {
                self.write("(");
                self.print_expr(inner);
                self.write(")");
            }
            Expr::Call { callee, args, .. } => {
                self.write(&callee.node);
                self.write("(");
                self.print_args(args);
                self.write(")");
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                self.print_expr(receiver);
                self.write(".");
                self.write(&method.node);
                self.write("(");
                self.print_args(args);
                self.write(")");
            }
            Expr::Field {
                receiver, field, ..
            } => {
                self.print_expr(receiver);
                self.write(".");
                self.write(&field.node);
            }
            Expr::Block(block_expr) => self.print_block_expr(block_expr),
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.write("if ");
                self.print_expr(cond);
                self.write(" ");
                self.print_block_expr(then_branch);
                self.write(" else ");
                match else_branch.as_ref() {
                    ElseBranch::Block(block_expr) => self.print_block_expr(block_expr),
                    ElseBranch::If(inner) => self.print_expr(inner),
                }
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                self.write("match ");
                self.print_expr(scrutinee);
                self.write(" ");
                self.print_match_arms(arms);
            }
        }
    }

    fn print_args(&mut self, args: &[Arg]) {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            match arg {
                Arg::Positional(expr) => self.print_expr(expr),
                Arg::Named { name, value } => {
                    self.write(&name.node);
                    self.write(" = ");
                    self.print_expr(value);
                }
            }
        }
    }

    fn print_block_expr(&mut self, block_expr: &BlockExpr) {
        if block_expr.stmts.is_empty() && block_expr.trailing.is_none() {
            self.write("{}");
            return;
        }
        self.write("{");
        self.newline();
        self.indent += 1;
        for stmt in &block_expr.stmts {
            self.push_indent();
            self.print_stmt(stmt);
            self.newline();
        }
        if let Some(trailing) = &block_expr.trailing {
            self.push_indent();
            self.print_expr(trailing);
            self.newline();
        }
        self.indent -= 1;
        self.push_indent();
        self.write("}");
    }

    fn print_literal(&mut self, lit: &Literal) {
        match lit {
            Literal::Number { text, unit } => {
                self.write(text);
                if let Some(unit) = unit {
                    self.write(unit);
                }
            }
            Literal::Str(s) => {
                self.write("\"");
                self.write(&escape_string(s));
                self.write("\"");
            }
            Literal::RawStr(s) => {
                self.write("r\"");
                self.write(s);
                self.write("\"");
            }
            Literal::Bool(b) => self.write(if *b { "true" } else { "false" }),
        }
    }

    fn print_import_path(&mut self, path: &ImportPath) {
        match path {
            ImportPath::Package { segments, .. } => {
                for (i, seg) in segments.iter().enumerate() {
                    if i > 0 {
                        self.write(".");
                    }
                    self.write(&seg.node);
                }
            }
            ImportPath::Relative {
                up_levels,
                segments,
                ..
            } => {
                if *up_levels == 0 {
                    self.write("./");
                } else {
                    for _ in 0..*up_levels {
                        self.write("../");
                    }
                }
                for (i, seg) in segments.iter().enumerate() {
                    if i > 0 {
                        self.write("/");
                    }
                    self.write(&seg.node);
                }
            }
        }
    }
}

/// Re-escapes a decoded [`Literal::Str`] value back into the exact set of
/// escapes `cad-lexer`'s `scan_string` decodes (`\"`, `\\`, `\n`, `\r`,
/// `\t`, `\0`) — the only characters that can ever appear in decoded
/// string content via an escape rather than a literal source character,
/// since `scan_string` itself rejects a raw, un-escaped `"` or newline
/// inside a string literal. Every other character round-trips unchanged.
fn escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            other => out.push(other),
        }
    }
    out
}
