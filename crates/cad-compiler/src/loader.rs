//! Module-loader skeleton (`AICAD-044`, "module/import syntax and loader
//! skeleton").
//!
//! Scope: given an entry `.aicad` file, resolve the transitive graph of
//! `import ./relative` file imports (parsing each discovered file,
//! deduplicating diamond imports, and detecting cyclic imports), and
//! record `import package.path` imports as recognized-but-unresolved
//! external references. Resolving a package path against an actual
//! package/dependency system is explicitly out of scope — Stage 2 has no
//! package manifest/lockfile/registry yet
//! (`docs/plan/12_PACKAGES_PLUGINS_EXTENSIONS.md` is not scheduled before
//! Stage 5+ per `project/TASKS.yaml`); see `cad_ast::item::ImportPath`'s
//! own doc comment. Symbol-level resolution (does `ISO4762` actually
//! exist in the target module, is it `pub`, name-collision handling) is
//! also out of scope — that is name-binding's job (`AICAD-050`+), not
//! this skeleton's; a selective import's `names` list is carried through
//! on [`cad_ast::Item::Import`] unexamined by this module.
//!
//! Determinism (`project/DECISION_LOG.md#DL-12`, D5 Level 1): module
//! discovery order is depth-first, source order (the order `import`
//! items appear in each file, entry file first) — [`LoadResult::modules`]
//! is a plain `Vec` in that discovery order, never a hash-map iteration.
//! `index_by_path` below is a `HashMap` used only for point lookups
//! (has this path already been loaded?), never iterated to produce
//! output, so its randomized bucket order cannot leak into any canonical
//! result — the requirement DL-12 actually states ("unordered maps ...
//! must not affect canonical compiler output") is about iterating such a
//! map to *produce* output, not about using one as an internal lookup
//! cache.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cad_ast::{ImportPath, Item, Program, Span, Spanned};
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SeverityLetter, SourceSpan};

/// One successfully loaded and parsed `.aicad` file.
#[derive(Debug, Clone)]
pub struct Module {
    /// Canonicalized (`std::fs::canonicalize`) absolute path — the
    /// loader's identity key for a file-based module, so two different
    /// relative spellings of the same file (`./a` from one importer,
    /// `../dir/a` from another) are recognized as the same module rather
    /// than loaded and parsed twice.
    pub path: PathBuf,
    pub program: Program,
}

/// The result of loading an entry file's whole reachable import graph.
#[derive(Debug, Clone)]
pub struct LoadResult {
    /// Every distinct file-based module reached from the entry file,
    /// entry file first, in depth-first source order. Empty only if the
    /// entry file itself could not be read/parsed at all is not
    /// possible — the entry file, if readable, is always `modules[0]`
    /// even if it has diagnostics.
    pub modules: Vec<Module>,
    /// Lexer/parser diagnostics from every loaded module, plus this
    /// loader's own `IMPORT-*` diagnostics (unresolved file, cyclic
    /// import, unresolved package path). Order: each module's own
    /// lexer/parser diagnostics as it is first loaded, interleaved with
    /// this loader's own diagnostics at the point they are raised during
    /// the same depth-first walk — deterministic for the same input.
    pub diagnostics: Vec<Diagnostic>,
}

struct Loader {
    modules: Vec<Module>,
    index_by_path: HashMap<PathBuf, usize>,
    diagnostics: Vec<Diagnostic>,
}

/// Loads `entry_path` and its transitive relative-import graph.
///
/// If `entry_path` itself cannot be read, `modules` is empty and
/// `diagnostics` carries a single `IMPORT-E001`.
pub fn load_entry(entry_path: &Path) -> LoadResult {
    let mut loader = Loader {
        modules: Vec::new(),
        index_by_path: HashMap::new(),
        diagnostics: Vec::new(),
    };
    let mut on_stack: Vec<PathBuf> = Vec::new();
    if let Some(entry_idx) = loader.load_file(entry_path, None, &mut on_stack) {
        debug_assert_eq!(entry_idx, 0, "the entry module is always loaded first");
    }
    LoadResult {
        modules: loader.modules,
        diagnostics: loader.diagnostics,
    }
}

/// Context needed to attribute a resolution failure (unreadable file,
/// cyclic import) to the *importing* file's own `import` statement,
/// rather than to the unreachable target.
struct ImportSite<'a> {
    importer_file: &'a str,
    importer_source: &'a str,
    item_span: Span,
}

