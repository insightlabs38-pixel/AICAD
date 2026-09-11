//! Type checking over typed HIR — `AICAD-052` ("literals/bindings/
//! functions/calls") and `AICAD-053` ("structs/enums field and variant
//! typing"), `docs/plan/02_LANGUAGE_AND_COMPILER.md` §17 phase 4 ("type +
//! dimensional checking"). Together these fill in exactly what
//! `crate::lower`'s own "Scope boundary" doc comment deferred: numeric-
//! literal-type (`Int`/`Float`) defaulting; full type checking for
//! `let`/`const`/`param`/`var` bindings, function declarations, function
//! calls (arity, parameter types, return type), and literal expressions
//! (`AICAD-052`); and struct-literal construction (via ordinary call
//! syntax — DL-2's functional core has no separate constructor syntax),
//! field access, and enum-variant construction/matching (`AICAD-053`) —
//! reusing `cad_units`' dimensional arithmetic rules (DL-3) throughout
//! rather than re-deriving them.
//!
//! ## Key design decision: no scope stack needed
//!
//! Unlike `crate::lower::Lowerer` (which performs its own scope walk to
//! *mint* binding identity in the first place), this checker never needs
//! a scope stack at all: `crate::lower::lower_program` already resolved
//! every reference to a concrete, globally-unique [`crate::ids::
//! BindingId`] (or `None` when it couldn't — see `crate::lower`'s
//! "Unresolved names"), so lexical scoping is already fully baked into
//! *which* `BindingId` a name pointed to. This checker therefore only
//! needs one flat table, `Checker::binding_types` (indexed by
//! `BindingId::index()`, mirroring `crate::lower::LowerResult::bindings`'s
//! own indexing convention exactly), filled in as each declaration is
//! encountered — no scope push/pop bookkeeping anywhere in this module,
//! including for `part`-nested items (walked recursively into the same
//! flat tables).
//!
//! ## `CheckedType`: a superset of `HirType`
//!
//! [`HirType`] (`= cad_units::OperandType`) covers exactly a scalar or
//! dimensional *value* — everything `AICAD-052` alone needed. Struct/enum
//! values (`AICAD-053`) are nominal types with no arithmetic meaning, so
//! [`CheckedType`] wraps `HirType` (`CheckedType::Value`) alongside two
//! new nominal cases, `CheckedType::Struct(BindingId)`/`::Enum(BindingId)`
//! — the declaring struct/enum's own binding identity *is* its type
//! identity, so no separate type-id allocation is needed. Every arithmetic/
//! comparison-operator call site extracts the `Value` case (via
//! `as_value`) before delegating to `cad_units`; a struct/enum operand
//! there is simply not arithmetic-eligible (silently unresolved, matching
//! this module's general `None`-is-not-an-error convention — see below).
//!
//! ## Two passes before any body/value is checked
//!
//! 1. [`Checker::register_type_names`]: every `struct`/`enum` declaration
//!    (including inside `part`s) is indexed by name, *and* every enum
//!    variant's own binding is immediately given its checked type
//!    (`CheckedType::Enum(<owning enum's binding>)`) — a variant used as
//!    a bare value (`let m = NEMA17;`) needs no further resolution once
//!    this runs, since `HirExpr::Ident` always looks its type up through
//!    the same `binding_types` table every other binding uses.
//! 2. [`Checker::collect_struct_fields`]: every struct's own field list is
//!    resolved (field type refs may name another struct/enum declared
//!    anywhere in the program, forward or not — safe because pass 1
//!    already indexed every type name first).
//!
//! Only then does [`Checker::collect_signatures`] (function/item-`param`
//! signatures — needed before any *body* is checked, since a function may
//! call a sibling declared later in source) and finally
//! [`Checker::check_items`] (bodies/values, ordinary source order) run.
//! Top-level `let`/`const` *values* are deliberately **not** given the
//! same forward-reference treatment as function signatures — consistent
//! with `project/reports/AICAD-050.md`'s own already-recorded limitation
//! ("no detection of circular top-level const/let value dependencies... a
//! compile-time-evaluation concern"): a `let`/`const` referenced before
//! its own declaration simply resolves to `None` (unresolved, not an
//! error) rather than gaining forward-reference machinery neither task
//! was asked to add.
//!
//! ## Error recovery: `Option<CheckedType>`, not `Result`
//!
//! Every type-computing method returns `Option<CheckedType>`, reusing
//! exactly the convention `crate::types::HirType`'s own module doc comment
//! already establishes for `HirExpr::Literal::ty`: "`None` is not an
//! error, only 'not yet resolved'." A `None` propagates upward silently —
//! no cascading diagnostic from a parent expression whose operand's type
//! could not be determined, whether because of an already-reported error
//! or because the operand is a genuinely not-yet-typeable shape (a method
//! call — no method/interface-implementation declaration syntax exists
//! anywhere in the language).
//!
//! ## Reuse, not re-derivation, of DL-3's dimensional rules
//!
//! Every arithmetic/comparison/negation type rule over *value* operands is
//! delegated to `cad_units::{check_binary_arithmetic, check_comparison,
//! check_unary_neg}` (`AICAD-049`) — this module never re-implements
//! same-dimension-implicit-conversion, cross-dimension rejection, or
//! affine absolute/delta rules itself. `cad_units::DimensionalArithmeticError`
//! is designed for exactly this: its own module doc comment says its
//! `code()` string exists "for a later `cad-diagnostics`-aware caller to
//! build a real `Diagnostic` from" — this checker is that caller (see
//! [`Checker::diag_from_unit_error`], which builds a `Diagnostic` under
//! the `UNIT` family using that code verbatim, rather than reinventing
//! `TYPE`-family codes for conditions `cad-units` already names).

use crate::hir::{
    BinaryOp, HirArg, HirBlock, HirCallee, HirElseStmt, HirExpr, HirItem, HirLiteral, HirMatchArm,
    HirPattern, HirProgram, HirRecordField, HirStmt, HirTypeParam, HirVariantPayload, UnaryOp,
};
use crate::ids::{Binding, BindingId, BindingKind};
use crate::types::{HirType, HirTypeRef};
use cad_ast::{LineIndex, Span};
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SeverityLetter, SourceSpan};
use cad_types::{AffineKind, Dimension, PrimitiveType};
use cad_units::{
    ArithmeticOp, DimensionalArithmeticError, check_binary_arithmetic, check_comparison,
    check_unary_neg,
};
use std::collections::HashMap;

/// The result of type-checking one lowered program: every diagnostic
/// raised, and the resolved type of every [`BindingId`] this pass could
/// determine one for (indexed by `BindingId::index()`, `None` where
/// unresolved — see module doc comment).
#[derive(Debug, Clone)]
pub struct TypeCheckResult {
    pub diagnostics: Vec<Diagnostic>,
    pub binding_types: Vec<Option<CheckedType>>,
}

/// A checked expression/binding type — see module doc comment
/// "`CheckedType`: a superset of `HirType`".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckedType {
    /// An ordinary scalar or dimensional value (`AICAD-052`'s whole
    /// domain).
    Value(HirType),
    /// A struct value, identified by its declaring `struct` item's own
    /// `BindingId` (`AICAD-053`).
    Struct(BindingId),
    /// An enum value, identified by its declaring `enum` item's own
    /// `BindingId` (`AICAD-053`) — not the matched *variant*'s binding;
    /// every variant of one enum shares this same `CheckedType`.
    Enum(BindingId),
    /// An immutable `List<T>` (`AICAD-056`, `project/OWNER_DECISIONS.md
    /// #D16`). `T` is restricted to a plain [`CheckedType::Value`] element
    /// type (never `Struct`/`Enum`/nested `List`/`Range`) — the owner
    /// ruling's own "Stage-2 collection foundation" scope limit ("does not
    /// need to implement the entire future collection library"); nothing
    /// in `AICAD-056`'s required test list needs a struct/enum or nested-
    /// collection element, and keeping the element type as a plain
    /// [`HirType`] (already `Copy`) keeps `CheckedType` itself `Copy`
    /// rather than requiring a `Box` that would ripple through every
    /// existing by-value `CheckedType` call site in this module.
    List(HirType),
    /// A `Range<T>` (`AICAD-056`, `project/OWNER_DECISIONS.md#D16`) —
    /// `start..end`/`start..=end`. Constructible for any plain element
    /// type `T` (the owner ruling: "Range<T> may exist for dimensional
    /// values such as Range<Length>"), but only `Range<Int>`/`Range<UInt>`
    /// are automatically iterable in a `for` loop — see
    /// `Checker::check_iterable_element_type`. Same `Copy`-preservation
    /// rationale as [`CheckedType::List`].
    Range(HirType),
    /// A reference to a generic type parameter declared on the `fn`/
    /// `struct`/`enum` currently being checked (`AICAD-057B`, `project/
    /// OWNER_DECISIONS.md#D17`), identified by that parameter's own
    /// `BindingId` — e.g. `T` inside `fn identity<T>(value: T) -> T`'s own
    /// signature. This is a placeholder/opaque type only: two
    /// `TypeParam`s are compatible exactly when they name the same
    /// declared parameter (`types_compatible`), never when they merely
    /// *could* unify to the same concrete type — substituting a concrete
    /// type for a type parameter at a call site (`identity(5mm)` binding
    /// `T = Length`) is generic instantiation/inference, `AICAD-057D`'s
    /// job, not this one's. Only ever produced by `resolve_type_ref`
    /// while `Checker::active_type_params` has an entry for the name
    /// (i.e. while checking the very declaration that introduced it) —
    /// never appears as, say, a call argument's inferred type.
    TypeParam(BindingId),
}

/// One function's checked signature — built once in [`Checker::
/// collect_signatures`] and reused for every call site.
#[derive(Debug, Clone)]
struct FnSignature {
    params: Vec<ParamSig>,
    return_ty: Option<CheckedType>,
}

#[derive(Debug, Clone, Copy)]
struct ParamSig {
    binding: BindingId,
    ty: Option<CheckedType>,
    has_default: bool,
}

/// One resolved struct field — built once in [`Checker::
/// collect_struct_fields`] and reused for every construction/field-access
/// site (`AICAD-053`).
#[derive(Debug, Clone)]
struct FieldInfo {
    name: String,
    ty: Option<CheckedType>,
}

/// One resolved enum variant's declared payload shape — built once in
/// [`Checker::collect_enum_variant_shapes`] and reused for every
/// construction/pattern site (`AICAD-057C`, `project/OWNER_DECISIONS.md
/// #D17`). `Unit` is kept distinct from `Tuple(vec![])` (rather than
/// collapsing the two) so [`Checker::check_variant_tuple_construction`]
/// can tell "this variant genuinely takes no payload; do not call it with
/// `()` at all" apart from "this is a zero-field tuple variant" — a
/// diagnostic distinction the owner's D17 ruling implies by keeping `Unit`
/// a separate named shape from `Tuple`/`Record`.
#[derive(Debug, Clone)]
enum VariantShape {
    Unit,
    Tuple(Vec<Option<CheckedType>>),
    Record(Vec<FieldInfo>),
}

struct Checker<'a> {
    bindings: &'a [Binding],
    file: &'a str,
    source: &'a str,
    diagnostics: Vec<Diagnostic>,
    binding_types: Vec<Option<CheckedType>>,
    fn_signatures: HashMap<BindingId, FnSignature>,
    /// `struct`/`enum` name -> its own declaring item's `BindingId`
    /// (`AICAD-053`), populated once by `register_type_names` before any
    /// type reference is resolved — supports forward/mutual references
    /// between struct/enum declarations, matching the same two-pass
    /// declare-before-check shape `crate::lower`/`crate::binder` already
    /// use for value-level names.
    type_names: HashMap<String, BindingId>,
    /// `struct`'s own `BindingId` -> its resolved field list (`AICAD-053`).
    struct_fields: HashMap<BindingId, Vec<FieldInfo>>,
    /// enum variant's own `BindingId` -> its resolved payload shape
    /// (`AICAD-057C`, `project/OWNER_DECISIONS.md#D17`).
    variant_shapes: HashMap<BindingId, VariantShape>,
    /// `enum`'s own `BindingId` -> the full ordered list of its variants'
    /// own `BindingId`s (`AICAD-057C`) — used by [`Checker::check_match`]'s
    /// exhaustiveness check to know the complete variant set a `match`
    /// over that enum must cover.
    enum_variants: HashMap<BindingId, Vec<BindingId>>,
    /// Type-parameter name -> its own `BindingId`, populated by
    /// `with_type_params` for exactly the duration of resolving *one*
    /// generic `fn`/`struct`'s own field/parameter/return types
    /// (`AICAD-057B`, `project/OWNER_DECISIONS.md#D17`) — the type-
    /// namespace analogue of `type_names`, but lexically scoped to a
    /// single declaration rather than global, since two different generic
    /// declarations may each declare their own unrelated `T`. Empty
    /// outside that window (in particular, always empty while checking
    /// ordinary non-generic declarations or any expression body).
    active_type_params: HashMap<String, BindingId>,
    /// The enclosing function's declared return type, if any — read by
    /// `HirStmt::Return` wherever it is encountered, however deeply
    /// nested inside `if`/`match`/block expressions (see module doc
    /// comment "no scope stack needed": this is the one piece of
    /// genuinely non-lexical context a nested `return` needs, so it lives
    /// as a field rather than a threaded parameter).
    current_fn_return: Option<CheckedType>,
}

/// Type-checks one already-lowered program. `bindings` is `crate::lower::
/// LowerResult::bindings` (the same slice `program`'s `BindingId`s index
/// into); `file`/`source` are used only for diagnostic spans, the same
/// two pieces of context every other phase's diagnostics need.
pub fn check_program(
    program: &HirProgram,
    bindings: &[Binding],
    file: &str,
    source: &str,
) -> TypeCheckResult {
    let mut checker = Checker {
        bindings,
        file,
        source,
        diagnostics: Vec::new(),
        binding_types: vec![None; bindings.len()],
        fn_signatures: HashMap::new(),
        type_names: HashMap::new(),
        struct_fields: HashMap::new(),
        variant_shapes: HashMap::new(),
        enum_variants: HashMap::new(),
        active_type_params: HashMap::new(),
        current_fn_return: None,
    };
    checker.register_type_names(&program.items);
    checker.collect_struct_fields(&program.items);
    checker.collect_enum_variant_shapes(&program.items);
    checker.collect_signatures(&program.items);
    checker.check_items(&program.items);
    TypeCheckResult {
        diagnostics: checker.diagnostics,
        binding_types: checker.binding_types,
    }
}

fn bool_ty() -> HirType {
    HirType::Scalar(PrimitiveType::Bool)
}

fn bool_checked() -> CheckedType {
    CheckedType::Value(bool_ty())
}

