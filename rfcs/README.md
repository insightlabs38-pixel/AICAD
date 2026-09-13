# AICAD RFCs

RFCs record foundational architectural decisions and rationale. The Stage-0 RFC packet was owner-approved when Stage 0 passed (`project/DECISION_LOG.md#DL-10`), so the original "Draft (Stage 0)" labels are superseded.

Later owner decisions may clarify or narrow an accepted RFC without erasing its rationale. Current normative interpretation is therefore: accepted RFC baseline + later `OWNER_DECISIONS.md` / `DECISION_LOG.md` rulings + canonical `specs/` artifacts. If an older RFC paragraph says a question is open but a later owner decision resolves it, the later ruling controls.

| RFC | Title | Current status |
|---|---|---|
| [0001](0001-language-principles.md) | Language principles | Accepted Stage-0 baseline — DL-10; later clarified by DL-12/13/14/15/17/19/21 |
| [0002](0002-geometry-runtime-kernel-abstraction.md) | Geometry/runtime and kernel abstraction | Accepted Stage-0 baseline — DL-10; later clarified by DL-5/6/12/17 |
| [0003](0003-semantic-references.md) | Semantic references | Accepted Stage-0 design contract — DL-10; Stage-4 implementation not started; D7/D8 remain partially open only where explicitly stated |
| [0004](0004-units-type-system.md) | Units/type system | Accepted Stage-0 baseline — DL-10; later clarified by DL-12/13/14/17/20/21 and AICAD-075A |
| [0005](0005-diagnostics.md) | Diagnostics | Accepted Stage-0 baseline — DL-10; diagnostic stability resolved by DL-18 |

`specs/language/` is the current canonical language compatibility surface. RFCs explain why the architecture has its shape and identify still-open boundaries; they should not be used to promote illustrative future syntax into current `.aicad` grammar.

D20 is resolved by DL-21 and implemented by AICAD-076A. The earlier transition-brief wording that treated D20 as open was stale and does not reopen or supersede DL-21.

An RFC does not resolve any decision still genuinely open in `project/OWNER_DECISIONS.md`. Current examples include the future automatic fingerprint-recovery policy under D7, the exact internal OCAF extent under D8, trusted/plugin boundaries D12/D15, and the formal public-distribution licensing review under D13.
