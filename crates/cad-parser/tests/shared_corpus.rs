//! Runs `crates/cad-parser` over the same `.aicad` fixture corpus that
//! `tree-sitter-aicad`'s own `test/corpus/` draws its cases from
//! (`tests/parser/corpus/`), per `docs/plan/02_LANGUAGE_AND_COMPILER.md`
//! §18-19's "the production parser and Tree-sitter grammar must share a
//! conformance test corpus so syntax never diverges" (`AICAD-062`).
//!
//! Every `tests/parser/corpus/positive/*.aicad` file must parse with zero
//! diagnostics; every `tests/parser/corpus/negative/*.aicad` file must
//! produce at least one.

use std::fs;
use std::path::PathBuf;

fn corpus_dir(subdir: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/parser/corpus")
        .join(subdir)
}

fn aicad_files(dir: &PathBuf) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "aicad"))
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "no .aicad fixtures found in {}",
        dir.display()
    );
    files
}

#[test]
fn positive_corpus_parses_with_no_diagnostics() {
    let dir = corpus_dir("positive");
    for path in aicad_files(&dir) {
        let source = fs::read_to_string(&path).unwrap();
        let (_, diagnostics) = cad_parser::parse_program(&source, &path.display().to_string());
        assert!(
            diagnostics.is_empty(),
            "{} was expected to parse cleanly but produced diagnostics: {diagnostics:?}",
            path.display(),
        );
    }
}

#[test]
fn negative_corpus_parses_with_at_least_one_diagnostic() {
    let dir = corpus_dir("negative");
    for path in aicad_files(&dir) {
        let source = fs::read_to_string(&path).unwrap();
        let (_, diagnostics) = cad_parser::parse_program(&source, &path.display().to_string());
        assert!(
            !diagnostics.is_empty(),
            "{} was expected to be malformed but produced no diagnostics",
            path.display(),
        );
    }
}