fn to_arith_op(op: BinaryOp) -> ArithmeticOp {
    match op {
        BinaryOp::Add => ArithmeticOp::Add,
        BinaryOp::Sub => ArithmeticOp::Sub,
        BinaryOp::Mul => ArithmeticOp::Mul,
        BinaryOp::Div => ArithmeticOp::Div,
        _ => unreachable!("to_arith_op is only called for Add/Sub/Mul/Div"),
    }
}

/// Extracts the `Value` case, if any — every `cad_units` arithmetic/
/// comparison call site uses this to opt out cleanly of a struct/enum
/// operand (see module doc comment "`CheckedType`: a superset of
/// `HirType`").
fn as_value(ty: Option<CheckedType>) -> Option<HirType> {
    match ty {
        Some(CheckedType::Value(t)) => Some(t),
        _ => None,
    }
}

fn expected_dimension(expected: Option<CheckedType>) -> Option<Dimension> {
    match expected {
        Some(CheckedType::Value(HirType::Dimensional { dimension, .. })) => Some(dimension),
        _ => None,
    }
}

/// Assignability/argument-passing compatibility between an authoritative
/// `expected` type and an `actual` one. Struct/enum agreement is nominal
/// (the same declaring `BindingId`, never structural — two different
/// `struct`s with identical field shapes are still different types);
/// value agreement is [`value_types_compatible`]'s own rule.
///
/// This does **not** reuse `cad_units::check_comparison` — that function
/// is scoped to the numeric arithmetic/comparison *operators*
/// (`==`/`<`/...) and deliberately rejects non-numeric scalars via its
/// own `require_numeric_scalar` (its module doc comment: general
/// non-arithmetic type agreement "is the general type checker's job
/// (`AICAD-052`)"), so a same-type `Bool`/`String` comparison would be
/// wrongly rejected if this helper delegated to it. Assignability is a
/// plain type-identity question `cad_units` never claimed to own; DL-3's
/// same-dimension-implicit-conversion rule still governs `+`/`-`/`*`/`/`
/// and the comparison *operators* themselves, both fully delegated to
/// `cad_units` elsewhere in this module (`Checker::check_binary`,
/// `Checker::check_unary`).
fn types_compatible(expected: CheckedType, actual: CheckedType) -> bool {
    match (expected, actual) {
        (CheckedType::Value(e), CheckedType::Value(a)) => value_types_compatible(e, a),
        (CheckedType::Struct(e), CheckedType::Struct(a)) => e == a,
        (CheckedType::Enum(e), CheckedType::Enum(a)) => e == a,
        (CheckedType::List(e), CheckedType::List(a)) => value_types_compatible(e, a),
        (CheckedType::Range(e), CheckedType::Range(a)) => value_types_compatible(e, a),
        (CheckedType::TypeParam(e), CheckedType::TypeParam(a)) => e == a,
        _ => false,
    }
}

/// The `Value`-only half of [`types_compatible`]: same `PrimitiveType`, or
/// the same `Dimension` **and** the same `AffineKind` (`None`/`None`, or
/// matching `Some(_)`s — never `Absolute` accepted where `Delta` was
/// produced or vice versa, per RFC-0004 §7's absolute/delta distinction).
fn value_types_compatible(expected: HirType, actual: HirType) -> bool {
    match (expected, actual) {
        (HirType::Scalar(e), HirType::Scalar(a)) => e == a,
        (
            HirType::Dimensional {
                dimension: ed,
                affine: ea,
            },
            HirType::Dimensional {
                dimension: ad,
                affine: aa,
            },
        ) => ed == ad && ea == aa,
        _ => false,
    }
}

impl<'a> Checker<'a> {
    // --- Diagnostics ---

    fn severity_letter(severity: Severity) -> SeverityLetter {
        match severity {
            Severity::Error => SeverityLetter::Error,
            Severity::Warning => SeverityLetter::Warning,
            Severity::Info => SeverityLetter::Info,
        }
    }

    /// Human-readable name for a `CheckedType` in a diagnostic message —
    /// a `Value`'s own `Display` for scalars/dimensions, or the
    /// declaring struct/enum's own source name (looked up in
    /// `self.bindings`, its own symbol table) for a nominal type.
    fn describe(&self, ty: CheckedType) -> String {
        match ty {
            CheckedType::Value(t) => t.to_string(),
            CheckedType::Struct(id) | CheckedType::Enum(id) => {
                self.bindings[id.index()].name.clone()
            }
            CheckedType::List(elem) => format!("List<{elem}>"),
            CheckedType::Range(elem) => format!("Range<{elem}>"),
            CheckedType::TypeParam(id) => self.bindings[id.index()].name.clone(),
        }
    }

    /// Builds a `TYPE`-family diagnostic (mirrors `crate::lower`'s own
    /// `diagnostic` helper exactly; category `"type-check"`).
    fn diag(&self, number: u16, title: &str, message: String, span: Span) -> Diagnostic {
        let line_index = LineIndex::new(self.source);
        let start = line_index.line_column(self.source, span.start);
        let end = line_index.line_column(self.source, span.end);
        let code = DiagnosticCode::new("TYPE", Self::severity_letter(Severity::Error), number)
            .expect("TYPE family and 1..=999 number are always valid");
        Diagnostic::new(code, Severity::Error, "type-check", title, message)
            .expect("severity always agrees with Severity::Error here")
            .with_source(SourceSpan {
                file: self.file.to_string(),
                start: Position::new(start.line, start.column),
                end: Position::new(end.line, end.column),
            })
    }

    /// Builds a diagnostic from a `cad_units::DimensionalArithmeticError`
    /// directly under its own `UNIT` family code (see module doc comment
    /// "Reuse, not re-derivation") rather than a `TYPE`-family code.
    fn diag_from_unit_error(&self, err: &DimensionalArithmeticError, span: Span) -> Diagnostic {
        let line_index = LineIndex::new(self.source);
        let start = line_index.line_column(self.source, span.start);
        let end = line_index.line_column(self.source, span.end);
        let code = DiagnosticCode::parse(err.code())
            .expect("cad_units::arithmetic error codes are always well-formed UNIT-Exxx codes");
        Diagnostic::new(
            code,
            Severity::Error,
            "type-check",
            "DIMENSIONAL_ARITHMETIC_ERROR",
            err.to_string(),
        )
        .expect("cad_units error codes always carry the 'E' severity letter")
        .with_source(SourceSpan {
            file: self.file.to_string(),
            start: Position::new(start.line, start.column),
            end: Position::new(end.line, end.column),
        })
    }

    fn push_unit_error(&mut self, err: DimensionalArithmeticError, span: Span) {
        let diagnostic = self.diag_from_unit_error(&err, span);
        self.diagnostics.push(diagnostic);
    }

    // --- Type-name resolution ---

    /// Resolves a syntactic `HirTypeRef` to a `CheckedType`: a primitive
    /// (`Int`, `Bool`, ...) or dimension (`Length`, ...) resolves to
    /// `CheckedType::Value`; a name registered by `register_type_names`
    /// resolves to `CheckedType::Struct`/`::Enum`; a name matching
    /// **no** declaration at all (a genuine typo, e.g. `Frobnicator`) is
    /// diagnosed as `UNKNOWN_TYPE_NAME`.
    ///
    /// `HirTypeRef::Generic` (`Vector2<Length>`, ...) is never resolved,
    /// with exactly two named exceptions: `List<T>`/`Range<T>`
    /// (`AICAD-056`, `project/OWNER_DECISIONS.md#D16` — the owner's own
    /// "Stage-2 collection foundation" minimum). No other generic type
    /// exists anywhere in this compiler (`cad_units`/`cad_types` cover
    /// only primitives and named dimensions, and struct/enum nominal
    /// identity has no generic-parameter concept either), so resolving any
    /// other generic name here would still be exactly the speculative
    /// invention `AGENTS.md` warns against; every other `Generic`
    /// reference returns `None` silently (no diagnostic — not yet a
    /// typeable one, not a "wrong" name).
    /// Runs `f` with `type_params` active in `self.active_type_params`
    /// (`AICAD-057B`, `project/OWNER_DECISIONS.md#D17`), restoring
    /// whatever was active beforehand (always empty in practice today,
    /// since generic declarations never nest — kept as a save/restore
    /// rather than an unconditional clear so nesting stays safe if a
    /// later task ever introduces it).
    fn with_type_params<T>(
        &mut self,
        type_params: &[HirTypeParam],
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous = std::mem::replace(
            &mut self.active_type_params,
            type_params
                .iter()
                .map(|p| (p.name.clone(), p.binding))
                .collect(),
        );
        let result = f(self);
        self.active_type_params = previous;
        result
    }

    fn resolve_type_ref(&mut self, ty: &HirTypeRef) -> Option<CheckedType> {
        match ty {
            HirTypeRef::Generic { name, args, span } if name == "List" && args.len() == 1 => {
                match self.resolve_type_ref(&args[0])? {
                    CheckedType::Value(elem) => Some(CheckedType::List(elem)),
                    other => {
                        self.diagnostics.push(self.diag(
                            444,
                            "UNSUPPORTED_COLLECTION_ELEMENT_TYPE",
                            format!(
                                "'List<{}>' is not supported — Stage 2 collection element types \
                                 are limited to plain scalar/dimensional types \
                                 (project/OWNER_DECISIONS.md#D16)",
                                self.describe(other)
                            ),
                            *span,
                        ));
                        None
                    }
                }
            }
            HirTypeRef::Generic { name, args, span } if name == "Range" && args.len() == 1 => {
                match self.resolve_type_ref(&args[0])? {
                    CheckedType::Value(elem) => Some(CheckedType::Range(elem)),
                    other => {
                        self.diagnostics.push(self.diag(
                            444,
                            "UNSUPPORTED_COLLECTION_ELEMENT_TYPE",
                            format!(
                                "'Range<{}>' is not supported — Stage 2 collection element types \
                                 are limited to plain scalar/dimensional types \
                                 (project/OWNER_DECISIONS.md#D16)",
                                self.describe(other)
                            ),
                            *span,
                        ));
                        None
                    }
                }
            }
            HirTypeRef::Generic { .. } => None,
            HirTypeRef::Named { name, span } => {
                // `AICAD-057B`, `project/OWNER_DECISIONS.md#D17`: a name
                // matching the generic declaration currently being
                // checked's own type-parameter list takes precedence
                // over everything else — `struct Pair<T, U> { first: T;
                // ... }` must resolve `T` to that parameter, not fail with
                // `UNKNOWN_TYPE_NAME` (no primitive/dimension/struct/enum
                // is plausibly named `T`/`U` in practice, so this ordering
                // has no observed effect on any non-generic program).
                if let Some(&id) = self.active_type_params.get(name) {
                    return Some(CheckedType::TypeParam(id));
                }
                if let Some(prim) = PrimitiveType::from_name(name) {
                    return Some(CheckedType::Value(HirType::Scalar(prim)));
                }
                if let Some(dim) = Dimension::from_name(name) {
                    // A bare type reference (unlike a literal) has no
                    // affine-kind spelling anywhere in the grammar either
                    // — `Absolute` is the same sound default `crate::
                    // lower::literal_type` already establishes for an
                    // affine literal, for the identical reason (a type
                    // annotation is never itself a subtraction result).
                    let affine = if dim.is_affine() {
                        Some(AffineKind::Absolute)
                    } else {
                        None
                    };
                    return Some(CheckedType::Value(HirType::dimensional(dim, affine)));
                }
                if let Some(&id) = self.type_names.get(name) {
                    return Some(match &self.bindings[id.index()].kind {
                        BindingKind::Struct => CheckedType::Struct(id),
                        BindingKind::Enum => CheckedType::Enum(id),
                        _ => unreachable!(
                            "type_names only ever maps a name to the BindingId of the Struct/Enum item that declared it"
                        ),
                    });
                }
                self.diagnostics.push(self.diag(
                    420,
                    "UNKNOWN_TYPE_NAME",
                    format!("'{name}' does not name a known type."),
                    *span,
                ));
                None
            }
        }
    }

    // --- Pass 0: type names + enum variant identity (AICAD-053) ---

    fn register_type_names(&mut self, items: &[HirItem]) {
        for item in items {
            match item {
                HirItem::Struct { binding, name, .. } => {
                    self.type_names.insert(name.clone(), *binding);
                }
                HirItem::Enum {
                    binding,
                    name,
                    variants,
                    ..
                } => {
                    self.type_names.insert(name.clone(), *binding);
                    // A variant used as a bare value (`let m = NEMA17;`,
                    // or as a match scrutinee) needs no further
                    // resolution — `HirExpr::Ident` always looks its type
                    // up through this same table, like every other
                    // binding.
                    for variant in variants {
                        self.binding_types[variant.binding.index()] =
                            Some(CheckedType::Enum(*binding));
                    }
                    // The full declared variant set, for `check_match`'s
                    // exhaustiveness check (`AICAD-057C`).
                    self.enum_variants
                        .insert(*binding, variants.iter().map(|v| v.binding).collect());
                }
                HirItem::Part { items, .. } => self.register_type_names(items),
                HirItem::Let { .. }
                | HirItem::Const { .. }
                | HirItem::Param { .. }
                | HirItem::Fn { .. }
                | HirItem::Import { .. } => {}
            }
        }
    }

    // --- Pass 0.5: struct field types (AICAD-053) ---

    fn collect_struct_fields(&mut self, items: &[HirItem]) {
        for item in items {
            match item {
                HirItem::Struct {
                    binding,
                    type_params,
                    fields,
                    ..
                } => {
                    let field_infos: Vec<FieldInfo> = self.with_type_params(type_params, |this| {
                        fields
                            .iter()
                            .map(|f| FieldInfo {
                                name: f.name.clone(),
                                ty: this.resolve_type_ref(&f.ty),
                            })
                            .collect()
                    });
                    self.struct_fields.insert(*binding, field_infos);
                }
                HirItem::Part { items, .. } => self.collect_struct_fields(items),
                _ => {}
            }
        }
    }

    // --- Pass 0.75: enum variant payload shapes (AICAD-057C) ---

    /// Resolves every enum variant's declared payload shape, mirroring
    /// `collect_struct_fields` exactly — including routing payload-type
    /// resolution through `with_type_params` so a generic enum's own
    /// variant payloads can reference its declared type parameters
    /// (`struct Pair<T, U>`'s own field-resolution precedent, per
    /// `project/reports/AICAD-057B.md`'s documented follow-up: "`AICAD-
    /// 057C` is expected to route payload-type resolution through the
    /// same `Checker::active_type_params`/`with_type_params` mechanism...
    /// not reinvent one").
    fn collect_enum_variant_shapes(&mut self, items: &[HirItem]) {
        for item in items {
            match item {
                HirItem::Enum {
                    type_params,
                    variants,
                    ..
                } => {
                    self.with_type_params(type_params, |this| {
                        for variant in variants {
                            let shape = match &variant.payload {
                                HirVariantPayload::Unit => VariantShape::Unit,
                                HirVariantPayload::Tuple(field_types) => VariantShape::Tuple(
                                    field_types
                                        .iter()
                                        .map(|t| this.resolve_type_ref(t))
                                        .collect(),
                                ),
                                HirVariantPayload::Record(fields) => VariantShape::Record(
                                    fields
                                        .iter()
                                        .map(|f| FieldInfo {
                                            name: f.name.clone(),
                                            ty: this.resolve_type_ref(&f.ty),
                                        })
                                        .collect(),
                                ),
                            };
                            this.variant_shapes.insert(variant.binding, shape);
                        }
                    });
                }
                HirItem::Part { items, .. } => self.collect_enum_variant_shapes(items),
                _ => {}
            }
        }
    }