impl Loader {
    /// Reads, parses, and (if newly loaded) recurses into `path`'s own
    /// relative imports. Returns the module's index in `self.modules`,
    /// or `None` if the file could not be read/parsed at all, or a cycle
    /// was detected. `site` is `None` only for the entry file (which has
    /// no importer to attribute a failure to).
    fn load_file(
        &mut self,
        path: &Path,
        site: Option<&ImportSite<'_>>,
        on_stack: &mut Vec<PathBuf>,
    ) -> Option<usize> {
        let canonical = match std::fs::canonicalize(path) {
            Ok(p) => p,
            Err(io_err) => {
                self.report_unreadable(path, io_err.to_string(), site);
                return None;
            }
        };
        if let Some(&idx) = self.index_by_path.get(&canonical) {
            if on_stack.contains(&canonical) {
                self.report_cycle(&canonical, on_stack, site);
                return None;
            }
            // Already fully loaded via another import path (a diamond
            // import) — reuse it rather than reparsing.
            return Some(idx);
        }

        let source = match std::fs::read_to_string(&canonical) {
            Ok(s) => s,
            Err(io_err) => {
                self.report_unreadable(path, io_err.to_string(), site);
                return None;
            }
        };
        let file_name = canonical.to_string_lossy().into_owned();
        let (program, diagnostics) = cad_parser::parse_program(&source, &file_name);
        self.diagnostics.extend(diagnostics);

        let idx = self.modules.len();
        self.modules.push(Module {
            path: canonical.clone(),
            program,
        });
        self.index_by_path.insert(canonical.clone(), idx);
        on_stack.push(canonical.clone());

        // Cloned (not borrowed) out of `self.modules[idx].program` up
        // front: the loop below recurses through `self.load_file`, which
        // needs `&mut self`, so nothing here can still be borrowing from
        // `self.modules` while that runs.
        let imports = collect_imports(&self.modules[idx].program.items);
        for (item_span, import_path) in imports {
            match import_path {
                ImportPath::Relative {
                    up_levels,
                    segments,
                    ..
                } => {
                    let child = resolve_relative_path(&canonical, up_levels, &segments);
                    let child_site = ImportSite {
                        importer_file: &file_name,
                        importer_source: &source,
                        item_span,
                    };
                    self.load_file(&child, Some(&child_site), on_stack);
                }
                ImportPath::Package { segments, .. } => {
                    self.report_unresolved_package(&segments, item_span, &file_name, &source);
                }
            }
        }

        on_stack.pop();
        Some(idx)
    }

    fn report_unreadable(
        &mut self,
        path: &Path,
        io_message: String,
        site: Option<&ImportSite<'_>>,
    ) {
        let message = format!(
            "Could not read imported file '{}': {io_message}.",
            path.display()
        );
        match site {
            Some(site) => self.diagnostics.push(diagnostic(
                1,
                Severity::Error,
                "UNRESOLVED_IMPORT",
                message,
                site.importer_file,
                site.importer_source,
                site.item_span,
            )),
            None => self.diagnostics.push(diagnostic_without_source(
                1,
                Severity::Error,
                "UNRESOLVED_IMPORT",
                message,
            )),
        }
    }

    fn report_cycle(
        &mut self,
        canonical: &Path,
        on_stack: &[PathBuf],
        site: Option<&ImportSite<'_>>,
    ) {
        let cycle_start = on_stack.iter().position(|p| p == canonical).unwrap_or(0);
        let mut chain: Vec<String> = on_stack[cycle_start..]
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        chain.push(canonical.display().to_string());
        let message = format!("Cyclic import: {}.", chain.join(" -> "));
        match site {
            Some(site) => self.diagnostics.push(diagnostic(
                2,
                Severity::Error,
                "CYCLIC_IMPORT",
                message,
                site.importer_file,
                site.importer_source,
                site.item_span,
            )),
            None => self.diagnostics.push(diagnostic_without_source(
                2,
                Severity::Error,
                "CYCLIC_IMPORT",
                message,
            )),
        }
    }

