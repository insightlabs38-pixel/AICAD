// Tree-sitter grammar for the AICAD language (AICAD-062).
//
// This grammar targets exactly the syntax subset already implemented by
// the production parser, `crates/cad-parser` (recursive-descent over
// `crates/cad-lexer`'s token stream), as of this task — not the full
// aspirational `specs/language/grammar.ebnf` sketch (which still lists
// `interface`/`assembly`/`requirement`/`test` declarations that no parser
// implements yet). Extending either parser to a broader subset is a
// later task's job, not this grammar's.
//
// Two context-sensitive restrictions from `crates/cad-parser` are mirrored
// here, since a tree-sitter grammar is a single context-free grammar and
// tree-sitter has no runtime parser flag:
//
// 1. `Parser::no_record_literal` (`if`/`while`/`for`/`match`'s own
//    condition/iterable/scrutinee): a bare `identifier "{"` is never a
//    record literal there, and this is a real threaded restriction (it
//    holds at *every* reachable operand inside the condition, not only
//    the leftmost one), reset the instant a `(`/`[`/call-arg-list/
//    record-literal-body is entered. Modeled below as a second, fully
//    parallel expression hierarchy (the `_b` suffix) that excludes
//    `record_literal` from its own primary and otherwise mirrors the
//    default (`_a`) hierarchy level for level, reverting to `_a` inside
//    any bracket.
// 2. `Parser::parse_block_expr`'s `is_definite_stmt_start` check: inside a
//    block used as a value, a bare `if`/`match` is *always* parsed as a
//    statement (never promoted to the block's trailing value), decided by
//    a single one-time peek at the very first token — not a threaded
//    flag. So unlike restriction 1, this only excludes `if_expr`/
//    `match_expr` from the *leftmost* derivation spine of the trailing
//    candidate (e.g. `-if a { 1mm } else { 2mm }` is still a perfectly
//    valid trailing value, since its first token is `-`, not `if`).
//    Modeled below as a third hierarchy (the `_c` suffix) that only
//    restricts its own left/receiver operand recursively, deferring to
//    the normal `_a` hierarchy for every right-hand operand.
//
// All three hierarchies alias their productions to the same public node
// types (`binary_expression`, `unary_expression`, `method_call_expression`,
// `field_expression`) so the resulting tree shape does not depend on which
// hierarchy matched.

const BINARY_LEVELS = [
  // [level name, next level name, operator(s), precedence]
  ['or', 'and', ['||'], 1],
  ['and', 'equality', ['&&'], 1],
  ['equality', 'relational', ['==', '!=', '~='], 1],
  ['relational', 'additive', ['<', '<=', '>', '>='], 1],
  ['additive', 'multiplicative', ['+', '-'], 1],
  ['multiplicative', 'unary', ['*', '/'], 1],
];

// Builds one fully self-contained expression hierarchy (`range` down to
// `primary`), suffixed `_<suffix>`, threading the restriction at every
// level (used for the default hierarchy and the `no_record_literal`
// hierarchy — see module doc comment restriction 1).
// NOTE on the `_impl`/`alias` split below: `alias(seq(...fields...), Name)`
// — aliasing an *inline* multi-field `seq()` directly — does not produce
// one aliased node wrapping that seq's fields in this tree-sitter version;
// it fragments, re-applying `Name` separately to each field instead (
// confirmed with an isolated minimal grammar while debugging this task).
// `alias()` only reliably renames a reference to an already-separately-
// defined rule. So every field-bearing production here is first given its
// own hidden `_..._impl_<suffix>` rule, and only *that rule's symbol* is
// aliased — never an inline `seq(...)`.
function threadedHierarchy(suffix, allowRecordLiteral) {
  const r = (level) => `_${level}_${suffix}`;
  const rules = {};

  rules[`_range_impl_${suffix}`] = ($) =>
    seq(
      field('start', $[r('or')]),
      field('operator', choice('..', '..=')),
      field('end', $[r('or')]),
    );
  rules[r('range')] = ($) =>
    choice(alias($[`_range_impl_${suffix}`], $.range_expression), $[r('or')]);

  for (const [level, next, ops, prec_] of BINARY_LEVELS) {
    rules[`_bin_impl_${level}_${suffix}`] = ($) =>
      prec.left(
        prec_,
        seq(
          field('left', $[r(level)]),
          field('operator', ops.length === 1 ? ops[0] : choice(...ops)),
          field('right', $[r(next)]),
        ),
      );
    rules[r(level)] = ($) =>
      choice(alias($[`_bin_impl_${level}_${suffix}`], $.binary_expression), $[r(next)]);
  }

  rules[`_unary_impl_${suffix}`] = ($) =>
    prec(2, seq(field('operator', choice('-', '!')), field('operand', $[r('unary')])));
  rules[r('unary')] = ($) =>
    choice(alias($[`_unary_impl_${suffix}`], $.unary_expression), $[r('postfix')]);

  rules[`_method_call_impl_${suffix}`] = ($) =>
    seq(
      field('receiver', $[r('postfix')]),
      '.',
      field('method', $.identifier),
      '(',
      field('args', optional($.args)),
      ')',
    );
  rules[`_field_impl_${suffix}`] = ($) =>
    seq(field('receiver', $[r('postfix')]), '.', field('field', $.identifier));
  rules[r('postfix')] = ($) =>
    choice(
      alias($[`_method_call_impl_${suffix}`], $.method_call_expression),
      alias($[`_field_impl_${suffix}`], $.field_expression),
      $[r('primary')],
    );

  rules[r('primary')] = ($) =>
    choice(
      $.bool_literal,
      $.number_literal,
      $.string_literal,
      $.raw_string_literal,
      $.call_expression,
      ...(allowRecordLiteral ? [$.record_literal] : []),
      $.identifier,
      $.paren_expr,
      $.list_expr,
      $.block_expr,
      $.if_expr,
      $.match_expr,
    );

  return rules;
}