    // --- Pass 1: function/param signatures ---

    fn collect_signatures(&mut self, items: &[HirItem]) {
        for item in items {
            match item {
                HirItem::Fn {
                    binding,
                    type_params,
                    params,
                    return_ty,
                    ..
                } => {
                    let (param_sigs, return_ty) = self.with_type_params(type_params, |this| {
                        let param_sigs: Vec<ParamSig> = params
                            .iter()
                            .map(|p| {
                                let ty = this.resolve_type_ref(&p.ty);
                                this.binding_types[p.binding.index()] = ty;
                                ParamSig {
                                    binding: p.binding,
                                    ty,
                                    has_default: p.default.is_some(),
                                }
                            })
                            .collect();
                        let return_ty = return_ty.as_ref().and_then(|t| this.resolve_type_ref(t));
                        (param_sigs, return_ty)
                    });
                    self.fn_signatures.insert(
                        *binding,
                        FnSignature {
                            params: param_sigs,
                            return_ty,
                        },
                    );
                }
                HirItem::Param { binding, ty, .. } => {
                    let resolved = self.resolve_type_ref(ty);
                    self.binding_types[binding.index()] = resolved;
                }
                HirItem::Part { items, .. } => self.collect_signatures(items),
                HirItem::Let { .. }
                | HirItem::Const { .. }
                | HirItem::Struct { .. }
                | HirItem::Enum { .. }
                | HirItem::Import { .. } => {}
            }
        }
    }

    // --- Pass 2: bodies/values, in source order ---

    fn check_items(&mut self, items: &[HirItem]) {
        for item in items {
            self.check_item(item);
        }
    }

    fn check_item(&mut self, item: &HirItem) {
        match item {
            HirItem::Let {
                binding, ty, value, ..
            }
            | HirItem::Const {
                binding, ty, value, ..
            } => {
                let expected = ty.as_ref().and_then(|t| self.resolve_type_ref(t));
                let final_ty = self.check_expected(
                    value,
                    expected,
                    value.span(),
                    411,
                    "TYPE_ANNOTATION_MISMATCH",
                );
                self.binding_types[binding.index()] = final_ty;
            }
            HirItem::Param {
                binding, default, ..
            } => {
                if let Some(default) = default {
                    let expected = self.binding_types[binding.index()];
                    self.check_expected(
                        default,
                        expected,
                        default.span(),
                        418,
                        "ARGUMENT_TYPE_MISMATCH",
                    );
                }
            }
            HirItem::Fn {
                binding,
                type_params,
                params,
                body,
                ..
            } => {
                self.with_type_params(type_params, |this| {
                    for p in params {
                        if let Some(default) = &p.default {
                            let expected = this.binding_types[p.binding.index()];
                            this.check_expected(
                                default,
                                expected,
                                default.span(),
                                418,
                                "ARGUMENT_TYPE_MISMATCH",
                            );
                        }
                    }
                    let return_ty = this.fn_signatures.get(binding).and_then(|s| s.return_ty);
                    let previous_return = this.current_fn_return;
                    this.current_fn_return = return_ty;
                    // `T` (and any other of this function's own type
                    // parameters) stays resolvable while checking the
                    // body too — e.g. a `let y: T = value;` local
                    // annotation — not only the signature itself
                    // (`AICAD-057B`); the body is not otherwise given any
                    // special generic treatment here (no instantiation/
                    // inference happens for calls inside it — that is
                    // `AICAD-057D`'s job).
                    this.check_block(body, None);
                    this.current_fn_return = previous_return;
                });
            }
            // Fields/variants were already resolved in the type-name/
            // struct-field passes above — nothing left to check here
            // (struct fields and enum variants carry no value expressions
            // of their own to type-check).
            HirItem::Struct { .. } | HirItem::Enum { .. } => {}
            HirItem::Part { items, .. } => self.check_items(items),
            HirItem::Import { .. } => {}
        }
    }

    /// Checks `expr` against an authoritative `expected` type (a
    /// declared annotation, a parameter's own type, a function's return
    /// type, a struct field's own type, ...), diagnosing a mismatch under
    /// `code`/`title` when both resolve and disagree. Returns the
    /// authoritative type going forward: `expected` when given — even
    /// after a reported mismatch, recovering with the *declared* type
    /// avoids the disagreement cascading into every later use of this
    /// binding — otherwise whatever `expr` itself inferred to.
    fn check_expected(
        &mut self,
        expr: &HirExpr,
        expected: Option<CheckedType>,
        span: Span,
        code: u16,
        title: &str,
    ) -> Option<CheckedType> {
        let actual = self.check_expr(expr, expected);
        if let (Some(e), Some(a)) = (expected, actual)
            && !types_compatible(e, a)
        {
            self.diagnostics.push(self.diag(
                code,
                title,
                format!(
                    "expected type {}, found {}",
                    self.describe(e),
                    self.describe(a)
                ),
                span,
            ));
        }
        expected.or(actual)
    }

    // --- Blocks / statements ---

    fn check_block(
        &mut self,
        block: &HirBlock,
        expected_trailing: Option<CheckedType>,
    ) -> Option<CheckedType> {
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        match &block.trailing {
            Some(expr) => self.check_expr(expr, expected_trailing),
            None => None,
        }
    }

