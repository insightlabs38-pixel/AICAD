//! AST -> Typed HIR lowering (`AICAD-051`), `docs/plan/02_LANGUAGE_AND_
//! COMPILER.md` §17 phase 7 ("Lower to typed HIR").
//!
//! [`lower_program`] takes one already-parsed `cad_ast::Program` (plus the
//! `file`/`source` context every other phase's diagnostics need for spans
//! — the same two pieces `cad_parser::parse_program`/`crate::binder::
//! bind_program` take) and produces a [`LowerResult`]: the lowered
//! [`crate::hir::HirProgram`], every [`crate::ids::Binding`] minted along
//! the way (the typed HIR's own symbol table, indexed by
//! `BindingId::index()`), and any lowering diagnostics.
//!
//! ## Relationship to `cad_compiler::binder`
//!
//! `AICAD-050`'s name binder (`crates/cad-compiler/src/binder.rs`) already
//! performs name resolution over a `cad_ast::Program` and is the
//! project's primary source of `UNDEFINED_NAME`/`DUPLICATE_BINDING`/
//! `ASSIGN_TO_IMMUTABLE` diagnostics — in the full pipeline
//! (`docs/plan/02_LANGUAGE_AND_COMPILER.md` §17), binding (phase 3) runs
//! well before HIR lowering (phase 7), so a program reaching this module
//! is expected to already be binder-clean. This module does *not* call
//! into `crate::binder` to get that resolution, for a structural reason,
//! not a preference: `crates/cad-compiler`'s own module doc comment
//! already lists `cad-hir` as one of the crates *it* composes ("composing
//! `cad-lexer`, `cad-parser`, `cad-ast`, `cad-hir`, ..."), so a `cad-hir
//! -> cad-compiler` dependency would be a workspace dependency cycle.
//!
//! `AGENTS.md`'s HIR invariant "lexical binding identity is explicit" is
//! still a hard requirement of *this* module's output regardless, so
//! [`Lowerer`] performs its own minimal scope walk — mirroring `crate::
//! binder::Binder`'s scope-stack/declare-then-check structure closely
//! enough that a HIR reader familiar with that module will recognize it —
//! solely to mint a [`crate::ids::BindingId`] per declaration and wire
//! every reference to the id its name resolves to. It intentionally does
//! **not** re-implement every diagnostic `crate::binder` already produces
//! (see "Scope boundary" below): the point is binding *identity*, not a
//! second copy of name-binding validation.
//!
//! ## Scope boundary: what this task does *not* do
//!
//! - **Duplicate-declaration detection.** Two `let a = 1; let a = 2;` in
//!   the same scope each get their own fresh `BindingId`, and the second
//!   mint simply overwrites the first in this pass's own scope map (so a
//!   later reference to `a` binds to the second declaration) — no
//!   diagnostic is raised. `crate::binder`'s `DUPLICATE_BINDING`
//!   (`TYPE-E402`) already owns this check and runs earlier in the real
//!   pipeline; re-deriving it here would duplicate, not extend, that
//!   work.
//! - **`Stmt::Assign` mutability enforcement (DL-2).** `HirStmt::Assign::
//!   target` resolves to *some* binding regardless of whether it is a
//!   `var` — `crate::binder`'s `ASSIGN_TO_IMMUTABLE` (`TYPE-E403`) already
//!   owns that check.
//! - **Numeric-literal-type defaulting (`Int` vs. `Float` vs. ...).**
//!   `cad_units::arithmetic`'s own module doc comment already defers this
//!   exact question to `AICAD-052`; `HirLiteral::Number` keeps the raw
//!   source text unparsed for the same reason (see `crate::hir::
//!   HirLiteral::Number`'s own doc comment).
//! - **Method/field member resolution.** `HirCallee::Method::name` and
//!   `HirExpr::Field::field` are never checked against any scope or type
//!   — resolving them needs the receiver's *type*, which does not exist
//!   until `AICAD-052`, exactly as `crate::binder`'s own module doc
//!   comment already establishes for the AST layer.
//! - **Type-name resolution.** `crate::types::HirTypeRef` (`: Length`,
//!   `: Vector2<Length>`) is carried through unresolved — see that type's
//!   own doc comment.
//! - **Struct-field/import-target/package-path resolution.** Same
//!   boundary `crate::binder` and `crate::loader` already document for
//!   their own layers; this pass adds no new resolution beyond what those
//!   two already do.
//!
//! ## Unresolved names
//!
//! Even though a binder-clean program is the expected input, this module
//! still handles an unresolved name gracefully rather than panicking
//! (consistent with `cad_ast::LineIndex::line_column`'s own stated
//! philosophy: "diagnostic code should never crash the compiler over a
//! ... edge case") — both because nothing in this crate's own API forces
//! a caller to run `crate::binder` first (`cad-hir` cannot even see that
//! module, per "Relationship to `cad_compiler::binder`" above), and
//! because it gives this task a genuine, testable "a construct that
//! cannot legally lower to a definite binding" case. `HirExpr::Ident::
//! binding`/`HirStmt::Assign::target`/`HirCallee::Fn::binding` are all
//! `None` exactly when this happens, and a single `TYPE-E410`
//! (`UNRESOLVED_BINDING`) diagnostic is recorded — a distinct code from
//! `crate::binder`'s `TYPE-E401` (`UNDEFINED_NAME`) since the two are
//! produced by different phases for what is, in a correctly-ordered
//! pipeline, the same underlying condition caught twice; see module doc
//! comment above.

use crate::builtins::{BuiltinFnSpec, catalogue as builtin_catalogue};
use crate::hir::{
    FunctionImplementation, HirArg, HirBlock, HirCallee, HirElseStmt, HirEnumVariant, HirExpr,
    HirField, HirImportPath, HirImportedName, HirItem, HirLiteral, HirMatchArm, HirParam,
    HirPattern, HirProgram, HirRecordField, HirRecordPatternField, HirStmt, HirTypeParam,
    HirVariantPayload,
};
use crate::ids::{Binding, BindingId, BindingKind};
use crate::types::{HirType, HirTypeRef};
use cad_ast::{
    Arg, Block, BlockExpr, ElseBranch, ElseClause, EnumVariant, Expr, FnParam, Item, Literal,
    MatchArm, MatchArmBody, Pattern, Program, Span, Spanned, Stmt, Type,
};
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SeverityLetter, SourceSpan};
use cad_types::{AffineKind, PrimitiveType};
use std::collections::HashMap;

