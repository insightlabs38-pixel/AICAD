//! Typed HIR node types — the semantic IR `crate::lower::lower_program`
//! produces from a `cad_ast::Program`, per `docs/plan/01_SYSTEM_
//! ARCHITECTURE.md` §5's IR-layer strategy (Source AST -> **Typed HIR** ->
//! Engineering HIR -> ...).
//!
//! ## Why HIR node types are not just re-exported AST types
//!
//! `cad_ast`'s node types are deliberately syntactic — e.g. `cad_ast::
//! Expr::MethodCall` exists specifically because, per that variant's own
//! doc comment, "the parser's contract is to represent exactly what was
//! written." HIR's contract is the opposite: it is the IR every later
//! phase (type checking, execution, Geometry IR lowering) consumes, and
//! `AGENTS.md`'s own HIR invariants describe HIR as no longer
//! surface-syntax-shaped (builder/method-call syntax "already desugared to
//! functional calls," control flow with "unambiguous value semantics").
//! Reusing `cad_ast` types directly would either force HIR to keep
//! syntax-only distinctions no later phase needs (`Expr::Paren`,
//! `MatchArmBody::Expr` vs. `::Block`, `ElseBranch::Block` vs. `::If`), or
//! require mutating `cad_ast` itself to remove them — corrupting the
//! parser's own "represent exactly what was written" contract for every
//! existing AST consumer (the pretty-printer's round-trip test,
//! `crate::binder`). Separate node types let each IR layer keep exactly
//! the distinctions *it* needs.
//!
//! Two small exceptions, reused directly rather than duplicated: `cad_ast::
//! {UnaryOp, BinaryOp}` are already bare operator tags with no
//! syntax-specific payload (no span, no surface-only variant) — nothing
//! about them is AST-specific, so re-exporting is reuse, not a layering
//! violation. `crate::types::HirType` similarly reuses `cad_units::
//! OperandType` rather than a duplicate enum, for the same reason (see its
//! own doc comment).
//!
//! ## Value-semantics unification (AGENTS.md: "control flow must have
//! unambiguous value semantics")
//!
//! `cad_ast` keeps two pairs of near-duplicate types apart purely for
//! syntactic reasons: `item::Block` (statement position, no trailing
//! value) vs. `expr::BlockExpr` (expression position, optional trailing
//! value), and `item::ElseClause`/`expr::ElseBranch` (statement- vs.
//! expression-position `else`). HIR collapses each pair into one type —
//! [`HirBlock`] (`trailing: None` is exactly the statement-position case)
//! and a plain [`HirExpr`] for an `if`/`match` expression's `else` arm
//! (always one of the `HirExpr::Block`/`HirExpr::If` variants this module
//! already has) — so a HIR consumer checks one place, not two, to learn
//! whether a construct produces a value.

use crate::builtins::BuiltinFnId;
use crate::ids::BindingId;
use crate::types::{HirType, HirTypeRef};
use cad_ast::Span;
pub use cad_ast::{BinaryOp, UnaryOp};

/// A callable function's implementation source (`project/DECISION_LOG.md
/// #DL-15`, resolving `project/OWNER_DECISIONS.md#D18`): either ordinary
/// AICAD source (`Aicad`, the only case that existed before this
/// decision), or a compiler/runtime-owned standard function (
/// `RuntimeBuiltin`) whose behavior `cad_runtime::interp::Interpreter`
/// provides natively rather than by executing a `HirBlock`. Both
/// participate in exactly the same ordinary name binding, argument/return
/// type checking, and call-expression semantics — `DL-15`'s own text:
/// "Runtime-backed functions participate in the same ordinary: name
/// resolution; argument checking; type checking; ...; call-expression
/// semantics ... as AICAD-defined functions." A `RuntimeBuiltin` is
/// deliberately *not* its own `HirExpr`/`BindingKind` variant (no
/// `HirExpr::GeometryCall`/`HirExpr::GeometryIntrinsic` — `DL-15` forbids
/// exactly that) and is not a compiler intrinsic (`project/
/// DECISION_LOG.md#DL-7`'s RFC-gated process is unrelated and unweakened
/// — see [`BuiltinFnId`]'s own module doc comment).
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionImplementation {
    Aicad(HirBlock),
    RuntimeBuiltin(BuiltinFnId),
}