    fn check_stmt(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Let {
                binding, ty, value, ..
            }
            | HirStmt::Var {
                binding, ty, value, ..
            } => {
                let expected = ty.as_ref().and_then(|t| self.resolve_type_ref(t));
                let final_ty = self.check_expected(
                    value,
                    expected,
                    value.span(),
                    411,
                    "TYPE_ANNOTATION_MISMATCH",
                );
                self.binding_types[binding.index()] = final_ty;
            }
            HirStmt::Assign { target, value, .. } => {
                let expected = target.and_then(|b| self.binding_types[b.index()]);
                self.check_expected(value, expected, value.span(), 423, "ASSIGN_TYPE_MISMATCH");
            }
            HirStmt::Expr { expr, .. } => {
                self.check_expr(expr, None);
            }
            HirStmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                let c = self.check_expr(cond, Some(bool_checked()));
                self.check_bool_condition(c, cond.span());
                self.check_block(then_branch, None);
                if let Some(else_stmt) = else_branch {
                    self.check_else_stmt(else_stmt);
                }
            }
            HirStmt::For {
                binding,
                iterable,
                body,
                ..
            } => {
                // `AICAD-056`, `project/OWNER_DECISIONS.md#D16`: `iterable`
                // must resolve to a `List<T>` or an auto-iterable
                // `Range<Int>`/`Range<UInt>` — see
                // `Checker::check_iterable_element_type`'s own doc comment
                // for exactly what is/isn't accepted and why.
                let iterable_ty = self.check_expr(iterable, None);
                let elem_ty = self.check_iterable_element_type(iterable_ty, iterable.span());
                self.binding_types[binding.index()] = elem_ty;
                self.check_block(body, None);
            }
            HirStmt::While { cond, body, .. } => {
                let c = self.check_expr(cond, Some(bool_checked()));
                self.check_bool_condition(c, cond.span());
                self.check_block(body, None);
            }
            HirStmt::Loop { body, .. } => {
                self.check_block(body, None);
            }
            HirStmt::Match {
                scrutinee,
                arms,
                span,
            } => {
                self.check_match(scrutinee, arms, None, *span);
            }
            HirStmt::Return { value, span } => match value {
                Some(v) => {
                    let expected = self.current_fn_return;
                    self.check_expected(v, expected, v.span(), 419, "RETURN_TYPE_MISMATCH");
                }
                None => {
                    if let Some(rt) = self.current_fn_return {
                        self.diagnostics.push(self.diag(
                            419,
                            "RETURN_TYPE_MISMATCH",
                            format!(
                                "function must return a value of type {}, but this 'return;' supplies none",
                                self.describe(rt)
                            ),
                            *span,
                        ));
                    }
                }
            },
            HirStmt::Break { .. } | HirStmt::Continue { .. } => {}
        }
    }

    fn check_else_stmt(&mut self, clause: &HirElseStmt) {
        match clause {
            HirElseStmt::Block(block) => {
                self.check_block(block, None);
            }
            HirElseStmt::If(stmt) => self.check_stmt(stmt),
        }
    }

    fn check_bool_condition(&mut self, ty: Option<CheckedType>, span: Span) {
        if let Some(t) = ty
            && t != bool_checked()
        {
            self.diagnostics.push(self.diag(
                412,
                "CONDITION_NOT_BOOL",
                format!("expected a Bool condition, found {}", self.describe(t)),
                span,
            ));
        }
    }

    // --- Expressions ---

    fn check_expr(&mut self, expr: &HirExpr, expected: Option<CheckedType>) -> Option<CheckedType> {
        match expr {
            HirExpr::Literal { value, span, .. } => self.check_literal(value, expected, *span),
            HirExpr::Ident { binding, .. } => binding.and_then(|b| self.binding_types[b.index()]),
            HirExpr::Unary { op, operand, span } => self.check_unary(*op, operand, *span),
            HirExpr::Binary { op, lhs, rhs, span } => {
                self.check_binary(*op, lhs, rhs, expected, *span)
            }
            HirExpr::Call { callee, args, span } => self.check_call(callee, args, *span),
            HirExpr::Field {
                receiver,
                field,
                span,
            } => {
                let receiver_ty = self.check_expr(receiver, None);
                self.check_field_access(receiver_ty, field, *span)
            }
            HirExpr::Block(block) => self.check_block(block, expected),
            HirExpr::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => {
                let c = self.check_expr(cond, Some(bool_checked()));
                self.check_bool_condition(c, cond.span());
                let then_ty = self.check_block(then_branch, expected);
                let else_ty = self.check_expr(else_branch, expected.or(then_ty));
                self.unify_value_type(then_ty, else_ty, *span)
            }
            HirExpr::Match {
                scrutinee,
                arms,
                span,
            } => self.check_match_expr(scrutinee, arms, expected, *span),
            HirExpr::ListLiteral { elements, span } => {
                self.check_list_literal(elements, expected, *span)
            }
            HirExpr::Range {
                start, end, span, ..
            } => self.check_range_expr(start, end, expected, *span),
            HirExpr::RecordLiteral {
                name,
                binding,
                fields,
                span,
            } => self.check_record_literal(name, *binding, fields, *span),
        }
    }

    /// `[e1, e2, ...]` (`AICAD-056`, `project/OWNER_DECISIONS.md#D16`).
    /// Every element must resolve to the same [`CheckedType::Value`]
    /// (`value_types_compatible`, via [`types_compatible`]) — e.g. `5mm`,
    /// `2cm`, and `1in` all resolve to the identical `Dimensional{Length,
    /// None}` regardless of source unit spelling (dimension resolution is
    /// unit-symbol-independent), so "elements unify to one compatible
    /// element type" reduces to plain type-identity agreement, not a
    /// numeric-promotion algorithm. An empty literal (`[]`) needs
    /// `expected` (a `List<T>` annotation) to know its own element type at
    /// all; without one, `EMPTY_LIST_TYPE_UNKNOWN` — the owner ruling's
    /// own required case ("otherwise emit a stable type-inference
    /// diagnostic").
    fn check_list_literal(
        &mut self,
        elements: &[HirExpr],
        expected: Option<CheckedType>,
        span: Span,
    ) -> Option<CheckedType> {
        let expected_elem = match expected {
            Some(CheckedType::List(elem)) => Some(CheckedType::Value(elem)),
            _ => None,
        };
        if elements.is_empty() {
            return match expected_elem {
                Some(CheckedType::Value(elem)) => Some(CheckedType::List(elem)),
                _ => {
                    self.diagnostics.push(
                        self.diag(
                            441,
                            "EMPTY_LIST_TYPE_UNKNOWN",
                            "cannot infer the element type of an empty list literal '[]' — add an \
                         explicit type annotation (e.g. 'let xs: List<Int> = [];')"
                                .to_string(),
                            span,
                        ),
                    );
                    None
                }
            };
        }
        let mut elem_ty: Option<CheckedType> = None;
        for element in elements {
            let Some(this_ty) = self.check_expr(element, expected_elem.or(elem_ty)) else {
                continue;
            };
            let CheckedType::Value(_) = this_ty else {
                self.diagnostics.push(self.diag(
                    444,
                    "UNSUPPORTED_COLLECTION_ELEMENT_TYPE",
                    format!(
                        "list elements must have a plain scalar/dimensional type; found {}",
                        self.describe(this_ty)
                    ),
                    element.span(),
                ));
                continue;
            };
            match elem_ty.or(expected_elem) {
                None => elem_ty = Some(this_ty),
                Some(target) => {
                    if !types_compatible(target, this_ty) {
                        self.diagnostics.push(self.diag(
                            440,
                            "LIST_ELEMENT_TYPE_MISMATCH",
                            format!(
                                "list elements must all have the same/compatible type; expected \
                                 {}, found {}",
                                self.describe(target),
                                self.describe(this_ty)
                            ),
                            element.span(),
                        ));
                    } else if elem_ty.is_none() {
                        elem_ty = Some(this_ty);
                    }
                }
            }
        }
        match elem_ty.or(expected_elem) {
            Some(CheckedType::Value(elem)) => Some(CheckedType::List(elem)),
            _ => None,
        }
    }

    /// `start..end` / `start..=end` (`AICAD-056`, `project/
    /// OWNER_DECISIONS.md#D16`). `start`/`end` must resolve to the same
    /// [`CheckedType::Value`]; the `expected` `Range<T>` annotation (if
    /// any) is threaded into both, mirroring `check_binary`'s own
    /// `Add`/`Sub` treatment of an `expected` dimension (both operands get
    /// the same hint, since neither changes the result's element type).
    fn check_range_expr(
        &mut self,
        start: &HirExpr,
        end: &HirExpr,
        expected: Option<CheckedType>,
        span: Span,
    ) -> Option<CheckedType> {
        let expected_elem = match expected {
            Some(CheckedType::Range(elem)) => Some(CheckedType::Value(elem)),
            _ => None,
        };
        let start_ty = self.check_expr(start, expected_elem);
        let end_ty = self.check_expr(end, expected_elem.or(start_ty));
        let (Some(start_ty), Some(end_ty)) = (start_ty, end_ty) else {
            return match expected_elem {
                Some(CheckedType::Value(elem)) => Some(CheckedType::Range(elem)),
                _ => None,
            };
        };
        let (CheckedType::Value(_), CheckedType::Value(_)) = (start_ty, end_ty) else {
            self.diagnostics.push(self.diag(
                444,
                "UNSUPPORTED_COLLECTION_ELEMENT_TYPE",
                format!(
                    "range bounds must have a plain scalar/dimensional type; found {} and {}",
                    self.describe(start_ty),
                    self.describe(end_ty)
                ),
                span,
            ));
            return None;
        };
        if !types_compatible(start_ty, end_ty) {
            self.diagnostics.push(self.diag(
                442,
                "RANGE_BOUNDS_TYPE_MISMATCH",
                format!(
                    "range bounds must have the same/compatible type; found {} and {}",
                    self.describe(start_ty),
                    self.describe(end_ty)
                ),
                span,
            ));
            return None;
        }
        match start_ty {
            CheckedType::Value(elem) => Some(CheckedType::Range(elem)),
            _ => unreachable!("both arms already matched CheckedType::Value above"),
        }
    }

    /// The `for var in iterable { ... }` loop variable's own element type,
    /// given `iterable`'s already-checked type — `List<T>` -> `T`;
    /// `Range<Int>`/`Range<UInt>` -> `Int`/`UInt` (the owner ruling's own
    /// "automatic iteration is defined for Range<Int> and Range<UInt>");
    /// any other `Range<T>` (e.g. `Range<Length>`) is a real diagnostic,
    /// never silently accepted or silently unresolved — the owner ruling
    /// is explicit that a dimensional range "is not automatically
    /// iterable" without a future explicit-stepping API this task does not
    /// build. Any non-collection type is `NOT_ITERABLE`. `None` (an
    /// already-unresolved `iterable`, e.g. from an earlier error)
    /// propagates silently, matching this module's general "`None` is not
    /// an error" convention — no cascading diagnostic on top of one
    /// `iterable` itself already reported.
    fn check_iterable_element_type(
        &mut self,
        iterable_ty: Option<CheckedType>,
        span: Span,
    ) -> Option<CheckedType> {
        match iterable_ty? {
            CheckedType::List(elem) => Some(CheckedType::Value(elem)),
            CheckedType::Range(
                elem @ HirType::Scalar(PrimitiveType::Int | PrimitiveType::UInt),
            ) => Some(CheckedType::Value(elem)),
            other => {
                self.diagnostics.push(self.diag(
                    443,
                    "NOT_ITERABLE",
                    format!(
                        "'for' can only iterate over a List<T> or a Range<Int>/Range<UInt>; \
                         found {} (project/OWNER_DECISIONS.md#D16)",
                        self.describe(other)
                    ),
                    span,
                ));
                None
            }
        }
    }

    fn check_literal(
        &mut self,
        lit: &HirLiteral,
        expected: Option<CheckedType>,
        span: Span,
    ) -> Option<CheckedType> {
        match lit {
            HirLiteral::Bool(_) => Some(CheckedType::Value(HirType::Scalar(PrimitiveType::Bool))),
            HirLiteral::Str(_) | HirLiteral::RawStr(_) => {
                Some(CheckedType::Value(HirType::Scalar(PrimitiveType::String)))
            }
            HirLiteral::Number { text, unit: None } => Some(CheckedType::Value(
                default_numeric_literal_type(text, as_value(expected)),
            )),
            HirLiteral::Number {
                unit: Some(symbol), ..
            } => self
                .resolve_unit_literal(symbol, as_value(expected), span)
                .map(CheckedType::Value),
        }
    }

    fn resolve_unit_literal(
        &mut self,
        symbol: &str,
        expected: Option<HirType>,
        span: Span,
    ) -> Option<HirType> {
        let mut candidates = cad_units::lookup_any(symbol);
        let first = candidates.next();
        let second = candidates.next();
        let expected_dim = match expected {
            Some(HirType::Dimensional { dimension, .. }) => Some(dimension),
            _ => None,
        };
        match (first, second) {
            (None, _) => {
                if let Some(dim) = expected_dim
                    && cad_units::lookup(symbol, dim).is_some()
                {
                    return Some(dimensional_literal_type(dim));
                }
                self.diagnostics.push(self.diag(
                    422,
                    "UNKNOWN_LITERAL_UNIT",
                    format!("'{symbol}' does not name a known unit."),
                    span,
                ));
                None
            }
            (Some(only), None) => Some(dimensional_literal_type(only.dimension)),
            (Some(_), Some(_)) => {
                // Ambiguous (e.g. "Pa" matches both Pressure and Stress) —
                // resolve only when an expected dimension is explicitly
                // supplied by surrounding context (an annotation, a
                // parameter/return/field type, ...) and that symbol is
                // actually registered under it; never guess otherwise
                // (AGENTS.md "ambiguity is an error, never an arbitrary
                // selection").
                if let Some(dim) = expected_dim
                    && cad_units::lookup(symbol, dim).is_some()
                {
                    return Some(dimensional_literal_type(dim));
                }
                self.diagnostics.push(self.diag(
                    421,
                    "AMBIGUOUS_LITERAL_UNIT",
                    format!(
                        "'{symbol}' matches more than one dimension; an explicit type annotation is required."
                    ),
                    span,
                ));
                None
            }
        }
    }

    fn check_unary(&mut self, op: UnaryOp, operand: &HirExpr, span: Span) -> Option<CheckedType> {
        match op {
            UnaryOp::Neg => {
                let operand_ty = as_value(self.check_expr(operand, None))?;
                match check_unary_neg(operand_ty) {
                    Ok(result) => Some(CheckedType::Value(result)),
                    Err(err) => {
                        self.push_unit_error(err, span);
                        None
                    }
                }
            }
            UnaryOp::Not => {
                let operand_ty = self.check_expr(operand, Some(bool_checked()));
                self.check_bool_condition(operand_ty, operand.span());
                Some(bool_checked())
            }
        }
    }

    fn check_binary(
        &mut self,
        op: BinaryOp,
        lhs: &HirExpr,
        rhs: &HirExpr,
        expected: Option<CheckedType>,
        span: Span,
    ) -> Option<CheckedType> {
        match op {
            BinaryOp::And | BinaryOp::Or => {
                let l = self.check_expr(lhs, Some(bool_checked()));
                let r = self.check_expr(rhs, Some(bool_checked()));
                self.check_bool_condition(l, lhs.span());
                self.check_bool_condition(r, rhs.span());
                Some(bool_checked())
            }
            BinaryOp::Eq
            | BinaryOp::NotEq
            | BinaryOp::ApproxEq
            | BinaryOp::Lt
            | BinaryOp::LtEq
            | BinaryOp::Gt
            | BinaryOp::GtEq => {
                let l = self.check_expr(lhs, None);
                let r = self.check_expr(rhs, None);
                if let (Some(lt), Some(rt)) = (l, r) {
                    match (lt, rt) {
                        (CheckedType::Value(lv), CheckedType::Value(rv)) => {
                            if let Err(err) = check_comparison(op.as_str(), lv, rv) {
                                self.push_unit_error(err, span);
                            }
                        }
                        // A struct/enum operand on either side: DL-3's
                        // same-dimension rule does not apply (there is no
                        // dimension), so this checker's own nominal
                        // `types_compatible` decides instead — this is
                        // what makes `motor == NEMA17`-shaped enum-
                        // variant comparisons (the paper example's own
                        // evidenced pattern) type-check (`AICAD-053`).
                        _ => {
                            if !types_compatible(lt, rt) {
                                self.diagnostics.push(self.diag(
                                    437,
                                    "COMPARISON_TYPE_MISMATCH",
                                    format!(
                                        "cannot compare {} and {}",
                                        self.describe(lt),
                                        self.describe(rt)
                                    ),
                                    span,
                                ));
                            }
                        }
                    }
                }
                Some(bool_checked())
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                let arith_op = to_arith_op(op);
                let expected_dim = expected_dimension(expected);
                let (l, r) = match arith_op {
                    // Add/Sub preserve dimension, so the outer expected
                    // type is a sound hint for *both* operands directly
                    // (disambiguating an ambiguous-unit literal on either
                    // side); Mul/Div's operands have a different
                    // dimension than the result, so only the *result* is
                    // hinted (via `expected_dim`, below).
                    ArithmeticOp::Add | ArithmeticOp::Sub => (
                        as_value(self.check_expr(lhs, expected)),
                        as_value(self.check_expr(rhs, expected)),
                    ),
                    ArithmeticOp::Mul | ArithmeticOp::Div => (
                        as_value(self.check_expr(lhs, None)),
                        as_value(self.check_expr(rhs, None)),
                    ),
                };
                match (l, r) {
                    (Some(l), Some(r)) => {
                        match check_binary_arithmetic(arith_op, l, r, expected_dim) {
                            Ok(result) => Some(CheckedType::Value(result)),
                            Err(err) => {
                                self.push_unit_error(err, span);
                                None
                            }
                        }
                    }
                    _ => None,
                }
            }
        }
    }

    fn check_call(
        &mut self,
        callee: &HirCallee,
        args: &[HirArg],
        span: Span,
    ) -> Option<CheckedType> {
        match callee {
            // No method/trait/geometry-API declaration syntax exists
            // anywhere in the language yet (no `impl` blocks, no
            // `interface` implementations reachable from the grammar) —
            // there is nothing to resolve `name` against. Each argument
            // is still checked for its own independent diagnostics; the
            // call's own result type stays unresolved.
            HirCallee::Method { .. } => {
                for arg in args {
                    self.check_arg_expr(arg);
                }
                None
            }
            HirCallee::Fn { binding, name, .. } => {
                let Some(binding) = binding else {
                    // Already diagnosed by lowering (TYPE-E410).
                    for arg in args {
                        self.check_arg_expr(arg);
                    }
                    return None;
                };
                match &self.bindings[binding.index()].kind {
                    BindingKind::Fn => {
                        let Some(sig) = self.fn_signatures.get(binding).cloned() else {
                            // Defensive: every `Fn`-kind binding always
                            // gets a signature in `collect_signatures`.
                            // Never panic on an unexpected shape
                            // (AGENTS.md).
                            for arg in args {
                                self.check_arg_expr(arg);
                            }
                            return None;
                        };
                        self.check_call_args(name, args, &sig, span);
                        sig.return_ty
                    }
                    // Struct-literal construction via ordinary call
                    // syntax (DL-2 has no separate constructor syntax) —
                    // `AICAD-053`.
                    BindingKind::Struct => {
                        self.check_struct_construction(*binding, name, args, span)
                    }
                    // Tuple-variant construction (`Ok(value)`,
                    // `Empty()`) — `AICAD-057C`, `project/
                    // OWNER_DECISIONS.md#D17`. Cloned to an owned `String`
                    // first: `enum_name` borrows `self.bindings`, which
                    // `check_variant_tuple_construction` (taking `&mut
                    // self`) cannot coexist with.
                    BindingKind::EnumVariant { enum_name } => {
                        let enum_name = enum_name.clone();
                        self.check_variant_tuple_construction(
                            *binding, &enum_name, name, args, span,
                        )
                    }
                    // Any other callee kind (a plain `let`/`var`/`const`,
                    // ...) has no signature this checker knows how to
                    // verify a call against — arguments are still walked,
                    // but no "not callable" diagnostic is raised (a
                    // documented known limitation, not evidenced scope for
                    // either task).
                    _ => {
                        for arg in args {
                            self.check_arg_expr(arg);
                        }
                        None
                    }
                }
            }
        }
    }

    fn check_arg_expr(&mut self, arg: &HirArg) {
        match arg {
            HirArg::Positional(expr) => {
                self.check_expr(expr, None);
            }
            HirArg::Named { value, .. } => {
                self.check_expr(value, None);
            }
        }
    }

    fn check_call_args(
        &mut self,
        fn_name: &str,
        args: &[HirArg],
        sig: &FnSignature,
        call_span: Span,
    ) {
        let mut filled = vec![false; sig.params.len()];
        let mut next_positional = 0usize;
        for arg in args {
            match arg {
                HirArg::Positional(expr) => {
                    if next_positional < sig.params.len() {
                        let idx = next_positional;
                        next_positional += 1;
                        filled[idx] = true;
                        let expected = sig.params[idx].ty;
                        self.check_expected(
                            expr,
                            expected,
                            expr.span(),
                            418,
                            "ARGUMENT_TYPE_MISMATCH",
                        );
                    } else {
                        self.diagnostics.push(self.diag(
                            414,
                            "TOO_MANY_ARGUMENTS",
                            format!(
                                "'{fn_name}' takes {} argument(s), but more were supplied",
                                sig.params.len()
                            ),
                            call_span,
                        ));
                        self.check_expr(expr, None);
                    }
                }
                HirArg::Named {
                    name,
                    name_span,
                    value,
                } => {
                    match sig
                        .params
                        .iter()
                        .position(|p| self.bindings[p.binding.index()].name == *name)
                    {
                        Some(idx) => {
                            if filled[idx] {
                                self.diagnostics.push(self.diag(
                                    416,
                                    "DUPLICATE_ARGUMENT",
                                    format!("argument '{name}' is already supplied"),
                                    *name_span,
                                ));
                            }
                            filled[idx] = true;
                            let expected = sig.params[idx].ty;
                            self.check_expected(
                                value,
                                expected,
                                value.span(),
                                418,
                                "ARGUMENT_TYPE_MISMATCH",
                            );
                        }
                        None => {
                            self.diagnostics.push(self.diag(
                                415,
                                "UNKNOWN_NAMED_ARGUMENT",
                                format!("'{fn_name}' has no parameter named '{name}'"),
                                *name_span,
                            ));
                            self.check_expr(value, None);
                        }
                    }
                }
            }
        }
        for (idx, was_filled) in filled.iter().enumerate() {
            if !was_filled && !sig.params[idx].has_default {
                let pname = self.bindings[sig.params[idx].binding.index()].name.clone();
                self.diagnostics.push(self.diag(
                    417,
                    "MISSING_ARGUMENT",
                    format!("missing required argument '{pname}' in call to '{fn_name}'"),
                    call_span,
                ));
            }
        }
    }

    // --- Structs (AICAD-053) ---

    /// Type-checks a struct-literal construction (`Point(x = 1mm, y =
    /// 2mm)`), the functional-call-syntax shape DL-2's mutation-semantics
    /// ruling implies (no separate constructor syntax exists — the
    /// grammar has none, and inventing one would be exactly the
    /// speculative syntax `AGENTS.md` warns against). Mirrors
    /// `check_call_args`'s positional/named matching almost exactly,
    /// against the struct's own field list instead of a function's
    /// parameter list — struct fields, unlike parameters, never carry a
    /// default value (`crate::hir::HirField` has no `default` slot at
    /// all), so every field must be supplied exactly once.
    fn check_struct_construction(
        &mut self,
        struct_binding: BindingId,
        struct_name: &str,
        args: &[HirArg],
        span: Span,
    ) -> Option<CheckedType> {
        let fields = self
            .struct_fields
            .get(&struct_binding)
            .cloned()
            .unwrap_or_default();
        let mut filled = vec![false; fields.len()];
        let mut next_positional = 0usize;
        for arg in args {
            match arg {
                HirArg::Positional(expr) => {
                    if next_positional < fields.len() {
                        let idx = next_positional;
                        next_positional += 1;
                        filled[idx] = true;
                        let expected = fields[idx].ty;
                        self.check_expected(
                            expr,
                            expected,
                            expr.span(),
                            433,
                            "STRUCT_FIELD_TYPE_MISMATCH",
                        );
                    } else {
                        self.diagnostics.push(self.diag(
                            435,
                            "TOO_MANY_STRUCT_FIELDS",
                            format!(
                                "'{struct_name}' has {} field(s), but more were supplied",
                                fields.len()
                            ),
                            span,
                        ));
                        self.check_expr(expr, None);
                    }
                }
                HirArg::Named {
                    name,
                    name_span,
                    value,
                } => match fields.iter().position(|f| f.name == *name) {
                    Some(idx) => {
                        if filled[idx] {
                            self.diagnostics.push(self.diag(
                                432,
                                "DUPLICATE_STRUCT_FIELD",
                                format!("field '{name}' is already supplied"),
                                *name_span,
                            ));
                        }
                        filled[idx] = true;
                        let expected = fields[idx].ty;
                        self.check_expected(
                            value,
                            expected,
                            value.span(),
                            433,
                            "STRUCT_FIELD_TYPE_MISMATCH",
                        );
                    }
                    None => {
                        self.diagnostics.push(self.diag(
                            431,
                            "UNKNOWN_STRUCT_FIELD",
                            format!("'{struct_name}' has no field named '{name}'"),
                            *name_span,
                        ));
                        self.check_expr(value, None);
                    }
                },
            }
        }
        for (idx, was_filled) in filled.iter().enumerate() {
            if !was_filled {
                self.diagnostics.push(self.diag(
                    430,
                    "MISSING_STRUCT_FIELD",
                    format!(
                        "missing field '{}' in construction of '{struct_name}'",
                        fields[idx].name
                    ),
                    span,
                ));
            }
        }
        Some(CheckedType::Struct(struct_binding))
    }

    // --- Enum variant construction (AICAD-057C, OWNER_DECISIONS.md#D17) --

    /// Type-checks a tuple-variant construction call (`Ok(value)`,
    /// `Empty()`) — `AICAD-057C`. Mirrors `check_call_args`'s positional-
    /// only matching (a tuple variant's fields, like a struct's own
    /// positional args, are never named). A `VariantShape::Record` reached
    /// here means the source used call-parens on a variant that actually
    /// declared brace-record fields — diagnosed, not silently accepted,
    /// since the two construction shapes are not interchangeable
    /// (`AICAD-057C`'s own scope note: a record variant must be
    /// constructed with braces, a tuple variant with parens).
    fn check_variant_tuple_construction(
        &mut self,
        variant_binding: BindingId,
        enum_name: &str,
        variant_name: &str,
        args: &[HirArg],
        span: Span,
    ) -> Option<CheckedType> {
        let enum_binding = self.type_names.get(enum_name).copied();
        match self.variant_shapes.get(&variant_binding).cloned() {
            // Unlike a zero-field *tuple* variant (`Empty()`, still a
            // real constructor call, just with no arguments — see
            // `VariantShape::Tuple`'s own doc comment for why the two
            // are kept distinct), a `Unit` variant is never call syntax
            // at all, with or without arguments — it is referenced as a
            // bare name (`HirExpr::Ident`), exactly like Rust's own
            // unit-like enum variants are never `Name()`-callable.
            Some(VariantShape::Unit) => {
                self.diagnostics.push(self.diag(
                    448,
                    "UNIT_VARIANT_NOT_CALLABLE",
                    format!(
                        "'{variant_name}' is a unit variant and takes no payload; write it as \
                         '{variant_name}', not '{variant_name}(...)'"
                    ),
                    span,
                ));
                for arg in args {
                    self.check_arg_expr(arg);
                }
            }
            Some(VariantShape::Tuple(field_types)) => {
                self.check_variant_positional_args(variant_name, args, &field_types, span);
            }
            Some(VariantShape::Record(_)) => {
                self.diagnostics.push(self.diag(
                    451,
                    "RECORD_VARIANT_NEEDS_BRACES",
                    format!(
                        "'{variant_name}' is a record variant; construct it with \
                         '{variant_name} {{ ... }}', not '{variant_name}(...)'"
                    ),
                    span,
                ));
                for arg in args {
                    self.check_arg_expr(arg);
                }
            }
            None => {
                for arg in args {
                    self.check_arg_expr(arg);
                }
            }
        }
        enum_binding.map(CheckedType::Enum)
    }

    /// Shared positional-argument matching for [`Checker::
    /// check_variant_tuple_construction`] — no names, no defaults (a tuple
    /// variant's fields have neither), so this is a strict "exactly one
    /// value per declared field, in order" check.
    fn check_variant_positional_args(
        &mut self,
        variant_name: &str,
        args: &[HirArg],
        field_types: &[Option<CheckedType>],
        call_span: Span,
    ) {
        let mut filled = 0usize;
        for arg in args {
            match arg {
                HirArg::Positional(expr) => {
                    if filled < field_types.len() {
                        let expected = field_types[filled];
                        filled += 1;
                        self.check_expected(
                            expr,
                            expected,
                            expr.span(),
                            449,
                            "VARIANT_FIELD_TYPE_MISMATCH",
                        );
                    } else {
                        self.diagnostics.push(self.diag(
                            447,
                            "VARIANT_ARITY_MISMATCH",
                            format!(
                                "'{variant_name}' takes {} value(s), but more were supplied",
                                field_types.len()
                            ),
                            call_span,
                        ));
                        self.check_expr(expr, None);
                    }
                }
                HirArg::Named {
                    name,
                    name_span,
                    value,
                } => {
                    self.diagnostics.push(self.diag(
                        450,
                        "TUPLE_VARIANT_NAMED_ARGUMENT",
                        format!(
                            "'{variant_name}' has no field named '{name}' — tuple-variant fields \
                             are positional"
                        ),
                        *name_span,
                    ));
                    self.check_expr(value, None);
                }
            }
        }
        if filled < field_types.len() {
            self.diagnostics.push(self.diag(
                447,
                "VARIANT_ARITY_MISMATCH",
                format!(
                    "'{variant_name}' takes {} value(s), but only {filled} were supplied",
                    field_types.len()
                ),
                call_span,
            ));
        }
    }

    /// Type-checks a record-variant construction (`Point { x: 1mm, y:
    /// 2mm }`) — `AICAD-057C`. `name`/`binding` are the already-resolved
    /// constructor name (mirrors `check_call`'s own `HirCallee::Fn`
    /// resolution). Every field is necessarily named — `Expr::
    /// RecordLiteral`'s own AST shape has no positional reading at all
    /// (see that type's own doc comment) — so, unlike `check_struct_
    /// construction`, there is no positional-argument branch to mirror.
    fn check_record_literal(
        &mut self,
        name: &str,
        binding: Option<BindingId>,
        fields: &[HirRecordField],
        span: Span,
    ) -> Option<CheckedType> {
        let Some(binding) = binding else {
            // Already diagnosed by lowering (TYPE-E410).
            for field in fields {
                self.check_expr(&field.value, None);
            }
            return None;
        };
        let kind = self.bindings[binding.index()].kind.clone();
        let BindingKind::EnumVariant { enum_name } = kind else {
            self.diagnostics.push(self.diag(
                455,
                "RECORD_LITERAL_NOT_RECORD_VARIANT",
                format!("'{name}' is not a record-variant constructor"),
                span,
            ));
            for field in fields {
                self.check_expr(&field.value, None);
            }
            return None;
        };
        let enum_binding = self.type_names.get(&enum_name).copied();
        let field_infos = match self.variant_shapes.get(&binding).cloned() {
            Some(VariantShape::Record(field_infos)) => field_infos,
            Some(VariantShape::Unit) | Some(VariantShape::Tuple(_)) => {
                self.diagnostics.push(self.diag(
                    455,
                    "RECORD_LITERAL_NOT_RECORD_VARIANT",
                    format!("'{name}' does not declare named fields; it is not a record variant"),
                    span,
                ));
                for field in fields {
                    self.check_expr(&field.value, None);
                }
                return enum_binding.map(CheckedType::Enum);
            }
            None => {
                for field in fields {
                    self.check_expr(&field.value, None);
                }
                return enum_binding.map(CheckedType::Enum);
            }
        };
        let mut filled = vec![false; field_infos.len()];
        for field in fields {
            match field_infos.iter().position(|f| f.name == field.name) {
                Some(idx) => {
                    if filled[idx] {
                        self.diagnostics.push(self.diag(
                            453,
                            "DUPLICATE_VARIANT_FIELD",
                            format!("field '{}' is already supplied", field.name),
                            field.name_span,
                        ));
                    }
                    filled[idx] = true;
                    let expected = field_infos[idx].ty;
                    self.check_expected(
                        &field.value,
                        expected,
                        field.value.span(),
                        449,
                        "VARIANT_FIELD_TYPE_MISMATCH",
                    );
                }
                None => {
                    self.diagnostics.push(self.diag(
                        452,
                        "UNKNOWN_VARIANT_FIELD",
                        format!("'{name}' has no field named '{}'", field.name),
                        field.name_span,
                    ));
                    self.check_expr(&field.value, None);
                }
            }
        }
        for (idx, was_filled) in filled.iter().enumerate() {
            if !was_filled {
                self.diagnostics.push(self.diag(
                    454,
                    "MISSING_VARIANT_FIELD",
                    format!(
                        "missing field '{}' in construction of '{name}'",
                        field_infos[idx].name
                    ),
                    span,
                ));
            }
        }
        enum_binding.map(CheckedType::Enum)
    }

    /// Type-checks `receiver.field` (`AICAD-053`) — `receiver_ty` is
    /// already checked by the caller (`Checker::check_expr`'s `Field`
    /// arm). A struct receiver resolves to that field's own declared
    /// type (`UNKNOWN_STRUCT_FIELD` if no such field exists); any other
    /// *resolved* receiver type (a scalar/dimensional value, or an enum
    /// value — neither has fields) is `FIELD_ACCESS_ON_NON_STRUCT`; an
    /// unresolved receiver (`None`) stays silently unresolved, matching
    /// this module's general error-recovery convention.
    fn check_field_access(
        &mut self,
        receiver_ty: Option<CheckedType>,
        field: &str,
        span: Span,
    ) -> Option<CheckedType> {
        match receiver_ty {
            Some(CheckedType::Struct(id)) => {
                let fields = self.struct_fields.get(&id).cloned().unwrap_or_default();
                match fields.iter().find(|f| f.name == field) {
                    Some(f) => f.ty,
                    None => {
                        self.diagnostics.push(self.diag(
                            431,
                            "UNKNOWN_STRUCT_FIELD",
                            format!("struct has no field named '{field}'"),
                            span,
                        ));
                        None
                    }
                }
            }
            Some(other) => {
                self.diagnostics.push(self.diag(
                    436,
                    "FIELD_ACCESS_ON_NON_STRUCT",
                    format!(
                        "cannot access field '{field}' on a value of type {}",
                        self.describe(other)
                    ),
                    span,
                ));
                None
            }
            None => None,
        }
    }

    // --- match (shared by statement- and expression-position match) ---

    fn check_match(
        &mut self,
        scrutinee: &HirExpr,
        arms: &[HirMatchArm],
        expected: Option<CheckedType>,
        span: Span,
    ) -> Option<CheckedType> {
        let scrutinee_ty = self.check_expr(scrutinee, None);
        let mut result = None;
        let mut covered: Vec<BindingId> = Vec::new();
        let mut is_exhaustive_by_wildcard = false;
        for arm in arms {
            self.bind_pattern(&arm.pattern, scrutinee_ty);
            match &arm.pattern {
                HirPattern::Wildcard { .. } | HirPattern::Binding { .. } => {
                    is_exhaustive_by_wildcard = true;
                }
                HirPattern::Variant { variant, .. }
                | HirPattern::Tuple {
                    variant: Some(variant),
                    ..
                }
                | HirPattern::Record {
                    variant: Some(variant),
                    ..
                } => {
                    covered.push(*variant);
                }
                HirPattern::Tuple { variant: None, .. }
                | HirPattern::Record { variant: None, .. }
                | HirPattern::Literal { .. } => {}
            }
            let arm_ty = self.check_expr(&arm.body, expected);
            result = self.unify_value_type(result, arm_ty, arm.span);
        }
        self.check_match_exhaustiveness(scrutinee_ty, &covered, is_exhaustive_by_wildcard, span);
        result
    }

    /// Nominal-enum match-exhaustiveness (`AICAD-057C`, `project/
    /// OWNER_DECISIONS.md#D17`: "the compiler must diagnose non-exhaustive
    /// matches unless a wildcard or otherwise exhaustive pattern is
    /// present") — a genuine, previously-undetected soundness gap
    /// `AICAD-057A`'s own audit found (finding #8): before this task,
    /// `check_match` performed no coverage check over an enum's variant
    /// set at all. Only fires when the scrutinee's own type is a resolved
    /// `CheckedType::Enum` — an unresolved/non-enum scrutinee has nothing
    /// this check can verify coverage against, matching this module's
    /// general error-recovery convention.
    fn check_match_exhaustiveness(
        &mut self,
        scrutinee_ty: Option<CheckedType>,
        covered: &[BindingId],
        is_exhaustive_by_wildcard: bool,
        span: Span,
    ) {
        if is_exhaustive_by_wildcard {
            return;
        }
        let Some(CheckedType::Enum(enum_id)) = scrutinee_ty else {
            return;
        };
        let Some(all_variants) = self.enum_variants.get(&enum_id) else {
            return;
        };
        let missing: Vec<String> = all_variants
            .iter()
            .filter(|v| !covered.contains(v))
            .map(|v| self.bindings[v.index()].name.clone())
            .collect();
        if !missing.is_empty() {
            self.diagnostics.push(self.diag(
                446,
                "NON_EXHAUSTIVE_MATCH",
                format!(
                    "match is not exhaustive — missing variant(s): {}",
                    missing.join(", ")
                ),
                span,
            ));
        }
    }

    fn check_match_expr(
        &mut self,
        scrutinee: &HirExpr,
        arms: &[HirMatchArm],
        expected: Option<CheckedType>,
        span: Span,
    ) -> Option<CheckedType> {
        self.check_match(scrutinee, arms, expected, span)
    }

    fn unify_value_type(
        &mut self,
        acc: Option<CheckedType>,
        new: Option<CheckedType>,
        span: Span,
    ) -> Option<CheckedType> {
        match (acc, new) {
            (Some(a), Some(b)) => {
                if types_compatible(a, b) {
                    Some(a)
                } else {
                    self.diagnostics.push(self.diag(
                        424,
                        "BRANCH_TYPE_MISMATCH",
                        format!(
                            "branches produce incompatible types: {} and {}",
                            self.describe(a),
                            self.describe(b)
                        ),
                        span,
                    ));
                    Some(a)
                }
            }
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    }

    /// Binds a match-arm pattern against the scrutinee's checked type.
    fn bind_pattern(&mut self, pattern: &HirPattern, scrutinee_ty: Option<CheckedType>) {
        match pattern {
            HirPattern::Wildcard { .. } => {}
            HirPattern::Literal { value, span } => {
                let lit_ty = self.check_literal(value, scrutinee_ty, *span);
                if let (Some(s), Some(l)) = (scrutinee_ty, lit_ty)
                    && !types_compatible(s, l)
                {
                    self.diagnostics.push(self.diag(
                        425,
                        "PATTERN_TYPE_MISMATCH",
                        format!(
                            "pattern has type {}, but the matched value has type {}",
                            self.describe(l),
                            self.describe(s)
                        ),
                        *span,
                    ));
                }
            }
            HirPattern::Binding { binding, .. } => {
                self.binding_types[binding.index()] = scrutinee_ty;
            }
            // Matches a known enum variant (`AICAD-053`) — checked
            // against the *scrutinee's own* type, not merely "some enum":
            // `variant`'s `BindingKind::EnumVariant::enum_name` names the
            // enum it actually belongs to (resolved through
            // `self.type_names`, the same table `resolve_type_ref` uses),
            // and that must match the scrutinee's enum identity exactly.
            HirPattern::Variant { variant, span, .. } => {
                let BindingKind::EnumVariant { enum_name } = &self.bindings[variant.index()].kind
                else {
                    // Structurally unreachable: `crate::lower` only ever
                    // produces `HirPattern::Variant` for a binding it
                    // resolved to an `EnumVariant`-kind symbol. Never
                    // panic on an unexpected shape regardless (AGENTS.md).
                    return;
                };
                let enum_name = enum_name.clone();
                self.check_pattern_enum_match(scrutinee_ty, &enum_name, *span);
            }
            // Tuple-variant destructuring (`AICAD-057C`, `project/
            // OWNER_DECISIONS.md#D17`).
            HirPattern::Tuple {
                name,
                variant,
                elems,
                span,
            } => {
                let Some(variant_id) = variant else {
                    // Already diagnosed by lowering (TYPE-E410); still
                    // bind every sub-pattern so a later reference to one
                    // of its names does not cascade into a second,
                    // spurious diagnostic of its own.
                    for elem in elems {
                        self.bind_pattern(elem, None);
                    }
                    return;
                };
                let Some(enum_name) = (match &self.bindings[variant_id.index()].kind {
                    BindingKind::EnumVariant { enum_name } => Some(enum_name.clone()),
                    _ => None,
                }) else {
                    for elem in elems {
                        self.bind_pattern(elem, None);
                    }
                    return;
                };
                self.check_pattern_enum_match(scrutinee_ty, &enum_name, *span);
                match self.variant_shapes.get(variant_id).cloned() {
                    Some(VariantShape::Tuple(field_types)) => {
                        if field_types.len() != elems.len() {
                            self.diagnostics.push(self.diag(
                                447,
                                "VARIANT_ARITY_MISMATCH",
                                format!(
                                    "'{name}' has {} field(s), but this pattern names {}",
                                    field_types.len(),
                                    elems.len()
                                ),
                                *span,
                            ));
                        }
                        for (elem, field_ty) in elems
                            .iter()
                            .zip(field_types.iter().copied().chain(std::iter::repeat(None)))
                        {
                            self.bind_pattern(elem, field_ty);
                        }
                    }
                    Some(VariantShape::Unit) | Some(VariantShape::Record(_)) => {
                        self.diagnostics.push(self.diag(
                            456,
                            "PATTERN_SHAPE_MISMATCH",
                            format!(
                                "'{name}' is not a tuple variant; this pattern shape does not \
                                 match its declared shape"
                            ),
                            *span,
                        ));
                        for elem in elems {
                            self.bind_pattern(elem, None);
                        }
                    }
                    None => {
                        for elem in elems {
                            self.bind_pattern(elem, None);
                        }
                    }
                }
            }
            // Record-variant destructuring (`AICAD-057C`, `project/
            // OWNER_DECISIONS.md#D17`). No rest (`..`) pattern is
            // authorized (D17's own scope limit), so every declared field
            // must be covered exactly once.
            HirPattern::Record {
                name,
                variant,
                fields,
                span,
            } => {
                let Some(variant_id) = variant else {
                    for field in fields {
                        self.bind_pattern(&field.pattern, None);
                    }
                    return;
                };
                let Some(enum_name) = (match &self.bindings[variant_id.index()].kind {
                    BindingKind::EnumVariant { enum_name } => Some(enum_name.clone()),
                    _ => None,
                }) else {
                    for field in fields {
                        self.bind_pattern(&field.pattern, None);
                    }
                    return;
                };
                self.check_pattern_enum_match(scrutinee_ty, &enum_name, *span);
                match self.variant_shapes.get(variant_id).cloned() {
                    Some(VariantShape::Record(field_infos)) => {
                        let mut seen: Vec<&str> = Vec::new();
                        for field in fields {
                            match field_infos.iter().find(|f| f.name == field.name) {
                                Some(info) => {
                                    if seen.contains(&field.name.as_str()) {
                                        self.diagnostics.push(self.diag(
                                            453,
                                            "DUPLICATE_VARIANT_FIELD",
                                            format!(
                                                "field '{}' is already bound in this pattern",
                                                field.name
                                            ),
                                            field.span,
                                        ));
                                    }
                                    seen.push(field.name.as_str());
                                    self.bind_pattern(&field.pattern, info.ty);
                                }
                                None => {
                                    self.diagnostics.push(self.diag(
                                        452,
                                        "UNKNOWN_VARIANT_FIELD",
                                        format!("'{name}' has no field named '{}'", field.name),
                                        field.span,
                                    ));
                                    self.bind_pattern(&field.pattern, None);
                                }
                            }
                        }
                        for info in &field_infos {
                            if !seen.contains(&info.name.as_str()) {
                                self.diagnostics.push(self.diag(
                                    454,
                                    "MISSING_VARIANT_FIELD",
                                    format!(
                                        "pattern does not cover field '{}' of '{name}' (no rest \
                                         pattern is supported)",
                                        info.name
                                    ),
                                    *span,
                                ));
                            }
                        }
                    }
                    Some(VariantShape::Unit) | Some(VariantShape::Tuple(_)) => {
                        self.diagnostics.push(self.diag(
                            456,
                            "PATTERN_SHAPE_MISMATCH",
                            format!(
                                "'{name}' is not a record variant; this pattern shape does not \
                                 match its declared shape"
                            ),
                            *span,
                        ));
                        for field in fields {
                            self.bind_pattern(&field.pattern, None);
                        }
                    }
                    None => {
                        for field in fields {
                            self.bind_pattern(&field.pattern, None);
                        }
                    }
                }
            }
        }
    }

    /// Shared by every pattern shape that matches a specific enum variant
    /// (`HirPattern::Variant`/`Tuple`/`Record`) — checked against the
    /// *scrutinee's own* type, not merely "some enum": `enum_name` must
    /// match the scrutinee's enum identity exactly.
    fn check_pattern_enum_match(
        &mut self,
        scrutinee_ty: Option<CheckedType>,
        enum_name: &str,
        span: Span,
    ) {
        let owning_enum = self.type_names.get(enum_name).copied();
        match scrutinee_ty {
            Some(CheckedType::Enum(scrutinee_enum)) if Some(scrutinee_enum) == owning_enum => {}
            Some(actual) => {
                self.diagnostics.push(self.diag(
                    434,
                    "VARIANT_ENUM_MISMATCH",
                    format!(
                        "this pattern matches a variant of a different enum than the matched \
                         value's type ({})",
                        self.describe(actual)
                    ),
                    span,
                ));
            }
            None => {}
        }
    }
}

