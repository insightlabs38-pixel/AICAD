//! The AICAD prelude — `Result<T, E>`/`Optional<T>` (`AICAD-057E`,
//! `project/OWNER_DECISIONS.md#D17`, `project/DECISION_LOG.md#DL-14`).
//!
//! `D17`'s own ruling text is explicit: "`Result<T,E>` and `Optional<T>`
//! are ordinary generic prelude/library types built using the same
//! enum/generic machinery available to user code... The compiler/runtime
//! must not contain `Result`-specific semantic machinery other than
//! ordinary prelude registration/loading." This module is that
//! registration/loading, and nothing else: [`PRELUDE_SOURCE`] is **actual
//! AICAD source text**, written using exactly the general `enum`/generic
//! declaration syntax `AICAD-057B` (declaration-site type parameters) and
//! `AICAD-057C` (tuple-variant payloads) already gave every user program —
//! not a hand-built AST/HIR tree, and not a new compiler-recognized name.
//! [`with_prelude`] parses that fixed text once and prepends its two
//! `enum` items to a user's own already-parsed [`Program`], producing one
//! combined program for [`crate::lower::lower_program`]/
//! [`crate::typeck::check_program`] to process exactly as if the user had
//! written the same two declarations at the top of their own file. There
//! is no `BindingKind`, `CheckedType`, `Value`, or diagnostic anywhere in
//! this crate (or `cad-runtime`) that mentions `Result`/`Optional`/`Ok`/
//! `Err`/`Some`/`None` by name — every one of those five names reaches the
//! compiler exclusively through this module's own source text, is parsed
//! by the same `cad_parser::parse_program` any `.aicad` file goes through,
//! and is thereafter an entirely ordinary generic enum/tuple-variant/unit-
//! variant declaration indistinguishable, to every later phase, from any
//! other user-defined one (`AICAD-057F`'s own required adversarial
//! generality proof already establishes this for the general machinery;
//! this module is the concrete case that machinery now serves).
//!
//! ## Why parsed text, not hand-built AST/HIR nodes
//!
//! Constructing `Item::Enum { .. }`/`HirItem::Enum { .. }` values directly
//! in Rust would still be "ordinary enum machinery" in one sense (the same
//! node shapes), but it would silently skip the parser entirely — a
//! `Result`/`Optional`-shaped item that only the compiler's own Rust code
//! can produce, never expressible by a user typing the equivalent source,
//! is exactly the kind of `Result`-specific machinery D17 rules out. Sourcing
//! the prelude as literal `.aicad` text closes that gap completely: the two
//! declarations below are byte-for-byte what a user could paste into their
//! own file, and [`with_prelude`] does nothing beyond parsing and
//! prepending them.
//!
//! ## Where this hooks into the pipeline
//!
//! `cad-hir` has no single "compile a whole program" production entry
//! point yet — `lower_program`/`check_program` are separate phase
//! functions, and every current full-pipeline caller (this crate's and
//! `cad-runtime`'s own tests) wires `cad_parser::parse_program` ->
//! `lower_program` -> `check_program` (-> `cad_runtime::Interpreter`)
//! together itself; `cad-cli` (`AICAD-061`, the eventual `cad build`
//! driver) does not exist yet (still the `AICAD-002`/`003` scaffolding
//! placeholder, no dependencies). [`with_prelude`] is the one extra step
//! any such caller inserts between "parse the user's own source" and
//! "lower the combined program": `let program = cad_hir::prelude::
//! with_prelude(&program);` right after `cad_parser::parse_program`,
//! before the result is handed to `lower_program`. This module does not
//! call `lower_program`/`check_program` itself, so a caller that wants a
//! program *without* the prelude (every pre-existing `AICAD-052`-`057D`
//! test, which deliberately exercises the bare generic/enum machinery in
//! isolation and in several cases already uses `Ok`/`Err`/`Result` as its
//! own unrelated non-generic test fixture names — seeing them collide with
//! a silently-injected prelude would be exactly the kind of hidden
//! behavior change this task must not cause) is entirely unaffected: the
//! prelude is opt-in per call site, never automatic inside `lower_program`/
//! `check_program` themselves.

use cad_ast::Program;

/// The prelude's own literal AICAD source text — `Result<T, E>`
/// (`Ok(T)`/`Err(E)`) and `Optional<T>` (`Some(T)`/`None`), exactly the two
/// declarations `DECISION_LOG.md#DL-14` names verbatim. Ordinary tuple
/// enum-variant syntax (`AICAD-057C`) on an ordinary two/one-parameter
/// generic enum declaration (`AICAD-057B`) — nothing else.
pub const PRELUDE_SOURCE: &str = "\
enum Result<T, E> {
    Ok(T),
    Err(E),
}