/// The result of lowering one program: the lowered HIR, every binding
/// minted along the way (indexed by `BindingId::index()`), and any
/// lowering diagnostics (see module doc comment "Unresolved names").
#[derive(Debug, Clone)]
pub struct LowerResult {
    pub program: HirProgram,
    pub bindings: Vec<Binding>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Lowers one already-parsed program to typed HIR. See module doc comment
/// for exactly what this does and does not resolve.
///
/// Every returned [`HirProgram`] is seeded with the Stage-2 Safe CAD
/// standard-function catalogue (`crate::builtins::catalogue`, `project/
/// DECISION_LOG.md#DL-15`): each builtin's name is declared in the module
/// scope *before* any of `program`'s own items are lowered, so a user call
/// site resolves to it exactly as `crate::prelude::with_prelude` makes
/// `Result`/`Optional` resolvable everywhere — except these names are
/// seeded as HIR nodes directly (see `crate::builtins`'s own module doc
/// comment "Why HIR-level, not source-text, seeding") rather than by
/// prepending parsed AST items. The synthetic builtin items themselves are
/// appended *after* the user's own lowered items in the returned
/// [`HirProgram::items`] (declaration order has no effect on `crate::
/// typeck`'s forward-reference-friendly two-pass checking or `cad_runtime`'s
/// own `index_fns`, both of which scan the full item list regardless of
/// position) — this keeps `result.program.items[0]` meaning exactly what
/// every pre-existing test already assumes it does: the caller's own first
/// declared item, not a builtin. Unlike the prelude, this seeding is
/// unconditional (every `lower_program` caller gets it — `DL-15`'s
/// functions are ordinary standard-library-shaped names, not an opt-in
/// extra), and a user declaration that happens to redeclare `box`/
/// `cylinder`/... simply overwrites the seeded binding in the lowerer's
/// own top-level scope map, identical to how any other same-name
/// redeclaration behaves (module doc comment "Duplicate-declaration
/// detection") — no special protection, per the same non-special-casing
/// precedent `crate::prelude` already established.
pub fn lower_program(program: &Program, file: &str, source: &str) -> LowerResult {
    let mut lowerer = Lowerer {
        file,
        source,
        scopes: vec![HashMap::new()],
        bindings: Vec::new(),
        diagnostics: Vec::new(),
    };
    let builtin_items = lowerer.seed_builtins();
    let mut items = lowerer.lower_items(&program.items);
    items.extend(builtin_items);
    LowerResult {
        program: HirProgram { items },
        bindings: lowerer.bindings,
        diagnostics: lowerer.diagnostics,
    }
}

/// Per-item bookkeeping `Lowerer::declare_item` hands back to `Lowerer::
/// lower_item_body`, so the body pass never has to re-resolve "which
/// binding did *this* item's own declaration just mint" by name lookup
/// (a name lookup could, for a same-scope duplicate name, return a
/// sibling item's id instead of this one's — see module doc comment
/// "Duplicate-declaration detection").
enum DeclaredItem {
    Simple(BindingId),
    /// A `fn`/`struct` declaration, carrying its own generic type
    /// parameters' newly minted ids in declaration order (empty for an
    /// ordinary, non-generic declaration) — `AICAD-057B`, `project/
    /// OWNER_DECISIONS.md#D17`.
    Generic {
        own: BindingId,
        type_params: Vec<BindingId>,
    },
    Enum {
        own: BindingId,
        /// Same as `Generic::type_params` — see its own doc comment.
        type_params: Vec<BindingId>,
        variants: Vec<BindingId>,
    },
    Import {
        names: Vec<BindingId>,
    },
}

struct Lowerer<'a> {
    file: &'a str,
    source: &'a str,
    /// Lexical scope stack, outermost (module scope) first — mirrors
    /// `crate::binder::Binder::scopes` exactly, except values are
    /// `BindingId`s (this pass's own minted identity) rather than a full
    /// `Symbol` (kind is looked up from `bindings` when needed, via
    /// `Lowerer::kind_of`).
    scopes: Vec<HashMap<String, BindingId>>,
    /// Every binding minted so far, in minting order — `bindings[id.
    /// index()]` is always that binding's own record.
    bindings: Vec<Binding>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lowerer<'a> {
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes
            .pop()
            .expect("push_scope/pop_scope calls are always balanced");
    }

    /// Mints a fresh `BindingId` for `name`/`kind`, records it in
    /// `bindings`, and declares it in the innermost active scope
    /// (overwriting any same-name entry already there — see module doc
    /// comment "Duplicate-declaration detection").
    fn mint(&mut self, name: &Spanned<String>, kind: BindingKind) -> BindingId {
        let id = BindingId::new(self.bindings.len() as u32);
        self.bindings.push(Binding {
            id,
            name: name.node.clone(),
            kind,
            span: name.span,
        });
        self.scopes
            .last_mut()
            .expect("lower_program always keeps at least one scope active")
            .insert(name.node.clone(), id);
        id
    }

    /// Mints a fresh `BindingId` for one of `crate::builtins::catalogue`'s
    /// own parameter entries, *without* declaring it in any active scope
    /// — a runtime-backed function's params have no `HirBlock` body to
    /// resolve names against (`crate::hir::FunctionImplementation::
    /// RuntimeBuiltin`), so, exactly like `mint_type_params`'s own
    /// identical reasoning, there is no lexical scope a builtin's own
    /// parameter name could ever need to be looked up in.
    fn mint_unscoped(&mut self, name: &str, kind: BindingKind, span: Span) -> BindingId {
        let id = BindingId::new(self.bindings.len() as u32);
        self.bindings.push(Binding {
            id,
            name: name.to_string(),
            kind,
            span,
        });
        id
    }

    /// Seeds every `crate::builtins::catalogue` entry as a top-level
    /// `HirItem::Fn` with a `FunctionImplementation::RuntimeBuiltin` body,
    /// declaring each one's name in the (currently empty) module scope —
    /// see `lower_program`'s own doc comment for why this must run before
    /// any caller-supplied item is lowered.
    fn seed_builtins(&mut self) -> Vec<HirItem> {
        let synthetic_span = Span::new(0, 0);
        builtin_catalogue()
            .into_iter()
            .map(|spec: BuiltinFnSpec| {
                let name_spanned = Spanned::new(spec.name.to_string(), synthetic_span);
                let binding = self.mint(&name_spanned, BindingKind::Fn);
                let params = spec
                    .params
                    .into_iter()
                    .map(|(param_name, ty)| HirParam {
                        binding: self.mint_unscoped(param_name, BindingKind::Param, synthetic_span),
                        name: param_name.to_string(),
                        ty,
                        default: None,
                        span: synthetic_span,
                    })
                    .collect();
                HirItem::Fn {
                    binding,
                    name: spec.name.to_string(),
                    is_pure: true,
                    type_params: Vec::new(),
                    params,
                    return_ty: Some(spec.return_ty),
                    body: FunctionImplementation::RuntimeBuiltin(spec.id),
                    span: synthetic_span,
                }
            })
            .collect()
    }

    /// Mints a fresh `BindingId` (`BindingKind::TypeParam`) for each
    /// generic type parameter declared on one `fn`/`struct`/`enum`
    /// (`AICAD-057B`, `project/OWNER_DECISIONS.md#D17`), diagnosing a
    /// duplicate name within that same declaration's own list (`struct
    /// Foo<T, T>`). Deliberately does **not** use `mint`/`self.scopes`:
    /// type parameters are a type-namespace name, not a value-level one
    /// (`crate::binder`'s own module doc comment already assigns "type-
    /// name resolution" entirely to the type checker), so they never
    /// belong in the value-lookup scope chain `resolve`/`resolve_or_
    /// diagnose` walk — inserting them there would let a type parameter
    /// shadow, or be shadowed by, an unrelated value binding, which
    /// nothing in D17 authorizes.
    fn mint_type_params(&mut self, type_params: &[Spanned<String>]) -> Vec<BindingId> {
        let mut seen: HashMap<&str, Span> = HashMap::new();
        let mut ids = Vec::with_capacity(type_params.len());
        for param in type_params {
            if let Some(&first_span) = seen.get(param.node.as_str()) {
                self.diagnostics.push(duplicate_type_parameter_diagnostic(
                    self.file,
                    self.source,
                    param,
                    first_span,
                ));
            } else {
                seen.insert(param.node.as_str(), param.span);
            }
            let id = BindingId::new(self.bindings.len() as u32);
            self.bindings.push(Binding {
                id,
                name: param.node.clone(),
                kind: BindingKind::TypeParam,
                span: param.span,
            });
            ids.push(id);
        }
        ids
    }

    fn resolve(&self, name: &str) -> Option<BindingId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn kind_of(&self, id: BindingId) -> &BindingKind {
        &self.bindings[id.index()].kind
    }

    /// Resolves `name` against the active scope chain, recording a
    /// `TYPE-E410` diagnostic (see module doc comment "Unresolved names")
    /// and returning `None` if it resolves nowhere.
    fn resolve_or_diagnose(&mut self, name: &Spanned<String>) -> Option<BindingId> {
        let resolved = self.resolve(&name.node);
        if resolved.is_none() {
            self.diagnostics
                .push(unresolved_binding_diagnostic(self.file, self.source, name));
        }
        resolved
    }

    // --- Items ---

    /// Declares every item in `items` in the current (innermost) scope
    /// first, then lowers each item's own body — the same two-pass,
    /// forward-reference-enabling structure as `crate::binder::Binder::
    /// bind_items`.
    fn lower_items(&mut self, items: &[Item]) -> Vec<HirItem> {
        let declared: Vec<DeclaredItem> =
            items.iter().map(|item| self.declare_item(item)).collect();
        items
            .iter()
            .zip(declared)
            .map(|(item, decl)| self.lower_item_body(item, decl))
            .collect()
    }

    fn declare_item(&mut self, item: &Item) -> DeclaredItem {
        match item {
            Item::Let { name, .. } => DeclaredItem::Simple(self.mint(name, BindingKind::Let)),
            Item::Const { name, .. } => DeclaredItem::Simple(self.mint(name, BindingKind::Const)),
            Item::Param { name, .. } => DeclaredItem::Simple(self.mint(name, BindingKind::Param)),
            Item::Fn {
                name, type_params, ..
            } => {
                let own = self.mint(name, BindingKind::Fn);
                let type_params = self.mint_type_params(type_params);
                DeclaredItem::Generic { own, type_params }
            }
            Item::Struct {
                name, type_params, ..
            } => {
                let own = self.mint(name, BindingKind::Struct);
                let type_params = self.mint_type_params(type_params);
                DeclaredItem::Generic { own, type_params }
            }
            Item::Enum {
                name,
                type_params,
                variants,
                ..
            } => {
                let own = self.mint(name, BindingKind::Enum);
                let type_params = self.mint_type_params(type_params);
                let variant_ids = variants
                    .iter()
                    .map(|variant| {
                        self.mint(
                            variant.name(),
                            BindingKind::EnumVariant {
                                enum_name: name.node.clone(),
                            },
                        )
                    })
                    .collect();
                DeclaredItem::Enum {
                    own,
                    type_params,
                    variants: variant_ids,
                }
            }
            Item::Part { name, .. } => DeclaredItem::Simple(self.mint(name, BindingKind::Part)),
            Item::Import { names, .. } => DeclaredItem::Import {
                names: names
                    .iter()
                    .flatten()
                    .map(|name| self.mint(name, BindingKind::Import))
                    .collect(),
            },
        }
    }

    fn lower_item_body(&mut self, item: &Item, decl: DeclaredItem) -> HirItem {
        match (item, decl) {
            (
                Item::Let {
                    name,
                    ty,
                    value,
                    span,
                },
                DeclaredItem::Simple(binding),
            ) => HirItem::Let {
                binding,
                name: name.node.clone(),
                ty: ty.as_ref().map(lower_type),
                value: self.lower_expr(value),
                span: *span,
            },
            (
                Item::Const {
                    name,
                    ty,
                    value,
                    span,
                },
                DeclaredItem::Simple(binding),
            ) => HirItem::Const {
                binding,
                name: name.node.clone(),
                ty: ty.as_ref().map(lower_type),
                value: self.lower_expr(value),
                span: *span,
            },
            (
                Item::Param {
                    name,
                    ty,
                    default,
                    span,
                },
                DeclaredItem::Simple(binding),
            ) => HirItem::Param {
                binding,
                name: name.node.clone(),
                ty: lower_type(ty),
                default: default.as_ref().map(|d| self.lower_expr(d)),
                span: *span,
            },
            (
                Item::Fn {
                    is_pure,
                    name,
                    type_params,
                    params,
                    return_ty,
                    body,
                    span,
                },
                DeclaredItem::Generic {
                    own: binding,
                    type_params: type_param_ids,
                },
            ) => {
                self.push_scope();
                let params = self.lower_fn_params(params);
                let body = self.lower_block(body);
                self.pop_scope();
                HirItem::Fn {
                    binding,
                    name: name.node.clone(),
                    is_pure: *is_pure,
                    type_params: lower_type_params(type_params, &type_param_ids),
                    params,
                    return_ty: return_ty.as_ref().map(lower_type),
                    body: FunctionImplementation::Aicad(body),
                    span: *span,
                }
            }
            (
                Item::Struct {
                    name,
                    type_params,
                    fields,
                    span,
                },
                DeclaredItem::Generic {
                    own: binding,
                    type_params: type_param_ids,
                },
            ) => HirItem::Struct {
                binding,
                name: name.node.clone(),
                type_params: lower_type_params(type_params, &type_param_ids),
                fields: fields
                    .iter()
                    .map(|field| HirField {
                        name: field.name.node.clone(),
                        ty: lower_type(&field.ty),
                        span: field.span,
                    })
                    .collect(),
                span: *span,
            },
            (
                Item::Enum {
                    name,
                    type_params,
                    variants,
                    span,
                },
                DeclaredItem::Enum {
                    own,
                    type_params: type_param_ids,
                    variants: variant_ids,
                },
            ) => HirItem::Enum {
                binding: own,
                name: name.node.clone(),
                type_params: lower_type_params(type_params, &type_param_ids),
                variants: variants
                    .iter()
                    .zip(variant_ids)
                    .map(|(variant, id)| HirEnumVariant {
                        binding: id,
                        name: variant.name().node.clone(),
                        payload: lower_variant_payload(variant),
                        span: variant.span(),
                    })
                    .collect(),
                span: *span,
            },
            (Item::Part { name, items, span }, DeclaredItem::Simple(binding)) => {
                self.push_scope();
                let items = self.lower_items(items);
                self.pop_scope();
                HirItem::Part {
                    binding,
                    name: name.node.clone(),
                    items,
                    span: *span,
                }
            }
            (Item::Import { path, names, span }, DeclaredItem::Import { names: ids }) => {
                let imported = match names {
                    Some(ns) => ns
                        .iter()
                        .zip(ids)
                        .map(|(name, id)| HirImportedName {
                            binding: id,
                            name: name.node.clone(),
                            span: name.span,
                        })
                        .collect(),
                    None => Vec::new(),
                };
                HirItem::Import {
                    path: lower_import_path(path),
                    names: imported,
                    span: *span,
                }
            }
            _ => unreachable!(
                "declare_item and lower_item_body always produce a matching DeclaredItem shape for the same Item"
            ),
        }
    }

    /// Lowers each default expression *before* declaring that parameter's
    /// own name, so `fn f(x: Int = x)`'s default resolves against the
    /// outer scope, never itself — same order `crate::binder::Binder::
    /// bind_fn_params` already establishes.
    fn lower_fn_params(&mut self, params: &[FnParam]) -> Vec<HirParam> {
        params
            .iter()
            .map(|param| {
                let default = param.default.as_ref().map(|d| self.lower_expr(d));
                let binding = self.mint(&param.name, BindingKind::Param);
                HirParam {
                    binding,
                    name: param.name.node.clone(),
                    ty: lower_type(&param.ty),
                    default,
                    span: param.span,
                }
            })
            .collect()
    }

    // --- Blocks / statements ---

    fn lower_block(&mut self, block: &Block) -> HirBlock {
        self.push_scope();
        let stmts = block.stmts.iter().map(|s| self.lower_stmt(s)).collect();
        self.pop_scope();
        HirBlock {
            stmts,
            trailing: None,
            span: block.span,
        }
    }

    fn lower_block_expr(&mut self, block: &BlockExpr) -> HirBlock {
        self.push_scope();
        let stmts = block.stmts.iter().map(|s| self.lower_stmt(s)).collect();
        let trailing = block
            .trailing
            .as_ref()
            .map(|t| Box::new(self.lower_expr(t)));
        self.pop_scope();
        HirBlock {
            stmts,
            trailing,
            span: block.span,
        }
    }

    fn lower_stmt(&mut self, stmt: &Stmt) -> HirStmt {
        match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                span,
            } => {
                let value = self.lower_expr(value);
                let binding = self.mint(name, BindingKind::Let);
                HirStmt::Let {
                    binding,
                    name: name.node.clone(),
                    ty: ty.as_ref().map(lower_type),
                    value,
                    span: *span,
                }
            }
            Stmt::Var {
                name,
                ty,
                value,
                span,
            } => {
                let value = self.lower_expr(value);
                let binding = self.mint(name, BindingKind::Var);
                HirStmt::Var {
                    binding,
                    name: name.node.clone(),
                    ty: ty.as_ref().map(lower_type),
                    value,
                    span: *span,
                }
            }
            Stmt::Assign { name, value, span } => {
                let value = self.lower_expr(value);
                let target = self.resolve_or_diagnose(name);
                HirStmt::Assign {
                    target,
                    name: name.node.clone(),
                    value,
                    span: *span,
                }
            }
            Stmt::Expr { expr, span } => HirStmt::Expr {
                expr: self.lower_expr(expr),
                span: *span,
            },
            Stmt::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => {
                let cond = self.lower_expr(cond);
                let then_branch = self.lower_block(then_branch);
                let else_branch = else_branch
                    .as_ref()
                    .map(|clause| self.lower_else_clause(clause));
                HirStmt::If {
                    cond,
                    then_branch,
                    else_branch,
                    span: *span,
                }
            }
            Stmt::For {
                var,
                iterable,
                body,
                span,
            } => {
                let iterable = self.lower_expr(iterable);
                self.push_scope();
                let binding = self.mint(var, BindingKind::ForLoopVar);
                let body = self.lower_block(body);
                self.pop_scope();
                HirStmt::For {
                    binding,
                    var: var.node.clone(),
                    iterable,
                    body,
                    span: *span,
                }
            }
            Stmt::While { cond, body, span } => HirStmt::While {
                cond: self.lower_expr(cond),
                body: self.lower_block(body),
                span: *span,
            },
            Stmt::Loop { body, span } => HirStmt::Loop {
                body: self.lower_block(body),
                span: *span,
            },
            Stmt::Match {
                scrutinee,
                arms,
                span,
            } => HirStmt::Match {
                scrutinee: self.lower_expr(scrutinee),
                arms: arms.iter().map(|arm| self.lower_match_arm(arm)).collect(),
                span: *span,
            },
            Stmt::Return { value, span } => HirStmt::Return {
                value: value.as_ref().map(|v| self.lower_expr(v)),
                span: *span,
            },
            Stmt::Break { span } => HirStmt::Break { span: *span },
            Stmt::Continue { span } => HirStmt::Continue { span: *span },
        }
    }

    fn lower_else_clause(&mut self, clause: &ElseClause) -> HirElseStmt {
        match clause {
            ElseClause::Block(block) => HirElseStmt::Block(self.lower_block(block)),
            ElseClause::If(stmt) => HirElseStmt::If(Box::new(self.lower_stmt(stmt))),
        }
    }

    // --- Expressions ---

    fn lower_expr(&mut self, expr: &Expr) -> HirExpr {
        match expr {
            Expr::Literal(lit) => {
                let value = lower_literal(&lit.node);
                let ty = literal_type(&value);
                HirExpr::Literal {
                    value,
                    ty,
                    span: lit.span,
                }
            }
            Expr::Ident(name) => {
                let binding = self.resolve_or_diagnose(name);
                HirExpr::Ident {
                    name: name.node.clone(),
                    binding,
                    span: name.span,
                }
            }
            Expr::Unary {
                op, operand, span, ..
            } => HirExpr::Unary {
                op: *op,
                operand: Box::new(self.lower_expr(operand)),
                span: *span,
            },
            Expr::Binary {
                op, lhs, rhs, span, ..
            } => HirExpr::Binary {
                op: *op,
                lhs: Box::new(self.lower_expr(lhs)),
                rhs: Box::new(self.lower_expr(rhs)),
                span: *span,
            },
            // Grouping carries no semantic meaning beyond its inner
            // expression (`cad_ast::Expr::Paren`'s own doc comment); HIR
            // has no formatter/round-trip requirement to preserve it for
            // (that already happened at the AST pretty-printer layer), so
            // lowering discards the wrapper and keeps only `inner` — see
            // task report "Decisions".
            Expr::Paren { inner, .. } => self.lower_expr(inner),
            Expr::Call { callee, args, span } => {
                let binding = self.resolve_or_diagnose(callee);
                let args = args.iter().map(|arg| self.lower_arg(arg)).collect();
                HirExpr::Call {
                    callee: HirCallee::Fn {
                        name: callee.node.clone(),
                        binding,
                        span: callee.span,
                    },
                    args,
                    span: *span,
                }
            }
            // Desugars `receiver.method(args)` into ordinary call shape
            // (AGENTS.md HIR invariant) — the receiver becomes the
            // desugared call's first argument. See `crate::hir::
            // HirCallee::Method`'s own doc comment.
            Expr::MethodCall {
                receiver,
                method,
                args,
                span,
            } => {
                let mut lowered_args = Vec::with_capacity(args.len() + 1);
                lowered_args.push(HirArg::Positional(self.lower_expr(receiver)));
                lowered_args.extend(args.iter().map(|arg| self.lower_arg(arg)));
                HirExpr::Call {
                    callee: HirCallee::Method {
                        name: method.node.clone(),
                        span: method.span,
                    },
                    args: lowered_args,
                    span: *span,
                }
            }
            Expr::Field {
                receiver,
                field,
                span,
            } => HirExpr::Field {
                receiver: Box::new(self.lower_expr(receiver)),
                field: field.node.clone(),
                span: *span,
            },
            Expr::Block(block_expr) => HirExpr::Block(self.lower_block_expr(block_expr)),
            Expr::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => {
                let cond = Box::new(self.lower_expr(cond));
                let then_branch = self.lower_block_expr(then_branch);
                let else_branch = Box::new(self.lower_else_branch(else_branch));
                HirExpr::If {
                    cond,
                    then_branch,
                    else_branch,
                    span: *span,
                }
            }
            Expr::Match {
                scrutinee,
                arms,
                span,
            } => HirExpr::Match {
                scrutinee: Box::new(self.lower_expr(scrutinee)),
                arms: arms.iter().map(|arm| self.lower_match_arm(arm)).collect(),
                span: *span,
            },
            Expr::ListLiteral { elements, span } => HirExpr::ListLiteral {
                elements: elements.iter().map(|e| self.lower_expr(e)).collect(),
                span: *span,
            },
            Expr::Range {
                start,
                end,
                inclusive,
                span,
            } => HirExpr::Range {
                start: Box::new(self.lower_expr(start)),
                end: Box::new(self.lower_expr(end)),
                inclusive: *inclusive,
                span: *span,
            },
            Expr::RecordLiteral { name, fields, span } => {
                let binding = self.resolve_or_diagnose(name);
                let fields = fields
                    .iter()
                    .map(|(field_name, value)| HirRecordField {
                        name: field_name.node.clone(),
                        name_span: field_name.span,
                        value: self.lower_expr(value),
                    })
                    .collect();
                HirExpr::RecordLiteral {
                    name: name.node.clone(),
                    binding,
                    fields,
                    span: *span,
                }
            }
        }
    }

    /// See `crate::hir::HirExpr::If`'s doc comment "Value-semantics
    /// unification": both `ElseBranch` shapes already collapse into a
    /// plain `HirExpr`.
    fn lower_else_branch(&mut self, branch: &ElseBranch) -> HirExpr {
        match branch {
            ElseBranch::Block(block_expr) => HirExpr::Block(self.lower_block_expr(block_expr)),
            ElseBranch::If(expr) => self.lower_expr(expr),
        }
    }

    fn lower_arg(&mut self, arg: &Arg) -> HirArg {
        match arg {
            Arg::Positional(expr) => HirArg::Positional(self.lower_expr(expr)),
            Arg::Named { name, value } => HirArg::Named {
                name: name.node.clone(),
                name_span: name.span,
                value: self.lower_expr(value),
            },
        }
    }

    fn lower_match_arm(&mut self, arm: &MatchArm) -> HirMatchArm {
        self.push_scope();
        let pattern = self.lower_pattern(&arm.pattern);
        let body = match &arm.body {
            MatchArmBody::Expr(expr) => self.lower_expr(expr),
            MatchArmBody::Block(block_expr) => HirExpr::Block(self.lower_block_expr(block_expr)),
        };
        self.pop_scope();
        HirMatchArm {
            pattern,
            body,
            span: arm.span,
        }
    }

    /// See `crate::hir::HirPattern`'s own doc comment for why this splits
    /// into two unambiguous variants instead of keeping the AST's single
    /// ambiguous `Pattern::Ident` shape.
    fn lower_pattern(&mut self, pattern: &Pattern) -> HirPattern {
        match pattern {
            Pattern::Wildcard(span) => HirPattern::Wildcard { span: *span },
            Pattern::Literal(lit) => HirPattern::Literal {
                value: lower_literal(&lit.node),
                span: lit.span,
            },
            Pattern::Ident(name) => {
                let existing = self.resolve(&name.node);
                let is_variant = matches!(
                    existing.map(|id| self.kind_of(id)),
                    Some(BindingKind::EnumVariant { .. })
                );
                if is_variant {
                    HirPattern::Variant {
                        name: name.node.clone(),
                        variant: existing.expect("is_variant is only true when existing is Some"),
                        span: name.span,
                    }
                } else {
                    let binding = self.mint(name, BindingKind::MatchBinding);
                    HirPattern::Binding {
                        name: name.node.clone(),
                        binding,
                        span: name.span,
                    }
                }
            }
            Pattern::Tuple { name, elems, span } => {
                let variant = self.resolve_or_diagnose(name);
                let elems = elems.iter().map(|e| self.lower_pattern(e)).collect();
                HirPattern::Tuple {
                    name: name.node.clone(),
                    variant,
                    elems,
                    span: *span,
                }
            }
            Pattern::Record { name, fields, span } => {
                let variant = self.resolve_or_diagnose(name);
                let fields = fields
                    .iter()
                    .map(|f| HirRecordPatternField {
                        name: f.name.node.clone(),
                        pattern: self.lower_pattern(&f.pattern),
                        span: f.span,
                    })
                    .collect();
                HirPattern::Record {
                    name: name.node.clone(),
                    variant,
                    fields,
                    span: *span,
                }
            }
        }
    }
}

