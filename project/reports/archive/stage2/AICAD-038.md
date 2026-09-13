# AICAD-038 — Create cad-diagnostics crate and JSON-schema conformance tests

## Objective
Implement `crates/cad-diagnostics` as a real crate (previously an
AICAD-002/003 placeholder) materializing the diagnostic schema/taxonomy/
suggestion-confidence contract frozen by RFC-0005, and add JSON-schema
conformance tests, per `project/TASKS.yaml` AICAD-038 (Stage-2 batch
S2-01, first task).

## Base commit
`1eaac7c` (this session's own prior commit recording DL-11/DL-12 and
advancing `project/CURRENT_STAGE.md` to Stage 2), on top of `09fdef5`
(Stage-1 final `origin/main` merge, PR #8).

## Plan references read
`rfcs/0005-diagnostics.md` (frozen diagnostic taxonomy/schema/suggestion
contract, and the explicitly-not-resolved D10 stability policy);
`docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §10-15; `specs/schemas/README.md`.

## Implementation

- `crates/cad-diagnostics/src/json.rs` — a dependency-free `Json` value
  type (`Null`/`Bool`/`Integer`/`Float`/`String`/`Array`/`Object`, with
  `Object` as an order-preserving `Vec<(String, Json)>`, never a hash map),
  a recursive-descent parser for the full JSON grammar, and a canonical
  compact serializer. **No third-party crate dependency (e.g. `serde`/
  `serde_json`) was added** — see "Decisions" below for why.
- `crates/cad-diagnostics/src/schema.rs` — a deliberately narrow JSON
  Schema *subset* validator supporting `type` (including union types like
  `["object","null"]`), `required`, `properties`, `items`, and `enum`. It
  does **not** resolve `$ref`/`$defs` and does **not** evaluate `pattern`
  (regex) — documented as a known limitation, not silently pretended away.
- `crates/cad-diagnostics/src/lib.rs` — `DiagnosticCode` (parses/validates
  `FAMILY-[EWI]###` against the 18-family RFC-0005 §2 taxonomy, rejecting
  unknown families, malformed shape, and out-of-range numbers),
  `Severity`, `Position`/`SourceSpan`, `SuggestionConfidence`/`Suggestion`,
  and `Diagnostic` (builder-style, `to_json()` producing the exact
  RFC-0005 §3 field order/shape). `Diagnostic::new` rejects a
  severity/code-letter mismatch (e.g. an `E` code paired with
  `Severity::Warning`) rather than silently allowing two views of one
  diagnostic to disagree.
