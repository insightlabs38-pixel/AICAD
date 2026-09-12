//! Formatter round-trip / structural-equivalence tests (`AICAD-045`).
//!
//! `cad-ast` cannot depend on `cad-parser` normally (that dependency runs
//! the other way: `cad-parser` depends on `cad-ast`'s AST types), but a
//! *dev*-dependency cycle for tests only is fine — Cargo builds test
//! binaries separately from the library, so `cad-ast`'s `Cargo.toml` adds
//! `cad-parser` under `[dev-dependencies]` for exactly this file.
//!
//! **Why these tests check printed-text stability, not `Program`
//! equality**: `Span`s are byte offsets into a specific source string
//! (`crates/cad-ast/src/lib.rs`'s own doc comment on `Span`), and
//! `Item`/`Stmt`/`Expr` all derive `PartialEq` over their span fields —
//! so a freshly reprinted program's re-parsed `Program` is never `==` to
//! the original (different byte offsets), even when structurally
//! identical. The meaningful, span-independent property is: printing is
//! a pure function of AST *structure*, so reformatting already-formatted
//! output is a no-op fixed point. That is what every test here checks.

fn format_source(source: &str) -> String {
    let (program, diagnostics) = cad_parser::parse_program(source, "t.aicad");
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics parsing {source:?}: {diagnostics:?}"
    );
    cad_ast::print_program(&program)
}

/// The core round-trip property: formatting is idempotent, and its
/// output always reparses cleanly (no new diagnostics introduced by the
/// printer itself).
fn assert_round_trips(source: &str) {
    let once = format_source(source);
    let (_, diagnostics) = cad_parser::parse_program(&once, "t.aicad");
    assert!(
        diagnostics.is_empty(),
        "printer output does not reparse cleanly for {source:?}:\n{once}\ndiagnostics: {diagnostics:?}"
    );
    let twice = format_source(&once);
    assert_eq!(
        once, twice,
        "formatting is not idempotent for {source:?}:\n--- once ---\n{once}\n--- twice ---\n{twice}"
    );
}

#[test]
fn round_trips_let_const_param() {
    assert_round_trips("let width = 80mm;");
    assert_round_trips("let width: Length = 80mm;");
    assert_round_trips("const PI = 3.14;");
    assert_round_trips("param width: Length;");
    assert_round_trips("param width: Length = 80mm;");
}

#[test]
fn round_trips_fn_decl() {
    assert_round_trips("fn add(a: Length, b: Length) -> Length { let sum = a + b; }");
    assert_round_trips("pure fn noop() { }");
    assert_round_trips("fn f(a: Length = 5mm) { }");
    assert_round_trips(
        "pure fn corner_points(size: Vector2<Length>, inset: Length) -> List<Point2> { return size; }",
    );
}

#[test]
fn round_trips_struct_and_enum() {
    assert_round_trips("struct Point2 { x: Length, y: Length }");
    assert_round_trips("struct Empty { }");
    assert_round_trips("enum MotorSize { NEMA17, NEMA23 }");
}

#[test]
fn round_trips_generic_declarations() {
    // AICAD-057B, project/OWNER_DECISIONS.md#D17: generic type-parameter
    // lists on struct/enum/fn declarations.
    assert_round_trips("struct Box<T> { value: T }");
    assert_round_trips("struct Pair<T, U> { first: T, second: U }");
    assert_round_trips("enum Container<T> { Empty }");
    assert_round_trips("fn identity<T>(value: T) -> T { return value; }");
}

#[test]
fn round_trips_part_with_nested_items() {
    assert_round_trips(
        "part Bracket { param width: Length = 80mm; let wall = width; fn area() -> Length { wall; } }",
    );
    assert_round_trips("part Empty { }");
}