/// A literal value, lowered from `cad_ast::Literal`. Structurally identical
/// today (both are "exactly what the source spelled"), but kept as HIR's
/// own type per this module's doc comment — a future phase that resolves
/// numeric-literal-type defaulting (`AICAD-052`, per `cad_units::
/// arithmetic`'s own documented scope boundary) grows *this* enum, not
/// `cad_ast::Literal`, which must keep representing exactly the source
/// text forever.
#[derive(Debug, Clone, PartialEq)]
pub enum HirLiteral {
    /// `text` is the exact source spelling of the numeral (`"5"`,
    /// `"12.4"`, `"1.5e-3"`); `unit`, if present, is the raw unit-suffix
    /// spelling (`"mm"`, `"Pa"`, ...). Neither is parsed/resolved any
    /// further here — see `crate::lower`'s module doc comment for exactly
    /// what *is* resolved (`HirExpr::Literal::ty`) and why parsing the
    /// magnitude to a concrete numeric representation is left to a later
    /// phase (Engineering HIR, per `docs/plan/01_SYSTEM_ARCHITECTURE.md`
    /// §5) rather than this one.
    Number {
        text: String,
        unit: Option<String>,
    },
    Str(String),
    RawStr(String),
    Bool(bool),
}

/// One call/method-call argument, lowered from `cad_ast::Arg`.
#[derive(Debug, Clone, PartialEq)]
pub enum HirArg {
    Positional(HirExpr),
    Named {
        name: String,
        name_span: Span,
        value: HirExpr,
    },
}

impl HirArg {
    pub fn span(&self) -> Span {
        match self {
            HirArg::Positional(expr) => expr.span(),
            HirArg::Named {
                name_span, value, ..
            } => name_span.join(value.span()),
        }
    }
}

/// A call's callee, after lowering has desugared `cad_ast::Expr::
/// MethodCall` into ordinary call shape (`AGENTS.md` HIR invariant:
/// "source AST builder/method-call syntax must already be desugared to
/// functional calls in HIR"). After lowering there is exactly one
/// call-shaped `HirExpr` variant ([`HirExpr::Call`]) for both an ordinary
/// `callee(args)` and a desugared `receiver.method(args)` — they differ
/// only in how `callee` resolves.
#[derive(Debug, Clone, PartialEq)]
pub enum HirCallee {
    /// `callee(args)` (`cad_ast::Expr::Call`) — a name resolved against
    /// lexical scope, exactly like [`HirExpr::Ident`]. `binding` is `None`
    /// only when lowering could not resolve `name` (see `crate::lower`'s
    /// module doc comment "Unresolved names").
    Fn {
        name: String,
        binding: Option<BindingId>,
        span: Span,
    },
    /// The desugared form of `receiver.method(args)`
    /// (`cad_ast::Expr::MethodCall`). `args[0]` is always the original
    /// receiver expression (lowering prepends it — see `crate::lower`'s
    /// `lower_expr`). `method` is resolved against the receiver's *type*,
    /// never lexical scope (`crates/cad-compiler/src/binder.rs`'s own
    /// established rule, restated in that module's doc comment: "method
    /// name is never checked as a scope name") — a type-checking concern
    /// (`AICAD-052`+), so unlike `Fn` there is no `binding` field here at
    /// all, not merely an always-`None` one.
    Method { name: String, span: Span },
}

impl HirCallee {
    pub fn span(&self) -> Span {
        match self {
            HirCallee::Fn { span, .. } | HirCallee::Method { span, .. } => *span,
        }
    }
}

