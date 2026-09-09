# AICAD-010 — Draft RFC-0004 units/type system

## Objective
Draft RFC-0004 (units/type system), covering the Stage-0 build item
"type/units model", per `project/TASKS.yaml` (AICAD-010).

## Dependencies checked
AICAD-009 (RFC-0003) — complete, see `project/reports/AICAD-009.md`. Owner
ruling DL-3 was recorded before this batch began.

## What was done

Wrote `rfcs/0004-units-type-system.md`:
1. Froze the type-system baseline from
   `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §2-4 (primitive types,
   the 20-dimension baseline list, initial unit literal set) verbatim.
2. Encoded DL-3/D4's four resolved sub-questions in detail, each as its
   own section since each was independently open before this ruling:
   - §5 canonicalization/implicit conversion: unique canonical
     representation, same-dimension implicit conversion only, structural
     (exponent-based) canonicalization of derived dimensions.
   - §6 tolerance arithmetic: conservative interval semantics as the
     `Tolerance<T>` default; RSS/statistical composition requires an
     explicit, separate, later API and is never assumed.
   - §7 affine units: absolute-vs-delta distinction for Celsius/Fahrenheit,
     with a worked example of why naive scale-only conversion is wrong for
     temperature *differences* specifically, and why adding two absolute
     affine quantities must be a type error.
3. Summarized (rather than re-deriving in full detail) the remaining,
   non-owner-decision parts of `03` §7-23 (geometry/semantic types,
   collections, structs/enums/interfaces/generics, ownership/control-flow/
   unsafe forms, numerical-precision policy, parameter/rationale metadata)
   as adopted-as-is, since none of those needed an owner ruling to freeze
   — they were not in dispute, only D4's four sub-questions were.
4. Listed alternatives considered per DL-3 sub-decision and explicitly
   noted what remains open (default project tolerance value is
   config/implementation, not a type-system freeze; D11's constraint-solver
   tolerance *usage* is separate from tolerance *arithmetic* and stays
   open).

No escalation condition was triggered: this RFC implements exactly DL-3's
ruling plus verbatim plan content; it does not change typed-units
semantics beyond what the owner already ruled, and it does not resolve D11.

## Files changed
- Added: `rfcs/0004-units-type-system.md`.

## Verification (exact commands/results)
```
$ grep -c "^## " rfcs/0004-units-type-system.md
14   # confirms all 14 planned sections are present

$ cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
(both exit 0 — no Rust source touched by this task)
```

Task-specific check: worked the affine-temperature example in §7 by hand
(20°C absolute -> 293.15K via +273.15 offset; a 5°C *delta* -> 5K, no
offset applied) to confirm the absolute-vs-delta distinction as stated is
internally consistent and matches standard engineering practice, not just
plausible-sounding prose.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific check: affine-temperature worked example verified by hand,
  as above.

## Limitations / follow-up
- The default project-wide geometric modeling tolerance value is left to
  implementation/configuration, not frozen by this language-level RFC.
- D11 (constraint IR/solver-independence, including tolerance usage inside
  the solver) remains open and is not addressed here.
