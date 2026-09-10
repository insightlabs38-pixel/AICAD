//! Declaration/statement AST node types.
//!
//! History: `AICAD-042` ("Implement declarations:
//! let/const/param/fn/struct/enum/part") added the `item` alternatives
//! `let_decl`/`const_decl`/`param_decl`/`fn_decl`/`struct_decl`/
//! `enum_decl`/`part_decl`, plus enough statement-level parsing
//! (`let_stmt`/`var_stmt`/`assign_stmt`/`expr_stmt`) for `fn`/`part`
//! bodies to be parseable at all. `AICAD-043` ("Implement control-flow
//! syntax: if/for/while/match/return") adds the remaining `statement`
//! alternatives (`if_stmt`/`for_stmt`/`while_stmt`/`loop_stmt`/
//! `match_stmt`/`return_stmt`/`break_stmt`/`continue_stmt`) to `Stmt`
//! below. `AICAD-044` ("Implement module/import syntax and loader
//! skeleton") adds `import_decl` (`Item::Import`/`ImportPath` below) —
//! see `ImportPath`'s own doc comment for the exact two forms
//! implemented and their evidence. Deliberately **still not** in scope
//! (left for later tasks that own them, per `AGENTS.md` "No speculative
//! future work"):
//! - `interface_decl`/`assembly_decl`/`requirement_decl`/`test_decl` —
//!   not named by any task's title yet, and several use keywords
//!   `crates/cad-lexer` deliberately has not reserved yet
//!   (`configuration`, `component`, `assembly`, `instance`, `mate`,
//!   `joint`, `requirement`, `test`, `constraint`, `expose`, `query`,
//!   `unsafe` are all still unreserved identifiers — see
//!   `crates/cad-lexer/src/token.rs`'s own doc comment).
//! - enum variants carrying data (tuple/struct variants) — the only
//!   evidence for enum syntax anywhere in frozen material
//!   (`examples/assemblies/stage0_paper_example.aicad`:
//!   `enum MotorSize { NEMA17, NEMA23 }`) shows unit variants only; no
//!   RFC/plan section specifies a data-variant grammar, so guessing one
//!   now would be exactly the kind of speculative syntax `AGENTS.md`
//!   warns against. Adding it later (`AICAD-053`'s "enums field and
//!   variant typing" or a dedicated grammar update) is a small additive
//!   grammar change, not a redesign. **Update (`AICAD-057B`,
//!   `project/OWNER_DECISIONS.md#D17`/`project/DECISION_LOG.md#DL-14`):**
//!   this remains true for variant *payloads* specifically (`AICAD-057C`'s
//!   job) — but `Item::Fn`/`Item::Struct`/`Item::Enum` now each carry an
//!   ordinary `type_params: Vec<Spanned<String>>` list (`struct
//!   Pair<T, U> { ... }`, `enum Optional<T> { ... }`, `fn identity<T>(...)`)
//!   per the owner's D17 ruling, which is no longer speculative syntax —
//!   see that ruling for the exact authorized shape and scope limits
//!   (no bounds/higher-kinded types/variance/specialization here).
//! - `assign_stmt`'s target stays a bare identifier, exactly as the
//!   grammar specifies (`assign_stmt = identifier "=" expression ";"`) —
//!   no field-assignment target (`a.b = x;`), which is consistent with
//!   DL-2's functional-core/no-in-place-mutation ruling: nothing in DL-2
//!   or the grammar sketch describes mutating a field through assignment.

use crate::{Expr, MatchArm, Span, Spanned};

/// A syntactic type reference as written in source (`: Length`,
/// `: Vector2<Length>`, `-> List<Point2>`). Purely syntactic — no
/// resolution, no relation yet to any `cad-types`/`cad-units` concept;
/// that binding happens starting `AICAD-046`.
///
/// Scope note: the grammar sketch's `type` production is never spelled
/// out anywhere in `specs/language/grammar.ebnf` or any frozen RFC, only
/// used informally (`param width: Length`, `Vector2<Length>`,
/// `List<Point2>` in the paper example). This implements exactly the two
/// shapes that example evidences — a bare name, or a name with
/// comma-separated generic type arguments — and nothing broader (no
/// array/tuple/function-type syntax, none of which has any evidence).
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Named(Spanned<String>),
    Generic {
        name: Spanned<String>,
        args: Vec<Type>,
        span: Span,
    },
}

impl Type {
    pub fn span(&self) -> Span {
        match self {
            Type::Named(name) => name.span,
            Type::Generic { span, .. } => *span,
        }
    }
}

/// A `struct` field: `name: Type`.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: Spanned<String>,
    pub ty: Type,
    pub span: Span,
}