/// An expression HIR node. Every variant carries its own [`Span`] covering
/// the exact source text it was lowered from (`AGENTS.md`: "preserve
/// source spans through ... lowering").
#[derive(Debug, Clone, PartialEq)]
pub enum HirExpr {
    Literal {
        value: HirLiteral,
        /// The literal's resolved type, when lowering alone can determine
        /// it unambiguously — see `crate::lower`'s module doc comment for
        /// exactly which literals resolve and which are left `None`.
        ty: Option<HirType>,
        span: Span,
    },
    /// A reference to a lexically bound name (`AGENTS.md` HIR invariant:
    /// "lexical binding identity is explicit"). `binding` is `None` only
    /// when lowering could not resolve `name` in scope — see
    /// `crate::lower`'s module doc comment "Unresolved names"; a lowering
    /// diagnostic is always also recorded in that case.
    Ident {
        name: String,
        binding: Option<BindingId>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        operand: Box<HirExpr>,
        span: Span,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<HirExpr>,
        rhs: Box<HirExpr>,
        span: Span,
    },
    /// `callee(args)` or a desugared `receiver.method(args)` — see
    /// [`HirCallee`].
    Call {
        callee: HirCallee,
        args: Vec<HirArg>,
        span: Span,
    },
    /// `receiver.field` — plain field/property read, not a call. Left
    /// unresolved against `receiver`'s type, same rule as `HirCallee::
    /// Method` (`crates/cad-compiler/src/binder.rs`: "field is resolved
    /// against the receiver's type, not a lexical scope").
    Field {
        receiver: Box<HirExpr>,
        field: String,
        span: Span,
    },
    Block(HirBlock),
    /// `if cond then_branch else else_branch`. `else_branch` is always
    /// present (the AST's own `Expr::If` requires it in expression
    /// position) and is always either `HirExpr::Block` or a nested
    /// `HirExpr::If` — see this module's doc comment "Value-semantics
    /// unification" for why `cad_ast::ElseBranch`'s two-variant wrapper
    /// does not survive lowering as its own type.
    If {
        cond: Box<HirExpr>,
        then_branch: HirBlock,
        else_branch: Box<HirExpr>,
        span: Span,
    },
    Match {
        scrutinee: Box<HirExpr>,
        arms: Vec<HirMatchArm>,
        span: Span,
    },
    /// `[e1, e2, ...]` — `project/OWNER_DECISIONS.md#D16`'s owner-approved
    /// list-literal syntax. Whether every element's type actually unifies
    /// to one compatible element type is `cad_hir::typeck`'s job, not
    /// lowering's — mirrors `HirExpr::Match`'s identical division of labor.
    ListLiteral {
        elements: Vec<HirExpr>,
        span: Span,
    },
    /// `start..end` (half-open) or `start..=end` (inclusive) —
    /// `project/OWNER_DECISIONS.md#D16`. Only `Range<Int>`/`Range<UInt>`
    /// are automatically iterable in a `for` loop (`cad_hir::typeck`
    /// enforces this); a `Range` value of any other element type still
    /// type-checks and lowers identically, it is simply not iterable.
    Range {
        start: Box<HirExpr>,
        end: Box<HirExpr>,
        inclusive: bool,
        span: Span,
    },
    /// `Name { field: expr, ... }` — record-variant construction
    /// (`AICAD-057C`, `project/OWNER_DECISIONS.md#D17`). `name`/`binding`
    /// mirror [`HirCallee::Fn`]'s own resolved-name shape (`binding` is
    /// `None` only when lowering could not resolve `name` — see
    /// `crate::lower`'s module doc comment "Unresolved names"). Kept as
    /// its own `HirExpr` variant rather than folded into `HirExpr::Call`:
    /// a record variant's fields are always named, never positional, so
    /// there is no ordinary argument-list reading to share with `Call`;
    /// keeping it distinct also lets the type checker/runtime reject a
    /// record variant constructed with parens (and vice versa) by
    /// construction, not by re-deriving "was this a brace or paren call"
    /// from an already-collapsed shape.
    RecordLiteral {
        name: String,
        binding: Option<BindingId>,
        fields: Vec<HirRecordField>,
        span: Span,
    },
}

