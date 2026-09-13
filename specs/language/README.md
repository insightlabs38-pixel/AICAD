# specs/language

This directory is the canonical current AICAD language-specification surface. It describes approved language semantics that are implemented or intentionally specified as part of the current Stage-3 baseline; future planning examples do not become current syntax merely by appearing under `docs/plan/` or a roadmap audit.

Canonical artifacts:

- `grammar.ebnf` — current source grammar/syntax surface;
- `semantics.md` — core evaluation, binding, control-flow, RuntimeBuiltin, geometry/source-boundary, and determinism semantics;
- `types.md` — primitive, dimensional, generic, collection, geometry/spatial, and tolerance-type boundaries;
- `diagnostics.md` — structured diagnostic identity, schema, compatibility, and stability contract.

Authority for resolving conflicts is, in order: explicit owner decisions and `project/DECISION_LOG.md`; accepted RFC/spec semantics; implemented behavior where a specification intentionally defines the current behavior; current stage-gate evidence; then frozen planning examples. If two higher-authority sources genuinely disagree, the conflict must be recorded for owner review rather than guessed through implementation or documentation cleanup.

The current source surface is intentionally narrower than AICAD's long-term design. In particular, internal sketch/entity/constraint/profile IR does not imply a public `.aicad` `sketch { ... }` construct; reserved or planned declaration names do not imply parser support; Stage-3 named outputs/raw topology selectors do not imply Stage-4 persistent semantic topology references; and future `Set`/`Map`, interfaces/bounds, comprehensions, closures/generators, raw/unsafe geometry syntax, assemblies, verification declarations, packages, and plugins remain outside the current canonical source grammar unless separately approved and added here.

Every accepted user-visible syntax/semantic change should update the relevant canonical artifact and its positive/negative conformance coverage. `docs/plan/` remains a frozen foundation/planning corpus used for design archaeology, not an automatic source of current normative truth.
