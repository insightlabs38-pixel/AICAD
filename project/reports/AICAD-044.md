# AICAD-044: Implement module/import syntax and loader skeleton

## Objective

Implement `import_decl` parsing (the one `item` alternative every
Stage-2 grammar/AST task through `AICAD-043` deliberately deferred —
see `crates/cad-ast/src/item.rs`'s own module doc comment) and a
minimal module-loader skeleton that resolves an entry file's transitive
`import ./relative` graph into parsed modules, with diamond-import
dedup and cyclic-import detection. This is the first task of Batch
S2-03 (`044 -> 045 -> STAGE2-A_FRONTEND.md` checkpoint) per
`project/TASKS.yaml` and the campaign's fixed batch order.

## Base commit

`3078d09` (Batch S2-02 complete, fast-forwarded onto this session's
own working branch from `origin/branch/tender-hypatia-w0huqo`, which
itself built on `origin/main` at `cbf769a`, PR #9 — Batch S2-01).

## Files changed

- `crates/cad-ast/src/item.rs`: adds `ImportPath` (`Package`/`Relative`
  variants) and `Item::Import`; updates the module's own scope doc
  comment (import was previously listed there as "not yet in scope").
- `crates/cad-ast/src/lib.rs`: exports `ImportPath`.
- `crates/cad-parser/src/lib.rs`: adds `parse_import_path` and
  `parse_import_names`; wires `Keyword::Import` into `parse_item`'s
  dispatch; 13 new tests (`import_tests` module).
- `crates/cad-compiler/Cargo.toml`: adds `cad-ast`/`cad-diagnostics`/
  `cad-parser` path dependencies (this crate was an empty placeholder
  until now).
- `crates/cad-compiler/src/lib.rs`: replaces the placeholder doc
  comment with the crate's real one; declares `pub mod loader;`.
- `crates/cad-compiler/src/loader.rs` (new): `load_entry`, `Module`,
  `LoadResult`; 9 tests (`loader::tests` module).
- `crates/cad-diagnostics`: no code change — this task is the first to
  actually use the pre-reserved `IMPORT` diagnostic family (see
  decision 4 below).

No third-party dependency added (`Cargo.lock` unchanged in that
respect).

## Material implementation decisions

1. **Exactly two `ImportPath` forms, drawn directly from
   `docs/plan/02_LANGUAGE_AND_COMPILER.md` §10's own three worked
   examples** (`import std.fasteners::{ISO4762};`, `import
   robotics.cycloidal;`, `import ./housing;`) and nothing broader: no
   `as` aliasing, no glob `*` import — neither has any plan/RFC
   evidence anywhere. `specs/language/grammar.ebnf` itself only ever
   *references* `import_decl` in its `item` production without ever
   defining it (confirmed by reading the whole file), so this task's
   own two-form grammar is the first concrete definition, operationalizing
   the plan doc's illustrations rather than inventing new syntax.
2. **Dispatch between the two forms uses one token of lookahead** (`.`
   starts `Relative`, anything else — always an identifier — starts
   `Package`): no package-path segment can itself start with `.`, so
   this is unambiguous. `Relative` walks `Dot`/`Slash` tokens directly
   (`../../foo/bar` is `Dot Dot Slash Dot Dot Slash Ident Slash Ident`)
   rather than adding a `DotDot` lexer token — `cad-lexer` needed no
   change, keeping this task's diff smaller and that crate's existing
   token set (frozen by `AICAD-039`/`040`) untouched.
3. **Selective-import symbol list (`::{A, B}`) is a syntactic node
   only** (`Item::Import::names: Option<Vec<Spanned<String>>>`) —
   resolving names against a target module's actual exported symbols
   is name-binding's job (`AICAD-050`+), not this task's. An empty
   list (`::{}`)  is syntactically accepted (no semantic "at least one
   name" check) — this task owns syntax, not a symbol-count policy no
   plan/RFC section specifies.
4. **Loader diagnostics use the pre-existing `IMPORT` family**
   (`crates/cad-diagnostics`'s `DIAGNOSTIC_FAMILIES`, evidently reserved
   ahead of time for exactly this), distinct from the parser's own
   `PARSE` family: `PARSE-E012` is the one new *syntax*-level code
   (malformed `./`/`../` prefix); `IMPORT-E001` (file unreadable),
   `IMPORT-E002` (cyclic import), and `IMPORT-I001` (package import
   recognized but unresolved, `Severity::Info` not `Error` — see
   decision 5) are the loader's own *resolution*-level codes. Keeping
   these families separate matches the existing precedent
   (lexer/parser errors are `PARSE-*`) and lets a future consumer
   distinguish "your import statement doesn't parse" from "it parses
   but the loader couldn't resolve it."
5. **A `Package`-path import is not an error.** Stage 2 has no
   package/dependency system (`docs/plan/12_PACKAGES_PLUGINS_EXTENSIONS.md`
   is not scheduled before Stage 5+ per `project/TASKS.yaml`), so a
   program using `import std.fasteners::{ISO4762};` is not making a
   mistake — the loader records it as `IMPORT-I001` (informational)
   and does not recurse into it, rather than either (a) inventing an
   unevidenced package-resolution scheme now (explicitly out of scope,
   "No speculative future work") or (b) treating valid, plan-evidenced
   syntax as a hard compile error before the system that resolves it
   exists.
6. **Cyclic-import detection uses an explicit `on_stack: Vec<PathBuf>`
   walked with `.contains()`, not a `HashSet`.** Import graphs are
   small (dozens of files at most for the foreseeable future), so an
   O(n) membership check avoids even raising the D5/DL-12 "unordered
   collection" question for this particular check. `index_by_path`
   (a `HashMap<PathBuf, usize>`, used only for point lookups — "has
   this canonical path already been loaded" — never iterated to
   produce output) does use a hash map; the loader module's own doc
   comment records why this does not conflict with DL-12's "unordered
   maps ... must not affect canonical compiler output": that
   requirement is about a map's *iteration order* leaking into output,
   not about using one as an internal cache reached only by key.
   `LoadResult::modules` (the actual output) is a plain `Vec` built in
   deterministic depth-first source order.
7. **Diamond imports are deduplicated by canonicalized path, not
   reparsed.** `A` importing both `B` and `C`, where both `B` and `C`
   import `D`, loads `D` exactly once and every importer of `D` shares
   the same `Module` — verified by
   `deduplicates_a_diamond_import_instead_of_reparsing`.
8. **The final path segment gets `.aicad` appended by the loader, never
   spelled in source** (`import ./housing;` resolves to
   `housing.aicad`), matching every plan-doc example and
   `DECISION_LOG.md#DL-4`'s canonical source extension.
9. **A resolution failure (unreadable file, cyclic import) is
   attributed to the *importing* file's `import` statement** (an
   `ImportSite` carrying the importer's own file/source/span), not to
   the unreachable target — the target may not exist or be readable at
   all, so it has no span to report against; this also matches how a
   human would expect to be pointed at "the import statement that's
   wrong," not an unreachable file.

## Exact commands and results

```
$ cargo build -p cad-ast -p cad-parser
Finished `dev` profile [unoptimized + debuginfo] target(s)   (clean)

$ cargo test -p cad-parser import_tests
test result: ok. 13 passed; 0 failed

$ cargo build -p cad-compiler
Finished `dev` profile [unoptimized + debuginfo] target(s)   (clean)

$ cargo clippy -p cad-compiler --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s)   (zero warnings, after
restructuring `diagnostic()`/`diagnostic_without_source()` to take one `Severity`
argument instead of a separate `SeverityLetter` + `Severity` pair, fixing a
`too_many_arguments` lint — see decision-adjacent note below)

$ cargo test -p cad-compiler
running 9 tests (loader::tests)
test result: ok. 9 passed; 0 failed

$ cargo test -p cad-ast -p cad-lexer -p cad-diagnostics -p cad-parser -p cad-compiler
test result: ok. 7 passed (cad-ast)
test result: ok. 9 passed (cad-compiler, new)
test result: ok. 20 passed (cad-diagnostics lib)
test result: ok. 10 passed (cad-diagnostics schema_conformance)
test result: ok. 27 passed (cad-lexer, unchanged)
test result: ok. 88 passed (cad-parser: 75 prior + 13 new AICAD-044)
Total: 161 passed, 0 failed.

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s)   (clean, whole workspace)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s)   (zero warnings, whole workspace)

$ cargo fmt --all -- --check
(no output — clean, after running `cargo fmt --all` once to apply this task's own
formatting)
```

Native/OCCT Stage-1 suite was not re-run — no native/kernel code
touched.

## Tests / regressions

**Parser (13 new tests, `import_tests`)**: whole-module package import
(`import robotics.cycloidal;`, the plan doc's own example) and a
single-segment form; selective package import (`import
std.fasteners::{ISO4762};`, the plan doc's own example), with multiple
names plus a trailing comma, and an empty list; current-dir relative
import (`import ./housing;`, the plan doc's own example), a
multi-segment relative path, single- and double-parent-level relative
imports; adversarial: a malformed `.`-prefix (`import .foo;`, neither
`./` nor `../` — `PARSE-E012`), a missing semicolon (`PARSE-E006`,
reused), a missing segment after a package-path `.` (`PARSE-E007`,
reused); a regression guard that `for x in items { }` is unaffected by
this task's `parse_item` change (`Keyword::In`/relative-path dispatch
never interact, but this locks in for-loops stay unaffected).

**Loader (9 new tests, `loader::tests`)**: a single module with no
imports; a two-file relative-import chain; a parent-directory (`../`)
import resolved from a subdirectory; a four-file diamond import
(`entry -> {a, b} -> c`) confirmed to load `c` exactly once, in
depth-first source order (`entry, a, c, b`); a two-file cyclic import
(`a -> b -> a`, exactly one `IMPORT-E002`, both files still loaded, no
hang); a single-file self-import cycle (`a -> a`); an unreadable
relative import (missing target file: `IMPORT-E001`, the rest of the
importing file's own items still parsed and returned); an unreadable
*entry* file (empty `modules`, one `IMPORT-E001` with no `source`
attribution — there is no importer to attribute it to); a
package-path import recorded as `IMPORT-I001` (`Severity::Info`)
without any attempted recursion (there is nothing to recurse into, and
no crash from trying).

No pre-existing test (161 total after this task, all previously
passing) was modified or weakened.

**Two real bugs found and fixed during this task's own development,
both in test fixtures rather than the implementation, but investigated
to a definite root cause rather than just adjusted to pass:**

1. An early version of the self-import-cycle test named its fixture
   file `loop.aicad` and wrote `import ./loop;` — `loop` is a reserved
   keyword (`Keyword::Loop`), so the lexer never produces an
   `Ident("loop")` token and `parse_import_path`'s
   `expect_ident("a path segment")` correctly rejected it
   (`PARSE-E007`). This is exactly the same `Keyword::In`-vs-`in`-unit
   class of "a reserved word can't double as a path segment" situation
   `AICAD-040`'s report already documented for a different keyword —
   confirms the parser's existing behavior is correct, not a bug in
   this task's code. Fixed by renaming the fixture to
   `selfimport.aicad`.
2. `reports_an_unreadable_relative_import_without_aborting_the_rest`
   asserted the entry module's `program.items.len()` was `1` when the
   fixture source (`"import ./missing;\nlet x = 1;"`) has two items
   (the `Import` and the `Let`) — a wrong assertion in the test itself,
   not a loader defect (the loader's actual item count was always
   correct; only the test's expectation was wrong). Fixed the
   assertion to `2`.

## Known limitations

- No symbol-level resolution: a selective import's `names` are neither
  checked against the target module's actual items nor connected to
  anything a later phase could use for name binding yet — deliberately
  deferred to `AICAD-050`+, per this task's own scope (a syntax +
  file-graph skeleton, not name binding).
- `Package`-path imports are never resolved to an actual module —
  intentional (decision 5); a later Stage-5+ package-system task will
  need to replace `report_unresolved_package`'s informational note
  with real resolution once that system exists, not before.
- The loader has no notion of a project root / `aicad.toml` manifest
  yet (no such manifest task exists before Stage 2 completes) — every
  relative import resolves purely against the *importing file's own*
  directory, which is sufficient for this skeleton and consistent with
  every plan-doc example, but a future package-system task may need to
  add root-relative resolution alongside it, not replacing it.
- `IMPORT-E001`'s message embeds the raw OS `io::Error::to_string()`
  text (e.g. "No such file or directory (os error 2)"), which is
  platform-dependent wording. This is consistent with D5/DL-12's own
  scope (Level 1 determinism governs canonical *compiler* output —
  parsed structure, diagnostics' codes/severity/source-span shape —
  not the literal wording of an OS error message embedded in a
  message string), but is noted here in case a later determinism
  checkpoint wants to revisit it.
- This is the first task of Batch S2-03. Per the campaign's fixed
  batch order, `AICAD-045` (minimal formatter/AST pretty-printer) is
  next, then the `STAGE2-A_FRONTEND.md` checkpoint; `AICAD-046` must
  not begin before that checkpoint passes.

## Unresolved questions

None requiring owner escalation. No `OWNER_DECISIONS.md` entry added.