impl HirExpr {
    pub fn span(&self) -> Span {
        match self {
            HirExpr::Literal { span, .. }
            | HirExpr::Ident { span, .. }
            | HirExpr::Unary { span, .. }
            | HirExpr::Binary { span, .. }
            | HirExpr::Call { span, .. }
            | HirExpr::Field { span, .. }
            | HirExpr::If { span, .. }
            | HirExpr::Match { span, .. }
            | HirExpr::ListLiteral { span, .. }
            | HirExpr::Range { span, .. }
            | HirExpr::RecordLiteral { span, .. } => *span,
            HirExpr::Block(block) => block.span,
        }
    }
}

/// One field in an [`HirExpr::RecordLiteral`] (`AICAD-057C`).
#[derive(Debug, Clone, PartialEq)]
pub struct HirRecordField {
    pub name: String,
    pub name_span: Span,
    pub value: HirExpr,
}

/// `"{" { statement } [ trailing_expression ] "}"`, unifying `cad_ast::
/// item::Block` and `cad_ast::expr::BlockExpr` — see this module's doc
/// comment "Value-semantics unification". `trailing: None` is exactly the
/// statement-position case (a plain `cad_ast::item::Block` never has a
/// trailing expression at all, so lowering one always produces
/// `trailing: None`).
#[derive(Debug, Clone, PartialEq)]
pub struct HirBlock {
    pub stmts: Vec<HirStmt>,
    pub trailing: Option<Box<HirExpr>>,
    pub span: Span,
}

/// A statement-position `if`'s `else` arm — `cad_ast::item::ElseClause`'s
/// HIR counterpart. Unlike expression-position `if` (see [`HirExpr::If`]),
/// a plain `HirBlock`/nested `HirStmt::If` pair is kept here (not
/// unified into a single value-producing type) because a *statement* has
/// no value to unify around in the first place — this is exactly the
/// "unambiguous value semantics" distinction this module's doc comment
/// describes: the two `if` shapes stay visibly different types precisely
/// because one produces a value and the other does not.
#[derive(Debug, Clone, PartialEq)]
pub enum HirElseStmt {
    Block(HirBlock),
    /// Always constructed from a nested `HirStmt::If`.
    If(Box<HirStmt>),
}