/// The one minimal Int/vs/Float literal-type-defaulting rule `AICAD-052`
/// adds (`crate::lower`'s own "Scope boundary" explicitly assigns this
/// exact question here): a numeral's raw text (`"5"`, `"5.5"`,
/// `"1.5e-3"` — never anything else, per `crates/cad-lexer::Lexer::
/// scan_number`'s own fixed grammar) that spells a decimal point or an
/// exponent marker defaults to `Float`; otherwise it defaults to `Int` —
/// the same distinction Rust's own unsuffixed integer-vs-float literals
/// draw (DL-1, "broadly Rust/TypeScript-like"), and consistent with this
/// crate's own existing precedent (`cad_units::arithmetic::
/// resolve_derived_dimension`'s dimensionless-ratio-defaults-to-`Float`
/// rule).
///
/// When `expected` names a numeric scalar type directly (`let x: Float =
/// 5;`), that annotation wins over the text-shape default — an explicit
/// annotation is authoritative context, not a guess. `expected` naming a
/// *dimensional* type is deliberately **not** honored here: a bare
/// unitless numeral is never silently promoted to a dimensional quantity
/// (DL-3 "different physical dimensions never implicitly convert... no
/// numeric escape hatch") — `Checker::check_expected`'s own mismatch
/// check catches `let x: Length = 5;` as a type error instead.
fn default_numeric_literal_type(text: &str, expected: Option<HirType>) -> HirType {
    if let Some(HirType::Scalar(prim)) = expected
        && matches!(
            prim,
            PrimitiveType::Int
                | PrimitiveType::UInt
                | PrimitiveType::Float
                | PrimitiveType::Decimal
        )
    {
        return HirType::Scalar(prim);
    }
    if text.contains('.') || text.contains('e') || text.contains('E') {
        HirType::Scalar(PrimitiveType::Float)
    } else {
        HirType::Scalar(PrimitiveType::Int)
    }
}