// --- Free helpers (no scope state needed) ---

fn lower_literal(literal: &Literal) -> HirLiteral {
    match literal {
        Literal::Number { text, unit } => HirLiteral::Number {
            text: text.clone(),
            unit: unit.clone(),
        },
        Literal::Str(s) => HirLiteral::Str(s.clone()),
        Literal::RawStr(s) => HirLiteral::RawStr(s.clone()),
        Literal::Bool(b) => HirLiteral::Bool(*b),
    }
}

/// Resolves a literal's `HirType` when lowering alone can determine it
/// unambiguously (see module doc comment "Scope boundary" and `crate::
/// types`'s own module doc comment):
///
/// - `Bool`/`Str`/`RawStr` always resolve to the obvious `PrimitiveType`.
/// - A `Number` with no unit suffix is left `None` — which numeric
///   `PrimitiveType` it defaults to is `AICAD-052`'s job.
/// - A `Number` with a unit suffix resolves only when `cad_units::
///   lookup_any` finds *exactly one* dimension for that symbol (e.g.
///   `"mm"` -> `Length`); an unknown symbol (no match) or an ambiguous one
///   (`"Pa"` matches both `Pressure` and `Stress` — `cad_units::registry`'s
///   own documented multiplicity) is left `None` rather than guessed at,
///   per `AGENTS.md`'s "ambiguity is an error, never an arbitrary
///   selection" (mirroring `cad_units::arithmetic`'s own identical rule
///   for derived dimensions). An affine dimension's literal defaults to
///   `AffineKind::Absolute` — RFC-0004 §7's affine quantities are absolute
///   unless produced by subtraction, and a bare literal is never a
///   subtraction result.
fn literal_type(literal: &HirLiteral) -> Option<HirType> {
    match literal {
        HirLiteral::Bool(_) => Some(HirType::Scalar(PrimitiveType::Bool)),
        HirLiteral::Str(_) | HirLiteral::RawStr(_) => Some(HirType::Scalar(PrimitiveType::String)),
        HirLiteral::Number { unit: None, .. } => None,
        HirLiteral::Number {
            unit: Some(symbol), ..
        } => {
            let mut candidates = cad_units::lookup_any(symbol);
            let first = candidates.next()?;
            if candidates.next().is_some() {
                None
            } else {
                let affine = if first.dimension.is_affine() {
                    Some(AffineKind::Absolute)
                } else {
                    None
                };
                Some(HirType::dimensional(first.dimension, affine))
            }
        }
    }
}