/// A block-level statement, lowered from `cad_ast::Stmt`.
#[derive(Debug, Clone, PartialEq)]
pub enum HirStmt {
    /// `let name [: Type] = expr;`. `binding` is this statement's own
    /// newly minted [`BindingId`] (a declaration, not a reference).
    Let {
        binding: BindingId,
        name: String,
        ty: Option<HirTypeRef>,
        value: HirExpr,
        span: Span,
    },
    /// `var name [: Type] = expr;`.
    Var {
        binding: BindingId,
        name: String,
        ty: Option<HirTypeRef>,
        value: HirExpr,
        span: Span,
    },
    /// `name = expr;` — explicit rebind (DL-2). `target` is `None` only
    /// when lowering could not resolve `name` — see `crate::lower`'s
    /// module doc comment "Unresolved names". Whether `target` actually
    /// names a `var` (not a `let`) binding is not re-checked here — DL-2
    /// mutability enforcement is `crates/cad-compiler/src/binder.rs`'s
    /// job (`AICAD-050`, already complete), which runs before HIR
    /// lowering in the full pipeline (`docs/plan/02_LANGUAGE_AND_COMPILER.
    /// md` §17: binding is phase 3, HIR lowering is phase 7); see
    /// `crate::lower`'s module doc comment "Scope boundary" for the full
    /// accounting.
    Assign {
        target: Option<BindingId>,
        name: String,
        value: HirExpr,
        span: Span,
    },
    Expr {
        expr: HirExpr,
        span: Span,
    },
    If {
        cond: HirExpr,
        then_branch: HirBlock,
        else_branch: Option<HirElseStmt>,
        span: Span,
    },
    /// `for var in iterable body`. `binding` is `var`'s own newly minted
    /// id, visible only inside `body`.
    For {
        binding: BindingId,
        var: String,
        iterable: HirExpr,
        body: HirBlock,
        span: Span,
    },
    While {
        cond: HirExpr,
        body: HirBlock,
        span: Span,
    },
    Loop {
        body: HirBlock,
        span: Span,
    },
    Match {
        scrutinee: HirExpr,
        arms: Vec<HirMatchArm>,
        span: Span,
    },
    Return {
        value: Option<HirExpr>,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
}

impl HirStmt {
    pub fn span(&self) -> Span {
        match self {
            HirStmt::Let { span, .. }
            | HirStmt::Var { span, .. }
            | HirStmt::Assign { span, .. }
            | HirStmt::Expr { span, .. }
            | HirStmt::If { span, .. }
            | HirStmt::For { span, .. }
            | HirStmt::While { span, .. }
            | HirStmt::Loop { span, .. }
            | HirStmt::Match { span, .. }
            | HirStmt::Return { span, .. }
            | HirStmt::Break { span }
            | HirStmt::Continue { span } => *span,
        }
    }
}

/// A `match` arm pattern, lowered from `cad_ast::expr::Pattern`. Unlike
/// the AST's single ambiguous `Ident` variant, HIR splits the two readings
/// `cad_ast::expr::Pattern`'s own doc comment describes ("binds a name, or
/// matches an enum-variant-shaped name; disambiguating those two readings
/// is a binding-phase concern") into two distinct, unambiguous variants —
/// [`HirPattern::Variant`] (a reference to an existing enum-variant
/// binding, no new identity introduced) and [`HirPattern::Binding`] (a
/// fresh declaration, with its own newly minted [`BindingId`]) — because a
/// semantic IR should never re-encode a question a prior pass already
/// answered.
#[derive(Debug, Clone, PartialEq)]
pub enum HirPattern {
    Wildcard {
        span: Span,
    },
    Literal {
        value: HirLiteral,
        span: Span,
    },
    /// Matches a known enum variant. `variant` is that variant's own
    /// (pre-existing) `BindingId` — a reference, not a declaration.
    Variant {
        name: String,
        variant: BindingId,
        span: Span,
    },
    /// Introduces a fresh per-arm binding, scoped to this arm's body only.
    /// `binding` is this pattern's own newly minted `BindingId`.
    Binding {
        name: String,
        binding: BindingId,
        span: Span,
    },
    /// `Name(p1, p2, ...)` — destructures a tuple-variant's payload
    /// positionally (`AICAD-057C`, `project/OWNER_DECISIONS.md#D17`).
    /// Unlike [`HirPattern::Variant`] (whose bare-identifier shape is
    /// genuinely ambiguous between "match this variant" and "bind a fresh
    /// name" until name resolution runs), `Name(...)`/`Name { ... }`
    /// syntax is never a fresh binding — there is no other legal reading
    /// — so `variant` is `Option<BindingId>` for the same reason
    /// `HirCallee::Fn::binding` is: `None` only when lowering could not
    /// resolve `name` at all (see `crate::lower`'s module doc comment
    /// "Unresolved names"), not a second valid interpretation.
    Tuple {
        name: String,
        variant: Option<BindingId>,
        elems: Vec<HirPattern>,
        span: Span,
    },
    /// `Name { field: p, ... }` — destructures a record-variant's payload
    /// by field name (`AICAD-057C`, `project/OWNER_DECISIONS.md#D17`).
    /// See [`HirPattern::Tuple`]'s own doc comment for why `variant` is
    /// `Option`.
    Record {
        name: String,
        variant: Option<BindingId>,
        fields: Vec<HirRecordPatternField>,
        span: Span,
    },
}

impl HirPattern {
    pub fn span(&self) -> Span {
        match self {
            HirPattern::Wildcard { span }
            | HirPattern::Literal { span, .. }
            | HirPattern::Variant { span, .. }
            | HirPattern::Binding { span, .. }
            | HirPattern::Tuple { span, .. }
            | HirPattern::Record { span, .. } => *span,
        }
    }
}

/// One field in an [`HirPattern::Record`] (`AICAD-057C`), mirroring
/// `cad_ast::expr::RecordPatternField` — `pattern` is always present here,
/// whether the source wrote `field` (`cad-parser` already desugars this
/// shorthand to `Pattern::Ident(field)` at parse time — see that AST
/// type's own doc comment) or an explicit `field: pattern`.
#[derive(Debug, Clone, PartialEq)]
pub struct HirRecordPatternField {
    pub name: String,
    pub pattern: HirPattern,
    pub span: Span,
}

/// `pattern => body`, unifying `cad_ast::expr::MatchArmBody`'s `Expr`/
/// `Block` alternatives into a plain [`HirExpr`] (the block form is
/// already representable as `HirExpr::Block`) — see this module's doc
/// comment "Value-semantics unification".
#[derive(Debug, Clone, PartialEq)]
pub struct HirMatchArm {
    pub pattern: HirPattern,
    pub body: HirExpr,
    pub span: Span,
}

/// A function parameter (`cad_ast::item::FnParam`). `binding` is this
/// parameter's own newly minted id.
#[derive(Debug, Clone, PartialEq)]
pub struct HirParam {
    pub binding: BindingId,
    pub name: String,
    pub ty: HirTypeRef,
    pub default: Option<HirExpr>,
    pub span: Span,
}

/// A `struct` field (`cad_ast::item::Field`). Deliberately carries no
/// `BindingId` — struct fields are not lexically scoped names at all
/// (`crates/cad-compiler/src/binder.rs`'s own module doc comment: field
/// names are "a separate namespace from this module's scope-based
/// value/variant bindings", left to `AICAD-053`'s "structs/enums field ...
/// typing"), so there is no lexical identity for lowering to mint here.
#[derive(Debug, Clone, PartialEq)]
pub struct HirField {
    pub name: String,
    pub ty: HirTypeRef,
    pub span: Span,
}

/// One `enum` variant. `binding` is this variant's own newly minted id
/// (kind `BindingKind::EnumVariant`) — what [`HirPattern::Variant`]'s own
/// `variant` field points back to. `payload` carries this variant's
/// declared shape (`AICAD-057C`, `project/OWNER_DECISIONS.md#D17`) —
/// syntactic type references only, unresolved (mirrors `HirField::ty`'s
/// own `HirTypeRef` convention); resolving them against a possibly-
/// generic enclosing enum's own type parameters is `cad_hir::typeck`'s
/// job (`Checker::active_type_params`/`with_type_params`, the same
/// mechanism `AICAD-057B` already built for struct fields).
#[derive(Debug, Clone, PartialEq)]
pub struct HirEnumVariant {
    pub binding: BindingId,
    pub name: String,
    pub payload: HirVariantPayload,
    pub span: Span,
}

/// The payload shape of one [`HirEnumVariant`] (`AICAD-057C`, `project/
/// OWNER_DECISIONS.md#D17`) — the three shapes the owner's D17 ruling
/// specifies.
#[derive(Debug, Clone, PartialEq)]
pub enum HirVariantPayload {
    Unit,
    Tuple(Vec<HirTypeRef>),
    Record(Vec<HirField>),
}

/// One generic type parameter declared on a `fn`/`struct`/`enum`
/// (`AICAD-057B`, `project/OWNER_DECISIONS.md#D17`). `binding` is this
/// parameter's own newly minted id (kind `BindingKind::TypeParam`) —
/// what a `HirTypeRef::Named` referring to it resolves to inside the
/// declaring item's own field/parameter/return types, via `crate::
/// typeck::Checker`'s `active_type_params` table.
#[derive(Debug, Clone, PartialEq)]
pub struct HirTypeParam {
    pub binding: BindingId,
    pub name: String,
    pub span: Span,
}

/// One name a selective `import ...::{Name}` brings into scope. `binding`
/// is this name's own newly minted id (kind `BindingKind::Import`) —
/// `crates/cad-compiler/src/binder.rs`'s own module doc comment: bound
/// into the importing module's scope "without verifying it actually
/// exists in the target file", unchanged here.
#[derive(Debug, Clone, PartialEq)]
pub struct HirImportedName {
    pub binding: BindingId,
    pub name: String,
    pub span: Span,
}

/// The target of an `import` declaration, lowered from `cad_ast::item::
/// ImportPath` with no further resolution — resolving a `Package` path
/// against a package registry, or a `Relative` path against the file
/// system, is `crate::loader`'s job (already done, for `Relative` paths,
/// by `AICAD-044`), not name-binding's or HIR lowering's.
#[derive(Debug, Clone, PartialEq)]
pub enum HirImportPath {
    Package {
        segments: Vec<String>,
        span: Span,
    },
    Relative {
        up_levels: u32,
        segments: Vec<String>,
        span: Span,
    },
}

/// A top-level (or `part`-body) declaration, lowered from `cad_ast::Item`.
#[derive(Debug, Clone, PartialEq)]
pub enum HirItem {
    Let {
        binding: BindingId,
        name: String,
        ty: Option<HirTypeRef>,
        value: HirExpr,
        span: Span,
    },
    Const {
        binding: BindingId,
        name: String,
        ty: Option<HirTypeRef>,
        value: HirExpr,
        span: Span,
    },
    /// `param name: Type [= default];` — item-level, distinct from
    /// [`HirParam`] (a function's own parameter list entry), matching
    /// `cad_ast::item::Item::Param` vs. `FnParam`'s own kept-distinct
    /// shapes.
    Param {
        binding: BindingId,
        name: String,
        ty: HirTypeRef,
        default: Option<HirExpr>,
        span: Span,
    },
    Fn {
        binding: BindingId,
        name: String,
        is_pure: bool,
        /// Empty for an ordinary, non-generic function (`AICAD-057B`,
        /// `project/OWNER_DECISIONS.md#D17`).
        type_params: Vec<HirTypeParam>,
        params: Vec<HirParam>,
        return_ty: Option<HirTypeRef>,
        body: FunctionImplementation,
        span: Span,
    },
    Struct {
        binding: BindingId,
        name: String,
        /// Empty for an ordinary, non-generic struct (`AICAD-057B`,
        /// `project/OWNER_DECISIONS.md#D17`).
        type_params: Vec<HirTypeParam>,
        fields: Vec<HirField>,
        span: Span,
    },
    Enum {
        binding: BindingId,
        name: String,
        /// Empty for an ordinary, non-generic enum (`AICAD-057B`,
        /// `project/OWNER_DECISIONS.md#D17`).
        type_params: Vec<HirTypeParam>,
        variants: Vec<HirEnumVariant>,
        span: Span,
    },
    Part {
        binding: BindingId,
        name: String,
        items: Vec<HirItem>,
        span: Span,
    },
    /// `names` is empty for a whole-module import (`cad_ast::item::Item::
    /// Import::names: None`) — see `crates/cad-compiler/src/binder.rs`'s
    /// own module doc comment for why a whole-module import binds no
    /// name.
    Import {
        path: HirImportPath,
        names: Vec<HirImportedName>,
        span: Span,
    },
}

impl HirItem {
    pub fn span(&self) -> Span {
        match self {
            HirItem::Let { span, .. }
            | HirItem::Const { span, .. }
            | HirItem::Param { span, .. }
            | HirItem::Fn { span, .. }
            | HirItem::Struct { span, .. }
            | HirItem::Enum { span, .. }
            | HirItem::Part { span, .. }
            | HirItem::Import { span, .. } => *span,
        }
    }
}

/// A whole lowered source file: `cad_ast::Program`'s HIR counterpart.
#[derive(Debug, Clone, PartialEq)]
pub struct HirProgram {
    pub items: Vec<HirItem>,
}
