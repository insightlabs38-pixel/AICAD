; Basic syntax highlighting for AICAD (AICAD-062).
; Keeps to categories `crates/cad-lexer`'s own reserved-word set and
; `specs/language/grammar.ebnf` actually define — no speculative captures
; for syntax neither parser implements yet.

[
  "let" "var" "const" "param" "fn" "struct" "enum" "part" "import"
  "if" "else" "for" "in" "while" "loop" "match" "return" "break" "continue"
  "pure"
] @keyword

[
  "true" "false"
] @constant.builtin.boolean

[
  "=" "==" "!=" "<" "<=" ">" ">=" "+" "-" "*" "/" "&&" "||" "!" "->" "=>"
  ".." "..=" "~="
] @operator

[ "(" ")" "[" "]" "{" "}" ] @punctuation.bracket
[ "," ";" ":" "::" "." ] @punctuation.delimiter

(number_literal) @number
(string_literal) @string
(raw_string_literal) @string
(bool_literal) @constant.builtin.boolean

(line_comment) @comment
(block_comment) @comment
(doc_comment) @comment.doc

(fn_decl name: (identifier) @function)
(call_expression callee: (identifier) @function.call)
(method_call_expression method: (identifier) @function.method.call)

(struct_decl name: (identifier) @type)
(enum_decl name: (identifier) @type)
(enum_variant name: (identifier) @constructor)
(generic_type name: (identifier) @type)
(type_params (identifier) @type.parameter)

(param name: (identifier) @variable.parameter)
(field name: (identifier) @property)
(record_field_init name: (identifier) @property)
(record_pattern_field name: (identifier) @property)

(identifier) @variable