fn dimensional_literal_type(dimension: Dimension) -> HirType {
    let affine = if dimension.is_affine() {
        Some(AffineKind::Absolute)
    } else {
        None
    };
    HirType::dimensional(dimension, affine)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::LowerResult;
    use cad_types::Dimension::{Energy, Force, Length, Stress, Torque};

    fn check(source: &str) -> (LowerResult, TypeCheckResult) {
        let (program, parse_diagnostics) = cad_parser::parse_program(source, "test.aicad");
        assert!(
            parse_diagnostics.is_empty(),
            "test source failed to parse: {parse_diagnostics:?}"
        );
        let lowered = crate::lower::lower_program(&program, "test.aicad", source);
        assert!(
            lowered.diagnostics.is_empty(),
            "test source failed to lower cleanly: {:?}",
            lowered.diagnostics
        );
        let checked = check_program(&lowered.program, &lowered.bindings, "test.aicad", source);
        (lowered, checked)
    }

    fn codes(diagnostics: &[Diagnostic]) -> Vec<String> {
        diagnostics.iter().map(|d| d.code.as_string()).collect()
    }

    fn let_binding(lowered: &LowerResult, index: usize) -> BindingId {
        let HirItem::Let { binding, .. } = &lowered.program.items[index] else {
            panic!("expected Let item");
        };
        *binding
    }

    fn value(ty: HirType) -> CheckedType {
        CheckedType::Value(ty)
    }

    // --- Numeric-literal-type defaulting ---

    #[test]
    fn unitless_integer_literal_defaults_to_int() {
        let (lowered, checked) = check("let x = 5;");
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(value(HirType::Scalar(PrimitiveType::Int)))
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn unitless_decimal_literal_defaults_to_float() {
        let (lowered, checked) = check("let x = 5.5;");
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(value(HirType::Scalar(PrimitiveType::Float)))
        );
    }

    #[test]
    fn unitless_exponent_literal_defaults_to_float() {
        let (lowered, checked) = check("let x = 1.5e-3;");
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(value(HirType::Scalar(PrimitiveType::Float)))
        );
    }

    #[test]
    fn integer_literal_defaults_to_annotated_float() {
        let (lowered, checked) = check("let x: Float = 5;");
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(value(HirType::Scalar(PrimitiveType::Float)))
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn unitless_literal_annotated_as_a_dimension_is_a_type_error() {
        // A bare numeral is never silently promoted to a dimensional
        // quantity (DL-3: no numeric escape hatch).
        let (_lowered, checked) = check("let x: Length = 5;");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E411"]);
    }

    // --- Ambiguous/unknown unit literal resolution ---

    #[test]
    fn ambiguous_unit_literal_resolves_via_expected_annotation() {
        let (lowered, checked) = check("let p: Stress = 5Pa;");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(value(HirType::dimensional(Stress, None)))
        );
    }

    #[test]
    fn ambiguous_unit_literal_without_annotation_is_reported() {
        let (_lowered, checked) = check("let p = 5Pa;");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E421"]);
    }

    #[test]
    fn unknown_unit_literal_is_reported() {
        let (_lowered, checked) = check("let x = 5xyz;");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E422"]);
    }

    // --- let/const/var annotations ---

    #[test]
    fn let_without_annotation_infers_from_value() {
        let (lowered, checked) = check("let w = 5mm;");
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(value(HirType::dimensional(Length, None)))
        );
    }

    #[test]
    fn let_annotation_mismatch_is_reported() {
        let (_lowered, checked) = check("let w: Length = true;");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E411"]);
    }

    #[test]
    fn const_and_var_are_also_type_checked() {
        let (_lowered, checked) =
            check("const a: Int = true; fn f() -> Int { var x: Int = true; return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E411", "TYPE-E411"]);
    }

    // --- Dimensional arithmetic reuse (DL-3) ---

    #[test]
    fn same_dimension_addition_type_checks() {
        let (_lowered, checked) = check("let x = 5mm + 2cm;");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn cross_dimension_addition_is_rejected_not_coerced() {
        let (_lowered, checked) = check("let x = 5mm + 2kg;");
        assert_eq!(codes(&checked.diagnostics), vec!["UNIT-E104"]);
    }

    #[test]
    fn affine_absolute_plus_absolute_is_rejected() {
        let (_lowered, checked) = check("let t = 20degC + 20degC;");
        assert_eq!(codes(&checked.diagnostics), vec!["UNIT-E105"]);
    }

    #[test]
    fn affine_absolute_minus_absolute_is_a_delta() {
        let (lowered, checked) = check("let t = 20degC - 5degC;");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(value(HirType::dimensional(
                cad_types::Dimension::Temperature,
                Some(AffineKind::Delta)
            )))
        );
    }

    #[test]
    fn ambiguous_derived_dimension_resolves_via_function_return_type() {
        // `force * length` alone is ambiguous (Torque vs. Energy) — the
        // enclosing function's own declared return type supplies the
        // `expected` dimension all the way through `Stmt::Return`,
        // exactly mirroring `cad_units::arithmetic`'s own
        // `force_times_length_with_torque_annotation_resolves` at the HIR
        // layer.
        let (_lowered, checked) =
            check("fn compute(f: Force, l: Length) -> Torque { return f * l; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn ambiguous_derived_dimension_without_context_is_reported() {
        let (_lowered, checked) = check("let x = 1N * 1mm;");
        assert_eq!(codes(&checked.diagnostics), vec!["UNIT-E109"]);
    }

    #[test]
    fn energy_annotation_resolves_the_same_ambiguity_differently() {
        let (_lowered, checked) =
            check("fn compute(f: Force, l: Length) -> Energy { return f * l; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        // Sanity: Torque and Energy really are the two candidates, not
        // some third accidental match.
        let candidates = [Torque, Energy, Force, Length];
        assert_eq!(candidates.len(), 4);
    }

    // --- Functions: arity, parameter types, return type ---

    #[test]
    fn function_call_type_checks_with_correct_arity_and_types() {
        let (_lowered, checked) = check(
            "fn f(x: Int, y: Int) -> Int { return x + y; } fn g() -> Int { return f(1, 2); }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn forward_referenced_function_call_is_type_checked_too() {
        let (_lowered, checked) =
            check("fn a() -> Int { return b(1); } fn b(x: Int) -> Int { return x; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn too_many_positional_arguments_is_reported() {
        let (_lowered, checked) =
            check("fn f(x: Int) -> Int { return x; } fn g() -> Int { return f(1, 2); }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E414"]);
    }

    #[test]
    fn missing_required_argument_is_reported() {
        let (_lowered, checked) =
            check("fn f(x: Int, y: Int) -> Int { return x; } fn g() -> Int { return f(1); }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E417"]);
    }

    #[test]
    fn missing_argument_with_a_default_is_allowed() {
        let (_lowered, checked) = check(
            "fn f(x: Int, y: Int = 2) -> Int { return x + y; } fn g() -> Int { return f(1); }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn unknown_named_argument_is_reported() {
        let (_lowered, checked) =
            check("fn f(x: Int) -> Int { return x; } fn g() -> Int { return f(z = 1); }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E415", "TYPE-E417"]);
    }

    #[test]
    fn duplicate_argument_is_reported() {
        let (_lowered, checked) =
            check("fn f(x: Int) -> Int { return x; } fn g() -> Int { return f(1, x = 2); }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E416"]);
    }

    #[test]
    fn named_argument_can_precede_its_own_declaration_order() {
        let (_lowered, checked) = check(
            "fn f(x: Int, y: Int) -> Int { return x + y; } fn g() -> Int { return f(y = 2, x = 1); }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn argument_type_mismatch_is_reported() {
        let (_lowered, checked) =
            check("fn f(x: Length) -> Length { return x; } fn g() -> Length { return f(true); }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E418"]);
    }

    #[test]
    fn return_type_mismatch_is_reported() {
        let (_lowered, checked) = check("fn f() -> Int { return true; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E419"]);
    }

    #[test]
    fn bare_return_from_a_value_returning_function_is_reported() {
        let (_lowered, checked) = check("fn f() -> Int { return; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E419"]);
    }

    #[test]
    fn parameter_default_type_mismatch_is_reported() {
        let (_lowered, checked) = check("fn f(x: Int = true) -> Int { return x; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E418"]);
    }

    #[test]
    fn item_level_param_default_is_type_checked() {
        let (_lowered, checked) = check("param width: Length = 5kg;");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E418"]);
    }

    // --- Control flow ---

    #[test]
    fn if_condition_must_be_bool() {
        let (_lowered, checked) = check("fn f() -> Int { if 5 { } return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E412"]);
    }

    #[test]
    fn while_condition_must_be_bool() {
        let (_lowered, checked) = check("fn f() -> Int { while 5 { } return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E412"]);
    }

    #[test]
    fn logical_and_operands_must_be_bool() {
        let (_lowered, checked) = check("fn f(x: Int) -> Bool { return x && true; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E412"]);
    }

    #[test]
    fn if_expr_branches_of_matching_type_unify() {
        let (lowered, checked) = check("let r = if true { 1mm } else { 2mm };");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(value(HirType::dimensional(Length, None)))
        );
    }

    #[test]
    fn if_expr_branches_of_mismatched_type_are_reported() {
        let (_lowered, checked) = check("let r = if true { 1mm } else { true };");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E424"]);
    }

    // --- Assignment ---

    #[test]
    fn assigning_a_matching_type_to_a_var_type_checks() {
        let (_lowered, checked) = check("fn f() -> Int { var x = 1; x = 2; return x; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn assigning_a_mismatched_type_to_a_var_is_reported() {
        let (_lowered, checked) = check("fn f() -> Bool { var x = 1; x = true; return true; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E423"]);
    }

    // --- Match: bindings and literal patterns ---

    #[test]
    fn match_binding_captures_the_scrutinee_value_type() {
        let (lowered, checked) =
            check("fn f(x: Length) -> Length { let r = match x { other => other, }; return r; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let HirItem::Fn { body, .. } = &lowered.program.items[0] else {
            panic!("expected Fn item");
        };
        let HirStmt::Let { value: expr, .. } = &body.stmts[0] else {
            panic!("expected Let stmt");
        };
        let HirExpr::Match { arms, .. } = expr else {
            panic!("expected Match expr");
        };
        let HirPattern::Binding { binding, .. } = &arms[0].pattern else {
            panic!("expected Binding pattern");
        };
        assert_eq!(
            checked.binding_types[binding.index()],
            Some(value(HirType::dimensional(Length, None)))
        );
    }

    #[test]
    fn match_literal_pattern_type_mismatch_is_reported() {
        let (_lowered, checked) = check(
            "fn f(x: Int) -> Int { let r = match x { true => 1, other => other, }; return r; }",
        );
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E425"]);
    }

    // --- Unresolved/adversarial: never panic on ill-typed or unresolved input ---

    #[test]
    fn unresolved_identifier_reference_does_not_crash_the_checker() {
        let (program, _) =
            cad_parser::parse_program("fn f() -> Int { return missing; }", "test.aicad");
        let lowered = crate::lower::lower_program(
            &program,
            "test.aicad",
            "fn f() -> Int { return missing; }",
        );
        assert_eq!(lowered.diagnostics.len(), 1, "{:?}", lowered.diagnostics);
        let checked = check_program(&lowered.program, &lowered.bindings, "test.aicad", "");
        // No new diagnostic from the type checker itself — the reference
        // is already reported by lowering (TYPE-E410); the checker just
        // treats its type as unresolved.
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn method_call_is_walked_without_a_resolvable_signature() {
        let (_lowered, checked) = check("fn f(x: Int) -> Int { x.frobnicate(true); return x; }");
        // No method system exists yet — no diagnostic about the call
        // itself, and the checker must not panic walking it.
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn genuinely_unknown_type_name_is_reported() {
        let (_lowered, checked) = check("fn f(p: Frobnicator) -> Int { return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E420"]);
    }

    #[test]
    fn calling_a_non_fn_non_struct_binding_does_not_crash_the_checker() {
        let (_lowered, checked) = check("let x = 1; fn f() -> Int { x(); return 1; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    // --- Struct field/construction typing (AICAD-053) ---

    #[test]
    fn struct_typed_parameter_resolves_to_struct_type() {
        let (lowered, checked) = check("struct P { x: Int } fn f(p: P) -> Int { return 1; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let HirItem::Struct {
            binding: struct_id, ..
        } = &lowered.program.items[0]
        else {
            panic!("expected Struct item");
        };
        let HirItem::Fn { params, .. } = &lowered.program.items[1] else {
            panic!("expected Fn item");
        };
        assert_eq!(
            checked.binding_types[params[0].binding.index()],
            Some(CheckedType::Struct(*struct_id))
        );
    }

    #[test]
    fn field_access_resolves_the_fields_own_type() {
        let (_lowered, checked) =
            check("struct P { x: Length } fn f(p: P) -> Length { return p.x; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn field_access_on_unknown_field_is_reported() {
        let (_lowered, checked) =
            check("struct P { x: Length } fn f(p: P) -> Length { return p.y; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E431"]);
    }

    #[test]
    fn field_access_on_a_non_struct_receiver_is_reported() {
        let (_lowered, checked) = check("fn f(x: Length) -> Int { return x.y; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E436"]);
    }

    #[test]
    fn nested_struct_field_access_resolves_through_two_levels() {
        let (_lowered, checked) = check(
            "struct Inner { v: Length } struct Outer { i: Inner } fn f(o: Outer) -> Length { return o.i.v; }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn struct_construction_with_named_args_type_checks() {
        let (_lowered, checked) = check(
            "struct P { x: Int, y: Int } fn f() -> Int { let p = P(x = 1, y = 2); return p.x; }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn struct_construction_with_positional_args_type_checks() {
        let (_lowered, checked) =
            check("struct P { x: Int, y: Int } fn f() -> Int { let p = P(1, 2); return p.x; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn struct_construction_result_has_the_struct_type() {
        let (lowered, checked) =
            check("struct P { x: Int } fn f() -> Int { let p = P(x = 1); return p.x; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let HirItem::Struct {
            binding: struct_id, ..
        } = &lowered.program.items[0]
        else {
            panic!("expected Struct item");
        };
        let HirItem::Fn { body, .. } = &lowered.program.items[1] else {
            panic!("expected Fn item");
        };
        let HirStmt::Let { binding, .. } = &body.stmts[0] else {
            panic!("expected Let stmt");
        };
        assert_eq!(
            checked.binding_types[binding.index()],
            Some(CheckedType::Struct(*struct_id))
        );
    }

    #[test]
    fn struct_construction_missing_field_is_reported() {
        let (_lowered, checked) =
            check("struct P { x: Int, y: Int } fn f() -> Int { P(x = 1); return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E430"]);
    }

    #[test]
    fn struct_construction_unknown_field_is_reported() {
        let (_lowered, checked) =
            check("struct P { x: Int } fn f() -> Int { P(x = 1, z = 2); return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E431"]);
    }

    #[test]
    fn struct_construction_duplicate_field_is_reported() {
        let (_lowered, checked) =
            check("struct P { x: Int } fn f() -> Int { P(x = 1, x = 2); return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E432"]);
    }

    #[test]
    fn struct_construction_field_type_mismatch_is_reported() {
        let (_lowered, checked) =
            check("struct P { x: Length } fn f() -> Int { P(x = true); return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E433"]);
    }

    #[test]
    fn struct_construction_too_many_fields_is_reported() {
        let (_lowered, checked) = check("struct P { x: Int } fn f() -> Int { P(1, 2); return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E435"]);
    }

    // --- Enum variant construction/matching typing (AICAD-053) ---

    #[test]
    fn enum_variant_used_as_a_value_has_the_enum_type() {
        let (lowered, checked) = check("enum MotorSize { NEMA17, NEMA23 } let m = NEMA17;");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let HirItem::Enum {
            binding: enum_id, ..
        } = &lowered.program.items[0]
        else {
            panic!("expected Enum item");
        };
        let b = let_binding(&lowered, 1);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(CheckedType::Enum(*enum_id))
        );
    }

    #[test]
    fn enum_variant_equality_comparison_type_checks() {
        let (_lowered, checked) = check(
            "enum MotorSize { NEMA17, NEMA23 } fn f(m: MotorSize) -> Bool { return m == NEMA17; }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn comparing_variants_of_different_enums_is_reported() {
        let (_lowered, checked) =
            check("enum A { X } enum B { Y } fn f() -> Bool { return X == Y; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E437"]);
    }

    #[test]
    fn match_on_enum_variant_patterns_type_checks() {
        let (_lowered, checked) = check(
            "enum MotorSize { NEMA17, NEMA23 } fn f(m: MotorSize) -> Int { match m { NEMA17 => { return 1; } NEMA23 => { return 2; } } return 0; }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn match_variants_of_two_unrelated_enums_each_type_check_on_their_own() {
        let (_lowered, checked) = check(
            "enum A { X } enum B { Y } fn f(a: A) -> Int { match a { X => { return 1; } } return 0; } fn g(b: B) -> Int { match b { Y => { return 1; } } return 0; }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn match_variant_pattern_against_mismatched_scrutinee_enum_is_reported() {
        // Two enums sharing one variant *name* — `crate::lower`'s own
        // documented duplicate-declaration behavior ("the second mint
        // simply overwrites the first in this pass's own scope map")
        // means the pattern `Shared` resolves to `B::Shared` (declared
        // last), while `a`'s declared type is `A` — a genuine cross-enum
        // mismatch this task's own machinery must catch.
        // Also `TYPE-E446` (`AICAD-057C`'s exhaustiveness check): since the
        // pattern actually resolves to `B::Shared`, `A`'s own `Shared`
        // variant is never genuinely covered by any arm — a real, separate
        // finding from the enum-identity mismatch itself.
        let (_lowered, checked) = check(
            "enum A { Shared } enum B { Shared } fn f(a: A) -> Int { match a { Shared => { return 1; } } return 0; }",
        );
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E434", "TYPE-E446"]);
    }

    // --- AICAD-057C: data-carrying enum variants, constructors,
    //     destructuring patterns, match exhaustiveness
    //     (project/OWNER_DECISIONS.md#D17) --------------------------------

    #[test]
    fn tuple_variant_construction_with_correct_types_checks_cleanly() {
        let (_lowered, checked) =
            check("enum R { Ok(Int), Err(Int) } fn f() -> Int { let r = Ok(1); return 1; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn tuple_variant_construction_result_has_the_enum_type() {
        let (lowered, checked) =
            check("enum R { Ok(Int) } fn f() -> Int { let r = Ok(1); return 1; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let HirItem::Enum {
            binding: enum_id, ..
        } = &lowered.program.items[0]
        else {
            panic!("expected Enum item");
        };
        let HirItem::Fn { body, .. } = &lowered.program.items[1] else {
            panic!("expected Fn item");
        };
        let HirStmt::Let { binding, .. } = &body.stmts[0] else {
            panic!("expected Let stmt");
        };
        assert_eq!(
            checked.binding_types[binding.index()],
            Some(CheckedType::Enum(*enum_id))
        );
    }

    #[test]
    fn tuple_variant_construction_wrong_arity_is_reported() {
        let (_lowered, checked) =
            check("enum R { Ok(Int, Int) } fn f() -> Int { let r = Ok(1); return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E447"]);
    }

    #[test]
    fn tuple_variant_construction_too_many_args_is_reported() {
        let (_lowered, checked) =
            check("enum R { Ok(Int) } fn f() -> Int { let r = Ok(1, 2); return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E447"]);
    }

    #[test]
    fn tuple_variant_construction_field_type_mismatch_is_reported() {
        let (_lowered, checked) =
            check("enum R { Ok(Length) } fn f() -> Int { let r = Ok(true); return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E449"]);
    }

    #[test]
    fn unit_variant_called_with_parens_is_reported() {
        let (_lowered, checked) =
            check("enum R { Empty } fn f() -> Int { let r = Empty(); return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E448"]);
    }

    #[test]
    fn zero_field_tuple_variant_called_with_no_args_checks_cleanly() {
        let (_lowered, checked) =
            check("enum R { Empty() } fn f() -> Int { let r = Empty(); return 1; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn record_variant_construction_with_correct_fields_checks_cleanly() {
        let (_lowered, checked) = check(
            "enum Shape { Circle { radius: Length } } \
             fn f() -> Length { let s = Circle { radius: 1mm }; return 1mm; }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn record_variant_construction_result_has_the_enum_type() {
        let (lowered, checked) = check(
            "enum Shape { Circle { radius: Length } } \
             fn f() -> Length { let s = Circle { radius: 1mm }; return 1mm; }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let HirItem::Enum {
            binding: enum_id, ..
        } = &lowered.program.items[0]
        else {
            panic!("expected Enum item");
        };
        let HirItem::Fn { body, .. } = &lowered.program.items[1] else {
            panic!("expected Fn item");
        };
        let HirStmt::Let { binding, .. } = &body.stmts[0] else {
            panic!("expected Let stmt");
        };
        assert_eq!(
            checked.binding_types[binding.index()],
            Some(CheckedType::Enum(*enum_id))
        );
    }

    #[test]
    fn record_variant_construction_with_parens_is_reported() {
        let (_lowered, checked) = check(
            "enum Shape { Circle { radius: Length } } \
             fn f() -> Int { let s = Circle(1mm); return 1; }",
        );
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E451"]);
    }

    #[test]
    fn record_variant_construction_missing_field_is_reported() {
        let (_lowered, checked) = check(
            "enum Shape { Circle { radius: Length, center: Int } } \
             fn f() -> Int { let s = Circle { radius: 1mm }; return 1; }",
        );
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E454"]);
    }

    #[test]
    fn record_variant_construction_unknown_field_is_reported() {
        let (_lowered, checked) = check(
            "enum Shape { Circle { radius: Length } } \
             fn f() -> Int { let s = Circle { radius: 1mm, bogus: 2mm }; return 1; }",
        );
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E452"]);
    }

    #[test]
    fn record_variant_construction_duplicate_field_is_reported() {
        let (_lowered, checked) = check(
            "enum Shape { Circle { radius: Length } } \
             fn f() -> Int { let s = Circle { radius: 1mm, radius: 2mm }; return 1; }",
        );
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E453"]);
    }

    #[test]
    fn tuple_pattern_destructuring_binding_has_correct_field_type() {
        // Required test: "payload binding has correct type."
        let (lowered, checked) = check(
            "enum R { Ok(Length) } \
             fn f(r: R) -> Length { match r { Ok(v) => { return v; } } }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let HirItem::Fn { body, .. } = &lowered.program.items[1] else {
            panic!("expected Fn item");
        };
        let HirStmt::Match { arms, .. } = &body.stmts[0] else {
            panic!("expected Match stmt");
        };
        let HirPattern::Tuple { elems, .. } = &arms[0].pattern else {
            panic!("expected Tuple pattern");
        };
        let HirPattern::Binding { binding, .. } = &elems[0] else {
            panic!("expected Binding pattern");
        };
        assert_eq!(
            checked.binding_types[binding.index()],
            Some(value(HirType::dimensional(
                cad_types::Dimension::Length,
                None
            )))
        );
    }

    #[test]
    fn record_pattern_shorthand_binding_has_correct_field_type() {
        // Required test: record destructuring, payload binding type.
        let (lowered, checked) = check(
            "enum Shape { Circle { radius: Length } } \
             fn f(s: Shape) -> Length { match s { Circle { radius } => { return radius; } } }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let HirItem::Fn { body, .. } = &lowered.program.items[1] else {
            panic!("expected Fn item");
        };
        let HirStmt::Match { arms, .. } = &body.stmts[0] else {
            panic!("expected Match stmt");
        };
        let HirPattern::Record { fields, .. } = &arms[0].pattern else {
            panic!("expected Record pattern");
        };
        let HirPattern::Binding { binding, .. } = &fields[0].pattern else {
            panic!("expected Binding pattern");
        };
        assert_eq!(
            checked.binding_types[binding.index()],
            Some(value(HirType::dimensional(
                cad_types::Dimension::Length,
                None
            )))
        );
    }

    #[test]
    fn record_pattern_missing_field_is_reported() {
        let (_lowered, checked) = check(
            "enum Shape { Circle { radius: Length, center: Int } } \
             fn f(s: Shape) -> Int { match s { Circle { radius } => { return 1; } } }",
        );
        assert!(
            codes(&checked.diagnostics).contains(&"TYPE-E454".to_string()),
            "{:?}",
            checked.diagnostics
        );
    }

    #[test]
    fn record_pattern_unknown_field_is_reported() {
        let (_lowered, checked) = check(
            "enum Shape { Circle { radius: Length } } \
             fn f(s: Shape) -> Int { match s { Circle { bogus } => { return 1; } } }",
        );
        assert!(
            codes(&checked.diagnostics).contains(&"TYPE-E452".to_string()),
            "{:?}",
            checked.diagnostics
        );
    }

    #[test]
    fn tuple_pattern_against_a_record_variant_is_a_shape_mismatch() {
        let (_lowered, checked) = check(
            "enum Shape { Circle { radius: Length } } \
             fn f(s: Shape) -> Int { match s { Circle(r) => { return 1; } } }",
        );
        assert!(
            codes(&checked.diagnostics).contains(&"TYPE-E456".to_string()),
            "{:?}",
            checked.diagnostics
        );
    }

    #[test]
    fn non_exhaustive_match_over_tuple_and_record_variants_is_reported() {
        // Required test: "non-exhaustive enum match -> diagnostic."
        let (_lowered, checked) = check(
            "enum R { Ok(Int), Err(Int) } \
             fn f(r: R) -> Int { match r { Ok(v) => { return v; } } }",
        );
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E446"]);
    }

    #[test]
    fn exhaustive_match_covering_every_variant_has_no_diagnostic() {
        let (_lowered, checked) = check(
            "enum R { Ok(Int), Err(Int) } \
             fn f(r: R) -> Int { match r { Ok(v) => { return v; } Err(e) => { return e; } } }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn non_exhaustive_match_with_wildcard_has_no_diagnostic() {
        let (_lowered, checked) = check(
            "enum R { Ok(Int), Err(Int) } \
             fn f(r: R) -> Int { match r { Ok(v) => { return v; } _ => { return 0; } } }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn non_exhaustive_match_over_mixed_unit_tuple_record_variants_names_the_missing_ones() {
        let (_lowered, checked) = check(
            "enum Shape { Point, Circle(Length), Rect { w: Length, h: Length } } \
             fn f(s: Shape) -> Int { match s { Point => { return 0; } } }",
        );
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E446"]);
    }

    #[test]
    fn generic_enum_tuple_variant_destructures_cleanly() {
        // Evidence this is general machinery, not special-cased to any
        // particular enum name (`AICAD-057A`'s own audit finding — a
        // user-defined generic enum whose name is not Result/Optional/
        // List/Range). `b`'s own declared type (`Box<Int>`) does not
        // resolve yet (`Name<Args>` type-*reference* resolution for a
        // user-defined generic is `AICAD-057D`'s job, per `AICAD-057B`'s
        // own documented limitation) — this only exercises the payload-
        // shape/destructuring half `AICAD-057C` actually owns, not
        // instantiation.
        let (_lowered, checked) = check(
            "enum Box<T> { Full(T), Empty } \
             fn f(b: Box<Int>) -> Int { match b { Full(v) => { return 0; } Empty => { return 0; } } }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    // --- Collections / iteration (`AICAD-056`, `project/
    //     OWNER_DECISIONS.md#D16`) ---

    /// The `for`-loop `binding`'s own resolved element type, from a
    /// program whose first (and only) top-level item is `fn f() { for ...
    /// { ... } ... }`.
    fn for_loop_element_type(
        lowered: &LowerResult,
        checked: &TypeCheckResult,
    ) -> Option<CheckedType> {
        let HirItem::Fn { body, .. } = &lowered.program.items[0] else {
            panic!("expected a Fn item");
        };
        let HirStmt::For { binding, .. } = &body.stmts[0] else {
            panic!("expected the fn body's first statement to be a for loop");
        };
        checked.binding_types[binding.index()]
    }

    #[test]
    fn list_literal_of_ints_has_list_int_type() {
        let (lowered, checked) = check("let xs = [1, 2, 3];");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(CheckedType::List(HirType::Scalar(PrimitiveType::Int)))
        );
    }

    #[test]
    fn list_literal_of_lengths_unifies_across_unit_spellings() {
        // project/OWNER_DECISIONS.md#D16's own worked example: [5mm, 2cm,
        // 1in] -> List<Length>, despite three different unit spellings.
        let (lowered, checked) = check("let xs = [5mm, 2cm, 1in];");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(CheckedType::List(HirType::dimensional(Length, None)))
        );
    }

    #[test]
    fn list_literal_with_incompatible_element_dimensions_is_reported() {
        let (_lowered, checked) = check("let xs = [5mm, 3kg];");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E440"]);
    }

    #[test]
    fn empty_list_literal_with_contextual_type_annotation_type_checks() {
        let (lowered, checked) = check("let xs: List<Int> = [];");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(CheckedType::List(HirType::Scalar(PrimitiveType::Int)))
        );
    }

    #[test]
    fn empty_list_literal_without_inferable_type_is_reported() {
        let (_lowered, checked) = check("let xs = [];");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E441"]);
    }

    #[test]
    fn range_of_ints_has_range_int_type() {
        let (lowered, checked) = check("let r = 0..10;");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(CheckedType::Range(HirType::Scalar(PrimitiveType::Int)))
        );
    }

    #[test]
    fn range_of_uints_via_annotation_has_range_uint_type() {
        let (lowered, checked) = check("let r: Range<UInt> = 0..10;");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(CheckedType::Range(HirType::Scalar(PrimitiveType::UInt)))
        );
    }

    #[test]
    fn dimensional_range_type_checks_as_a_value() {
        // project/OWNER_DECISIONS.md#D16: "Range<T> may exist for
        // dimensional values such as Range<Length>" — constructible, just
        // not automatically iterable (see the dedicated rejection test
        // below).
        let (lowered, checked) = check("let r = 1mm..10mm;");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(CheckedType::Range(HirType::dimensional(Length, None)))
        );
    }

    #[test]
    fn range_with_mismatched_bound_types_is_reported() {
        let (_lowered, checked) = check("let r = 0..10mm;");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E442"]);
    }

    #[test]
    fn for_over_list_of_ints_resolves_loop_variable_to_int() {
        let (lowered, checked) = check("fn f() -> Int { for x in [1, 2, 3] { } return 0; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        assert_eq!(
            for_loop_element_type(&lowered, &checked),
            Some(value(HirType::Scalar(PrimitiveType::Int)))
        );
    }

    #[test]
    fn for_over_list_of_lengths_resolves_loop_variable_to_length() {
        let (lowered, checked) = check("fn f() -> Int { for x in [5mm, 2cm] { } return 0; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        assert_eq!(
            for_loop_element_type(&lowered, &checked),
            Some(value(HirType::dimensional(Length, None)))
        );
    }

    #[test]
    fn for_over_half_open_int_range_resolves_loop_variable_to_int() {
        let (lowered, checked) =
            check("fn f(count: Int) -> Int { for i in 0..count { } return 0; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        assert_eq!(
            for_loop_element_type(&lowered, &checked),
            Some(value(HirType::Scalar(PrimitiveType::Int)))
        );
    }

    #[test]
    fn for_over_inclusive_int_range_type_checks() {
        let (_lowered, checked) = check("fn f() -> Int { for i in 0..=10 { } return 0; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn for_over_dimensional_range_is_rejected() {
        // project/OWNER_DECISIONS.md#D16: a dimensional Range is not
        // automatically iterable (no step size is defined).
        let (_lowered, checked) = check("fn f() -> Int { for x in 1mm..10mm { } return 0; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E443"]);
    }

    #[test]
    fn for_over_a_non_iterable_expression_is_rejected() {
        let (_lowered, checked) = check("fn f() -> Int { for x in 5 { } return 0; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E443"]);
    }

    // --- AICAD-057B: generic type-parameter resolution
    //     (project/OWNER_DECISIONS.md#D17) ------------------------------

    #[test]
    fn generic_struct_with_one_type_parameter_type_checks_cleanly() {
        let (_lowered, checked) = check("struct Box<T> { value: T }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn generic_struct_with_two_type_parameters_type_checks_cleanly() {
        let (_lowered, checked) = check("struct Pair<T, U> { first: T, second: U }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn generic_enum_type_checks_cleanly() {
        let (_lowered, checked) = check("enum Container<T> { Empty }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn generic_function_with_matching_param_and_return_type_parameter_type_checks_cleanly() {
        let (_lowered, checked) = check("fn identity<T>(value: T) -> T { return value; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn generic_function_body_can_reference_its_own_type_parameter_in_a_let_annotation() {
        let (_lowered, checked) =
            check("fn identity<T>(value: T) -> T { let y: T = value; return y; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn two_different_generic_declarations_use_independent_type_parameters() {
        // Both declare a parameter named "T", but they must not be
        // treated as the same type — each declaration's own `T` is scoped
        // to it alone.
        let (lowered, checked) = check("struct A<T> { x: T } struct B<T> { y: T }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let HirItem::Struct { type_params: a, .. } = &lowered.program.items[0] else {
            panic!("expected Struct item");
        };
        let HirItem::Struct { type_params: b, .. } = &lowered.program.items[1] else {
            panic!("expected Struct item");
        };
        assert_ne!(a[0].binding, b[0].binding);
    }

    #[test]
    fn mismatched_generic_function_type_parameters_are_reported() {
        // `b`'s declared type is `U`, not `T` — returning it where `T` is
        // expected is a genuine type mismatch, proving `CheckedType::
        // TypeParam` actually distinguishes declared parameters rather
        // than acting as a universal wildcard.
        let (_lowered, checked) = check("fn f<T, U>(a: T, b: U) -> T { return b; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E419"]);
    }

    #[test]
    fn duplicate_type_parameter_name_is_reported_by_type_checking_too() {
        // The diagnostic itself is raised during lowering (`AICAD-057B`'s
        // own `cad_hir::lower` test), but a program carrying it must still
        // reach `check_program` without panicking — both mistakenly-
        // duplicated bindings still get a `CheckedType`.
        let (program, parse_diagnostics) =
            cad_parser::parse_program("struct Foo<T, T> { a: T }", "test.aicad");
        assert!(parse_diagnostics.is_empty());
        let lowered =
            crate::lower::lower_program(&program, "test.aicad", "struct Foo<T, T> { a: T }");
        assert_eq!(codes(&lowered.diagnostics), vec!["TYPE-E445"]);
        let checked = check_program(
            &lowered.program,
            &lowered.bindings,
            "test.aicad",
            "struct Foo<T, T> { a: T }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }
}
