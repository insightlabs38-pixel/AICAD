# AICAD-009 — Draft RFC-0003 semantic references

## Objective
Draft RFC-0003 (semantic references), covering the Stage-0 build item
"feature/reference semantics", per `project/TASKS.yaml` (AICAD-009). This
is the RFC for the plan's highest-risk subsystem
(`docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` Risk A) and the one whose
contract Stage 4's hard gate exists to prove.

## Dependencies checked
AICAD-008 (RFC-0002) — complete, see `project/reports/AICAD-008.md`; this
RFC depends on RFC-0002's ruling that kernel handles own no durable
identity (RFC-0002 §3) and builds the layer that does. Owner rulings DL-8
and DL-9 were recorded before this batch began.

## What was done

Wrote `rfcs/0003-semantic-references.md`:
1. Froze reference classes, the seven reference-construction strategies,
   explicit semantic exports, the query/predicate model, and the feature-
   DAG/lineage contract verbatim from `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md`
   §2-6, §8-10 — no changes, only consolidation into one frozen document.
2. Encoded DL-8/D7 (fail-closed resolution): exactly three outcomes
   (`Resolved`/`Ambiguous`/`Broken`), never an arbitrary best-candidate
   selection.
3. Added §6.1, a precise distinction the owner ruling's wording made
   necessary: `06`'s own reference-construction-strategy list (item 6) and
   durability table (`query_geometric` tier) both mention
   geometry-fingerprint-style criteria, which could be misread as
   endorsing the very automatic fallback DL-8 disables. §6.1 states
   explicitly that **author-written** geometric queries remain fully
   supported (they are not "silent" — the program asked for exactly that
   discriminator), while what is disabled is the *resolver* automatically
   inventing a fingerprint comparison the source never requested, purely
   to force a `Resolved` outcome. Without this distinction, the RFC would
   have appeared to contradict `06`'s own predicate/durability
   vocabulary.
4. Encoded DL-9/D8 (kernel-independent semantic graph, directional):
   `cad-references` is authoritative; OCAF may be prototyped only as an
   internal OCCT-side aid behind `cad-occt-bridge`, never surfaced through
   `cad-kernel-api` (RFC-0002 §3) or depended on by public/serialized
   identity; extent of internal use stays open and prototype-driven,
   pointed at `project/experiments/`.
5. Listed alternatives considered and explicitly carried forward what
   remains open: full resolution precedence ordering across all seven
   construction strategies (Stage-4 implementation work, not a Stage-0
   freeze), extent of internal OCAF usage, and the future automatic-
   fingerprint-recovery policy threshold.

No escalation condition was triggered: this RFC codifies the fail-closed
outcome contract and kernel-independent-graph directional ruling exactly
as the owner stated them; it does not permit ambiguity to select silently
(the opposite — it forecloses that), does not select between the OCAF/
custom-graph alternatives beyond what DL-9 already directionally settled,
and does not touch the Stage-4 benchmark itself (only states what that
benchmark must prove).

## Files changed
- Added: `rfcs/0003-semantic-references.md`.

## Verification (exact commands/results)
```
$ grep -c "^## " rfcs/0003-semantic-references.md
11   # confirms all 11 planned sections are present

$ cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
(both exit 0 — no Rust source touched by this task)
```

Task-specific check: re-read `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md`
in full a second time specifically to confirm §6.1's distinction (explicit
geometric query vs. automatic fingerprint fallback) does not contradict
anything else in that document — `06` §3 item 6 itself already calls
geometric fingerprint "a fallback discriminator" for *authored* queries,
which is consistent with, not contradictory to, disabling only the
resolver's own automatic use of it.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific check: cross-read against `06` for internal consistency,
  as above.

## Limitations / follow-up
- Full resolution precedence across the seven construction strategies is
  intentionally left to Stage-4 implementation (AICAD-088) informed by the
  fixture corpus (AICAD-096), not frozen here.
- D8's extent-of-internal-OCAF-usage question and the future automatic-
  fingerprint-recovery policy remain open, as intended.