fn lower_type(ty: &Type) -> HirTypeRef {
    match ty {
        Type::Named(name) => HirTypeRef::Named {
            name: name.node.clone(),
            span: name.span,
        },
        Type::Generic { name, args, span } => HirTypeRef::Generic {
            name: name.node.clone(),
            args: args.iter().map(lower_type).collect(),
            span: *span,
        },
    }
}

/// Lowers one [`EnumVariant`]'s declared shape to [`HirVariantPayload`]
/// (`AICAD-057C`, `project/OWNER_DECISIONS.md#D17`) — purely syntactic,
/// same as `lower_type`; resolving each field's `HirTypeRef` is
/// `cad_hir::typeck`'s job.
fn lower_variant_payload(variant: &EnumVariant) -> HirVariantPayload {
    match variant {
        EnumVariant::Unit(_) => HirVariantPayload::Unit,
        EnumVariant::Tuple { fields, .. } => {
            HirVariantPayload::Tuple(fields.iter().map(lower_type).collect())
        }
        EnumVariant::Record { fields, .. } => HirVariantPayload::Record(
            fields
                .iter()
                .map(|f| HirField {
                    name: f.name.node.clone(),
                    ty: lower_type(&f.ty),
                    span: f.span,
                })
                .collect(),
        ),
    }
}