// Restriction 2 (`block_expr`'s trailing slot never promotes a bare
// `if`/`match` to be the block's value) is handled without a third parallel
// hierarchy — seeding `_stmt`'s `if_stmt`/`match_stmt` alternatives with a
// higher dynamic precedence than `if_expr`/`match_expr` (see `conflicts`
// below) is enough to make the GLR runtime prefer the statement reading
// whenever both are reachable over the same span, matching
// `Parser::parse_block_expr`'s one-time leading-token dispatch without
// needing a textually separate "no leading if/match" primary chain (which,
// tried first, produced an unresolvable reduce-reduce conflict against the
// default hierarchy's own `_primary_a` — both `_stmt`'s plain `expr_stmt`
// and a distinctly-named restricted trailing chain are reachable from the
// exact same leading tokens inside `{ ... }`, and giving them different
// rule names made every state that reaches a shared leaf like
// `number_literal` ambiguous between the two chains).

module.exports = grammar({
  name: 'aicad',

  extras: ($) => [/\s/, $.line_comment, $.block_comment, $.doc_comment],

  word: ($) => $.identifier,

  conflicts: ($) => [
    [$.match_stmt, $.match_expr],
    [$.block, $.block_expr],
    [$.match_arm, $._primary_a],
  ],

  rules: {
    source_file: ($) => repeat($._item),

    _item: ($) =>
      choice(
        $.let_decl,
        $.const_decl,
        $.param_decl,
        $.fn_decl,
        $.struct_decl,
        $.enum_decl,
        $.part_decl,
        $.import_decl,
      ),

    // ---- declarations (`crates/cad-parser`'s `parse_item`) --------------

    let_decl: ($) =>
      seq(
        'let',
        field('name', $.identifier),
        optional(seq(':', field('type', $._type))),
        '=',
        field('value', $._expr),
        ';',
      ),
    const_decl: ($) =>
      seq(
        'const',
        field('name', $.identifier),
        optional(seq(':', field('type', $._type))),
        '=',
        field('value', $._expr),
        ';',
      ),
    param_decl: ($) =>
      seq(
        'param',
        field('name', $.identifier),
        ':',
        field('type', $._type),
        optional(seq('=', field('default', $._expr))),
        ';',
      ),

    fn_decl: ($) =>
      seq(
        optional(field('pure', 'pure')),
        'fn',
        field('name', $.identifier),
        optional(field('type_params', $.type_params)),
        '(',
        field('params', optional($.params)),
        ')',
        optional(seq('->', field('return_type', $._type))),
        field('body', $.block),
      ),
    params: ($) => seq($.param, repeat(seq(',', $.param)), optional(',')),
    param: ($) =>
      seq(
        field('name', $.identifier),
        ':',
        field('type', $._type),
        optional(seq('=', field('default', $._expr))),
      ),

    type_params: ($) => seq('<', $.identifier, repeat(seq(',', $.identifier)), optional(','), '>'),

    struct_decl: ($) =>
      seq(
        'struct',
        field('name', $.identifier),
        optional(field('type_params', $.type_params)),
        '{',
        field('fields', optional($.struct_fields)),
        '}',
      ),
    struct_fields: ($) => seq($.field, repeat(seq(',', $.field)), optional(',')),
    field: ($) => seq(field('name', $.identifier), ':', field('type', $._type)),

    enum_decl: ($) =>
      seq(
        'enum',
        field('name', $.identifier),
        optional(field('type_params', $.type_params)),
        '{',
        field('variants', optional($.enum_variants)),
        '}',
      ),
    enum_variants: ($) => seq($.enum_variant, repeat(seq(',', $.enum_variant)), optional(',')),
    enum_variant: ($) =>
      seq(
        field('name', $.identifier),
        optional(field('payload', choice($.tuple_variant_payload, $.record_variant_payload))),
      ),
    tuple_variant_payload: ($) =>
      seq('(', optional(seq($._type, repeat(seq(',', $._type)), optional(','))), ')'),
    record_variant_payload: ($) => seq('{', optional($.struct_fields), '}'),

    part_decl: ($) =>
      seq('part', field('name', $.identifier), '{', field('items', repeat($._item)), '}'),

    import_decl: ($) =>
      seq(
        'import',
        field('path', $.import_path),
        optional(seq('::', '{', field('names', optional($.import_names)), '}')),
        ';',
      ),
    import_path: ($) => choice($.relative_import_path, $.package_import_path),
    relative_import_path: ($) =>
      seq(
        repeat(seq('..', '/')),
        '.',
        '/',
        $.identifier,
        repeat(seq('/', $.identifier)),
      ),
    package_import_path: ($) => seq($.identifier, repeat(seq('.', $.identifier))),
    import_names: ($) => seq($.identifier, repeat(seq(',', $.identifier)), optional(',')),

    // ---- types ------------------------------------------------------------

    _type: ($) => choice($.generic_type, $.identifier),
    generic_type: ($) =>
      seq(field('name', $.identifier), '<', field('args', $.type_args), '>'),
    type_args: ($) => seq($._type, repeat(seq(',', $._type)), optional(',')),

    // ---- statements/blocks --------------------------------------------

    block: ($) => seq('{', repeat($._stmt), '}'),

    _stmt: ($) =>
      choice(
        $.let_stmt,
        $.var_stmt,
        $.assign_stmt,
        $.if_stmt,
        $.for_stmt,
        $.while_stmt,
        $.loop_stmt,
        $.match_stmt,
        $.return_stmt,
        $.break_stmt,
        $.continue_stmt,
        $.expr_stmt,
      ),

    let_stmt: ($) =>
      seq(
        'let',
        field('name', $.identifier),
        optional(seq(':', field('type', $._type))),
        '=',
        field('value', $._expr),
        ';',
      ),
    var_stmt: ($) =>
      seq(
        'var',
        field('name', $.identifier),
        optional(seq(':', field('type', $._type))),
        '=',
        field('value', $._expr),
        ';',
      ),
    assign_stmt: ($) => seq(field('name', $.identifier), '=', field('value', $._expr), ';'),
    expr_stmt: ($) => seq(field('expr', $._expr), ';'),

    // `prec.dynamic` outranks `if_expr`'s default (0) so that, wherever a
    // bare `if` inside a block is reachable as *either* an `if_stmt` (via
    // `_stmt`, always) *or* an `if_expr` (only at `block_expr`'s own
    // trailing slot), the GLR runtime keeps the statement reading — this
    // is what makes a bare `if`/`match` never become a block's trailing
    // value (`Parser::parse_block_expr`'s `is_definite_stmt_start`), see
    // `conflicts` below.
    if_stmt: ($) =>
      prec.dynamic(
        1,
        seq(
          'if',
          field('condition', $._expr_no_struct),
          field('consequence', $.block),
          optional(seq('else', field('alternative', choice($.block, $.if_stmt)))),
        ),
      ),
    for_stmt: ($) =>
      seq(
        'for',
        field('variable', $.identifier),
        'in',
        field('iterable', $._expr_no_struct),
        field('body', $.block),
      ),
    while_stmt: ($) =>
      seq('while', field('condition', $._expr_no_struct), field('body', $.block)),
    loop_stmt: ($) => seq('loop', field('body', $.block)),
    match_stmt: ($) =>
      prec.dynamic(
        1,
        seq(
          'match',
          field('scrutinee', $._expr_no_struct),
          '{',
          field('arms', repeat($.match_arm)),
          '}',
        ),
      ),
    return_stmt: ($) => seq('return', optional(field('value', $._expr)), ';'),
    break_stmt: () => seq('break', ';'),
    continue_stmt: () => seq('continue', ';'),

    // ---- expression-position control flow ------------------------------

    block_expr: ($) => seq('{', repeat($._stmt), optional(field('trailing', $._expr)), '}'),

    if_expr: ($) =>
      seq(
        'if',
        field('condition', $._expr_no_struct),
        field('consequence', $.block_expr),
        'else',
        field('alternative', choice($.block_expr, $.if_expr)),
      ),
    match_expr: ($) =>
      seq(
        'match',
        field('scrutinee', $._expr_no_struct),
        '{',
        field('arms', repeat($.match_arm)),
        '}',
      ),
    // `crates/cad-parser`'s `parse_match_arms` dispatches purely on
    // whether the token right after `=>` is `{`: if so, the block form
    // (trailing `,` optional) always wins, even though a bare `{ ... }`
    // is *also* an ordinary `_expr` (via `block_expr` as a primary) that
    // would otherwise make the comma-requiring expression form
    // ambiguously reachable too. `prec.dynamic` plus the `conflicts`
    // entry below reproduce that same deterministic preference.
    match_arm: ($) =>
      choice(
        prec.dynamic(
          1,
          seq(field('pattern', $.pattern), '=>', field('body', $.block_expr), optional(',')),
        ),
        seq(field('pattern', $.pattern), '=>', field('body', $._expr), ','),
      ),

    // ---- patterns -------------------------------------------------------

    pattern: ($) =>
      choice(
        $.wildcard_pattern,
        $.bool_literal,
        $.number_literal,
        $.string_literal,
        $.raw_string_literal,
        $.tuple_pattern,
        $.record_pattern,
        $.identifier,
      ),
    wildcard_pattern: () => '_',
    tuple_pattern: ($) =>
      seq(
        field('name', $.identifier),
        '(',
        optional(seq($.pattern, repeat(seq(',', $.pattern)), optional(','))),
        ')',
      ),
    record_pattern: ($) =>
      seq(field('name', $.identifier), '{', optional($.record_pattern_fields), '}'),
    record_pattern_fields: ($) =>
      seq($.record_pattern_field, repeat(seq(',', $.record_pattern_field)), optional(',')),
    record_pattern_field: ($) =>
      seq(field('name', $.identifier), optional(seq(':', field('pattern', $.pattern)))),

    // ---- shared/canonical expression leaves (always reset to the full,
    //      unrestricted `_a` hierarchy internally; identical regardless of
    //      which hierarchy's primary refers to them) --------------------

    paren_expr: ($) => seq('(', $._expr, ')'),
    list_expr: ($) =>
      seq('[', optional(seq($._expr, repeat(seq(',', $._expr)), optional(','))), ']'),
    call_expression: ($) =>
      seq(field('callee', $.identifier), '(', field('args', optional($.args)), ')'),
    args: ($) => seq($._arg, repeat(seq(',', $._arg)), optional(',')),
    _arg: ($) => choice($.named_arg, $._expr),
    named_arg: ($) => seq(field('name', $.identifier), '=', field('value', $._expr)),
    record_literal: ($) =>
      seq(field('name', $.identifier), '{', optional($.record_field_inits), '}'),
    record_field_inits: ($) =>
      seq($.record_field_init, repeat(seq(',', $.record_field_init)), optional(',')),
    record_field_init: ($) => seq(field('name', $.identifier), ':', field('value', $._expr)),

    // ---- expression entry points ----------------------------------------
    //
    // `_expr` (hierarchy `_a`): the default, fully unrestricted grammar —
    // let/const/param/field values, call/named args, list elements, record
    // field values, return values, range operands' partner side, etc.
    //
    // `_expr_no_struct` (hierarchy `_b`): `if`/`while`/`for`/`match`'s own
    // condition/iterable/scrutinee only (restriction 1). `block_expr`'s
    // trailing slot (restriction 2) reuses this same `_expr` — see the
    // module doc comment and the `if_stmt`/`match_stmt` `prec.dynamic`.
    _expr: ($) => $._range_a,
    _expr_no_struct: ($) => $._range_b,

    ...threadedHierarchy('a', true),
    ...threadedHierarchy('b', false),

    // ---- literals/identifiers -------------------------------------------

    bool_literal: () => choice('true', 'false'),
    number_literal: () =>
      token(
        seq(
          /[0-9]+/,
          optional(seq('.', /[0-9]+/)),
          optional(seq(/[eE][+-]?/, /[0-9]+/)),
          optional(/[\p{XID_Start}_][\p{XID_Continue}]*/),
        ),
      ),
    string_literal: () => token(seq('"', repeat(choice(/[^"\\\n]/, seq('\\', /./))), '"')),
    raw_string_literal: () => token(seq('r"', repeat(/[^"]/), '"')),

    identifier: () => /[\p{XID_Start}_][\p{XID_Continue}]*/,

    line_comment: () => token(choice(seq('//', /[^/\n][^\n]*/), '//')),
    block_comment: () => token(seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/')),
    doc_comment: () => token(seq('///', /[^\n]*/)),
  },
});