#[test]
fn round_trips_imports() {
    assert_round_trips("import robotics.cycloidal;");
    assert_round_trips("import std.fasteners::{ISO4762};");
    assert_round_trips("import std.fasteners::{ISO4762, ISO7380};");
    assert_round_trips("import ./housing;");
    assert_round_trips("import ./lib/housing;");
    assert_round_trips("import ../housing;");
    assert_round_trips("import ../../a/b;");
}

#[test]
fn round_trips_statements() {
    assert_round_trips("fn f() { let a = 1; var b = 2; b = 3; a; }");
    assert_round_trips("fn f() { for p in points { p; } }");
    assert_round_trips("fn f() { while a { a; } }");
    assert_round_trips("fn f() { loop { break; } }");
    assert_round_trips("fn f() { loop { continue; } }");
    assert_round_trips("fn f() -> Length { return 1mm; }");
    assert_round_trips("fn f() { return; }");
}

#[test]
fn round_trips_if_statement_forms() {
    assert_round_trips("fn f() { if a { b; } }");
    assert_round_trips("fn f() { if a { b; } else { c; } }");
    assert_round_trips("fn f() { if a { b; } else if c { d; } else { e; } }");
}

#[test]
fn round_trips_match_statement() {
    assert_round_trips("fn f() { match material { Plastic => 3mm, Aluminum => 2mm, _ => 1mm, } }");
    assert_round_trips("fn f() { match x { 1 => { a; } _ => { b; } } }");
}