enum Optional<T> {
    Some(T),
    None,
}
";

/// Parses [`PRELUDE_SOURCE`] and returns a new [`Program`] whose items are
/// the prelude's own two `enum` declarations followed by every item in
/// `user_program`, in that order. Prelude-first ordering is not itself
/// load-bearing correctness (`lower_program`/`check_program` both declare
/// every item in one program-wide pass before checking any body, so a
/// user function may already forward-reference a *later* sibling
/// regardless of order — the same reason `Ok`/`Err`/`Some`/`None`/
/// `Result`/`Optional` need no import statement or special ordering to be
/// usable anywhere in the user's own program), but it keeps the combined
/// program's own item list intuitive to read/debug, matching how a human
/// would actually lay the file out if they pasted the prelude in by hand.
///
/// A user item that happens to redeclare one of these five names is not
/// specially rejected or specially protected here — see `crate::lower`'s
/// own module doc comment ("Duplicate-declaration detection... `crate::
/// binder`'s `DUPLICATE_BINDING` already owns this check") for the
/// existing, already-documented behavior a same-named later declaration
/// gets in this pipeline (the later declaration's own mint simply
/// overwrites the earlier one in the lowerer's internal name table, no
/// diagnostic from this crate); this module adds no new special case for
/// that condition, since doing so would itself be exactly the kind of
/// `Result`-specific machinery `D17` prohibits.
///
/// # Panics
///
/// Panics if [`PRELUDE_SOURCE`] itself fails to parse. That string is
/// fixed, compiler-controlled text covered by this module's own
/// `prelude_source_parses_lowers_and_type_checks_cleanly_alone` test, never
/// user input — a parse failure here would be a genuine internal bug in
/// this module, not a reportable diagnostic about anyone's program.
pub fn with_prelude(user_program: &Program) -> Program {
    let (prelude_program, diagnostics) = cad_parser::parse_program(PRELUDE_SOURCE, "<prelude>");
    assert!(
        diagnostics.is_empty(),
        "AICAD prelude source failed to parse: {diagnostics:?}"
    );
    let mut items = prelude_program.items;
    items.extend(user_program.items.iter().cloned());
    Program { items }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prelude_source_parses_lowers_and_type_checks_cleanly_alone() {
        // The prelude's own two declarations, with no user code at all,
        // must be clean at every phase — this is the module's own
        // sanity check that `PRELUDE_SOURCE` is genuinely valid,
        // unremarkable AICAD source, not merely "close enough."
        let (program, parse_diagnostics) = cad_parser::parse_program(PRELUDE_SOURCE, "<prelude>");
        assert!(parse_diagnostics.is_empty(), "{parse_diagnostics:?}");
        let lowered = crate::lower::lower_program(&program, "<prelude>", PRELUDE_SOURCE);
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        let checked = crate::typeck::check_program(
            &lowered.program,
            &lowered.bindings,
            "<prelude>",
            PRELUDE_SOURCE,
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn with_prelude_prepends_the_two_prelude_enums_before_user_items() {
        let (user_program, diags) = cad_parser::parse_program("let x = 1;", "test.aicad");
        assert!(diags.is_empty(), "{diags:?}");
        let combined = with_prelude(&user_program);
        // Two prelude enums, then the user's own single `let`.
        assert_eq!(combined.items.len(), 3);
        assert!(matches!(combined.items[0], cad_ast::Item::Enum { .. }));
        assert!(matches!(combined.items[1], cad_ast::Item::Enum { .. }));
        assert!(matches!(combined.items[2], cad_ast::Item::Let { .. }));
    }

    #[test]
    fn with_prelude_leaves_an_empty_user_program_with_just_the_prelude() {
        let (user_program, diags) = cad_parser::parse_program("", "test.aicad");
        assert!(diags.is_empty(), "{diags:?}");
        let combined = with_prelude(&user_program);
        assert_eq!(combined.items.len(), 2);
    }

    #[test]
    fn combined_program_with_ordinary_user_code_lowers_and_type_checks_cleanly() {
        // A user program that uses `Result`/`Ok`/`Err` with no declaration
        // of its own anywhere — proves the merge actually makes the
        // prelude's names resolvable end to end (lowering + type
        // checking), not merely that the two item lists concatenate.
        let source = "fn half(n: Int) -> Result<Int, String> { \
                 if n < 0 { return Err(\"negative\"); } \
                 return Ok(n); \
             }";
        let (user_program, diags) = cad_parser::parse_program(source, "test.aicad");
        assert!(diags.is_empty(), "{diags:?}");
        let combined = with_prelude(&user_program);
        let lowered = crate::lower::lower_program(&combined, "test.aicad", source);
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        let checked =
            crate::typeck::check_program(&lowered.program, &lowered.bindings, "test.aicad", source);
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }
}