/// A function parameter: `name: Type [= default]` (`fn_decl`'s `param`
/// production). Distinct from item-level `Item::Param` (the `param`
/// *declaration* keyword) even though both happen to share the same
/// `name: Type [= default]` shape — one is a parameter list entry, the
/// other a top-level/part-level declaration; keeping them as separate
/// types avoids conflating "this is a function parameter" with "this is
/// a part's configurable input" should their shapes ever need to diverge.
#[derive(Debug, Clone, PartialEq)]
pub struct FnParam {
    pub name: Spanned<String>,
    pub ty: Type,
    pub default: Option<Expr>,
    pub span: Span,
}

/// A block-level statement (`statement` in the grammar, restricted to
/// this task's scope — see module doc comment).
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `let name [: Type] = expr;` — immutable local binding.
    Let {
        name: Spanned<String>,
        ty: Option<Type>,
        value: Expr,
        span: Span,
    },
    /// `var name [: Type] = expr;` — rebindable local binding (DL-2:
    /// rebinding is explicit; this declares the binding rebinding will
    /// later target via `Stmt::Assign`).
    Var {
        name: Spanned<String>,
        ty: Option<Type>,
        value: Expr,
        span: Span,
    },
    /// `name = expr;` — explicit rebind of an existing `var` binding
    /// (DL-2). Whether `name` actually refers to a `var` (not a `let`) is
    /// a binding-phase (`AICAD-050`) concern, not the parser's.
    Assign {
        name: Spanned<String>,
        value: Expr,
        span: Span,
    },
    /// `expr;` — an expression evaluated for its side effect, its value
    /// discarded (per DL-2, e.g. `body.cut(hole);` does not rebind
    /// `body`).
    Expr { expr: Expr, span: Span },
    /// `if cond block [else (block | if_stmt)]` — `AICAD-043`. Unlike
    /// expression-position `if` (`cad_ast::Expr::If`), `else` is
    /// optional here, exactly as the grammar's `if_stmt` production
    /// specifies (`["else" , (block | if_stmt)]`).
    If {
        cond: Expr,
        then_branch: Block,
        else_branch: Option<ElseClause>,
        span: Span,
    },
    /// `for name in iterable block`.
    For {
        var: Spanned<String>,
        iterable: Expr,
        body: Block,
        span: Span,
    },
    /// `while cond block`.
    While { cond: Expr, body: Block, span: Span },
    /// `loop block`.
    Loop { body: Block, span: Span },
    /// `match scrutinee { arm* }` used as a statement (value discarded).
    Match {
        scrutinee: Expr,
        arms: Vec<MatchArm>,
        span: Span,
    },
    /// `return [expr];`.
    Return { value: Option<Expr>, span: Span },
    /// `break;` — no value, exactly as the grammar's `break_stmt`
    /// production specifies (`"break" , ";"`); no evidence anywhere
    /// supports a value-carrying `break`.
    Break { span: Span },
    /// `continue;`.
    Continue { span: Span },
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. }
            | Stmt::Var { span, .. }
            | Stmt::Assign { span, .. }
            | Stmt::Expr { span, .. }
            | Stmt::If { span, .. }
            | Stmt::For { span, .. }
            | Stmt::While { span, .. }
            | Stmt::Loop { span, .. }
            | Stmt::Match { span, .. }
            | Stmt::Return { span, .. }
            | Stmt::Break { span }
            | Stmt::Continue { span } => *span,
        }
    }
}

/// `if_stmt`'s `else` arm: either a plain `block`, or another `if_stmt`
/// (an `else if` chain) — exactly the grammar's own
/// `["else" , (block | if_stmt)]`.
#[derive(Debug, Clone, PartialEq)]
pub enum ElseClause {
    Block(Block),
    /// Always constructed from a nested `Stmt::If`.
    If(Box<Stmt>),
}

/// `"{" { statement } "}"` — a plain statement block with no trailing
/// value-producing expression. `block_expr` (which *does* allow a
/// trailing expression as its value) is `AICAD-043`'s own extension of
/// this same brace-delimited shape; kept as a distinct type rather than
/// guessing its shape here.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

