# AICAD-150 — configuration validity rules

## Result

`crates/cad-configurations/src/rule.rs` adds typed, declarative D29
configuration validity rules evaluated over a `Configuration`'s own named
values, with deterministic order and structured violation/error evidence.
23/23 crate tests pass (12 new).

## Changes

- `Rule` — a small boolean-combinator enum (`Equals`/`NotEquals`/`And`/
  `Or`/`Not`/`Implies`/`Ref`) over `Configuration::named_value`. `docs/plan/
  07` §11's `require ratio in [5,10,20]` is `Or([Equals(5), Equals(10),
  Equals(20)])`; `forbid A && B` is `Not(And([A, B]))` — no dedicated `In`
  variant needed.
- `RuleSet` — a `BTreeMap<RuleName, Rule>` registry (mirrors
  `ComponentDefinitionRegistry`'s "same name/different content is rejected,
  identical redefinition is a no-op" precedent exactly).
- `evaluate(rules, configuration) -> Result<RuleEvaluation, RuleError>` —
  evaluates every rule in `RuleSet`'s own deterministic name order,
  collecting structured `RuleViolation`s (never a bare bool). `Rule::Ref`
  lets one named rule reuse another; cyclic `Ref` chains are caught with
  the same `on_stack` DFS technique `cad_assemblies::graph::expand_into`
  already uses for definition cycles, seeded with the currently-evaluating
  rule's own name so a chain is always reported rooted at the rule actually
  under evaluation. New diagnostics `ASM-E010` (rule violated), `ASM-E011`
  (undefined `Ref`), `ASM-E012` (cyclic `Ref`), continuing from
  `ASM-E009`.

## Decisions

- **Not a textual DSL.** The task's own objective warns against a "bespoke
  hidden rule language." `Rule` is an ordinary, fully inspectable Rust
  value with no parser/lexer of its own — the same relationship
  `cad_assemblies::mate`/`joint` already have to their own still-absent
  source syntax (see `rule.rs`'s own module doc comment). Reusing the
  actual future `.aicad` expression language is deferred until Stage-6/7
  wires real configuration source syntax; there is no such syntax to reuse
  today (`DL-31`).
- **Missing values never vacuously satisfy `Equals` or `NotEquals`** — a
  rule referencing an unset value always counts as violated, never silently
  passing (`a_missing_named_value_never_satisfies_equals_or_not_equals`).
- **Empty `Or` is unsatisfiable**, matching ordinary logical convention
  rather than an arbitrary vacuous-true default.

## Verification

- `cargo test -p cad-configurations` — 23/23 PASS.
- `cargo fmt -p cad-configurations -- --check` — PASS.
- `cargo clippy -p cad-configurations --all-targets --all-features -- -D warnings` — PASS.

## Limitations

- Rule values reuse `ParameterValue` (typed engineering quantities /
  scalars); there is still no enum-valued configuration option type
  (`AICAD-149`'s own disclosed limitation) — rules over enum-shaped
  options (`MotorSize`/`HousingMaterial`) are not yet expressible until
  that type exists, which is new public language semantics outside this
  task's scope.
- No `.aicad` surface syntax for declaring `configuration_rules { ... }` —
  Rust-IR only, matching `AICAD-149`.

## Next

`AICAD-151` (suppression) builds on both `overlay.rs` and this module's
`Configuration`/evaluation shape next.
