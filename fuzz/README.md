# Bounded fuzzing

AICAD-079C adds a parser-only `cargo-fuzz` target. It exercises mature
lexer/parser malformed-input handling and deliberately does **not** execute
untrusted geometry or pretend a Stage-4 resolver exists.

Nightly/manual CI bounds the run by time and maximum input length. Future
Stage-4 work may add reference-recipe parsing/normalization targets only after
those real interfaces exist.