#[test]
fn round_trips_expression_forms() {
    assert_round_trips("let x = 1 + 2 * 3;");
    assert_round_trips("let x = (1 + 2) * 3;");
    assert_round_trips("let x = -a;");
    assert_round_trips("let x = !flag;");
    assert_round_trips("let x = a == b && c != d;");
    assert_round_trips("let x = a ~= b;");
    assert_round_trips("let x = cut(body, hole);");
    assert_round_trips("let x = body.cut(hole);");
    assert_round_trips("let x = body.cut(hole, name = \"slot\");");
    assert_round_trips("let x = size.x - inset;");
    assert_round_trips("let x = \"hello\\nworld\\t\\\"quoted\\\"\";");
    assert_round_trips(r#"let x = r"C:\no\escapes";"#);
    assert_round_trips("let x = true;");
    assert_round_trips("let x = false;");
}

#[test]
fn round_trips_if_expr_matching_paper_example_shape() {
    // The Stage-0 paper example's own if_expr: a value bound directly
    // from an if/else, per docs/plan and examples/assemblies/
    // stage0_paper_example.aicad.
    assert_round_trips("let wall = if a { 3mm } else { 4mm };");
    assert_round_trips("let wall = if a { 3mm } else if b { 4mm } else { 5mm };");
}

#[test]
fn round_trips_match_expr() {
    assert_round_trips("let x = match m { Plastic => 3mm, Aluminum => 2mm, };");
}

#[test]
fn round_trips_block_expr_with_and_without_trailing_value() {
    assert_round_trips("let x = { let a = 1; a };");
    assert_round_trips("fn f() { { let a = 1; a; }; }");
}

#[test]
fn round_trips_a_program_combining_every_implemented_construct() {
    assert_round_trips(
        "import ./housing;\n\
         import std.fasteners::{ISO4762};\n\
         \n\
         enum MotorSize { NEMA17, NEMA23 }\n\
         \n\
         struct Point2 { x: Length, y: Length }\n\
         \n\
         const PI = 3.14;\n\
         param width: Length = 80mm;\n\
         \n\
         pure fn corner_points(size: Vector2<Length>, inset: Length) -> List<Point2> {\n\
         \x20   return size;\n\
         }\n\
         \n\
         part Bracket {\n\
         \x20   param wall: Length = if width ~= 80mm { 3mm } else { 4mm };\n\
         \x20   fn area() -> Length {\n\
         \x20       var total = 0mm;\n\
         \x20       for p in corner_points(width, wall) {\n\
         \x20           if p.x > 0mm {\n\
         \x20               total = total + p.x;\n\
         \x20           } else {\n\
         \x20               continue;\n\
         \x20           }\n\
         \x20       }\n\
         \x20       match total {\n\
         \x20           0mm => { return 0mm; }\n\
         \x20           _ => total,\n\
         \x20       }\n\
         \x20   }\n\
         }\n",
    );
}

#[test]
fn round_trips_data_carrying_enum_variants() {
    // AICAD-057C, project/OWNER_DECISIONS.md#D17: Unit/Tuple/Record
    // variant declarations.
    assert_round_trips("enum Optional<T> { Some(T), None }");
    assert_round_trips("enum Result<T, E> { Ok(T), Err(E) }");
    assert_round_trips("enum Shape { Circle { radius: Length }, Point }");
    assert_round_trips(
        "enum Message { Quit, Move { x: Length, y: Length }, Write(String), ChangeColor(Int, Int, Int) }",
    );
}

#[test]
fn round_trips_record_literal_construction() {
    assert_round_trips("let p = Point { x: 1mm, y: 2mm };");
    assert_round_trips("let p = Point {};");
}

#[test]
fn round_trips_tuple_and_record_patterns() {
    assert_round_trips("fn f() { match r { Ok(v) => v, Err(e) => e, } }");
    assert_round_trips("fn f() { match r { Ok(v) => { return v; } Err(e) => { return e; } } }");
    assert_round_trips("fn f() { match p { Point { x, y } => x, Origin => 0mm, } }");
    // Explicit (non-shorthand) record-pattern field renaming.
    assert_round_trips("fn f() { match p { Point { x: a, y: b } => a, } }");
    // Nested tuple pattern.
    assert_round_trips("fn f() { match r { Ok(Some(v)) => v, Ok(None) => 0mm, Err(e) => e, } }");
}

#[test]
fn record_literal_does_not_misparse_a_following_if_block() {
    // `Parser::no_record_literal`'s own reason for existing (`AICAD-
    // 057C`): the condition's trailing `{` must open the `if`'s block,
    // never a record literal, exactly like Rust's own identical
    // restriction.
    assert_round_trips("fn f() { if cond { a; } }");
    assert_round_trips("fn f() { while cond { a; } }");
    assert_round_trips("fn f() { match cond { _ => a, } }");
    // A record literal remains parseable as a condition when explicitly
    // parenthesized.
    assert_round_trips("fn f() { if (Point { x: 1mm, y: 2mm }).x > 0mm { a; } }");
    // ...and unrestricted again once inside a nested, unambiguous
    // delimiter (call args here).
    assert_round_trips("fn f() { if f(Point { x: 1mm, y: 2mm }) { a; } }");
}

/// Locks in the printer's own chosen canonical style for a few
/// representative constructs, so a future accidental style change shows
/// up as a diff here rather than only as a (still-passing) round-trip.
#[test]
fn canonical_output_matches_expected_style() {
    assert_eq!(
        format_source("struct Point2 { x: Length, y: Length }"),
        "struct Point2 {\n    x: Length,\n    y: Length,\n}\n"
    );
    assert_eq!(format_source("struct Empty { }"), "struct Empty {}\n");
    assert_eq!(
        format_source("enum MotorSize { NEMA17, NEMA23 }"),
        "enum MotorSize {\n    NEMA17,\n    NEMA23,\n}\n"
    );
    assert_eq!(
        format_source("fn add(a: Length, b: Length) -> Length { let sum = a + b; }"),
        "fn add(a: Length, b: Length) -> Length {\n    let sum = a + b;\n}\n"
    );
    assert_eq!(
        format_source("fn f() { if a { b; } else { c; } }"),
        "fn f() {\n    if a {\n        b;\n    } else {\n        c;\n    }\n}\n"
    );
    assert_eq!(format_source("import ../../a/b;"), "import ../../a/b;\n");
}
