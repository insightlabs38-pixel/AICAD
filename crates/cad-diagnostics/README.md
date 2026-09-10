# cad-diagnostics

Structured diagnostic schema/codes (`PARSE-E###`, `TYPE-E###`, `GEOM-E###`,
`REF-E###`, ...), suggestion-confidence contract
(machine_applicable/likely_fix/informational), and JSON diagnostic shape.
Diagnostic code/schema stability policy is an open owner decision — see
`project/OWNER_DECISIONS.md` D10; every code/field here is provisional
until that decision is ruled on.

Implemented (AICAD-038): `Diagnostic`/`DiagnosticCode`/`Severity`/
`SourceSpan`/`Suggestion` types matching RFC-0005 §3-4 exactly; a
dependency-free `json` module (canonical serializer + parser); a `schema`
module implementing a deliberately narrow JSON-Schema subset (`type`,
`required`, `properties`, `items`, `enum` — no `$ref`, no `pattern`
evaluation) used by `tests/schema_conformance.rs` to validate `Diagnostic`
output against the canonical `specs/schemas/diagnostic.schema.json`
artifact. No third-party crate dependency was added — see
`project/reports/AICAD-038.md` for why.

A diagnostic's `message`/`title`/`suggestions` must never require an
OCCT-specific concept to understand (RFC-0005 §5); this crate does not
enforce that at runtime (no reliable automated check exists), so it
remains a convention for every diagnostic-raising call site to honor.

Plan references: `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §10-13;
`rfcs/0005-diagnostics.md`; task AICAD-038.