    fn report_unresolved_package(
        &mut self,
        segments: &[Spanned<String>],
        item_span: Span,
        file: &str,
        source: &str,
    ) {
        let path_text = segments
            .iter()
            .map(|s| s.node.as_str())
            .collect::<Vec<_>>()
            .join(".");
        let message = format!(
            "Package import '{path_text}' is recognized but not resolved: Stage 2 has no \
             package/dependency system yet (see project/OWNER_DECISIONS.md and \
             docs/plan/12_PACKAGES_PLUGINS_EXTENSIONS.md)."
        );
        self.diagnostics.push(diagnostic(
            1,
            Severity::Info,
            "UNRESOLVED_PACKAGE_IMPORT",
            message,
            file,
            source,
            item_span,
        ));
    }
}

/// Resolves a `Relative` import path against the importing file's own
/// canonical path. `up_levels` steps out of the importer's directory
/// before appending `segments`; the final segment gets a `.aicad`
/// extension appended (`DECISION_LOG.md#DL-4`'s canonical source
/// extension) — imports never spell the extension themselves, matching
/// every plan-doc example (`import ./housing;`, not `import
/// ./housing.aicad;`).
fn resolve_relative_path(importer: &Path, up_levels: u32, segments: &[Spanned<String>]) -> PathBuf {
    let mut base = importer.parent().map(Path::to_path_buf).unwrap_or_default();
    for _ in 0..up_levels {
        base.pop();
    }
    for (i, seg) in segments.iter().enumerate() {
        if i + 1 == segments.len() {
            base.push(format!("{}.aicad", seg.node));
        } else {
            base.push(&seg.node);
        }
    }
    base
}

/// Depth-first, source-order collection of every `Item::Import` reachable
/// from `items`, including ones nested inside `Item::Part` bodies (the
/// only item-nesting form the grammar currently has) — the parser's
/// `import_decl` is one `item` alternative among others, so it can
/// syntactically appear anywhere `item` can, per `specs/language/grammar.ebnf`.
fn collect_imports(items: &[Item]) -> Vec<(Span, ImportPath)> {
    let mut out = Vec::new();
    collect_imports_into(items, &mut out);
    out
}

fn collect_imports_into(items: &[Item], out: &mut Vec<(Span, ImportPath)>) {
    for item in items {
        match item {
            Item::Import { path, span, .. } => out.push((*span, path.clone())),
            Item::Part { items, .. } => collect_imports_into(items, out),
            _ => {}
        }
    }
}

/// Every diagnostic this loader raises has an `E`-coded severity except
/// [`Loader::report_unresolved_package`]'s informational note — this maps
/// the two `Severity` values actually used here to the matching
/// [`SeverityLetter`] `DiagnosticCode::new` requires, so callers pass one
/// `Severity` instead of two arguments that must always agree.
fn severity_letter(severity: Severity) -> SeverityLetter {
    match severity {
        Severity::Error => SeverityLetter::Error,
        Severity::Warning => SeverityLetter::Warning,
        Severity::Info => SeverityLetter::Info,
    }
}

fn diagnostic(
    number: u16,
    severity: Severity,
    title: &str,
    message: String,
    file: &str,
    source: &str,
    span: Span,
) -> Diagnostic {
    let line_index = cad_ast::LineIndex::new(source);
    let start = line_index.line_column(source, span.start);
    let end = line_index.line_column(source, span.end);
    let code = DiagnosticCode::new("IMPORT", severity_letter(severity), number)
        .expect("IMPORT family and 1..=999 number are always valid");
    Diagnostic::new(code, severity, "import", title, message)
        .expect("severity_letter always agrees with the severity passed in")
        .with_source(SourceSpan {
            file: file.to_string(),
            start: Position::new(start.line, start.column),
            end: Position::new(end.line, end.column),
        })
}