fn lower_import_path(path: &cad_ast::ImportPath) -> HirImportPath {
    match path {
        cad_ast::ImportPath::Package { segments, span } => HirImportPath::Package {
            segments: segments.iter().map(|s| s.node.clone()).collect(),
            span: *span,
        },
        cad_ast::ImportPath::Relative {
            up_levels,
            segments,
            span,
        } => HirImportPath::Relative {
            up_levels: *up_levels,
            segments: segments.iter().map(|s| s.node.clone()).collect(),
            span: *span,
        },
    }
}

fn severity_letter(severity: Severity) -> SeverityLetter {
    match severity {
        Severity::Error => SeverityLetter::Error,
        Severity::Warning => SeverityLetter::Warning,
        Severity::Info => SeverityLetter::Info,
    }
}

/// Builds a source-spanned diagnostic — mirrors `crate::binder`'s own
/// `diagnostic` helper exactly (family `TYPE`, category `"hir-lowering"`).
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
    Diagnostic::new(code, severity, "hir-lowering", title, message)
        .expect("severity_letter always agrees with the severity passed in")
        .with_source(SourceSpan {
            file: file.to_string(),
            start: Position::new(start.line, start.column),
            end: Position::new(end.line, end.column),
        })
}

fn unresolved_binding_diagnostic(file: &str, source: &str, name: &Spanned<String>) -> Diagnostic {
    diagnostic(
        410,
        Severity::Error,
        "UNRESOLVED_BINDING",
        format!(
            "Cannot find '{}' in this scope while lowering to HIR.",
            name.node
        ),
        file,
        source,
        name.span,
    )
}

