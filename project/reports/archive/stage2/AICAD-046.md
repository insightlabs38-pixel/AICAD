# AICAD-046: Create cad-types primitive type representation

## Objective

Implement RFC-0004 §2's ordinary primitive types and §3's minimum
first-class dimension list as `cad-types`'s semantic type representation,
per `project/TASKS.yaml` AICAD-046 (Stage-2 batch S2-04, first of three
tasks). This is the resolution target for `cad-ast`'s purely syntactic
`Type::Named`/`Type::Generic` (`crates/cad-ast/src/item.rs`, whose own doc
comment defers that binding to `AICAD-046`).

## Base commit

`5c8f0a7` (`origin/branch/wonderful-thompson-19031x`'s Batch S2-03
checkpoint — this invocation's starting point after fast-forwarding its own
harness-assigned branch, `branch/wonderful-thompson-fkbtu6`, onto it; see
this session's `SESSION_HANDOFF.md` update for the branch-reconciliation
account).

## Plan references read

`rfcs/0004-units-type-system.md` §2 (primitive types), §3 (dimensional
quantity types), §5 (canonicalization/quantity-shape patch, specifically
the `affine_kind` discriminant), §7 (affine units); cross-checked against
`docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §2-3 (the un-patched
source both freeze from); `crates/cad-ast/src/item.rs`'s `Type` doc comment
(confirms this crate is the intended resolution target and that no
resolution logic exists yet anywhere in the workspace).

## Implementation

- `crates/cad-types/src/primitive.rs` (new): `PrimitiveType` — an 8-variant
  `Copy` enum (`Bool`, `Int`, `UInt`, `Float`, `Decimal`, `String`, `Bytes`,
  `Char`), with `ALL`, `name()` (source-level spelling), `from_name()`
  (case-sensitive resolution), and `Display`.
- `crates/cad-types/src/dimension.rs` (new): `Dimension` — a 21-variant
  `Copy` enum for RFC-0004 §3's minimum first-class dimension list, with
  the same `ALL`/`name()`/`from_name()`/`Display` shape, plus
  `is_affine()` (`true` only for `Temperature`, per RFC-0004 §7 — "initially
  Celsius and Fahrenheit"). Also `AffineKind` (`Absolute` | `Delta`), the
  discriminant RFC-0004 §5's Stage-0-independent-review patch requires so
  the type checker can "admit or reject an operation" on affine quantities
  without inferring it from unit spelling.
- `crates/cad-types/src/lib.rs`: re-exports `PrimitiveType`, `Dimension`,
  `AffineKind`; crate-level doc comment states this task's scope and its
  three explicit exclusions (derived-dimension vector canonicalization,
  concrete unit literals/conversions, `Tolerance<T>`-family types — see
  Decisions below).
- `crates/cad-types/README.md`: replaced the stale placeholder text with a
  "Status" section pointing at what is/isn't implemented and this report.

No third-party dependency added; `crates/cad-types/Cargo.toml` is
unchanged (empty `[dependencies]`).

## Decisions made and why

1. **`Dimension` stays a flat, closed 21-variant enum here — no exponent
   vector, no `Type` sum type unifying it with `PrimitiveType`.** RFC-0004
   §5 explicitly separates two different canonicalization concerns:
   "quantities of the same dimension may be implicitly converted" (needs
   only *named* dimension identity, which variant equality already gives)
   versus "derived dimensions are canonicalized structurally, by dimension
   exponents" (needs an actual vector algebra). The task's own title,
   "primitive type representation," and the batch's next task's title,
   "Create cad-units dimensional **vector**/canonicalization," draw this
   same line. Building the exponent-vector machinery here would duplicate
   work `AICAD-047` explicitly owns and risk a design that doesn't match
   what `AICAD-047` actually needs. I also did not introduce a unifying
   `Type` enum (`Type::Primitive(PrimitiveType) | Type::Dimensional(...)`)
   — no task in batches S2-04/S2-05 (`046`-`050`) is titled for it, and
   how primitives, dimensions, structs, enums, and generics compose into
   one type representation is squarely `AICAD-051`'s ("Create typed HIR")
   and `AICAD-052`/`053`'s territory; guessing that shape now risks
   conflicting with those tasks' own design decisions. `AGENTS.md`'s "No
   speculative future work" rule: "public APIs, semantics,... or major
   abstractions owned by a later task must wait for that task."
2. **`Tolerance<T>`/`Range<T>`/`Distribution<T>`/`Fit` (RFC-0004 §6) are
   deliberately not implemented in this task**, even though RFC-0004 §14's
   "Impact" section says `cad-types`/`cad-units` implement "§2-7 starting
   Stage 2 (AICAD-046-049)" and §6 falls in that range. None of
   `AICAD-046`-`050`'s actual titles name tolerances, and RFC-0004 §6's
   types are generic wrappers over a dimension/primitive (`Tolerance<T>`),
   not primitives or dimensions themselves — they are better scoped to
   whichever task first needs them (a constraint/requirements-adjacent
   task, going by `docs/plan/03`'s own §6 cross-reference to `13`'s
   tolerance-stack methods) than guessed at here. Flagging this explicitly
   so a future session doesn't assume §6 was silently dropped — see
   `crates/cad-types/README.md`'s "Not yet implemented" list.
3. **`AffineKind` lives in `cad-types::dimension`, not a separate module,
   and is modeled as `Option<AffineKind>` at call sites rather than a
   third "not applicable" variant.** RFC-0004 §5's patch frames it as a
   discriminant *of a dimension's quantity shape*, so it belongs next to
   `Dimension` itself rather than in a new file; a three-variant enum
   (`Absolute | Delta | NotApplicable`) would let a `Length` value
   nonsensically claim to be "absolute," which is exactly the kind of
   state the type shape shouldn't be able to represent. `is_affine()`
   gives the one place that needs to decide whether `Some`/`None` is even
   legal for a given dimension.
4. **`from_name` is case-sensitive and exact-match only.** Every worked
   example in RFC-0004/`docs/plan/03` capitalizes type names exactly as
   listed (`Length`, `Bool`, ...); no evidence anywhere suggests
   case-insensitive or fuzzy type-name resolution, and adding it would be
   unvalidated speculative behavior.
5. **No escalation triggered.** This task adds a new closed representation
   strictly inside RFC-0004's own frozen §2/§3/§5-patch text; it does not
   change public syntax (the AST's `Type::Named` is untouched), does not
   yet perform any conversion/arithmetic that could weaken determinism or
   validation, and does not introduce kernel-specific types. The two
   scope-boundary decisions above (1 and 2) keep this task from reaching
   into territory later tasks own, rather than resolving an open
   architecture question myself.

## Files changed

- Added: `crates/cad-types/src/primitive.rs`, `crates/cad-types/src/dimension.rs`.
- Modified: `crates/cad-types/src/lib.rs`, `crates/cad-types/README.md`,
  `project/TASKS.yaml` (AICAD-046 `status: done`).
- Added: `project/reports/AICAD-046.md` (this report).

## Verification (exact commands/results)

```
$ cargo test -p cad-types
running 14 tests
test dimension::tests::display_matches_name ... ok
test dimension::tests::affine_kind_display ... ok
test dimension::tests::affine_kind_variants_are_distinct ... ok
test dimension::tests::all_covers_every_variant_exactly_once ... ok
test dimension::tests::from_name_rejects_unknown_and_primitive_names ... ok
test dimension::tests::from_name_round_trips_every_variant ... ok
test dimension::tests::matches_rfc_0004_section_3_minimum_list_exactly ... ok
test dimension::tests::only_temperature_is_affine ... ok
test primitive::tests::all_covers_every_variant_exactly_once ... ok
test primitive::tests::display_matches_name ... ok
test primitive::tests::from_name_rejects_unknown_and_dimensional_names ... ok
test primitive::tests::from_name_round_trips_every_variant ... ok
test primitive::tests::is_copy_and_eq ... ok
test primitive::tests::name_matches_rfc_0004_section_2_spelling ... ok
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt -p cad-types -- --check
(exit code 0, no output)

$ cargo clippy -p cad-types --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
```

Full-workspace build/test/lint (`cargo build --workspace --all-targets`,
`cargo test --workspace`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `cargo fmt --all -- --check`) re-run before
this task's commit; results recorded in `project/SESSION_HANDOFF.md`.

## Tests/regressions

14 new tests (`primitive.rs`: 6, `dimension.rs`: 8), all positive/structural
(exhaustiveness, name round-trip, RFC-list pinning, `Display`). No
regressions found or introduced; no prior behavior touched (new crate
module, no other crate depends on `cad-types` yet).

## Known limitations

- No exponent-vector algebra yet (`AICAD-047`).
- No unit literals/conversions yet (`AICAD-048`).
- No `Tolerance<T>`-family types (open, see Decisions §2).
- `PrimitiveType`/`Dimension` are not yet wired into `cad-ast::item::Type`
  resolution, name binding, or type checking — that is `AICAD-050`/`052`.

## Unresolved questions

None requiring owner escalation. The `Tolerance<T>`-family scope note
(Decisions §2) is a documented deferral, not a blocker for `AICAD-047`/`048`.