/// Same as [`diagnostic`] but for the one case with no importing file to
/// attribute a span to: the entry file itself could not be read.
fn diagnostic_without_source(
    number: u16,
    severity: Severity,
    title: &str,
    message: String,
) -> Diagnostic {
    let code = DiagnosticCode::new("IMPORT", severity_letter(severity), number)
        .expect("IMPORT family and 1..=999 number are always valid");
    Diagnostic::new(code, severity, "import", title, message)
        .expect("severity_letter always agrees with the severity passed in")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A fresh, empty directory under the OS temp dir, unique per call
    /// (process id + an atomic counter, so parallel `cargo test` threads
    /// never collide) — this crate has no dev-dependency on a `tempfile`-
    /// style crate (matching every other Stage-2 crate so far's zero-
    /// third-party-dependency choice), so tests manage their own
    /// directories directly.
    fn temp_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cad-compiler-loader-test-{}-{label}-{n}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    fn write(dir: &Path, name: &str, contents: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create test file's parent dir");
        }
        std::fs::write(&path, contents).expect("write test fixture file");
        path
    }

    fn module_paths(result: &LoadResult) -> Vec<PathBuf> {
        result.modules.iter().map(|m| m.path.clone()).collect()
    }

    #[test]
    fn loads_a_single_module_with_no_imports() {
        let dir = temp_dir("single");
        let entry = write(&dir, "entry.aicad", "let x = 1;");
        let result = load_entry(&entry);
        assert_eq!(result.modules.len(), 1);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.modules[0].program.items.len(), 1);
    }

    #[test]
    fn loads_a_relative_import_chain() {
        let dir = temp_dir("chain");
        let entry = write(&dir, "entry.aicad", "import ./housing;\nlet x = 1;");
        let housing = write(&dir, "housing.aicad", "let y = 2;");
        let result = load_entry(&entry);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            module_paths(&result),
            vec![
                std::fs::canonicalize(&entry).unwrap(),
                std::fs::canonicalize(&housing).unwrap(),
            ]
        );
    }

    #[test]
    fn resolves_a_parent_directory_relative_import() {
        let dir = temp_dir("updir");
        let entry = write(&dir, "sub/entry.aicad", "import ../sibling;");
        let sibling = write(&dir, "sibling.aicad", "let s = 1;");
        let result = load_entry(&entry);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            module_paths(&result),
            vec![
                std::fs::canonicalize(&entry).unwrap(),
                std::fs::canonicalize(&sibling).unwrap(),
            ]
        );
    }

    #[test]
    fn deduplicates_a_diamond_import_instead_of_reparsing() {
        let dir = temp_dir("diamond");
        let entry = write(&dir, "entry.aicad", "import ./a;\nimport ./b;");
        write(&dir, "a.aicad", "import ./c;");
        write(&dir, "b.aicad", "import ./c;");
        write(&dir, "c.aicad", "let z = 1;");
        let result = load_entry(&entry);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        // entry -> a -> c (first discovery), then b's own import of c
        // reuses the already-loaded module rather than appending a
        // second copy or reparsing it.
        assert_eq!(result.modules.len(), 4);
        let names: Vec<String> = result
            .modules
            .iter()
            .map(|m| m.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["entry.aicad", "a.aicad", "c.aicad", "b.aicad"]);
    }

    #[test]
    fn detects_a_two_file_cyclic_import() {
        let dir = temp_dir("cycle2");
        let a = write(&dir, "a.aicad", "import ./b;");
        write(&dir, "b.aicad", "import ./a;");
        let result = load_entry(&a);
        assert_eq!(result.modules.len(), 2, "{:?}", module_paths(&result));
        assert_eq!(
            result
                .diagnostics
                .iter()
                .filter(|d| d.code.as_string() == "IMPORT-E002")
                .count(),
            1,
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn detects_a_self_import_cycle() {
        let dir = temp_dir("selfcycle");
        let a = write(&dir, "selfimport.aicad", "import ./selfimport;");
        let result = load_entry(&a);
        assert_eq!(result.modules.len(), 1);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code.as_string() == "IMPORT-E002"),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn reports_an_unreadable_relative_import_without_aborting_the_rest() {
        let dir = temp_dir("missing");
        let entry = write(&dir, "entry.aicad", "import ./missing;\nlet x = 1;");
        let result = load_entry(&entry);
        assert_eq!(result.modules.len(), 1);
        assert_eq!(result.modules[0].program.items.len(), 2);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code.as_string() == "IMPORT-E001" && d.severity == Severity::Error),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn reports_an_unreadable_entry_file_with_no_source_attribution() {
        let dir = temp_dir("missingentry");
        let missing = dir.join("does_not_exist.aicad");
        let result = load_entry(&missing);
        assert!(result.modules.is_empty());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code.as_string(), "IMPORT-E001");
        assert!(result.diagnostics[0].source.is_none());
    }

    #[test]
    fn records_a_package_import_as_unresolved_without_recursing() {
        let dir = temp_dir("package");
        let entry = write(&dir, "entry.aicad", "import std.fasteners::{ISO4762};");
        let result = load_entry(&entry);
        assert_eq!(result.modules.len(), 1);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code.as_string(), "IMPORT-I001");
        assert_eq!(result.diagnostics[0].severity, Severity::Info);
    }
}