/// `AICAD-057B`, `project/OWNER_DECISIONS.md#D17`: a `fn`/`struct`/`enum`
/// declared the same generic type-parameter name twice (`struct Foo<T,
/// T>`).
fn duplicate_type_parameter_diagnostic(
    file: &str,
    source: &str,
    duplicate: &Spanned<String>,
    first_span: Span,
) -> Diagnostic {
    let line_index = cad_ast::LineIndex::new(source);
    let first_start = line_index.line_column(source, first_span.start);
    diagnostic(
        445,
        Severity::Error,
        "DUPLICATE_TYPE_PARAMETER",
        format!(
            "type parameter '{}' is declared more than once (first declared at line {}, column {}).",
            duplicate.node, first_start.line, first_start.column
        ),
        file,
        source,
        duplicate.span,
    )
}

/// Zips a declaration's own syntactic type-parameter names with the fresh
/// `BindingId`s `Lowerer::mint_type_params` already minted for them, in
/// the same order, into the HIR-layer `HirTypeParam` list.
fn lower_type_params(type_params: &[Spanned<String>], ids: &[BindingId]) -> Vec<HirTypeParam> {
    type_params
        .iter()
        .zip(ids)
        .map(|(param, &binding)| HirTypeParam {
            binding,
            name: param.node.clone(),
            span: param.span,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::BindingKind;
    use cad_types::Dimension;

    fn lower(source: &str) -> LowerResult {
        let (program, parse_diagnostics) = cad_parser::parse_program(source, "test.aicad");
        assert!(
            parse_diagnostics.is_empty(),
            "test source failed to parse: {parse_diagnostics:?}"
        );
        lower_program(&program, "test.aicad", source)
    }

    fn codes(diagnostics: &[Diagnostic]) -> Vec<String> {
        diagnostics.iter().map(|d| d.code.as_string()).collect()
    }

    // --- Literals with units (typed engineering quantities) ---

    #[test]
    fn length_literal_resolves_to_dimensional_length() {
        let result = lower("let width = 5mm;");
        let HirItem::Let { value, .. } = &result.program.items[0] else {
            panic!("expected Let item");
        };
        let HirExpr::Literal { value: lit, ty, .. } = value else {
            panic!("expected Literal expr");
        };
        assert_eq!(
            *lit,
            HirLiteral::Number {
                text: "5".to_string(),
                unit: Some("mm".to_string()),
            }
        );
        assert_eq!(*ty, Some(HirType::dimensional(Dimension::Length, None)));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn affine_temperature_literal_defaults_to_absolute() {
        let result = lower("let t = 20degC;");
        let HirItem::Let { value, .. } = &result.program.items[0] else {
            panic!("expected Let item");
        };
        let HirExpr::Literal { ty, .. } = value else {
            panic!("expected Literal expr");
        };
        assert_eq!(
            *ty,
            Some(HirType::dimensional(
                Dimension::Temperature,
                Some(AffineKind::Absolute)
            ))
        );
    }

    #[test]
    fn ambiguous_unit_literal_is_left_unresolved() {
        // "Pa" matches both Pressure and Stress (cad_units::registry's own
        // documented multiplicity) — a skeleton lowering pass must not
        // guess, per AGENTS.md's "ambiguity is an error, never an
        // arbitrary selection".
        let result = lower("let p = 5Pa;");
        let HirItem::Let { value, .. } = &result.program.items[0] else {
            panic!("expected Let item");
        };
        let HirExpr::Literal { ty, .. } = value else {
            panic!("expected Literal expr");
        };
        assert_eq!(*ty, None);
    }

    #[test]
    fn unknown_unit_symbol_is_left_unresolved() {
        let result = lower("let x = 5xyz;");
        let HirItem::Let { value, .. } = &result.program.items[0] else {
            panic!("expected Let item");
        };
        let HirExpr::Literal { ty, .. } = value else {
            panic!("expected Literal expr");
        };
        assert_eq!(*ty, None);
    }

    #[test]
    fn unitless_number_literal_type_defaulting_is_deferred() {
        // Int vs. Float literal-type defaulting is explicitly AICAD-052's
        // job (cad_units::arithmetic's own documented scope boundary).
        let result = lower("let x = 5;");
        let HirItem::Let { value, .. } = &result.program.items[0] else {
            panic!("expected Let item");
        };
        let HirExpr::Literal { ty, .. } = value else {
            panic!("expected Literal expr");
        };
        assert_eq!(*ty, None);
    }

    #[test]
    fn bool_and_string_literals_resolve_to_scalar_types() {
        let result = lower(r#"let a = true; let b = "hi"; let c = r"raw";"#);
        let expect_scalar = |item: &HirItem, expected: PrimitiveType| {
            let HirItem::Let { value, .. } = item else {
                panic!("expected Let item");
            };
            let HirExpr::Literal { ty, .. } = value else {
                panic!("expected Literal expr");
            };
            assert_eq!(*ty, Some(HirType::Scalar(expected)));
        };
        expect_scalar(&result.program.items[0], PrimitiveType::Bool);
        expect_scalar(&result.program.items[1], PrimitiveType::String);
        expect_scalar(&result.program.items[2], PrimitiveType::String);
    }

    // --- Declarations / explicit binding identity ---

    #[test]
    fn let_and_later_reference_share_the_same_binding_id() {
        let result = lower("let width = 5mm; fn area() -> Length { width; }");
        let HirItem::Let { binding, .. } = &result.program.items[0] else {
            panic!("expected Let item");
        };
        let HirItem::Fn { body, .. } = &result.program.items[1] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        let HirStmt::Expr { expr, .. } = &body.stmts[0] else {
            panic!("expected Expr stmt");
        };
        let HirExpr::Ident { binding: used, .. } = expr else {
            panic!("expected Ident expr");
        };
        assert_eq!(Some(*binding), *used);
    }

    #[test]
    fn forward_reference_between_sibling_fns_resolves_to_a_real_binding() {
        let result = lower("fn a() -> Int { b(); } fn b() -> Int { 1; }");
        let HirItem::Fn {
            binding: b_binding, ..
        } = &result.program.items[1]
        else {
            panic!("expected Fn item");
        };
        let HirItem::Fn { body, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        let HirStmt::Expr { expr, .. } = &body.stmts[0] else {
            panic!("expected Expr stmt");
        };
        let HirExpr::Call { callee, .. } = expr else {
            panic!("expected Call expr");
        };
        let HirCallee::Fn { binding, .. } = callee else {
            panic!("expected Fn callee");
        };
        assert_eq!(*binding, Some(*b_binding));
    }

    #[test]
    fn fn_param_gets_its_own_binding_kind() {
        let result = lower("fn f(x: Int) -> Int { x; }");
        let HirItem::Fn { params, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        let binding_id = params[0].binding;
        assert_eq!(result.bindings[binding_id.index()].kind, BindingKind::Param);
        assert_eq!(result.bindings[binding_id.index()].name, "x");
    }

    #[test]
    fn struct_and_enum_declarations_lower_with_bindings() {
        let result = lower("struct Point { x: Float, y: Float } enum Size { Small, Large }");
        let HirItem::Struct {
            binding, fields, ..
        } = &result.program.items[0]
        else {
            panic!("expected Struct item");
        };
        assert_eq!(result.bindings[binding.index()].kind, BindingKind::Struct);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "x");

        let HirItem::Enum {
            binding, variants, ..
        } = &result.program.items[1]
        else {
            panic!("expected Enum item");
        };
        assert_eq!(result.bindings[binding.index()].kind, BindingKind::Enum);
        assert_eq!(variants.len(), 2);
        assert_eq!(
            result.bindings[variants[0].binding.index()].kind,
            BindingKind::EnumVariant {
                enum_name: "Size".to_string()
            }
        );
    }

    #[test]
    fn const_and_item_level_param_lower_with_bindings() {
        let result = lower("const A = 1; param width: Length = 5mm;");
        let HirItem::Const { binding, .. } = &result.program.items[0] else {
            panic!("expected Const item");
        };
        assert_eq!(result.bindings[binding.index()].kind, BindingKind::Const);
        let HirItem::Param { binding, ty, .. } = &result.program.items[1] else {
            panic!("expected Param item");
        };
        assert_eq!(result.bindings[binding.index()].kind, BindingKind::Param);
        assert!(matches!(ty, HirTypeRef::Named { name, .. } if name == "Length"));
    }

    #[test]
    fn part_body_sees_module_scope_and_gets_its_own_bindings() {
        let result = lower("let width = 5mm; part Box { fn area() -> Length { width; } }");
        let HirItem::Part { items, .. } = &result.program.items[1] else {
            panic!("expected Part item");
        };
        let HirItem::Fn { body, .. } = &items[0] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        let HirStmt::Expr { expr, .. } = &body.stmts[0] else {
            panic!("expected Expr stmt");
        };
        let HirExpr::Ident { binding, .. } = expr else {
            panic!("expected Ident expr");
        };
        assert!(binding.is_some(), "part body must see module-scope `width`");
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn selective_import_binds_a_name_whole_module_import_binds_nothing() {
        let result = lower(
            "import std.fasteners::{ISO4762}; import robotics.cycloidal; fn f() -> Int { ISO4762; }",
        );
        let HirItem::Import { names, .. } = &result.program.items[0] else {
            panic!("expected Import item");
        };
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].name, "ISO4762");
        let HirItem::Import { names, .. } = &result.program.items[1] else {
            panic!("expected Import item");
        };
        assert!(names.is_empty(), "whole-module import binds no name");
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    // --- Control flow / value semantics ---

    #[test]
    fn statement_block_never_has_a_trailing_value() {
        let result = lower("fn f() -> Int { let a = 1; a; }");
        let HirItem::Fn { body, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        assert!(body.trailing.is_none());
        assert_eq!(body.stmts.len(), 2);
    }

    #[test]
    fn expression_block_carries_its_trailing_value() {
        let result = lower("fn f() -> Int { let r = { let a = 1; a }; r; }");
        let HirItem::Fn { body, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        let HirStmt::Let { value, .. } = &body.stmts[0] else {
            panic!("expected Let stmt");
        };
        let HirExpr::Block(inner) = value else {
            panic!("expected Block expr");
        };
        assert!(inner.trailing.is_some());
    }

    #[test]
    fn if_expr_else_if_chain_lowers_to_nested_if_exprs() {
        let result =
            lower("fn f(c: Bool) -> Int { let r = if c { 1 } else if c { 2 } else { 3 }; r; }");
        let HirItem::Fn { body, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        let HirStmt::Let { value, .. } = &body.stmts[0] else {
            panic!("expected Let stmt");
        };
        let HirExpr::If { else_branch, .. } = value else {
            panic!("expected If expr");
        };
        assert!(
            matches!(else_branch.as_ref(), HirExpr::If { .. }),
            "else-if chain must lower to a nested HirExpr::If, not a wrapper type"
        );
    }

    #[test]
    fn if_stmt_else_clause_lowers_to_hir_else_stmt() {
        let result = lower("fn f(c: Bool) -> Int { if c { 1; } else { 2; } 0; }");
        let HirItem::Fn { body, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        let HirStmt::If { else_branch, .. } = &body.stmts[0] else {
            panic!("expected If stmt");
        };
        assert!(matches!(else_branch, Some(HirElseStmt::Block(_))));
    }

    #[test]
    fn for_loop_variable_gets_its_own_binding_scoped_to_the_body() {
        let result = lower("fn f(xs: Int) -> Int { for x in xs { x; } 0; }");
        let HirItem::Fn { body, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        let HirStmt::For { binding, body, .. } = &body.stmts[0] else {
            panic!("expected For stmt");
        };
        let HirStmt::Expr { expr, .. } = &body.stmts[0] else {
            panic!("expected Expr stmt");
        };
        let HirExpr::Ident { binding: used, .. } = expr else {
            panic!("expected Ident expr");
        };
        assert_eq!(Some(*binding), *used);
    }

    #[test]
    fn while_and_loop_bodies_lower() {
        let result =
            lower("fn f() -> Int { while true { let a = 1; a; } loop { let a = 2; a; } 0; }");
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let HirItem::Fn { body, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        assert!(matches!(body.stmts[0], HirStmt::While { .. }));
        assert!(matches!(body.stmts[1], HirStmt::Loop { .. }));
    }

    // --- Match: variant vs. fresh binding ---

    #[test]
    fn match_arm_identifier_matching_a_variant_references_the_variant_binding() {
        let result = lower(
            "enum MotorSize { NEMA17, NEMA23 } fn f(m: MotorSize) -> Int { match m { NEMA17 => 1, NEMA23 => 2, } 0; }",
        );
        let HirItem::Enum { variants, .. } = &result.program.items[0] else {
            panic!("expected Enum item");
        };
        let nema17_id = variants[0].binding;
        let HirItem::Fn { body, .. } = &result.program.items[1] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        let HirStmt::Match { arms, .. } = &body.stmts[0] else {
            panic!("expected Match stmt");
        };
        let HirPattern::Variant { variant, .. } = &arms[0].pattern else {
            panic!("expected Variant pattern");
        };
        assert_eq!(*variant, nema17_id);
    }

    #[test]
    fn match_arm_identifier_not_matching_a_variant_is_a_fresh_binding() {
        let result = lower(
            "enum MotorSize { NEMA17 } fn f(m: MotorSize) -> MotorSize { match m { other => other, } }",
        );
        let HirItem::Fn { body, .. } = &result.program.items[1] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        // `match` used as the fn body's own last statement is still
        // statement-position `match_stmt` (`cad_ast::item::Block` has no
        // trailing-expression slot at all — see `crate::hir::HirBlock`'s
        // doc comment), so this lowers to `HirStmt::Match` directly, not
        // `HirStmt::Expr` wrapping a `HirExpr::Match`.
        let HirStmt::Match { arms, .. } = &body.stmts[0] else {
            panic!("expected Match stmt");
        };
        let HirPattern::Binding { binding, name, .. } = &arms[0].pattern else {
            panic!("expected Binding pattern");
        };
        assert_eq!(name, "other");
        let HirExpr::Ident { binding: used, .. } = &arms[0].body else {
            panic!("expected Ident body");
        };
        assert_eq!(*used, Some(*binding));
    }

    // --- Method-call desugaring (AGENTS.md HIR invariant) ---

    #[test]
    fn method_call_desugars_to_a_functional_call_with_receiver_as_first_arg() {
        let result = lower("fn f(x: Int, y: Int) -> Int { x.frobnicate(y); }");
        let HirItem::Fn { body, params, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        let HirStmt::Expr { expr, .. } = &body.stmts[0] else {
            panic!("expected Expr stmt");
        };
        let HirExpr::Call { callee, args, .. } = expr else {
            panic!("expected Call expr");
        };
        let HirCallee::Method { name, .. } = callee else {
            panic!("method call must desugar to a HirCallee::Method, not HirCallee::Fn");
        };
        assert_eq!(name, "frobnicate");
        assert_eq!(args.len(), 2, "receiver must be prepended as args[0]");
        let HirArg::Positional(receiver_expr) = &args[0] else {
            panic!("expected Positional arg");
        };
        let HirExpr::Ident {
            binding: receiver_binding,
            ..
        } = receiver_expr
        else {
            panic!("expected Ident expr");
        };
        assert_eq!(*receiver_binding, Some(params[0].binding));
    }

    #[test]
    fn field_access_is_not_desugared_to_a_call() {
        let result = lower("fn f(x: Int) -> Int { x.frobnicate; }");
        let HirItem::Fn { body, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        let HirStmt::Expr { expr, .. } = &body.stmts[0] else {
            panic!("expected Expr stmt");
        };
        assert!(matches!(expr, HirExpr::Field { .. }));
    }

    // --- Grouping is discarded, spans still survive lowering ---

    #[test]
    fn paren_grouping_is_discarded_keeping_only_the_inner_expr() {
        let result = lower("fn f() -> Int { (1 + 2); }");
        let HirItem::Fn { body, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        let HirStmt::Expr { expr, .. } = &body.stmts[0] else {
            panic!("expected Expr stmt");
        };
        assert!(matches!(expr, HirExpr::Binary { .. }));
    }

    #[test]
    fn spans_survive_lowering() {
        let source = "let width = 5mm;";
        let result = lower(source);
        let HirItem::Let {
            name, value, span, ..
        } = &result.program.items[0]
        else {
            panic!("expected Let item");
        };
        assert_eq!(span.text(source), "let width = 5mm;");
        assert_eq!(*name, "width");
        assert_eq!(value.span().text(source), "5mm");
    }

    // --- Negative / adversarial: unresolved names ---

    #[test]
    fn undefined_identifier_lowers_with_no_binding_and_a_diagnostic() {
        let result = lower("fn f() -> Int { missing; }");
        let HirItem::Fn { body, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        let HirStmt::Expr { expr, .. } = &body.stmts[0] else {
            panic!("expected Expr stmt");
        };
        let HirExpr::Ident { binding, .. } = expr else {
            panic!("expected Ident expr");
        };
        assert_eq!(*binding, None);
        assert_eq!(codes(&result.diagnostics), vec!["TYPE-E410"]);
    }

    #[test]
    fn undefined_call_callee_lowers_with_no_binding_and_a_diagnostic() {
        let result = lower("fn f() -> Int { missing_fn(); }");
        let HirItem::Fn { body, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        let HirStmt::Expr { expr, .. } = &body.stmts[0] else {
            panic!("expected Expr stmt");
        };
        let HirExpr::Call { callee, .. } = expr else {
            panic!("expected Call expr");
        };
        let HirCallee::Fn { binding, .. } = callee else {
            panic!("expected Fn callee");
        };
        assert_eq!(*binding, None);
        assert_eq!(codes(&result.diagnostics), vec!["TYPE-E410"]);
    }

    #[test]
    fn undefined_assign_target_lowers_with_no_binding_and_a_diagnostic() {
        let result = lower("fn f() -> Int { missing = 2; }");
        let HirItem::Fn { body, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        let FunctionImplementation::Aicad(body) = body else {
            panic!("expected an Aicad-sourced fn body in this test fixture");
        };
        let HirStmt::Assign { target, .. } = &body.stmts[0] else {
            panic!("expected Assign stmt");
        };
        assert_eq!(*target, None);
        assert_eq!(codes(&result.diagnostics), vec!["TYPE-E410"]);
    }

    #[test]
    fn method_and_field_names_are_never_checked_as_scope_names() {
        // Unlike a plain undefined identifier, an undefined method/field
        // name is never even a lowering question — it is resolved against
        // the receiver's type later (AICAD-052+), so no diagnostic fires
        // here even though `frobnicate` is declared nowhere.
        let result = lower("fn f(x: Int) -> Int { x.frobnicate(); x.frobnicate; }");
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn diagnostics_carry_a_source_span() {
        let result = lower("fn f() -> Int { missing; }");
        assert_eq!(result.diagnostics.len(), 1);
        assert!(result.diagnostics[0].source.is_some());
        assert_eq!(
            result.diagnostics[0].source.as_ref().unwrap().file,
            "test.aicad"
        );
    }

    // --- Bindings registry ---

    #[test]
    fn bindings_registry_is_indexed_by_binding_id() {
        let result = lower("let a = 1;");
        let HirItem::Let { binding, .. } = &result.program.items[0] else {
            panic!("expected Let item");
        };
        assert_eq!(result.bindings[binding.index()].id, *binding);
        assert_eq!(result.bindings[binding.index()].name, "a");
    }

    // --- AICAD-057B: generic type-parameter lowering
    //     (project/OWNER_DECISIONS.md#D17) ------------------------------

    #[test]
    fn generic_struct_type_params_mint_bindings_with_type_param_kind() {
        let result = lower("struct Pair<T, U> { first: T, second: U }");
        let HirItem::Struct { type_params, .. } = &result.program.items[0] else {
            panic!("expected Struct item");
        };
        assert_eq!(type_params.len(), 2);
        assert_eq!(type_params[0].name, "T");
        assert_eq!(type_params[1].name, "U");
        for param in type_params {
            assert_eq!(
                result.bindings[param.binding.index()].kind,
                BindingKind::TypeParam
            );
        }
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn generic_enum_type_params_are_lowered() {
        let result = lower("enum Container<T> { Empty }");
        let HirItem::Enum { type_params, .. } = &result.program.items[0] else {
            panic!("expected Enum item");
        };
        assert_eq!(type_params.len(), 1);
        assert_eq!(type_params[0].name, "T");
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn generic_fn_type_params_are_lowered() {
        let result = lower("fn identity<T>(value: T) -> T { return value; }");
        let HirItem::Fn { type_params, .. } = &result.program.items[0] else {
            panic!("expected Fn item");
        };
        assert_eq!(type_params.len(), 1);
        assert_eq!(type_params[0].name, "T");
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }

    #[test]
    fn ordinary_non_generic_declarations_lower_with_an_empty_type_param_list() {
        let result = lower("struct Point2 { x: Length } fn f() { }");
        let HirItem::Struct { type_params, .. } = &result.program.items[0] else {
            panic!("expected Struct item");
        };
        assert!(type_params.is_empty());
        let HirItem::Fn { type_params, .. } = &result.program.items[1] else {
            panic!("expected Fn item");
        };
        assert!(type_params.is_empty());
    }

    #[test]
    fn duplicate_type_parameter_name_is_reported() {
        let result = lower("struct Foo<T, T> { a: T }");
        assert_eq!(codes(&result.diagnostics), vec!["TYPE-E445"]);
        let HirItem::Struct { type_params, .. } = &result.program.items[0] else {
            panic!("expected Struct item");
        };
        // Both occurrences still each mint their own binding — lowering
        // recovers rather than dropping the second one.
        assert_eq!(type_params.len(), 2);
    }

    #[test]
    fn type_parameter_names_are_never_inserted_into_the_value_scope() {
        // `T` used as a type parameter must not shadow, or be confused
        // with, an ordinary value-level name — it never enters `Lowerer::
        // scopes` at all (see `Lowerer::mint_type_params`'s own doc
        // comment). A same-named value identifier used inside the
        // function body still resolves as an ordinary undefined-name
        // question, completely independent of the type parameter.
        let result = lower("fn identity<T>(value: T) -> T { T; return value; }");
        // `T;` as an *expression* is an undefined value identifier — the
        // type parameter `T` never leaks into value-identifier
        // resolution.
        assert_eq!(codes(&result.diagnostics), vec!["TYPE-E410"]);
    }
}