/// A top-level (or `part`-body) declaration (`item` in the grammar,
/// restricted to this task's scope — see module doc comment).
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// `let name [: Type] = expr;` at item scope.
    Let {
        name: Spanned<String>,
        ty: Option<Type>,
        value: Expr,
        span: Span,
    },
    /// `const name [: Type] = expr;` at item scope.
    Const {
        name: Spanned<String>,
        ty: Option<Type>,
        value: Expr,
        span: Span,
    },
    /// `param name: Type [= default];` — unlike `let`/`const`, the type
    /// annotation is mandatory (matches the only concrete evidence,
    /// `examples/assemblies/stage0_paper_example.aicad`'s
    /// `param width: Length = 80mm;`, and params are a part's typed
    /// configurable-input surface, where an explicit type is load-
    /// bearing, not optional ergonomics).
    Param {
        name: Spanned<String>,
        ty: Type,
        default: Option<Expr>,
        span: Span,
    },
    /// `["pure"] fn name ["<" type_param { "," type_param } [","] ">"]
    /// (params) [-> Type] block`. `type_params` is empty for an ordinary,
    /// non-generic function (`AICAD-057B`, `project/OWNER_DECISIONS.md
    /// #D17`).
    Fn {
        is_pure: bool,
        name: Spanned<String>,
        type_params: Vec<Spanned<String>>,
        params: Vec<FnParam>,
        return_ty: Option<Type>,
        body: Block,
        span: Span,
    },
    /// `struct name ["<" type_param { "," type_param } [","] ">"]
    /// { field, field, ... }`. `type_params` is empty for an ordinary,
    /// non-generic struct (`AICAD-057B`, `project/OWNER_DECISIONS.md
    /// #D17`).
    Struct {
        name: Spanned<String>,
        type_params: Vec<Spanned<String>>,
        fields: Vec<Field>,
        span: Span,
    },
    /// `enum name ["<" type_param { "," type_param } [","] ">"]
    /// { Variant, Variant, ... }` — unit variants only, see module doc
    /// comment (`AICAD-057C` adds tuple/record payload variants).
    /// `type_params` is empty for an ordinary, non-generic enum
    /// (`AICAD-057B`, `project/OWNER_DECISIONS.md#D17`).
    Enum {
        name: Spanned<String>,
        type_params: Vec<Spanned<String>>,
        variants: Vec<Spanned<String>>,
        span: Span,
    },
    /// `part name { item* }`.
    Part {
        name: Spanned<String>,
        items: Vec<Item>,
        span: Span,
    },
    /// `import path [ "::" "{" name , { "," name } [","] "}" ] ";"` — see
    /// [`ImportPath`]'s doc comment for the two `path` forms.
    /// `names: None` imports the whole module; `Some(names)` is a
    /// selective import of just those symbols (`import
    /// std.fasteners::{ISO4762};`). Resolving `names` against the target
    /// module's actual exported symbols is name-binding's job
    /// (`AICAD-050`+), not this task's — this is a syntactic node only.
    Import {
        path: ImportPath,
        names: Option<Vec<Spanned<String>>>,
        span: Span,
    },
}

impl Item {
    pub fn span(&self) -> Span {
        match self {
            Item::Let { span, .. }
            | Item::Const { span, .. }
            | Item::Param { span, .. }
            | Item::Fn { span, .. }
            | Item::Struct { span, .. }
            | Item::Enum { span, .. }
            | Item::Part { span, .. }
            | Item::Import { span, .. } => *span,
        }
    }
}

/// The target of an `import` declaration (`Item::Import::path`).
///
/// Exactly the two forms `docs/plan/02_LANGUAGE_AND_COMPILER.md` §10
/// evidences and nothing broader (no aliasing/`as`, no glob `*` import,
/// neither of which any plan/RFC section shows):
///
/// ```text
/// import std.fasteners::{ISO4762};   // Package
/// import robotics.cycloidal;         // Package, whole-module
/// import ./housing;                  // Relative
/// ```
///
/// Resolving a `Package` path against an actual package registry/lockfile
/// is out of scope for `AICAD-044` — Stage 2 has no package/dependency
/// system yet (that is `docs/plan/12_PACKAGES_PLUGINS_EXTENSIONS.md`'s
/// job, not scheduled before Stage 5+ per `project/TASKS.yaml`). The
/// module-loader skeleton (`crates/cad-compiler`) resolves `Relative`
/// paths to files on disk (with cyclic-import detection) and records
/// `Package` paths as recognized-but-unresolved external references
/// rather than inventing an unevidenced resolution scheme.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportPath {
    /// `std.fasteners`, `robotics.cycloidal` — a dotted package-namespace
    /// path, at least one segment.
    Package {
        segments: Vec<Spanned<String>>,
        span: Span,
    },
    /// `./housing`, `../lib/housing` — a filesystem path relative to the
    /// importing file's own directory. `up_levels` counts leading `../`
    /// steps (0 for a bare `./...` path); `segments` is the remaining
    /// `/`-separated path, at least one segment, with no file extension
    /// (the loader appends `.aicad`, per `DECISION_LOG.md#DL-4`'s
    /// canonical source extension).
    Relative {
        up_levels: u32,
        segments: Vec<Spanned<String>>,
        span: Span,
    },
}

impl ImportPath {
    pub fn span(&self) -> Span {
        match self {
            ImportPath::Package { span, .. } | ImportPath::Relative { span, .. } => *span,
        }
    }
}

/// A whole parsed source file: `program = { item }`.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}