- `specs/schemas/diagnostic.schema.json` — the canonical JSON Schema
  artifact `specs/schemas/README.md` and RFC-0005 §9 call for, populated
  from RFC-0005 §3 (fully inlined, no `$ref`, since the validator above
  doesn't resolve references).
- `crates/cad-diagnostics/tests/schema_conformance.rs` — loads the real
  checked-in schema file (not an embedded copy) and validates: the exact
  RFC-0005 §3 `REF-E102` example, the exact RFC-0005 §4 `GEOM-E204` fillet
  example (all three suggestion-confidence levels), a minimal diagnostic
  with every optional field absent, and `warning`/`info` severities.
  Negative/adversarial tests: a hand-built instance missing required
  fields, an invalid `severity` enum value, a wrong-typed `candidates`
  field, unknown-family/malformed diagnostic codes, and a
  severity/code-letter mismatch — all correctly rejected.

## Decisions made and why

1. **No third-party crate dependency added.** Every crate in this
   workspace through the end of Stage 1 (100 completed tasks) has zero
   external dependencies (`Cargo.lock` confirmed empty of any non-`cad-*`
   entry before this task). Adding `serde`/`serde_json` now would be the
   first such dependency in the project and is not required by anything
   in RFC-0005 or `AGENTS.md` — it is autonomously-allowed engineering
   judgment, not a mandated escalation, but given the strong existing
   precedent I chose the more conservative option consistent with it,
   and it has a genuine correctness benefit: `project/DECISION_LOG.md#DL-12`
   (D5 Level 1) requires AICAD-owned canonical serialization to be
   byte-identical for identical input, and a hand-rolled `Json::Object`
   backed by an order-preserving `Vec` makes that trivial to guarantee
   without configuring a third-party library's map-ordering behavior.
   This is not a closed decision — a later task may still choose to adopt
   `serde` once broader serialization needs (HIR, Geometry IR) make a
   hand-rolled approach the wrong tradeoff; nothing here forecloses that.
2. **The schema validator is a deliberate subset, not a general JSON
   Schema engine.** It implements exactly the keywords
   `diagnostic.schema.json` uses. `pattern` (regex) is not evaluated —
   `DiagnosticCode::parse`'s own stricter Rust-level check is the
   authoritative format enforcement, and the schema's `pattern` field is
   documentation. `$ref` is not supported, so the schema file is fully
   inlined (duplicating the `start`/`end` position sub-schema once) rather
   than factored through `$defs`. Both limitations are stated in-code
   (`schema.rs` module docs) and in the schema file's own `description`
   field, not left implicit.
3. **`Diagnostic::new` enforces severity/code-letter consistency.** RFC-0005
   doesn't explicitly require this, but allowing a diagnostic whose code
   says `E` and whose `severity` field says `"warning"` would let one
   diagnostic instance disagree with itself — a correctness bug in
   whatever raises it. Rejecting it at construction is the smallest fix
   consistent with "never silently weaken validation."
4. **Suggestion confidence always serializes as the suggestion object's
   last key.** RFC-0005 §4's own example puts `confidence` in varying
   positions relative to other keys; since JSON object key order carries
   no schema meaning but D5 Level 1 wants deterministic canonical output,
   I picked one fixed rule (`confidence` last) and documented it, rather
   than leaving suggestion field order as whatever insertion order a
   caller happens to use.
5. Every code/schema field in this crate is explicitly documented as
   provisional pending `project/OWNER_DECISIONS.md` D10 (diagnostic code/
   schema stability policy, still open), per RFC-0005 §7's own instruction
   — stated in both the crate's module docs and README, not silently
   treated as frozen.

No escalation condition was triggered: this materializes an already-frozen
RFC (RFC-0005) exactly, adds no new public language syntax/semantics,
leaks no kernel-specific type, and does not touch reference resolution or
any open architecture alternative.

## Files changed
- Added: `crates/cad-diagnostics/src/json.rs`,
  `crates/cad-diagnostics/src/schema.rs`,
  `crates/cad-diagnostics/tests/schema_conformance.rs`,
  `specs/schemas/diagnostic.schema.json`.
- Modified: `crates/cad-diagnostics/src/lib.rs` (placeholder ->
  implementation), `crates/cad-diagnostics/README.md`,
  `project/TASKS.yaml` (AICAD-038 status).

## Verification (exact commands/results)
```
$ cargo build -p cad-diagnostics
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.49s

$ cargo test -p cad-diagnostics
running 20 tests ... test result: ok. 20 passed; 0 failed
running 10 tests ... test result: ok. 10 passed; 0 failed   (tests/schema_conformance.rs)

$ cargo fmt --all -- --check
(exit code 0, no output)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.73s

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.24s
(zero warnings)
```

## Regression found and fixed during this task
The schema validator's first draft applied `"required"` unconditionally to
every instance, including a `null` value under a `"type": ["object",
"null"]` field (e.g. `"source"`). This incorrectly flagged three missing
sub-fields (`file`/`start`/`end`) on every diagnostic that legitimately
omits an optional `source`/`entity`/etc. Root-caused (JSON Schema's
`required` keyword only constrains object instances) and fixed in
`schema.rs`'s `validate_node` by gating the `required` check on the
instance actually being a `Json::Object`. Added a permanent regression
test (`required_is_ignored_for_a_null_instance_under_a_nullable_object_type`)
and re-ran the full suite (all 30 tests pass).

## Known limitations / follow-up
- The schema validator's `pattern`/`$ref` gaps (see Decisions §2) are
  acceptable for this crate's own fixture schema but would need
  addressing (or a real JSON Schema library adopted) before
  `specs/schemas/diagnostic.schema.json` grows `$defs`-based sharing with
  the other schemas `specs/schemas/README.md` lists
  (`build.schema.json`, etc. — not yet created, out of this task's scope).
- No runtime enforcement of RFC-0005 §5's kernel-independence constraint
  (a diagnostic's `message`/`title`/`suggestions` must never require an
  OCCT-specific concept) exists — flagged as a convention in the README,
  not an automated check, since no reliable automated check exists for
  "requires an OCCT-specific concept to understand."
- `build.schema.json`/`package.schema.json`/`skill.schema.json`/
  `artifact.schema.json` (also listed in `specs/schemas/README.md`) remain
  unpopulated — out of AICAD-038's scope (diagnostics only); future tasks
  (`cad-cli`, `cad-packages`, etc.) will populate them when reached.
